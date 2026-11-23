use core::ops::Range;
use rayon::iter::{
    IntoParallelIterator, IntoParallelRefMutIterator, ParallelBridge, ParallelIterator,
};
use ultraviolet::{Vec3, Vec3x4};
use wide::f32x4;

use super::{integrators::Integrator, ray, wider, world};
use crate::utils::samplers::AnySampler;

type RayBuf = Vec<([u32; 4], Vec3x4, ray::Rayx4)>;

pub(crate) struct Screen {
    frame: Box<[Vec3]>,

    raysone: RayBuf,
    raystwo: RayBuf,
}

pub(crate) struct ScreenBasis {
    /// Camera position
    pub(super) eyepos: Vec3x4,
    /// Forward direction (into screen)
    pub(super) tofwd: Vec3x4,
    /// Top direction (to screen top)
    pub(super) totop: Vec3x4,
    /// Right direction (to screen right)
    pub(super) torgh: Vec3x4,
}

impl ScreenBasis {
    pub fn new_normalize(eye: Vec3, lookat: Vec3, up: Vec3, fovy: f32, ar: f32) -> Self {
        let halfheight = (fovy / 2.).tan();

        let tofwd = (lookat - eye).normalized();
        let up = tofwd.cross(up).cross(tofwd).normalized();
        let totop = halfheight * up;
        let torgh = halfheight * ar * tofwd.cross(up);

        Self {
            eyepos: Vec3x4::splat(eye),
            tofwd: Vec3x4::splat(tofwd),
            totop: Vec3x4::splat(totop),
            torgh: Vec3x4::splat(torgh),
        }
    }
}

#[allow(private_bounds)]
impl Screen {
    /// Arguments:
    /// - frame: persistent frame
    /// - focusdst: focal plane distance, scene units
    /// - lensrad: lens radius, ??? units
    /// - res: (width, height, sqrt(samples))
    /// - hslice: range of heights
    /// - seed: sampler pcg32 seed
    pub fn create<I: Integrator>(
        integrator: &I,
        frame: Box<[Vec3]>,
        basis: &ScreenBasis,
        res: (u32, u32, u32),
        hslice: Range<u32>,
    ) -> Self {
        debug_assert!(hslice.end <= res.1);
        debug_assert!((res.0 * hslice.len() as u32 * res.2 * res.2) % 4 == 0);
        debug_assert_eq!(frame.len() as u32, res.0 * res.1);
        #[cfg(feature = "gaussian_mix")]
        let weights = wider::gaussian_weights(res.2 as _);
        #[cfg(not(feature = "gaussian_mix"))]
        let weights = Vec3x4::broadcast(f32x4::splat(((res.2 * res.2) as f32).recip()));
        Self {
            frame,
            raysone: wider::Chunku32x4::from(hslice.flat_map(|j| {
                (0..res.0).flat_map(move |i| {
                    (0..res.2).flat_map(move |nj| (0..res.2).map(move |ni| [i, j, ni, nj]))
                })
            }))
            .par_bridge()
            .map(|[i, j, ni, nj]| {
                (
                    (j * res.0 + i).into(),
                    #[cfg(feature = "gaussian_mix")]
                    Vec3x4::broadcast(n.as_array_ref().map(|n| weights[n as usize]).into()),
                    #[cfg(not(feature = "gaussian_mix"))]
                    weights,
                    integrator.screenray(basis, res, i, j, ni, nj),
                )
            })
            .collect(),
            raystwo: Vec::new(),
        }
    }

    pub fn process<I: Integrator>(&mut self, integrator: &I, world: &world::World, max_depth: u16) {
        #[cfg(any(
            feature = "aovraydir",
            feature = "aovdepth",
            feature = "aovnormal",
            feature = "aovalbedo"
        ))]
        {
            self._aov(world);
            return;
        }
        for depth in (0..=max_depth).step_by(2) {
            Self::_pass(
                integrator,
                &mut self.frame,
                &mut self.raysone,
                &mut self.raystwo,
                world,
                depth,
                depth == max_depth,
            );
            if self.raystwo.is_empty() || depth == max_depth {
                break;
            }
            Self::_pass(
                integrator,
                &mut self.frame,
                &mut self.raystwo,
                &mut self.raysone,
                world,
                depth + 1,
                depth + 1 == max_depth,
            );
            if self.raysone.is_empty() {
                break;
            }
        }
    }

    pub fn write(self) -> Box<[Vec3]> {
        self.frame
    }

    fn _pass<I: Integrator>(
        integrator: &I,
        frame: &mut Box<[Vec3]>,
        src: &mut RayBuf,
        dst: &mut RayBuf,
        world: &world::World,
        depth: u16,
        last_pass: bool,
    ) {
        debug_assert!(!src.is_empty() && dst.is_empty());

        // Process Intersections
        src.par_iter_mut()
            .for_each(|(_, _, rays)| world.intersect(rays, depth));

        // Run Integrator
        let dst_est = src.len() * 2;
        let outputs = core::mem::take(src)
            .into_par_iter()
            .map(|(idxs, wghs, rays)| integrator.integrate(&world, idxs, wghs, rays, depth))
            .collect_vec_list();
        dst.reserve(dst_est.saturating_sub(dst.len()));

        // Write Colors, Collect Children
        let mut childaccum = ChildAccum::new();
        for output in outputs {
            for ((idxs, contribs), children) in output {
                for i in 0..4 {
                    frame[idxs[i] as usize] += contribs[i];
                }
                if last_pass {
                    continue;
                }
                for (idx, wgh, pos, dir, samp) in children {
                    if childaccum.push(idx, wgh, pos, dir, samp) {
                        let (idxs, wghs, poss, dirs, samps) = childaccum.pop();
                        dst.push((
                            idxs,
                            wghs.into(),
                            ray::Rayx4::new(poss.into(), dirs.into(), samps.into()),
                        ));
                    }
                }
            }
        }
        if let Some((idxs, wghs, poss, dirs, samps)) = childaccum.flush() {
            dst.push((
                idxs,
                wghs.into(),
                ray::Rayx4::new(poss.into(), dirs.into(), samps.into()),
            ));
        }

        // Finish Pass
        #[cfg(debug_assertions)]
        println!("screen: pass count dst {}", dst.len() * 4);
    }

    #[allow(unused_variables, unused_mut)]
    fn _aov(&mut self, world: &world::World) {
        self.raysone
            .par_iter_mut()
            .for_each(|(_, _, rays)| world.intersect(rays, 0));
        self.raysone.iter_mut().for_each(|(idxs, _, rays)| {
            let mut out = [Vec3::zero(); 4];
            #[cfg(feature = "aovraydir")]
            {
                out = (f32x4::HALF * rays.dir + Vec3x4::broadcast(f32x4::HALF)).into();
            }
            #[cfg(feature = "aovdepth")]
            {
                if let Some(hits) = rays.get() {
                    out = Vec3x4::broadcast(hits.t.cmp_gt(0.).blend(hits.t.recip(), f32x4::ZERO))
                        .into();
                }
            }
            #[cfg(feature = "aovnormal")]
            {
                if let Some(hits) = rays.get() {
                    out = (f32x4::HALF * hits.norm + Vec3x4::broadcast(f32x4::HALF)).into();
                }
            }
            #[cfg(feature = "aovalbedo")]
            {
                if let Some(hits) = rays.get() {
                    let color: [Vec3; 4] = hits.color.into();
                    let t = hits.t.as_array_ref();
                    for i in 0..4 {
                        if color[i].x.is_nan() && !t[i].is_nan() {
                            out[i] = hits.material[i].as_deref().map_or(
                                const { crate::api::RtMaterial::default_material().albedo },
                                |m| m.prop.albedo,
                            );
                        } else {
                            out[i] = color[i];
                        }
                    }
                }
            }
            for i in 0..4 {
                self.frame[idxs[i] as usize] = out[i];
            }
        });
        self.raysone.clear();
    }
}

wider::make_accum4!(ChildAccum; wgh: Vec3, pos: Vec3, dir: Vec3, samp: AnySampler);
