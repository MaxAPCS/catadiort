use ultraviolet::{Vec2, Vec3};

use super::Light;
use crate::engine::{integrators::RECAST_EPSILON, ray};

pub struct PlaneLight {
    pub intensity: Vec3,
    pub cen: Vec3,
    pub norm: Vec3,
}

impl Light for PlaneLight {
    fn rayto(&self, pos: Vec3, _: Vec2) -> ray::ShadowRay {
        let dir = self.cen - pos;
        let dst_sq = dir.mag_sq();
        let dst = dst_sq.sqrt();
        let dir = dir / dst;
        let cos = self.norm.dot(-dir);
        ray::ShadowRay::new(
            pos,
            dir,
            dst - RECAST_EPSILON / cos,
            self.intensity * cos.max(0.) / dst_sq,
        )
    }
}
