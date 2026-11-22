use ultraviolet::{Vec2, Vec3};

mod lightdirectional;
mod lightmesh;
mod lightpoint;
mod lightsphere;
mod lightspot;
mod primitivemesh;
mod primitivesphere;

pub(crate) use lightdirectional::*;
pub(crate) use lightmesh::*;
pub(crate) use lightpoint::*;
pub(crate) use lightsphere::*;
pub(crate) use lightspot::*;
pub(crate) use primitivemesh::*;
pub(crate) use primitivesphere::*;

pub(super) trait Primitive {
    fn bounds(&self) -> (Vec3, Vec3);
    fn centroid(&self) -> Vec3;

    /// Intersect `ray`. t (0, max_t)
    fn obstruct(&self, ray: &mut super::ray::LightRay, depth: u16);

    /// Intersect and modify `rays`. t [0, inf)
    fn intersect(&self, rays: &mut super::ray::Rayx4, depth: u16);
}

pub(super) trait Light {
    fn rayto(&self, pos: Vec3, uv: Vec2) -> super::ray::LightRay;
    fn cos_angular_radius(&self, p: Vec3) -> f32 {
        let _ = p;
        1.
    }
}
