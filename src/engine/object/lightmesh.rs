use std::sync::Arc;
use ultraviolet::{Mat4, Vec2, Vec3};

use super::{Light, Primitive, primitivemesh::*};
use crate::api::{RtCutoutFn, RtShadeContext};
use crate::engine::integrators::RECAST_EPSILON;
use crate::engine::{Material, ray};
use crate::utils::{hashing::ONE_MINUS_EPSILON, warp};

pub struct MeshLight {
    /// World to Object Transform
    wotrans: Mat4,
    /// Dual of Object to World Transform
    owtrans_dual: Mat4,

    /// Cutout Shader Reference
    cutout: RtCutoutFn,
    cutoutparams: Option<Arc<[u8]>>,

    /// Material Properties
    material: Arc<Material>,

    /// Objectspace Triangles
    otris: Arc<[(f32, Tri)]>,
    /// Objectspace Center
    ocen: Vec3,

    /// Objectspace Extent
    orad: f32,
    /// Objectspace Area
    oarea: f32,
    /// Worldspace stand-in normal (NaN if not flat)
    wnorm: Vec3,
}

impl Light for MeshLight {
    fn rayto(&self, rwpos: Vec3, uv: Vec2) -> ray::LightRay {
        let owtrans = self.wotrans.inversed();
        let wrad = self.orad * self.extract_avg_scale();
        // Stand-ins
        if uv.x.is_nan() {
            let wcen = owtrans.transform_point3(self.ocen);
            let warea = self.wotrans.determinant().abs() * self.oarea;
            if self.wnorm.x.is_nan() {
                let dir = wcen - rwpos;
                let dst_sq = dir.mag_sq();
                let dst = dst_sq.sqrt();
                return ray::LightRay::new(
                    rwpos,
                    dir / dst,
                    dst - wrad,
                    self.material.prop.emission * warea * (1. / 4.) / dst_sq,
                );
            } else {
                let dir = wcen - rwpos;
                let dst_sq = dir.mag_sq();
                let dst = dst_sq.sqrt();
                let dir = dir / dst;
                let cos = self.wnorm.dot(-dir);
                return ray::LightRay::new(
                    rwpos,
                    dir,
                    dst - RECAST_EPSILON / cos,
                    self.material.prop.emission * warea * cos.max(0.) / dst_sq,
                );
            }
        }
        // Total Area
        let toarea = self.otris.last().unwrap().0;
        // Sample Indexing Area
        let uoarea = uv.x.min(ONE_MINUS_EPSILON) * toarea;
        // Sampled Triangle & Index
        let idx = self.otris.partition_point(|t| t.0 < uoarea);
        let tri = &self.otris[idx];

        // Previous Triangle Cumulative Area
        let poarea = if idx == 0 { 0. } else { self.otris[idx - 1].0 };
        // Sampled Triangle Barycentric Coordinates
        let obc = warp::sample_uniform_triangle(Vec2::new(
            ((uoarea - poarea) / (tri.0 - poarea)).min(ONE_MINUS_EPSILON),
            uv.y,
        ));
        // Sampled Triangle Position
        let opos = tri.1.pos(obc);

        let ropos = self.wotrans.transform_point3(rwpos);
        let odir = opos - ropos;
        let odst = odir.mag();
        let odir = odir / odst;

        if tri.1.onorm.dot(-odir) <= 0. {
            return ray::LightRay::empty();
        }

        let mut hit = tri.1.hit(opos, odir, odst, (obc.y, obc.z));
        hit.opos = hit.wpos;
        hit.wpos = owtrans.transform_point3(hit.wpos);
        hit.wdir = owtrans.transform_vec3(hit.wdir).normalized();
        hit.wnorm = self.owtrans_dual.transform_vec3(hit.wnorm).normalized();
        hit.norm = self.owtrans_dual.transform_vec3(hit.norm).normalized();
        hit.material = Some(self.material.clone());
        if !self.cutout(&hit, u16::MAX) {
            return ray::LightRay::empty();
        }
        let (_, matr) = hit.shade(1); // todo use source depth + 1

        let wdst2 = (hit.wpos - rwpos).mag_sq();
        let owareatrans =
            self.owtrans_dual.transform_vec3(tri.1.onorm).mag() / self.wotrans.determinant().abs();
        let density_inv = toarea * owareatrans * hit.wnorm.dot(-hit.wdir) / wdst2;
        ray::LightRay::new(
            rwpos,
            hit.wdir,
            hit.t, // todo fixme this is a hack
            matr.emission * density_inv,
        )
    }

    fn cos_angular_radius(&self, wpos: Vec3) -> f32 {
        let opos = self.wotrans.transform_point3(wpos);
        let odst2 = (opos - self.ocen).mag_sq();
        let orad2 = self.orad * self.orad;
        if odst2 < orad2 {
            -1.
        } else {
            (1. - orad2 / odst2).sqrt()
        }
    }
}
impl MeshLight {
    pub fn emissive_create(triangles: &Box<[Tri]>, meshref: &MeshReference) -> Option<Self> {
        if !meshref.material.prop.emits() {
            return None;
        }

        // Objectspace first normal
        let onorm0 = triangles[0].onorm;
        let mut flat = true;

        // Objectspace area-accumulating triangles
        let mut aotris = Vec::with_capacity(triangles.len());

        // Objectspace area-accumulating centroid
        let mut aocen = Vec3::zero();

        // Objectspace area x2
        let mut odoubarea = 0.;

        for tri in triangles {
            let operp = tri.operp();
            let oarea = operp.mag();
            aocen += tri.centroid() * oarea;
            odoubarea += oarea;
            aotris.push((odoubarea, tri.clone()));

            if onorm0.dot(operp / oarea) <= 0.9998 {
                flat = false;
            }
        }

        // Objectspace centroid
        let ocen = aocen / odoubarea;

        let wotrans = meshref.owtrans.inversed();
        let owtrans_dual = wotrans.transposed();
        Some(Self {
            wotrans,
            owtrans_dual,
            cutout: meshref.cutout,
            cutoutparams: meshref.cutoutparams.clone(),
            material: meshref.material.clone(),
            otris: aotris.into(),
            ocen,
            orad: Mesh::triextent(triangles, ocen),
            oarea: odoubarea / 2.,
            wnorm: if !flat {
                Vec3::broadcast(f32::NAN)
            } else {
                owtrans_dual.transform_vec3(onorm0).normalized()
            },
        })
    }

    pub fn emissive_instance(other: Option<&Self>, meshref: &MeshReference) -> Option<Self> {
        let Some(other) = other else {
            return None;
        };
        if !meshref.material.prop.emits() {
            return None;
        }
        let wotrans = meshref.owtrans.inversed();
        let owtrans_dual = wotrans.transposed();
        Some(Self {
            wotrans,
            owtrans_dual,
            cutout: meshref.cutout,
            cutoutparams: meshref.cutoutparams.clone(),
            material: meshref.material.clone(),
            otris: other.otris.clone(),
            ocen: other.ocen,
            orad: other.orad,
            oarea: other.oarea,
            wnorm: if other.wnorm.x.is_nan() {
                Vec3::broadcast(f32::NAN)
            } else {
                owtrans_dual
                    .transform_vec3(other.otris[0].1.onorm)
                    .normalized()
            },
        })
    }

    #[inline]
    fn cutout(&self, hit: &ray::Hit, depth: u16) -> bool {
        let Some(cutout) = self.cutout else {
            return true;
        };
        let mut ctx = RtShadeContext::new(
            &self.cutoutparams,
            hit.wpos,
            hit.wdir,
            hit.wnorm,
            hit.t,
            hit.opos,
            hit.norm,
            hit.uv,
            depth,
            self.material.prop,
        );
        (unsafe { cutout(&mut ctx) } != 0)
    }

    #[inline]
    fn extract_avg_scale(&self) -> f32 {
        let inv3 = self.wotrans.inversed().truncate();
        (inv3.cols[0].mag() + inv3.cols[1].mag() + inv3.cols[2].mag()) / 3.
    }
}
