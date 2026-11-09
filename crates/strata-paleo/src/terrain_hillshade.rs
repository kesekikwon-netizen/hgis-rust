//! TPI 적분영상·경사/음영기복·Imhof 다방향 음영.
//!
//! `terrain_edt_tpi` 에서 분리했다.

use crate::grid_transform::{GridTransform, HGIS_NODATA_FLOAT};

/// C++ `HGIS_TPI_DEFAULT_RADIUS_M` (50.0m) — 기본 지형위치지수(TPI) 분석 반경 상수
pub const HGIS_TPI_DEFAULT_RADIUS_M: f64 = 50.0;

pub fn compute_tpi_integral_image(tf: &GridTransform, z_grid: &[f64], radius_m: f64) -> Vec<f64> {
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

pub fn compute_slope_and_hillshade(tf: &GridTransform, z_grid: &[f64]) -> (Vec<f64>, Vec<f64>) {
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
            let hs = alt.sin() * slp_rad.cos() + alt.cos() * slp_rad.sin() * (az - aspect).cos();
            shade[r * w + c] = hs.clamp(0.1, 1.0);
        }
    }
    (slope, shade)
}

/// `HgisPaleoTerrain.cpp` (`67a3f33`) — Imhof 4방향(225°·270°·315°·360°) 다중방위 음영기복도 및 경사도 색상 램프(35% 블렌딩) RGBA 합성
#[must_use]
pub fn compute_imhof_multidirectional_hillshade_and_slope_rgba(
    tf: &GridTransform,
    z_grid: &[f64],
) -> (Vec<f64>, Vec<u8>) {
    let w = tf.width;
    let h = tf.height;
    let n = w * h;
    let mut slope_deg = vec![0.0_f64; n];
    let mut rgba = vec![240_u8; n * 4];
    for i in 0..n {
        rgba[i * 4 + 3] = 255;
    }
    if w < 3 || h < 3 || z_grid.len() < n {
        return (slope_deg, rgba);
    }

    let cs8 = 8.0 * tf.cell_size.max(0.1);
    let alt = 45.0_f64.to_radians();
    let azimuths: [(f64, f64); 4] = [
        (225.0_f64.to_radians(), 0.15),
        (270.0_f64.to_radians(), 0.25),
        (315.0_f64.to_radians(), 0.40),
        (360.0_f64.to_radians(), 0.20),
    ];

    for r in 1..(h - 1) {
        for c in 1..(w - 1) {
            let z = |dr: isize, dc: isize| -> f64 {
                z_grid[(r as isize + dr) as usize * w + (c as isize + dc) as usize]
            };
            let idx = r * w + c;
            let mut has_nodata = false;
            for dr in -1..=1 {
                for dc in -1..=1 {
                    let zv = z(dr, dc);
                    if !zv.is_finite() || zv <= HGIS_NODATA_FLOAT + 1.0 {
                        has_nodata = true;
                        break;
                    }
                }
                if has_nodata {
                    break;
                }
            }
            if has_nodata {
                slope_deg[idx] = HGIS_NODATA_FLOAT;
                rgba[idx * 4..idx * 4 + 4].copy_from_slice(&[0, 0, 0, 0]);
                continue;
            }
            let dzdx = ((z(-1, 1) + 2.0 * z(0, 1) + z(1, 1))
                - (z(-1, -1) + 2.0 * z(0, -1) + z(1, -1)))
                / cs8;
            let dzdy = ((z(1, -1) + 2.0 * z(1, 0) + z(1, 1))
                - (z(-1, -1) + 2.0 * z(-1, 0) + z(-1, 1)))
                / cs8;
            let slp_rad = (dzdx * dzdx + dzdy * dzdy).sqrt().atan();
            let slp_deg = slp_rad.to_degrees();
            slope_deg[idx] = slp_deg;

            let aspect = dzdy.atan2(-dzdx);
            let mut hs_weighted = 0.0_f64;
            for &(az, weight) in &azimuths {
                let hs = (alt.sin() * slp_rad.cos()
                    + alt.cos() * slp_rad.sin() * (az - aspect).cos())
                .clamp(0.05, 1.0);
                hs_weighted += hs * weight;
            }
            // Imhof warm sunlight / cool indigo shadow base
            let imhof_r = (hs_weighted * 248.0 + (1.0 - hs_weighted) * 52.0).clamp(0.0, 255.0);
            let imhof_g = (hs_weighted * 244.0 + (1.0 - hs_weighted) * 64.0).clamp(0.0, 255.0);
            let imhof_b = (hs_weighted * 228.0 + (1.0 - hs_weighted) * 92.0).clamp(0.0, 255.0);

            // Slope tint ramp: 0~5° pale yellow, 5~15° warm ochre, >15° deep terracotta
            let (slp_r, slp_g, slp_b) = if slp_deg <= 5.0 {
                (245.0, 238.0, 185.0)
            } else if slp_deg <= 15.0 {
                (217.0, 142.0, 48.0)
            } else {
                (168.0, 58.0, 36.0)
            };

            let blend = 0.35;
            rgba[idx * 4] = (imhof_r * (1.0 - blend) + slp_r * blend).round() as u8;
            rgba[idx * 4 + 1] = (imhof_g * (1.0 - blend) + slp_g * blend).round() as u8;
            rgba[idx * 4 + 2] = (imhof_b * (1.0 - blend) + slp_b * blend).round() as u8;
            rgba[idx * 4 + 3] = 255;
        }
    }
    (slope_deg, rgba)
}
