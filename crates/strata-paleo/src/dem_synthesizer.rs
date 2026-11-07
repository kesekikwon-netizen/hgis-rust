use std::io;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use crate::terrain_edt_tpi::{idw_interpolate_grid, GridTransform};

/// `PaleoAnalysisPipeline.cpp` (`901d83b`) — 수치지형도 등고선(`N3L_F0010000`) 및 표고점(`N3P_F0020000`) 입력
#[derive(Debug, Clone)]
pub struct TopographicContourFeature {
    pub vertices_xy: Vec<[f64; 2]>,
    pub elevation_m: f64,
}

/// `PaleoAnalysisPipeline.cpp` (`ceb5fd2`) — 도엽별 DEM 타일 패치
#[derive(Debug, Clone)]
pub struct DemTilePatch {
    pub sheet_code: String,
    pub transform: GridTransform,
    pub elevations_m: Vec<f64>,
}

pub fn synthesize_precision_dem_from_topographic_points(
    tf: &GridTransform,
    contour_pts_3d: &[[f64; 3]],
) -> Vec<f64> {
    idw_interpolate_grid(tf, contour_pts_3d, 2.0, 0.0)
}

/// `PaleoAnalysisRunner::buildNgiiDemFromTopographic` (`901d83b`) — 수치지형도 등고선 브레이크라인과 표고점으로부터 정밀 DEM 격자 합성
#[must_use]
pub fn synthesize_dem_from_contours_and_spots(
    tf: &GridTransform,
    contours: &[TopographicContourFeature],
    spot_elevations_xyz: &[[f64; 3]],
) -> Vec<f64> {
    let sample_step_m = tf.cell_size.abs().max(1.0);
    let mut pts_3d: Vec<[f64; 3]> =
        Vec::with_capacity(spot_elevations_xyz.len() + contours.len() * 8);
    pts_3d.extend_from_slice(spot_elevations_xyz);

    for contour in contours {
        let z = contour.elevation_m;
        if !z.is_finite() {
            continue;
        }
        for (&a, &b) in contour
            .vertices_xy
            .iter()
            .zip(contour.vertices_xy.iter().skip(1))
        {
            let seg_len = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
            let steps = ((seg_len / sample_step_m).ceil() as usize).clamp(1, 64);
            for s in 0..=steps {
                let t = (s as f64) / (steps as f64);
                pts_3d.push([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, z]);
            }
        }
        if contour.vertices_xy.len() == 1 {
            let p = contour.vertices_xy[0];
            pts_3d.push([p[0], p[1], z]);
        }
    }

    idw_interpolate_grid(tf, &pts_3d, 2.0, 15.0)
}

/// C++ `NgiiContourElevation::interpolateDemFromContours` 1:1 대응:
/// 입력 등고선/표고점 유효성 검사, `std::atomic<bool>` 취소 체크, 표준 고도 범위(`-50.0..=2000.0 m`) 검증
pub fn interpolate_dem_from_contours_validated(
    tf: &GridTransform,
    contours: &[TopographicContourFeature],
    spot_elevations_xyz: &[[f64; 3]],
    cancel_requested: Option<&std::sync::atomic::AtomicBool>,
) -> Result<Vec<f64>, String> {
    use std::sync::atomic::Ordering;

    if tf.width == 0 || tf.height == 0 || tf.cell_size <= 0.0 {
        return Err("유효하지 않은 격자 해상도 또는 크기입니다".to_owned());
    }
    if contours.is_empty() && spot_elevations_xyz.is_empty() {
        return Err("등고선 및 표고점 입력이 비어 있습니다".to_owned());
    }
    for c in contours {
        if cancel_requested.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
            return Err("등고선 DEM 보간 작업이 취소되었습니다".to_owned());
        }
        if c.vertices_xy.is_empty() || !c.elevation_m.is_finite() {
            return Err("누락된 등고선 표고 속성 또는 파손된 기하가 감지되었습니다".to_owned());
        }
        if !(-50.0..=2000.0).contains(&c.elevation_m) {
            return Err(format!(
                "표준 고도 범위(-50.0~2000.0m)를 벗어난 등고선 표고값: {}",
                c.elevation_m
            ));
        }
    }
    let mut dem = synthesize_dem_from_contours_and_spots(tf, contours, spot_elevations_xyz);
    for z in &mut dem {
        *z = z.clamp(-50.0, 2000.0);
    }
    Ok(dem)
}

/// `PaleoAnalysisPipeline.cpp` (`ceb5fd2`) — 다중 도엽 DEM 병합 (겹치는 도엽 경계 평균 블렌딩 + 빈 칸 IDW 보간)
#[must_use]
pub fn merge_multi_sheet_dem_grids(
    target_tf: &GridTransform,
    patches: &[DemTilePatch],
    fallback_z: f64,
) -> Vec<f64> {
    let n = target_tf.width * target_tf.height;
    let mut accum = vec![0.0_f64; n];
    let mut weight = vec![0.0_f64; n];
    let mut sample_pts_3d: Vec<[f64; 3]> = Vec::new();

    for patch in patches {
        if patch.elevations_m.len() < patch.transform.width * patch.transform.height {
            continue;
        }
        for r in 0..patch.transform.height {
            for c in 0..patch.transform.width {
                let z = patch.elevations_m[r * patch.transform.width + c];
                if !z.is_finite() || z <= -9000.0 {
                    continue;
                }
                let [wx, wy] = patch.transform.pixel_to_world(r, c);
                sample_pts_3d.push([wx, wy, z]);
                if let Some((tr, tc)) = target_tf.world_to_cell(wx, wy) {
                    let idx = tr * target_tf.width + tc;
                    accum[idx] += z;
                    weight[idx] += 1.0;
                }
            }
        }
    }

    let mut out = vec![fallback_z; n];
    let mut has_missing = false;
    for i in 0..n {
        if weight[i] > 0.0 {
            out[i] = accum[i] / weight[i];
        } else {
            has_missing = true;
        }
    }

    if has_missing && !sample_pts_3d.is_empty() {
        let filled = idw_interpolate_grid(target_tf, &sample_pts_3d, 2.0, fallback_z);
        for i in 0..n {
            if weight[i] == 0.0 {
                out[i] = filled[i];
            }
        }
    }
    out
}

/// `PaleoLayerWriter.cpp` (`ceb5fd2`) — Windows 파일 잠금(EBUSY / Access Denied) 회피용 원자적 스테이징 기록 및 백오프 재시도
pub fn atomic_write_dem_avoiding_file_lock(
    target_path: &Path,
    payload: &[u8],
) -> io::Result<PathBuf> {
    if let Some(parent) = target_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let staging_path = target_path.with_extension(format!("tmp.{}", std::process::id()));
    std::fs::write(&staging_path, payload)?;

    let mut last_err = None;
    for attempt in 0..6 {
        let _ = std::fs::remove_file(target_path);
        match std::fs::rename(&staging_path, target_path) {
            Ok(()) => return Ok(target_path.to_path_buf()),
            Err(e) => {
                last_err = Some(e);
                thread::sleep(Duration::from_millis(25 * (attempt + 1)));
            }
        }
    }
    // If target_path is still locked by an external reader, keep a numbered unlocked file so the pipeline never aborts
    let fallback_path = target_path.with_extension("unlocked.tif");
    std::fs::rename(&staging_path, &fallback_path).map_err(|_| {
        last_err.unwrap_or_else(|| io::Error::other("DEM file lock retry exhausted"))
    })?;
    Ok(fallback_path)
}

/// C++ `HGIS_SEAM_BLEND_WIDTH_M` (20.0m) 1:1 상수 — 인접 1:5,000 수치지형도 도엽 간 경계 단차 제거 중첩 폭
pub const HGIS_SEAM_BLEND_WIDTH_M: f32 = 20.0;

/// C++ `HGIS_DEM_NODATA_VAL` (-9999.0f) 및 유효성 판정 임계값(-9990.0f)
pub const HGIS_DEM_NODATA_VAL: f32 = -9999.0;
pub const HGIS_DEM_NODATA_THRESHOLD: f32 = -9990.0;

/// C++ `KaDemSynthesizer::featherSeamOverlap` 1:1 대응:
/// 두 인접 도엽의 경계 중첩 구간(`blend_width_m`, 기본 `20.0m`)에서 거리 `dist_m`에 따른
/// 정규화 가중치 `alpha = clamp(dist_m / blend_width_m, 0.0, 1.0)`로 선형 블렌딩한다.
/// 한쪽 타일이 `NoData` (`<= -9990.0`)일 때는 오염을 방지하기 위해 유효 타일 표고값을 100% 보존한다.
#[must_use]
pub fn feather_seam_overlap_cell(val_a: f32, val_b: f32, dist_m: f32, blend_width_m: f32) -> f32 {
    let valid_a = val_a.is_finite() && val_a > HGIS_DEM_NODATA_THRESHOLD;
    let valid_b = val_b.is_finite() && val_b > HGIS_DEM_NODATA_THRESHOLD;
    match (valid_a, valid_b) {
        (false, true) => val_b,
        (true, false) => val_a,
        (false, false) => HGIS_DEM_NODATA_VAL,
        (true, true) => {
            let width = if blend_width_m.is_finite() && blend_width_m > 0.0 {
                blend_width_m
            } else {
                HGIS_SEAM_BLEND_WIDTH_M
            };
            let alpha = (dist_m / width).clamp(0.0, 1.0);
            (1.0 - alpha) * val_a + alpha * val_b
        }
    }
}

pub const HGIS_CONTOUR_INDEX_FACTOR: i64 = 5;
pub const HGIS_DEFAULT_STREAM_THRESH_CELL: i32 = 500;
pub const HGIS_D8_EAST: u8 = 1;
pub const HGIS_D8_SOUTHEAST: u8 = 2;
pub const HGIS_D8_SOUTH: u8 = 4;
pub const HGIS_D8_SOUTHWEST: u8 = 8;
pub const HGIS_D8_WEST: u8 = 16;
pub const HGIS_D8_NORTHWEST: u8 = 32;
pub const HGIS_D8_NORTH: u8 = 64;
pub const HGIS_D8_NORTHEAST: u8 = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContourType {
    Intermediate,
    Index,
}

/// C++ `KaMarchingSquaresEngine` — 등치선 모서리 선형 내분비 `t = (iso - z0) / (z1 - z0)` 산출 (`|z1 - z0| <= 1e-6` 평탄 모서리는 `0.5` 클램핑)
#[must_use]
pub fn interpolate_contour_edge_fraction(iso_elev_m: f64, z0: f64, z1: f64) -> f64 {
    let diff = z1 - z0;
    if diff.abs() <= 1e-6 || !diff.is_finite() {
        return 0.5;
    }
    ((iso_elev_m - z0) / diff).clamp(0.0, 1.0)
}

/// C++ `KaContourGenerator` — 주곡선 간격 `base_interval_m`의 5배수(`HGIS_CONTOUR_INDEX_FACTOR = 5`) 표고 여부를 판정하여 계곡선(`ContourType::Index`) 분류
#[must_use]
pub fn classify_contour_type(iso_elev_m: f64, base_interval_m: f64) -> ContourType {
    if base_interval_m <= 1e-6 || !base_interval_m.is_finite() {
        return ContourType::Intermediate;
    }
    let step = (iso_elev_m / base_interval_m).round() as i64;
    if step % HGIS_CONTOUR_INDEX_FACTOR == 0 {
        ContourType::Index
    } else {
        ContourType::Intermediate
    }
}

/// C++ `KaFlowDirectionD8::computeD8FlowDirection` — 8방위(`[E, SE, S, SW, W, NW, N, NE]`) 중 최대 하강 경사(`(z_c - z_n) / dist > 0`) 방향의 D8 비트마스크 코드 반환 (싱크/평탄면은 `0`)
#[must_use]
pub fn compute_d8_cell_flow_direction(
    center_z: f32,
    neighbors_clockwise_from_east: &[f32; 8],
    cell_size_m: f64,
) -> u8 {
    if !center_z.is_finite() || center_z <= HGIS_DEM_NODATA_THRESHOLD {
        return 0;
    }
    let cs = cell_size_m.max(1e-3);
    let diag = cs * std::f64::consts::SQRT_2;
    let codes: [u8; 8] = [
        HGIS_D8_EAST,
        HGIS_D8_SOUTHEAST,
        HGIS_D8_SOUTH,
        HGIS_D8_SOUTHWEST,
        HGIS_D8_WEST,
        HGIS_D8_NORTHWEST,
        HGIS_D8_NORTH,
        HGIS_D8_NORTHEAST,
    ];
    let mut best_slope = 0.0_f64;
    let mut best_code = 0_u8;
    for (idx, &nz) in neighbors_clockwise_from_east.iter().enumerate() {
        if !nz.is_finite() || nz <= HGIS_DEM_NODATA_THRESHOLD {
            continue;
        }
        let drop = f64::from(center_z - nz);
        if drop <= 0.0 {
            continue;
        }
        let dist = if idx % 2 == 0 { cs } else { diag };
        let slope = drop / dist;
        if slope > best_slope {
            best_slope = slope;
            best_code = codes[idx];
        }
    }
    best_code
}

/// C++ `KaPaleoStreamExtractor::extractPaleoStreamNetwork` — 흐름 누적 셀 수 `acc >= streamThresholdCells` 수계 셀 인덱스 필터링
#[must_use]
pub fn filter_stream_cells_by_threshold(
    accumulation_cells: &[i32],
    threshold_cells: i32,
) -> Vec<usize> {
    let thresh = if threshold_cells > 0 {
        threshold_cells
    } else {
        HGIS_DEFAULT_STREAM_THRESH_CELL
    };
    accumulation_cells
        .iter()
        .enumerate()
        .filter_map(|(idx, &acc)| if acc >= thresh { Some(idx) } else { None })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contour_dem_synthesis_and_multi_sheet_merge_produce_continuous_elevation_surface() {
        let tf = GridTransform {
            origin_x: 0.0,
            origin_y: 100.0,
            cell_size: 10.0,
            width: 10,
            height: 10,
        };
        let contours = vec![
            TopographicContourFeature {
                vertices_xy: vec![[0.0, 90.0], [100.0, 90.0]],
                elevation_m: 40.0,
            },
            TopographicContourFeature {
                vertices_xy: vec![[0.0, 10.0], [100.0, 10.0]],
                elevation_m: 10.0,
            },
        ];
        let spots = vec![[50.0, 50.0, 25.0]];
        let dem = synthesize_dem_from_contours_and_spots(&tf, &contours, &spots);
        assert_eq!(dem.len(), 100);
        // North row (near y=90) should be higher than south row (near y=10)
        assert!(dem[5] > dem[85]);

        // Merge two adjacent 5x10 sheet tiles
        let left_tf = GridTransform {
            origin_x: 0.0,
            origin_y: 100.0,
            cell_size: 10.0,
            width: 5,
            height: 10,
        };
        let right_tf = GridTransform {
            origin_x: 50.0,
            origin_y: 100.0,
            cell_size: 10.0,
            width: 5,
            height: 10,
        };
        let merged = merge_multi_sheet_dem_grids(
            &tf,
            &[
                DemTilePatch {
                    sheet_code: "359121".to_owned(),
                    transform: left_tf,
                    elevations_m: vec![20.0; 50],
                },
                DemTilePatch {
                    sheet_code: "359122".to_owned(),
                    transform: right_tf,
                    elevations_m: vec![30.0; 50],
                },
            ],
            15.0,
        );
        assert_eq!(merged.len(), 100);
        assert!((merged[0] - 20.0).abs() < 1e-6);
        assert!((merged[9] - 30.0).abs() < 1e-6);

        let tmp = std::env::temp_dir().join(format!("ka_dem_lock_test_{}.tif", std::process::id()));
        let written = atomic_write_dem_avoiding_file_lock(&tmp, b"GTIFF_DEM_BYTES")
            .expect("atomic DEM write should succeed");
        assert!(written.exists());
        let _ = std::fs::remove_file(written);
    }

    #[test]
    fn test_ngii_contour_interpolation_1m_resolution_and_valid_elevation_bounds() {
        use std::sync::atomic::AtomicBool;

        let tf_1m = GridTransform {
            origin_x: 219_800.0,
            origin_y: 361_620.0,
            cell_size: 1.0,
            width: 20,
            height: 20,
        };
        let contours = vec![
            TopographicContourFeature {
                vertices_xy: vec![[219_800.0, 361_618.0], [219_820.0, 361_618.0]],
                elevation_m: 35.0,
            },
            TopographicContourFeature {
                vertices_xy: vec![[219_800.0, 361_610.0], [219_820.0, 361_610.0]],
                elevation_m: 30.0,
            },
            TopographicContourFeature {
                vertices_xy: vec![[219_800.0, 361_602.0], [219_820.0, 361_602.0]],
                elevation_m: 25.0,
            },
        ];
        let spots = vec![[219_810.0, 361_614.0, 32.5]];
        let cancel = AtomicBool::new(false);
        let dem = interpolate_dem_from_contours_validated(&tf_1m, &contours, &spots, Some(&cancel))
            .expect("1.0m contour DEM interpolation must succeed");
        assert_eq!(dem.len(), 400);
        assert!((tf_1m.cell_size - 1.0).abs() < 1e-9);
        assert!(dem.iter().all(|&z| (-50.0..=2000.0).contains(&z)));
    }

    #[test]
    fn test_feather_seam_linear_midpoint_51m_and_nodata_preservation_50m() {
        // 1. TestKaDemSynthesizer::testFeatherSeamLinearMidpoint:
        // Tile A = 50.0m, Tile B = 52.0m, d = 10.0m over 20.0m blend width (alpha = 0.5) -> 51.0f
        let blended = feather_seam_overlap_cell(50.0, 52.0, 10.0, HGIS_SEAM_BLEND_WIDTH_M);
        assert!((blended - 51.0).abs() < 1e-6);

        // 2. TestKaDemSynthesizer::testFeatherSeamNoDataPreservation:
        // Tile A = 50.0m, Tile B = -9999.0m (NoData), d = 14.0m (alpha = 0.7) -> 50.0f preserved
        let preserved_a =
            feather_seam_overlap_cell(50.0, HGIS_DEM_NODATA_VAL, 14.0, HGIS_SEAM_BLEND_WIDTH_M);
        assert!((preserved_a - 50.0).abs() < 1e-6);

        let preserved_b =
            feather_seam_overlap_cell(HGIS_DEM_NODATA_VAL, 52.0, 6.0, HGIS_SEAM_BLEND_WIDTH_M);
        assert!((preserved_b - 52.0).abs() < 1e-6);

        let both_nodata = feather_seam_overlap_cell(
            HGIS_DEM_NODATA_VAL,
            HGIS_DEM_NODATA_VAL,
            10.0,
            HGIS_SEAM_BLEND_WIDTH_M,
        );
        assert!((both_nodata - HGIS_DEM_NODATA_VAL).abs() < 1e-5);

        // 3. TestKaContourGenerator & TestKaPaleoStreamExtractor
        assert!((interpolate_contour_edge_fraction(15.0, 10.0, 20.0) - 0.5).abs() < 1e-9);
        assert_eq!(classify_contour_type(8.0, 2.0), ContourType::Intermediate);
        assert_eq!(classify_contour_type(10.0, 2.0), ContourType::Index);
        assert_eq!(classify_contour_type(12.0, 2.0), ContourType::Intermediate);

        // Center 50.0m, South(idx 2) = 40.0m (slope 1.0), Southeast(idx 1) = 30.0m (slope 20 / 14.142 = 1.414) -> SE (code 2)
        let neighbors = [50.0_f32, 30.0, 40.0, 50.0, 50.0, 50.0, 50.0, 50.0];
        assert_eq!(
            compute_d8_cell_flow_direction(50.0, &neighbors, 10.0),
            HGIS_D8_SOUTHEAST
        );
        assert_eq!(filter_stream_cells_by_threshold(&[300, 600], 500), vec![1]);
    }
}
