//! Felzenszwalb 거리변환과 7종 지형 분류.
//!
//! `terrain_edt_tpi` 에서 분리했다.

use serde::{Deserialize, Serialize};

use rayon::prelude::*;

use crate::grid_transform::{GridTransform, HGIS_NODATA_FLOAT};

pub const EDT_INF: f64 = 1.0e20;

/// C++ `HGIS_MAX_EDT_DISTANCE_M` (10,000.0m) — 하천 픽셀 부재 시 최대 거리 클램프 상수 (10km)
pub const HGIS_MAX_EDT_DISTANCE_M: f64 = 10_000.0;

/// C++ `RemTpiClassifier::PaleoLandformType` — 7대 고지형 분류 및 NoData
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaleoLandformType {
    AlluvialFan,
    NaturalLevee,
    Backswamp,
    PaleoChannel,
    FluvialTerrace,
    Hillslope,
    Floodplain,
    NoData,
}

/// C++ `TerrainEdtTpi::computeFelzenszwalbEdt` — 1D 하한 포락선(Lower Envelope) O(N) 유클리드 거리 변환
/// `binary_grid`에서 `0`은 하천(거리 0.0), `1`(또는 0이 아닌 값)은 비하천(거리 INF) 셀을 의미한다.
#[must_use]
pub fn compute_felzenszwalb_edt_1d(binary_grid: &[u8], cell_size: f64) -> Vec<f64> {
    let n = binary_grid.len();
    if n == 0 || cell_size <= 0.0 || !binary_grid.contains(&0) {
        return Vec::new();
    }
    let mut f = vec![EDT_INF; n];
    for (i, &val) in binary_grid.iter().enumerate() {
        if val == 0 {
            f[i] = 0.0;
        }
    }

    let mut d = vec![0.0_f64; n];
    let mut v = vec![0_usize; n];
    let mut z = vec![0.0_f64; n + 1];
    let mut k = 0_usize;
    v[0] = 0;
    z[0] = f64::NEG_INFINITY;
    z[1] = f64::INFINITY;

    for q in 1..n {
        let fq = f[q] + (q * q) as f64;
        let mut s;
        loop {
            let vk = v[k];
            let fvk = f[vk] + (vk * vk) as f64;
            s = (fq - fvk) / (2.0 * (q as f64 - vk as f64));
            if s > z[k] {
                break;
            }
            if k == 0 {
                break;
            }
            k -= 1;
        }
        if s <= z[k] {
            v[0] = q;
            z[0] = f64::NEG_INFINITY;
            z[1] = f64::INFINITY;
        } else {
            k += 1;
            v[k] = q;
            z[k] = s;
            z[k + 1] = f64::INFINITY;
        }
    }

    k = 0;
    for (q, d_val) in d.iter_mut().enumerate() {
        while z[k + 1] < q as f64 {
            k += 1;
        }
        let dx = q as f64 - v[k] as f64;
        let dist_sq = dx * dx + f[v[k]];
        *d_val = dist_sq.sqrt() * cell_size;
    }
    d
}

/// C++ `RemTpiClassifier::classifyTo7Paleo` — REM(상대고도), TPI(지형위치지수), 경사도(도) 기반 7대 고지형 분류
#[must_use]
pub fn classify_to_7_paleo(rem: f64, tpi: f64, slope_deg: f64) -> PaleoLandformType {
    if !rem.is_finite()
        || !tpi.is_finite()
        || !slope_deg.is_finite()
        || rem <= HGIS_NODATA_FLOAT + 1.0
        || tpi <= HGIS_NODATA_FLOAT + 1.0
        || slope_deg <= HGIS_NODATA_FLOAT + 1.0
    {
        return PaleoLandformType::NoData;
    }
    if slope_deg >= 15.0 {
        PaleoLandformType::Hillslope
    } else if rem < 1.5 && tpi < -0.3 {
        PaleoLandformType::PaleoChannel
    } else if rem < 3.0 && tpi > 0.3 && slope_deg < 5.0 {
        PaleoLandformType::NaturalLevee
    } else if rem < 3.0 && tpi <= -0.1 && slope_deg < 3.0 {
        PaleoLandformType::Backswamp
    } else if rem >= 5.0 && slope_deg < 5.0 && tpi >= 0.0 {
        PaleoLandformType::FluvialTerrace
    } else if (3.0..15.0).contains(&slope_deg) && tpi > 0.1 {
        PaleoLandformType::AlluvialFan
    } else {
        PaleoLandformType::Floodplain
    }
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
        return vec![HGIS_MAX_EDT_DISTANCE_M; n];
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
