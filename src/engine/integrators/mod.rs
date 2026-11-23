mod cook;
mod whitted;

pub(crate) use cook::*;
pub(crate) use whitted::*;

pub(super) const RECAST_EPSILON: f32 = 1e-4;

pub(super) trait Integrator: Sync + Send {
    fn screenray(
        &self,
        basis: &crate::engine::screen::ScreenBasis,
        res: (u32, u32, u32),
        i: wide::u32x4,
        j: wide::u32x4,
        ni: wide::u32x4,
        nj: wide::u32x4,
    ) -> crate::engine::ray::Rayx4;

    /// Returns:
    ///  - contribs: (indexes, colors)
    ///  - children: [(index, weight, pos, dir, sampler)]
    fn integrate(
        &self,
        world: &super::world::World,
        idxs: [u32; 4],
        wghs: ultraviolet::Vec3x4,
        rays: super::ray::Rayx4,
        depth: u16,
    ) -> (
        ([u32; 4], [ultraviolet::Vec3; 4]),
        smallvec::SmallVec<
            [(
                u32,
                ultraviolet::Vec3,
                ultraviolet::Vec3,
                ultraviolet::Vec3,
                crate::utils::samplers::AnySampler,
            ); 8],
        >,
    );
}
