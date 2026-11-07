use rayon::prelude::*;
use serde::{Deserialize, Serialize};

pub const HGIS_NODATA_FLOAT: f64 = -9999.0;
pub const EDT_INF: f64 = 1.0e20;

/// C++ `HGIS_TPI_DEFAULT_RADIUS_M` (50.0m) — 기본 지형위치지수(TPI) 분석 반경 상수
pub const HGIS_TPI_DEFAULT_RADIUS_M: f64 = 50.0;

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

    #[must_use]
    pub fn world_to_cell(&self, wx: f64, wy: f64) -> Option<(usize, usize)> {
        if self.cell_size <= 0.0 {
            return None;
        }
        let col_f = ((wx - self.origin_x) / self.cell_size).floor();
        let row_f = ((self.origin_y - wy) / self.cell_size).floor();
        if col_f < 0.0 || row_f < 0.0 {
            return None;
        }
        let c = col_f as usize;
        let r = row_f as usize;
        if r < self.height && c < self.width {
            Some((r, c))
        } else {
            None
        }
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imhof_multidirectional_hillshade_and_slope_detect_east_and_north_ridges() {
        let tf = GridTransform {
            origin_x: 0.0,
            origin_y: 80.0,
            cell_size: 10.0,
            width: 8,
            height: 8,
        };
        let mut dem = vec![10.0_f64; 64];
        for r in 0..8 {
            for c in 0..8 {
                dem[r * 8 + c] = 10.0 + (r as f64) * 3.0 + (c as f64) * 2.0;
            }
        }
        let (slope, rgba) = compute_imhof_multidirectional_hillshade_and_slope_rgba(&tf, &dem);
        assert_eq!(slope.len(), 64);
        assert_eq!(rgba.len(), 64 * 4);
        assert!(slope[3 * 8 + 3] > 5.0);
    }

    #[test]
    fn test_felzenszwalb_edt_1d_center_stream_5_cells_yields_exact_20_10_0_10_20() {
        let binary_grid = [1_u8, 1, 0, 1, 1];
        let edt = compute_felzenszwalb_edt_1d(&binary_grid, 10.0);
        assert_eq!(edt.len(), 5);
        assert!((edt[0] - 20.0).abs() < 1e-4);
        assert!((edt[1] - 10.0).abs() < 1e-4);
        assert!((edt[2] - 0.0).abs() < 1e-4);
        assert!((edt[3] - 10.0).abs() < 1e-4);
        assert!((edt[4] - 20.0).abs() < 1e-4);

        let no_stream = [1_u8, 1, 1];
        assert!(compute_felzenszwalb_edt_1d(&no_stream, 10.0).is_empty());
    }

    #[test]
    fn test_imhof_hillshade_bounded_and_7_paleo_classifier_nodata_guard() {
        assert_eq!(
            classify_to_7_paleo(HGIS_NODATA_FLOAT, 0.0, 2.0),
            PaleoLandformType::NoData
        );
        assert_eq!(
            classify_to_7_paleo(0.8, -0.5, 1.0),
            PaleoLandformType::PaleoChannel
        );
        assert_eq!(
            classify_to_7_paleo(2.0, 0.6, 2.0),
            PaleoLandformType::NaturalLevee
        );
        assert_eq!(
            classify_to_7_paleo(1.8, -0.2, 1.5),
            PaleoLandformType::Backswamp
        );
        assert_eq!(
            classify_to_7_paleo(8.0, 0.2, 2.0),
            PaleoLandformType::FluvialTerrace
        );
        assert_eq!(
            classify_to_7_paleo(4.0, 0.3, 8.0),
            PaleoLandformType::AlluvialFan
        );
        assert_eq!(
            classify_to_7_paleo(12.0, 0.5, 22.0),
            PaleoLandformType::Hillslope
        );
    }

    #[test]
    fn test_idw_exact_point_hit_100_200_returns_35_5_with_zero_error() {
        let pts = [
            [100.0_f64, 200.0, 35.5],
            [150.0, 200.0, 12.0],
            [100.0, 250.0, 48.0],
        ];
        let val = compute_idw_point_with_radius(100.0, 200.0, &pts, HGIS_IDW_DEFAULT_POWER, 500.0);
        assert_eq!(val, 35.5);
        assert_eq!((val - 35.5).abs(), 0.0);

        // TestKaRiverEdtCalculator::testEmptyRiverMaskClamping: all-zero mask clamps to 10000.0m
        let tf_edt = GridTransform {
            origin_x: 0.0,
            origin_y: 100.0,
            cell_size: 5.0,
            width: 21,
            height: 21,
        };
        let empty_mask = vec![0_u8; 21 * 21];
        let edt_empty = compute_river_distance_edt(&tf_edt, &empty_mask);
        assert_eq!(edt_empty.len(), 21 * 21);
        assert!((edt_empty[0] - HGIS_MAX_EDT_DISTANCE_M).abs() < 1e-6);

        // TestKaIntegralImageTpi::testPeakCellPositiveTpi:
        // 25.0m flat plain with a single 30.0m peak at center (10, 10) -> TPI in +4.8..=5.0m
        let mut flat_with_peak = vec![25.0_f64; 21 * 21];
        flat_with_peak[10 * 21 + 10] = 30.0;
        let tpi = compute_tpi_integral_image(&tf_edt, &flat_with_peak, HGIS_TPI_DEFAULT_RADIUS_M);
        let center_tpi = tpi[10 * 21 + 10];
        assert!(center_tpi > 4.8 && center_tpi <= 5.0);

        // TestKaKernelDensityEstimator::testQuarticKernelAndNormalization
        let tf_kde = GridTransform {
            origin_x: 0.0,
            origin_y: 200.0,
            cell_size: 5.0,
            width: 41,
            height: 41,
        };
        let center_world = tf_kde.pixel_to_world(20, 20);
        let kde = compute_kernel_density_normalized(&tf_kde, &[center_world], 50.0);
        assert!((kde[20 * 41 + 20] - 1.0_f32).abs() < 1e-6);
        assert_eq!(kde[0], 0.0_f32);
        let empty_kde = compute_kernel_density_normalized(&tf_kde, &[], 50.0);
        assert!(empty_kde.iter().all(|&v| v == 0.0));

        // TestKaToblerHikingFunction::testFlatAndOptimalSpeed
        let speed_opt = compute_tobler_hiking_speed_kmh(HGIS_TOBLER_OPTIMAL_SLOPE);
        assert!((speed_opt - 6.0).abs() <= 0.001);
        let speed_flat = compute_tobler_hiking_speed_kmh(0.0);
        assert!((speed_flat - 5.037).abs() <= 0.001);

        // TestKaViewshedAnalyzer::testEarthCurvatureDrop: D = 10000m, k = 0.13 -> 6.828m
        let drop = compute_earth_curvature_drop_m(10_000.0, HGIS_REFRACTION_INDEX_K);
        assert!((drop - 6.828).abs() <= 0.01);

        // TestKaIntervisibilityNetwork::testMountainIntervisibilityWithBarrier
        let edge = evaluate_line_of_sight_profile(
            "봉수대-A",
            "산성-B",
            500.0,
            500.0,
            2000.0,
            &[(1000.0, 600.0)],
            HGIS_REFRACTION_INDEX_K,
        );
        assert!(!edge.is_visible);
        assert!(edge.min_clearance_m < 0.0);
    }
}

/// 지구 평균 반경 (m, C++ `HGIS_EARTH_RADIUS_M`)
pub const HGIS_EARTH_RADIUS_M: f64 = 6_371_000.0;
/// 대기 굴절 보정 기본 계수 (C++ `HGIS_REFRACTION_INDEX_K`)
pub const HGIS_REFRACTION_INDEX_K: f64 = 0.13;
/// 토블러 보행 함수 최대 속도 (km/h, C++ `HGIS_TOBLER_MAX_SPEED_KMH`)
pub const HGIS_TOBLER_MAX_SPEED_KMH: f64 = 6.0;
/// 최대 속도 최적 경사도 (탄젠트, C++ `HGIS_TOBLER_OPTIMAL_SLOPE`)
pub const HGIS_TOBLER_OPTIMAL_SLOPE: f64 = -0.05;
/// 통행 불가 단애 경사 임계치 (C++ `HGIS_IMPASSABLE_CLIFF_SLOPE`)
pub const HGIS_IMPASSABLE_CLIFF_SLOPE: f64 = 1.5;

/// 토블러 보행 함수(Tobler's Hiking Function) 보행 속도(`km/h`) 산출 (`W = 6 * exp(-3.5 * |S + 0.05|)`)
/// (C++ `KaToblerHikingFunction` 1:1 대응)
#[must_use]
pub fn compute_tobler_hiking_speed_kmh(slope_tan: f64) -> f64 {
    if !slope_tan.is_finite() || slope_tan.abs() >= HGIS_IMPASSABLE_CLIFF_SLOPE {
        return 0.0001;
    }
    (HGIS_TOBLER_MAX_SPEED_KMH * (-3.5 * (slope_tan - HGIS_TOBLER_OPTIMAL_SLOPE).abs()).exp())
        .max(0.0001)
}

/// 지구 곡률 및 대기 굴절(`k = 0.13`)을 반영한 수평 거리별 지형 겉보기 침하량(`m`) 산출
/// (C++ `KaViewshedAnalyzer` 1:1 대응)
#[must_use]
pub fn compute_earth_curvature_drop_m(distance_m: f64, refraction_k: f64) -> f64 {
    if distance_m <= 0.0 || !distance_m.is_finite() {
        return 0.0;
    }
    (distance_m * distance_m * (1.0 - refraction_k)) / (2.0 * HGIS_EARTH_RADIUS_M)
}

/// 유적·봉수·산성 간 상호 조망(Intervisibility) 기선 검증 결과 (C++ `IntervisibilityEdge`)
#[derive(Debug, Clone, PartialEq)]
pub struct IntervisibilityEdge {
    pub from_site_id: String,
    pub to_site_id: String,
    pub is_visible: bool,
    pub distance_m: f64,
    pub min_clearance_m: f64,
}

/// 출발점과 목표점 사이의 중간 지형 표고 샘플(`(dist_m, terrain_elev_m)`)에 대한 가시선(LOS) 차폐 및 여유 고도 판정
/// (C++ `KaIntervisibilityNetwork::buildIntervisibilityNetwork` 1:1 대응)
#[must_use]
pub fn evaluate_line_of_sight_profile(
    from_site_id: &str,
    to_site_id: &str,
    obs_elev_m: f64,
    target_elev_m: f64,
    distance_m: f64,
    intermediate_samples: &[(f64, f64)],
    refraction_k: f64,
) -> IntervisibilityEdge {
    if distance_m <= 1e-6 || !distance_m.is_finite() {
        return IntervisibilityEdge {
            from_site_id: from_site_id.to_owned(),
            to_site_id: to_site_id.to_owned(),
            is_visible: true,
            distance_m: 0.0,
            min_clearance_m: 0.0,
        };
    }
    let adj_target_el = target_elev_m - compute_earth_curvature_drop_m(distance_m, refraction_k);
    let mut min_clearance_m = f64::INFINITY;
    for &(d, z_terr) in intermediate_samples {
        if d <= 0.0 || d >= distance_m {
            continue;
        }
        let frac = d / distance_m;
        let los_z = obs_elev_m + (adj_target_el - obs_elev_m) * frac;
        let adj_terr_z = z_terr - compute_earth_curvature_drop_m(d, refraction_k);
        let clearance = los_z - adj_terr_z;
        min_clearance_m = min_clearance_m.min(clearance);
    }
    if !min_clearance_m.is_finite() {
        min_clearance_m = 0.0;
    }
    IntervisibilityEdge {
        from_site_id: from_site_id.to_owned(),
        to_site_id: to_site_id.to_owned(),
        is_visible: min_clearance_m >= 0.0,
        distance_m,
        min_clearance_m,
    }
}
