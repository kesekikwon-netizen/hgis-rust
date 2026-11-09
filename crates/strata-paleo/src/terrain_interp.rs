//! 커널 밀도(KDE)·역거리가중(IDW) 보간.
//!
//! `terrain_edt_tpi` 에서 분리했다.

use crate::grid_transform::HGIS_NODATA_FLOAT;

use rayon::prelude::*;

use crate::grid_transform::GridTransform;

pub const HGIS_IDW_DEFAULT_POWER: f64 = 2.0;

pub const HGIS_IDW_DIST_EPSILON: f64 = 1.0e-6;

pub const HGIS_DEFAULT_KDE_BANDWIDTH_M: f64 = 50.0;

/// C++ `KaKernelDensityEstimator` — Quartic(Biweight) 커널 가중치 `K(u) = (15/16) * (1 - u^2)^2` (`u = d / h < 1`)
#[must_use]
pub fn compute_quartic_kernel_weight(dist_m: f64, bandwidth_m: f64) -> f64 {
    let h = if bandwidth_m > 0.0 && bandwidth_m.is_finite() {
        bandwidth_m
    } else {
        HGIS_DEFAULT_KDE_BANDWIDTH_M
    };
    if dist_m < 0.0 || dist_m >= h || !dist_m.is_finite() {
        return 0.0;
    }
    let u = dist_m / h;
    let one_minus_u2 = 1.0 - u * u;
    (15.0 / 16.0) * one_minus_u2 * one_minus_u2
}

/// C++ `KaKernelDensityEstimator::computeKernelDensity` — 행 단위 병렬 Quartic 커널 누적 및 `[0.0, 1.0]` 정규화 (빈 데이터셋 0.0f 조기 반환)
#[must_use]
pub fn compute_kernel_density_normalized(
    tf: &GridTransform,
    artifact_points: &[[f64; 2]],
    bandwidth_m: f64,
) -> Vec<f32> {
    let n = tf.width * tf.height;
    if artifact_points.is_empty() || n == 0 {
        return vec![0.0_f32; n];
    }
    let h = if bandwidth_m > 0.0 && bandwidth_m.is_finite() {
        bandwidth_m
    } else {
        HGIS_DEFAULT_KDE_BANDWIDTH_M
    };
    let raw: Vec<f64> = (0..n)
        .into_par_iter()
        .map(|idx| {
            let r = idx / tf.width;
            let c = idx % tf.width;
            let [wx, wy] = tf.pixel_to_world(r, c);
            let mut sum = 0.0;
            for &[px, py] in artifact_points {
                let d = (wx - px).hypot(wy - py);
                if d < h {
                    sum += compute_quartic_kernel_weight(d, h);
                }
            }
            sum
        })
        .collect();
    let max_d = raw.iter().copied().fold(0.0_f64, f64::max);
    if max_d <= 0.0 {
        return vec![0.0_f32; n];
    }
    raw.into_iter().map(|v| (v / max_d) as f32).collect()
}

/// C++ `IdwInterpolator::computeIdwPoint` — 거리 0 입실론(`d < 1e-6`) 시 루프 즉시 중단 후 관측점 원본 표고 100% 보존 및 탐색 반경 제한
#[must_use]
pub fn compute_idw_point_with_radius(
    wx: f64,
    wy: f64,
    pts_xyv: &[[f64; 3]],
    power: f64,
    search_radius: f64,
) -> f64 {
    let mut sum_w = 0.0;
    let mut sum_wv = 0.0;
    let use_radius = search_radius > 0.0 && search_radius.is_finite();
    for &[px, py, val] in pts_xyv {
        let d = (wx - px).hypot(wy - py);
        if d < HGIS_IDW_DIST_EPSILON {
            return val;
        }
        if use_radius && d > search_radius {
            continue;
        }
        let w = 1.0 / d.powf(power);
        sum_w += w;
        sum_wv += w * val;
    }
    if sum_w > 0.0 {
        sum_wv / sum_w
    } else {
        HGIS_NODATA_FLOAT
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
