use rayon::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GridTransform {
    pub origin_x: f64,
    pub origin_y: f64,
    pub cell_size: f64,
    pub width: usize,
    pub height: usize,
}

impl GridTransform {
    pub const fn pixel_to_world(&self, r: usize, c: usize) -> [f64; 2] {
        [
            self.origin_x + (c as f64 + 0.5) * self.cell_size,
            self.origin_y - (r as f64 + 0.5) * self.cell_size,
        ]
    }
}

pub fn idw_interpolate_grid(
    tf: &GridTransform,
    pts_xyv: &[[f64; 3]],
    power: f64,
    default_val: f64,
) -> Vec<f64> {
    let n = tf.width * tf.height;
    if pts_xyv.is_empty() {
        return vec![default_val; n];
    }
    (0..n)
        .into_par_iter()
        .map(|idx| {
            let r = idx / tf.width;
            let c = idx % tf.width;
            let [wx, wy] = tf.pixel_to_world(r, c);
            let mut sum_w = 0.0;
            let mut sum_wv = 0.0;
            for &[px, py, val] in pts_xyv {
                let d2 = (wx - px).powi(2) + (wy - py).powi(2);
                if d2 < 1e-6 {
                    return val;
                }
                let w = 1.0 / d2.powf(power * 0.5);
                sum_w += w;
                sum_wv += w * val;
            }
            if sum_w > 0.0 {
                sum_wv / sum_w
            } else {
                default_val
            }
        })
        .collect()
}

pub fn compute_river_distance_edt(tf: &GridTransform, river_mask: &[u8]) -> Vec<f64> {
    let n = tf.width * tf.height;
    let mut river_pts = Vec::new();
    for (idx, &m) in river_mask.iter().enumerate().take(n) {
        if m != 0 {
            let r = idx / tf.width;
            let c = idx % tf.width;
            river_pts.push((r as f64, c as f64));
        }
    }
    if river_pts.is_empty() {
        return vec![1000.0; n];
    }
    (0..n)
        .into_par_iter()
        .map(|idx| {
            let r = (idx / tf.width) as f64;
            let c = (idx % tf.width) as f64;
            let min_d2 = river_pts
                .iter()
                .map(|&(rr, rc)| (r - rr).powi(2) + (c - rc).powi(2))
                .fold(f64::INFINITY, f64::min);
            min_d2.sqrt() * tf.cell_size
        })
        .collect()
}

pub fn compute_tpi_integral_image(
    tf: &GridTransform,
    z_grid: &[f64],
    radius_m: f64,
) -> Vec<f64> {
    let w = tf.width;
    let h = tf.height;
    let rad_px = ((radius_m / tf.cell_size).round() as usize).max(1);
    let mut sat = vec![0.0_f64; (w + 1) * (h + 1)];
    for r in 0..h {
        let mut row_sum = 0.0;
        for c in 0..w {
            row_sum += z_grid[r * w + c];
            sat[(r + 1) * (w + 1) + (c + 1)] = sat[r * (w + 1) + (c + 1)] + row_sum;
        }
    }
    let mut tpi = vec![0.0_f64; w * h];
    for r in 0..h {
        let r0 = r.saturating_sub(rad_px);
        let r1 = (r + rad_px + 1).min(h);
        for c in 0..w {
            let c0 = c.saturating_sub(rad_px);
            let c1 = (c + rad_px + 1).min(w);
            let count = ((r1 - r0) * (c1 - c0)) as f64;
            let sum = sat[r1 * (w + 1) + c1] + sat[r0 * (w + 1) + c0]
                - sat[r0 * (w + 1) + c1]
                - sat[r1 * (w + 1) + c0];
            let mean = sum / count.max(1.0);
            tpi[r * w + c] = z_grid[r * w + c] - mean;
        }
    }
    tpi
}

pub fn compute_slope_and_hillshade(
    tf: &GridTransform,
    z_grid: &[f64],
) -> (Vec<f64>, Vec<f64>) {
    let w = tf.width;
    let h = tf.height;
    let mut slope = vec![0.0_f64; w * h];
    let mut shade = vec![0.75_f64; w * h];
    if w < 3 || h < 3 {
        return (slope, shade);
    }
    let cs8 = 8.0 * tf.cell_size.max(0.1);
    let alt = 45.0_f64.to_radians();
    let az = 315.0_f64.to_radians();

    for r in 1..(h - 1) {
        for c in 1..(w - 1) {
            let z = |dr: isize, dc: isize| -> f64 {
                z_grid[(r as isize + dr) as usize * w + (c as isize + dc) as usize]
            };
            let dzdx = ((z(-1, 1) + 2.0 * z(0, 1) + z(1, 1))
                - (z(-1, -1) + 2.0 * z(0, -1) + z(1, -1)))
                / cs8;
            let dzdy = ((z(1, -1) + 2.0 * z(1, 0) + z(1, 1))
                - (z(-1, -1) + 2.0 * z(-1, 0) + z(-1, 1)))
                / cs8;
            let slp_rad = (dzdx * dzdx + dzdy * dzdy).sqrt().atan();
            slope[r * w + c] = slp_rad.to_degrees();
            let aspect = dzdy.atan2(-dzdx);
            let hs = alt.sin() * slp_rad.cos()
                + alt.cos() * slp_rad.sin() * (az - aspect).cos();
            shade[r * w + c] = hs.clamp(0.1, 1.0);
        }
    }
    (slope, shade)
}
