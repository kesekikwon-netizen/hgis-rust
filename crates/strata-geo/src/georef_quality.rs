//! 정합 품질 등급·잔차 요약, 서브픽셀 중심, 출력 경로, 선분 클립.
//!
//! C++ 대응은 각 항목 주석의 원본 함수명을 따른다.


/// 도면 정합 최우수 등급(Grade A) RMSE 상한 (0.10m, C++ `HGIS_GEOREF_EXCELLENT_RMSE_M`)
pub const HGIS_GEOREF_EXCELLENT_RMSE_M: f64 = 0.10;
/// 도면 정합 실무 허용 등급(Grade B) RMSE 상한 (0.25m, C++ `HGIS_GEOREF_MAX_PERMISSIBLE_M`)
pub const HGIS_GEOREF_MAX_PERMISSIBLE_M: f64 = 0.25;
/// 서브픽셀 십자선 탐색 윈도우 반경 (7px -> 15x15 윈도우, C++ `HGIS_SUBPIXEL_WINDOW_RADIUS`)
pub const HGIS_SUBPIXEL_WINDOW_RADIUS: i32 = 7;

/// 도면 정합 잔차(RMSE) 품질 등급 (C++ `GeorefQualityGrade`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeorefQualityGrade {
    GradeAExcellent,
    GradeBGood,
    GradeCCaution,
    GradeDCritical,
}

/// 서브픽셀 15x15 가중 무게중심 탐색 결과 (C++ `SubPixelWindowResult`)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SubPixelWindowResult {
    pub refined_u: f64,
    pub refined_v: f64,
    pub contrast_score: f64,
    pub is_confident: bool,
}

/// 정합 잔차(RMSE, 단위: m)로부터 A~D 4단계 정합 품질 등급 산정
/// (C++ `GeorefQuality::evaluateRmseGrade` 1:1 대응)
#[must_use]
pub fn evaluate_rmse_grade(rmse_m: f64) -> GeorefQualityGrade {
    if !rmse_m.is_finite() || !(0.0..=0.50).contains(&rmse_m) {
        GeorefQualityGrade::GradeDCritical
    } else if rmse_m <= HGIS_GEOREF_EXCELLENT_RMSE_M {
        GeorefQualityGrade::GradeAExcellent
    } else if rmse_m <= HGIS_GEOREF_MAX_PERMISSIBLE_M {
        GeorefQualityGrade::GradeBGood
    } else {
        GeorefQualityGrade::GradeCCaution
    }
}

/// 기준점 개수(`pair_count`) 및 잔차(`rmse_m`, `max_residual_m`)에 따른 한국어 정합 품질 안내 문구 생성
/// (C++ `GeorefQuality::georefQualitySummary` 1:1 대응)
#[must_use]
pub fn georef_quality_summary_ko(pair_count: usize, rmse_m: f64, max_residual_m: f64) -> String {
    match pair_count {
        0 | 1 => "지도와 그림에서 같은 자리를 최소 2곳 찍어 주세요".to_owned(),
        2 => "기준점 2개 · 한 점 더 찍으면 기울기도 맞습니다".to_owned(),
        3 => "기준점 3개 · 한 점 더 찍으면 어긋남을 잽니다".to_owned(),
        _ => format!(
            "기준점 {pair_count}개 · 평균 어긋남 {rmse_m:.2} m · 최대 {max_residual_m:.2} m"
        ),
    }
}

/// 15x15(`2 * HGIS_SUBPIXEL_WINDOW_RADIUS + 1`) 그레이스케일 패치에서 어두운 도면 십자선 교차점의 서브픽셀 무게중심 추출 (대비 부족 시 정수 클릭 좌표 폴백)
/// (C++ `GeorefPixelWindow::findSubPixelCrosshairCenter` 1:1 대응)
#[must_use]
pub fn find_subpixel_crosshair_center_15x15(
    gray_patch_15x15: &[u8],
    click_u0: i32,
    click_v0: i32,
) -> SubPixelWindowResult {
    let side = (2 * HGIS_SUBPIXEL_WINDOW_RADIUS + 1) as usize;
    if gray_patch_15x15.len() < side * side {
        return SubPixelWindowResult {
            refined_u: f64::from(click_u0),
            refined_v: f64::from(click_v0),
            contrast_score: 0.0,
            is_confident: false,
        };
    }
    let mut min_val = u8::MAX;
    let mut max_val = u8::MIN;
    for &px in &gray_patch_15x15[..side * side] {
        min_val = min_val.min(px);
        max_val = max_val.max(px);
    }
    let contrast = f64::from(max_val.saturating_sub(min_val)) / 255.0;
    if contrast < 0.15 {
        return SubPixelWindowResult {
            refined_u: f64::from(click_u0),
            refined_v: f64::from(click_v0),
            contrast_score: contrast,
            is_confident: false,
        };
    }
    let threshold = f64::from(min_val) + f64::from(max_val - min_val) * 0.35;
    let mut row_dark = vec![0.0_f64; side];
    let mut col_dark = vec![0.0_f64; side];
    for row in 0..side {
        for col in 0..side {
            let val = f64::from(gray_patch_15x15[row * side + col]);
            if val <= threshold {
                let d = threshold - val + 1.0;
                row_dark[row] += d;
                col_dark[col] += d;
            }
        }
    }
    let mut sum_w = 0.0_f64;
    let mut sum_du = 0.0_f64;
    let mut sum_dv = 0.0_f64;
    for (row, &rd) in row_dark.iter().enumerate() {
        for (col, &cd) in col_dark.iter().enumerate() {
            let val = f64::from(gray_patch_15x15[row * side + col]);
            if val <= threshold {
                let w = (threshold - val + 1.0) * rd.powi(2) * cd.powi(2);
                let du = (col as i32) - HGIS_SUBPIXEL_WINDOW_RADIUS;
                let dv = (row as i32) - HGIS_SUBPIXEL_WINDOW_RADIUS;
                sum_w += w;
                sum_du += w * f64::from(du);
                sum_dv += w * f64::from(dv);
            }
        }
    }
    if sum_w <= 1e-9 {
        return SubPixelWindowResult {
            refined_u: f64::from(click_u0),
            refined_v: f64::from(click_v0),
            contrast_score: contrast,
            is_confident: false,
        };
    }
    SubPixelWindowResult {
        refined_u: f64::from(click_u0) + (sum_du / sum_w).clamp(-7.0, 7.0),
        refined_v: f64::from(click_v0) + (sum_dv / sum_w).clamp(-7.0, 7.0),
        contrast_score: contrast,
        is_confident: true,
    }
}

/// 정합 결과 저장 시 기존 파일을 덮어쓰지 않도록 `stem_2.ext` ~ `stem_9999.ext` 고유 출력 경로 생성
/// (C++ `GeorefBackup::uniqueOutputPath` 1:1 대응)
#[must_use]
pub fn unique_georef_output_path(
    preferred_path: &std::path::Path,
    existing_paths: &std::collections::BTreeSet<std::path::PathBuf>,
) -> std::path::PathBuf {
    if !existing_paths.contains(preferred_path) && !preferred_path.exists() {
        return preferred_path.to_path_buf();
    }
    let parent = preferred_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new(""));
    let stem = preferred_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("정합결과");
    let ext = preferred_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("tif");
    for n in 2..=9999_u32 {
        let candidate = if ext.is_empty() {
            parent.join(format!("{stem}_{n}"))
        } else {
            parent.join(format!("{stem}_{n}.{ext}"))
        };
        if !existing_paths.contains(&candidate) && !candidate.exists() {
            return candidate;
        }
    }
    parent.join(format!("{stem}_9999.{ext}"))
}

/// Liang-Barsky 알고리즘을 사용해 선분(`seg_start -> seg_end`)을 조사구역 사각 경계(`bbox = [min_x, min_y, max_x, max_y]`) 내부로 정밀 절단(Clip)
/// (C++ `SurveyScopeClip::clipVectorLayerByBoundary` 선형 교차 절단 1:1 대응)
#[must_use]
pub fn clip_line_segment_by_bbox(
    seg_start: [f64; 2],
    seg_end: [f64; 2],
    bbox: [f64; 4],
) -> Option<([f64; 2], [f64; 2])> {
    let min_x = bbox[0].min(bbox[2]);
    let min_y = bbox[1].min(bbox[3]);
    let max_x = bbox[0].max(bbox[2]);
    let max_y = bbox[1].max(bbox[3]);

    let dx = seg_end[0] - seg_start[0];
    let dy = seg_end[1] - seg_start[1];
    let p = [-dx, dx, -dy, dy];
    let q = [
        seg_start[0] - min_x,
        max_x - seg_start[0],
        seg_start[1] - min_y,
        max_y - seg_start[1],
    ];

    let mut u1 = 0.0_f64;
    let mut u2 = 1.0_f64;
    for i in 0..4 {
        if p[i].abs() <= 1e-12 {
            if q[i] < 0.0 {
                return None;
            }
        } else {
            let t = q[i] / p[i];
            if p[i] < 0.0 {
                if t > u2 {
                    return None;
                }
                if t > u1 {
                    u1 = t;
                }
            } else {
                if t < u1 {
                    return None;
                }
                if t < u2 {
                    u2 = t;
                }
            }
        }
    }
    Some((
        [seg_start[0] + u1 * dx, seg_start[1] + u1 * dy],
        [seg_start[0] + u2 * dx, seg_start[1] + u2 * dy],
    ))
}
