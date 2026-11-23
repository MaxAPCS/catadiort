use core::mem::MaybeUninit;

use image::{ImageBuffer, Luma, Rgb, Rgb32FImage, imageops};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use ultraviolet::{Rotor3, Vec2, Vec3};

use crate::engine::ray;

use super::Light;

const UNIFORM_PROB: f64 = 64f64.recip();
const ONE_MINUS_EPSILON: f64 = 1. - f64::EPSILON;

pub struct EnvironmentLight {
    tex: Rgb32FImage,
    // Environment Rotation (radians about +y)
    rot: f32,

    density: Box<[f64]>,
    row_cdf: Box<[f64]>,
    col_cdf: Box<[f64]>,
}

impl Light for EnvironmentLight {
    fn rayto(&self, wpos: Vec3, uv: Vec2) -> ray::LightRay {
        // Total Luminance
        let tlum = *self.row_cdf.last().unwrap();

        if tlum <= 0. {
            return ray::LightRay::empty();
        }

        let u = (uv.x as f64).rem_euclid(1.); // wrap
        let v = (uv.y as f64).min(ONE_MINUS_EPSILON); // clamp
        let (w, h) = (self.tex.width() as f64, self.tex.height() as f64);

        let (x, y) = if u < UNIFORM_PROB {
            (v, u / UNIFORM_PROB)
        } else {
            // Row Sample Indexing Luminance
            let srlum = ((u - UNIFORM_PROB) / (1. - UNIFORM_PROB)) * tlum;
            // Row Sampled Index
            let sridx = self
                .row_cdf
                .partition_point(|&l| l <= srlum)
                .min(self.row_cdf.len() - 1);
            // Next Row Luminance
            let nrlum = self.row_cdf[sridx];
            // Previous Row Luminance
            let prlum = if sridx == 0 {
                0.
            } else {
                self.row_cdf[sridx - 1]
            };
            // Sampled Row Residual Luminance
            let srres = ((srlum - prlum) / (nrlum - prlum)).min(ONE_MINUS_EPSILON);

            let col_cdf = &self.col_cdf
                [sridx * self.tex.width() as usize..(sridx + 1) * self.tex.width() as usize];

            // Column Total Luminance
            let tclum = *col_cdf.last().unwrap();
            // Column Sample Indexing Luminance
            let sclum = v * tclum;
            // Column Sampled Index
            let scidx = col_cdf
                .partition_point(|&l| l <= sclum)
                .min(col_cdf.len() - 1);
            // Next Column Luminance
            let nclum = col_cdf[scidx];
            // Previous Column Luminance
            let pclum = if scidx == 0 { 0. } else { col_cdf[scidx - 1] };
            // Sampled Column Residual Luminance
            let scres = ((sclum - pclum) / (nclum - pclum)).min(ONE_MINUS_EPSILON);

            let x = (scidx as f64 + scres) / w;
            let y = (sridx as f64 + srres) / h;
            (x, y)
        };

        let pix = imageops::sample_bilinear(&self.tex, x as _, y as _).unwrap();
        let px = ((x * w) as usize).min(self.tex.width() as usize - 1);
        let py = ((y * h) as usize).min(self.tex.height() as usize - 1);
        let pix_density = (self.tex.width() * self.tex.height()) as f64
            * self.density[px + py * self.tex.width() as usize]
            / tlum;
        let inv_density =
            2. * core::f64::consts::PI * core::f64::consts::PI * (y * core::f64::consts::PI).sin()
                / (UNIFORM_PROB + (1. - UNIFORM_PROB) * pix_density);
        ray::LightRay::new(
            wpos,
            inv_uv((x, y)).rotated_by(Rotor3::from_rotation_xz(-self.rot)),
            f32::INFINITY,
            Vec3::from(pix.0) * inv_density as f32,
        )
    }

    fn cos_angular_radius(&self, _p: Vec3) -> f32 {
        -1.
    }
}

impl EnvironmentLight {
    pub fn new(tex: Rgb32FImage, rot: f32) -> Self {
        let pixels = (tex.width() * tex.height()) as usize;
        let mut row_cdf = Vec::with_capacity(tex.height() as _);
        let mut col_cdf = vec![MaybeUninit::uninit(); pixels].into_boxed_slice();
        let mut density = vec![MaybeUninit::uninit(); pixels].into_boxed_slice();

        let lum = ImageBuffer::<Luma<f64>, _>::from_vec(
            tex.width(),
            tex.height(),
            tex.par_pixels().map(luma).collect(),
        )
        .unwrap();
        let mut lum = ImageBuffer::from_par_fn(lum.width(), lum.height(), |x, y| {
            let mut max = 0f64;
            let x = x as i32;
            for x in x - 1..=x + 1 {
                for y in y.saturating_sub(1)..=(y + 1).min(lum.height() - 1) {
                    max = max.max(lum.get_pixel(x.rem_euclid(lum.width() as _) as _, y)[0]);
                }
            }
            // Polar Angle
            let theta = core::f64::consts::PI * (y as f64 + 0.5) / lum.height() as f64;
            Luma::from([max * theta.sin()])
        });
        let lumflr = ((lum.as_raw().par_iter().sum::<f64>() / lum.len() as f64) * 1e-6).max(1e-20);
        lum.par_pixels_mut().for_each(|p| {
            if p[0] < lumflr {
                p[0] = lumflr
            }
        });

        let mut alum = 0.0f64;
        for (_, row) in lum.enumerate_rows() {
            // Accumulating Luminance
            let mut arlum = 0.0f64;
            for (x, y, pix) in row {
                arlum += pix[0];
                let idx = x as usize + (y * tex.width()) as usize;
                density[idx].write(pix[0]);
                col_cdf[idx].write(arlum);
            }
            alum += arlum;
            row_cdf.push(alum);
        }

        Self {
            tex,
            rot,
            density: unsafe { density.assume_init() },
            row_cdf: row_cdf.into_boxed_slice(),
            col_cdf: unsafe { col_cdf.assume_init() },
        }
    }
}

#[inline]
fn luma(pix: &Rgb<f32>) -> f64 {
    0.2126 * pix[0] as f64 + 0.7152 * pix[1] as f64 + 0.0722 * pix[2] as f64
}

#[inline]
fn inv_uv((u, v): (f64, f64)) -> Vec3 {
    let (st, ct) = (v * core::f64::consts::PI).sin_cos();
    let (sp, cp) = (u * core::f64::consts::PI * 2.).sin_cos();
    Vec3::new((st * sp) as f32, ct as f32, (st * cp) as f32)
}
