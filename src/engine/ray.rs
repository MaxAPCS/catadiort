use std::sync::Arc;
use ultraviolet::{Mat4, Vec3, Vec3x4};
use wide::{CmpLe, CmpLt, f32x4};

use super::{Material, wider};
use crate::{
    api::{RtMaterial, RtShadeContext},
    utils::samplers::AnySamplerx4,
};

pub(super) struct Rayx4 {
    /// Worldspace Ray Origin
    pub pos: Vec3x4,
    /// Worldspace Ray Direction
    pub dir: Vec3x4,
    /// Hit Record (always flattened)
    pub hits: smallvec::SmallVec<[Hitx4; 1]>,

    // cookism
    pub sampler: AnySamplerx4,
}

impl Rayx4 {
    #[inline]
    pub const fn new(pos: Vec3x4, dir: Vec3x4, sampler: AnySamplerx4) -> Self {
        Self {
            pos,
            dir,
            hits: smallvec::SmallVec::new_const(),
            sampler,
        }
    }

    #[inline]
    pub fn transformed(&self, trans: &Mat4) -> Self {
        Self {
            pos: wider::transform_pointx4(trans, &self.pos),
            dir: wider::transform_vecx4(trans, self.dir),
            hits: smallvec::SmallVec::new_const(),
            sampler: self.sampler.clone(),
        }
    }

    #[inline]
    /// Get front hit mask (fast-path)
    ///
    /// If hits are pre-cutout, this doesn't help
    pub fn mask(&self, t: f32x4) -> f32x4 {
        if self.hits.len() == 1 {
            let max_t = self.hits.last().unwrap().t;
            return max_t.is_nan() | t.cmp_lt(max_t);
        }
        !t.is_nan()
    }

    #[inline]
    /// Apply hit to front (fast-path)
    ///
    /// If hits are pre-cutout, use the slow path: `.hits.push()`
    pub fn hit(&mut self, hit: Hitx4) {
        match self.hits.last_mut() {
            Some(lhit) => lhit.apply(hit),
            None => self.hits.push(hit),
        }
    }

    #[inline]
    /// Collapse and pop hit from front
    pub fn get(mut self) -> Option<Hitx4> {
        let Some(mut hit) = self.hits.pop() else {
            return None;
        };
        for nhit in self.hits {
            hit.apply(nhit);
        }
        Some(hit)
    }
}

pub(super) struct LightRay {
    /// Worldspace Ray Origin
    pub pos: Vec3,
    /// Worldspace Ray Direction
    pub dir: Vec3,
    /// Maximum Hit Time (exclusive)
    pub max_t: f32,
    /// ( N_j * L_jk * S_jk * p_jk )
    irradiance: Vec3,
    /// slow-path pre-cutout hits
    pub hits: Vec<Hit>,
}

macro_rules! material {
    ($material:expr) => {
        $material
            .as_deref()
            .unwrap_or(const { &Material::default() })
    };
}

impl LightRay {
    #[inline]
    pub const fn new(pos: Vec3, dir: Vec3, max_t: f32, irradiance: Vec3) -> Self {
        Self {
            pos,
            dir,
            max_t,
            irradiance,
            hits: Vec::new(),
        }
    }

    #[inline]
    pub const fn empty() -> Self {
        Self {
            pos: Vec3::broadcast(f32::NAN),
            dir: Vec3::broadcast(f32::NAN),
            max_t: f32::NAN,
            irradiance: Vec3::broadcast(0.),
            hits: Vec::new(),
        }
    }

    #[inline]
    pub fn transformed(&self, trans: &Mat4) -> Self {
        Self {
            pos: trans.transform_point3(self.pos),
            dir: trans.transform_vec3(self.dir),
            max_t: self.max_t,
            irradiance: self.irradiance,
            hits: Vec::new(),
        }
    }

    #[inline]
    /// Get ray obstruction (fast-path)
    ///
    /// If hits are pre-cutout, this doesn't help
    pub fn occluded(&self) -> bool {
        self.irradiance.x <= 0. && self.irradiance.y <= 0. && self.irradiance.z <= 0.
    }

    #[inline]
    /// Apply hit to irradiance (fast-path)
    ///
    /// If hits are pre-cutout, use the slow path: `.hits.push()`
    #[allow(unused_variables)]
    pub fn hit(&mut self, material: &Material) {
        #[cfg(feature = "shadow_tint")]
        {
            self.irradiance *= material.prop.albedo * material.prop.transmission;
        }
        #[cfg(not(feature = "shadow_tint"))]
        {
            self.irradiance = Vec3::zero();
        }
    }

    #[inline]
    /// Collapse and pop irradiance
    pub fn get(mut self) -> Vec3 {
        for hit in &self.hits {
            let matr = material!(hit.material).prop;
            self.irradiance *= matr.albedo * matr.transmission;
            if self.occluded() {
                break;
            }
        }
        self.irradiance
    }
}

impl core::ops::Mul<f32> for LightRay {
    type Output = Self;

    fn mul(mut self, rhs: f32) -> Self::Output {
        self.irradiance *= rhs;
        self
    }
}

pub(super) struct Hitx4 {
    /// Objectspace Hit Position
    pub opos: Vec3x4,
    /// Worldspace Hit Position
    pub wpos: Vec3x4,
    /// Worldspace Incidient Direction
    pub wdir: Vec3x4,
    /// Worldspace Surface Normal, normalized
    pub wnorm: Vec3x4,
    /// Ray Time
    pub t: f32x4,

    /// Shading Normal, normalized, flipped
    pub norm: Vec3x4,
    /// Shading UV
    pub uv: (f32x4, f32x4),
    /// Shading Color
    pub color: Vec3x4,
    /// Shading Material
    pub material: [Option<Arc<Material>>; 4],
}

impl Hitx4 {
    fn apply(&mut self, mut other: Self) {
        let me = (!self.t.is_nan()) & (other.t.is_nan() | self.t.cmp_le(other.t));

        self.opos = Vec3x4::blend(me, self.opos, other.opos);
        self.wpos = Vec3x4::blend(me, self.wpos, other.wpos);
        self.wdir = Vec3x4::blend(me, self.wdir, other.wdir);
        self.wnorm = Vec3x4::blend(me, self.wnorm, other.wnorm);
        self.t = f32x4::blend(me, self.t, other.t);

        self.norm = Vec3x4::blend(me, self.norm, other.norm);
        self.uv = (
            f32x4::blend(me, self.uv.0, other.uv.0),
            f32x4::blend(me, self.uv.1, other.uv.1),
        );
        self.color = Vec3x4::blend(me, self.color, other.color);

        let me: [u32; 4] = bytemuck::cast(me);
        for i in 0..4 {
            if me[i] == 0 {
                core::mem::swap(&mut self.material[i], &mut other.material[i]);
            }
        }
    }
}

pub(super) struct Hit {
    /// Objectspace Hit Position
    pub opos: Vec3,
    /// Worldspace Hit Position
    pub wpos: Vec3,
    /// Worldspace Incident Direction (towards wpos)
    pub wdir: Vec3,
    /// Worldspace Surface Normal (opposite wdir), normalized
    pub wnorm: Vec3,
    /// Ray Time
    pub t: f32,

    /// Shading Normal (opposite wdir), normalized
    pub norm: Vec3,
    /// Shading UV
    pub uv: (f32, f32),
    /// Shading Color
    pub color: Vec3,
    /// Shading Material
    pub material: Option<Arc<Material>>,
}

impl Into<[Hit; 4]> for Hitx4 {
    fn into(self) -> [Hit; 4] {
        let opos: [Vec3; 4] = self.opos.into();
        let wpos: [Vec3; 4] = self.wpos.into();
        let wdir: [Vec3; 4] = self.wdir.into();
        let wnorm: [Vec3; 4] = self.wnorm.into();
        let t = self.t.as_array_ref();
        let norm: [Vec3; 4] = self.norm.into();
        let u = self.uv.0.as_array_ref();
        let v = self.uv.1.as_array_ref();
        let color: [Vec3; 4] = self.color.into();

        let mut i = 0;
        self.material.map(|material| {
            let h = Hit {
                opos: opos[i],
                wpos: wpos[i],
                wdir: wdir[i],
                wnorm: wnorm[i],
                t: t[i],
                norm: norm[i],
                uv: (u[i], v[i]),
                color: color[i],
                material,
            };
            i += 1;
            h
        })
    }
}

impl From<[Hit; 4]> for Hitx4 {
    fn from(value: [Hit; 4]) -> Self {
        let opos = core::array::from_fn(|i| value[i].opos);
        let wpos = core::array::from_fn(|i| value[i].wpos);
        let wdir = core::array::from_fn(|i| value[i].wdir);
        let wnorm = core::array::from_fn(|i| value[i].wnorm);
        let t = core::array::from_fn(|i| value[i].t);
        let norm = core::array::from_fn(|i| value[i].norm);
        let u = core::array::from_fn(|i| value[i].uv.0);
        let v = core::array::from_fn(|i| value[i].uv.1);
        let color = core::array::from_fn(|i| value[i].color);

        Self {
            opos: opos.into(),
            wpos: wpos.into(),
            wdir: wdir.into(),
            wnorm: wnorm.into(),
            t: t.into(),
            norm: norm.into(),
            uv: (u.into(), v.into()),
            color: color.into(),
            material: value.map(|val| val.material),
        }
    }
}

impl Hit {
    pub fn shade(&self, depth: u16) -> (Vec3, RtMaterial) {
        let material = material!(self.material);
        let mut matr = material.prop;
        if !self.color.x.is_nan() {
            matr.albedo = self.color;
        };
        if self.t.is_nan() {
            return (self.norm, matr);
        }
        let Some(shader) = material.shader else {
            return (self.norm, matr);
        };
        let mut ctx = RtShadeContext::new(
            &material.shaderparams,
            self.wpos,
            self.wdir,
            self.wnorm,
            self.t,
            self.opos,
            self.norm,
            self.uv,
            depth,
            matr,
        );
        unsafe { shader(&mut ctx) };
        (ctx.N, ctx.m)
    }
}
