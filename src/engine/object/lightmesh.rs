use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::sync::Arc;
use ultraviolet::{Mat4, Vec2, Vec3};

use super::{Light, primitivemesh::*};
use crate::api::{RtCutoutFn, RtShadeContext};
use crate::engine::{Material, integrators::RECAST_EPSILON, ray};
use crate::utils::{hashing::ONE_MINUS_EPSILON, warp};

pub struct MeshLight {
    /// World to Object Transform
    wotrans: Mat4,
    /// Object to World Transform
    owtrans: Mat4,
    /// Object to World Dual Transform
    owtrans_dual: Mat4,
    /// Cutout Shader Reference
    cutout: RtCutoutFn,
    cutoutparams: Option<Arc<[u8]>>,
    /// Material Properties
    material: Arc<Material>,

    /// Objectspace Triangles
    pub otris: Arc<[Tri]>,
    /// Worldspace cumulative Area
    warea: Box<[f32]>,
    /// Worldspace area-weighted Centroid
    wcen: Vec3,
    /// Worldspace Extent
    wrad: f32,
    /// Worldspace Normal (NaN if not flat)
    wnorm: Vec3,
}

impl Light for MeshLight {
    fn rayto(&self, rwpos: Vec3, uv: Vec2) -> ray::LightRay {
        // Total Worldspace Area
        let twarea = *self.warea.last().unwrap();

        // Stand-ins
        if uv.x.is_nan() {
            let dir = self.wcen - rwpos;
            let dst_sq = dir.mag_sq();
            let dst = dst_sq.sqrt();
            let dir = dir / dst;
            if self.wnorm.x.is_nan() {
                // Point Light
                return ray::LightRay::new(
                    rwpos,
                    dir,
                    dst - self.wrad,
                    self.material.prop.emission * twarea * (1. / 4.) / dst_sq,
                );
            } else {
                // Plane Light
                let cos = self.wnorm.dot(-dir);
                return ray::LightRay::new(
                    rwpos,
                    dir,
                    dst - RECAST_EPSILON / cos,
                    self.material.prop.emission * twarea * cos.max(0.) / dst_sq,
                );
            }
        }

        // Sample Indexing Worldspace Area
        let swarea = (uv.x * twarea).min(twarea);
        // Sampled Index
        let sidx = self.warea.partition_point(|&c| c <= swarea);
        // Sampled Objectspace Triangle
        let sotri = &self.otris[sidx];
        // Previous Triangle Worldspace Area
        let pwarea = if sidx == 0 { 0. } else { self.warea[sidx - 1] };

        // Sampled Triangle Barycentric Coordinates
        let sbc = warp::sample_uniform_triangle(Vec2::new(
            ((swarea - pwarea) / (self.warea[sidx] - pwarea)).min(ONE_MINUS_EPSILON),
            uv.y,
        ));
        // Sampled Objectspace Position
        let sopos = sotri.opos(sbc);
        // Sampled Worldspace Position
        let swpos = self.owtrans.transform_point3(sopos);

        let wdir = swpos - rwpos;
        let wdst2 = wdir.mag_sq();
        let wdst = wdst2.sqrt();
        let wdir = wdir / wdst;
        let wnorm = self.owtrans_dual.transform_vec3(sotri.onorm).normalized();

        let cos = wnorm.dot(-wdir);
        if cos <= 0. {
            return ray::LightRay::empty();
        }

        let ropos = self.wotrans.transform_point3(rwpos);
        let odir = sopos - ropos;
        let odst = odir.mag();
        let odir = odir / odst;

        let mut hit = sotri.hit(sopos, odir, odst, (sbc.y, sbc.z));
        hit.opos = sopos;
        hit.wpos = swpos;
        hit.wdir = wdir;
        hit.wnorm = wnorm;
        hit.t = wdst;
        hit.norm = self.owtrans_dual.transform_vec3(hit.norm).normalized();
        hit.material = Some(self.material.clone());
        if !self.cutout(&hit, u16::MAX) {
            return ray::LightRay::empty();
        }
        let (_, matr) = hit.shade(1); // fixme use source depth + 1

        let density_inv = twarea * cos / wdst2;
        ray::LightRay::new(rwpos, wdir, wdst, matr.emission * density_inv)
    }

    fn cos_angular_radius(&self, wpos: Vec3) -> f32 {
        let wrad2 = self.wrad * self.wrad;
        let wdst2 = (wpos - self.wcen).mag_sq();
        if wdst2 > wrad2 {
            (1. - wrad2 / wdst2).sqrt()
        } else {
            -1.
        }
    }
}
impl MeshLight {
    pub fn from_emissive(triangles: Arc<[Tri]>, meshref: &MeshReference) -> Option<Self> {
        if !meshref.material.prop.emits() || triangles.is_empty() {
            return None;
        }

        let owtrans = meshref.owtrans;
        let wotrans = owtrans.inversed();
        let owtrans_dual = wotrans.transposed();

        // Worldspace first normal
        let wnorm0 = owtrans_dual.transform_vec3(triangles[0].onorm).normalized();
        let mut flat = true;

        // Worldspace vertices
        let mut wverts = Vec::with_capacity(triangles.len() * 3);

        // Worldspace area-accumulating triangles
        let mut warea = Vec::with_capacity(triangles.len());
        let mut awarea = 0.;

        // Worldspace area-accumulating centroid
        let mut awcen = Vec3::zero();

        for tri in triangles.iter() {
            let wv = tri.oposs().map(|v| owtrans.transform_point3(v));
            let wperp = (wv[1] - wv[0]).cross(wv[2] - wv[0]);
            let w2area = wperp.mag();

            awarea += w2area / 2.;
            warea.push(awarea);
            awcen += (wv[0] + wv[1] + wv[2]) * (w2area / 6.);
            wverts.extend_from_slice(&wv);

            if w2area > 0. && (wperp / w2area).dot(wnorm0) <= 0.9998 {
                flat = false;
            }
        }

        // Worldspace centroid & extent
        let wcen = awcen / awarea;
        let wrad = wverts
            .into_par_iter()
            .fold(
                || f32::NEG_INFINITY,
                |arad2, wv| arad2.max((wv - wcen).mag_sq()),
            )
            .reduce(|| f32::NEG_INFINITY, f32::max)
            .sqrt();

        Some(Self {
            wotrans,
            owtrans,
            owtrans_dual,
            cutout: meshref.cutout,
            cutoutparams: meshref.cutoutparams.clone(),
            material: meshref.material.clone(),

            otris: triangles,
            warea: warea.into_boxed_slice(),
            wcen,
            wrad,
            wnorm: if flat {
                wnorm0
            } else {
                Vec3::broadcast(f32::NAN)
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
}
