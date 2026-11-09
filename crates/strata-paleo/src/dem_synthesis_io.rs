//! 등고선·표고점으로부터 정밀 DEM 합성과 다중 도엽 병합·원자적 저장.
//!
//! `dem_synthesizer` 에서 분리했다.

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
