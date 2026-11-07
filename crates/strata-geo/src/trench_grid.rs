use geo::{Area, BoundingRect, Contains, Coord, LineString, Polygon, Rotate};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrenchStandardLength {
    M20,
    M15,
    M10,
}

impl TrenchStandardLength {
    pub const fn meters(self) -> f64 {
        match self {
            Self::M20 => 20.0,
            Self::M15 => 15.0,
            Self::M10 => 10.0,
        }
    }

    pub const fn label_ko(self) -> &'static str {
        match self {
            Self::M20 => "20 m",
            Self::M15 => "15 m",
            Self::M10 => "10 m",
        }
    }
}

/// `KaTrenchDialogPlan.cpp:321-324` — 곧은 트렌치 및 ㄱ·ㄴ자 4방향 트렌치(팔 15m + 15m, 폭 2m)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TrenchShapeKind {
    #[default]
    Straight,
    LShapeSw,
    LShapeNe,
    LShapeSe,
    LShapeNw,
}

impl TrenchShapeKind {
    pub const ALL: [Self; 5] = [
        Self::Straight,
        Self::LShapeSw,
        Self::LShapeNe,
        Self::LShapeSe,
        Self::LShapeNw,
    ];

    #[must_use]
    pub const fn label_ko(self) -> &'static str {
        match self {
            Self::Straight => "곧은 트렌치",
            Self::LShapeSw => "ㄴ자 트렌치 (15 m + 15 m)",
            Self::LShapeNe => "ㄱ자 트렌치 (15 m + 15 m)",
            Self::LShapeSe => "ㄴ자 좌우 반대 트렌치 (15 m + 15 m)",
            Self::LShapeNw => "ㄱ자 좌우 반대 트렌치 (15 m + 15 m)",
        }
    }

    #[must_use]
    pub const fn arm_bend(self) -> Option<u8> {
        match self {
            Self::Straight => None,
            Self::LShapeSw => Some(0),
            Self::LShapeNe => Some(1),
            Self::LShapeSe => Some(2),
            Self::LShapeNw => Some(3),
        }
    }
}

/// `TrenchLShape.cpp::squareL` — 15 m × 15 m 칸 안에서 바깥 모서리 방향(0..=3)에 따른 6각 ㄱ·ㄴ자 폴리곤 생성
/// 폭(`width`)이 암 길이(`arm`) 이상이거나 0 이하인 비정상 파라미터일 때는 빈 벡터(Null 기하)를 반환합니다.
#[must_use]
pub fn build_lshape_ring(
    u0: f64,
    v0: f64,
    arm: f64,
    width: f64,
    bend: u8,
    grid_step: Option<f64>,
) -> Vec<[f64; 2]> {
    if width <= 0.0 || arm <= 0.0 || width >= arm {
        return Vec::new();
    }
    let (u_base, v_base) = if let Some(step) = grid_step.filter(|&s| s > 0.0) {
        ((u0 / step).round() * step, (v0 / step).round() * step)
    } else {
        (u0, v0)
    };
    let east = bend == 1 || bend == 2;
    let north = bend == 1 || bend == 3;
    let cx = if east { u_base + arm } else { u_base };
    let cy = if north { v_base + arm } else { v_base };
    let sx = if east { -1.0 } else { 1.0 };
    let sy = if north { -1.0 } else { 1.0 };
    let at = |du: f64, dv: f64| -> [f64; 2] { [cx + du * sx, cy + dv * sy] };
    vec![
        at(0.0, 0.0),
        at(arm, 0.0),
        at(arm, width),
        at(width, width),
        at(width, arm),
        at(0.0, arm),
        at(0.0, 0.0),
    ]
}

/// C++ `KaTrenchDialog::alignToGridIntersections(QList<QgsFeature> &trenches, double gridInterval = 10.0) -> int`
pub fn align_trenches_to_grid_intersections(
    trenches: &mut [Vec<[f64; 2]>],
    grid_interval_m: f64,
) -> usize {
    let step = grid_interval_m.max(1.0);
    let mut moved_count = 0usize;
    for ring in trenches.iter_mut() {
        let Some(&origin) = ring.first() else {
            continue;
        };
        let snapped_x = (origin[0] / step).round() * step;
        let snapped_y = (origin[1] / step).round() * step;
        let dx = snapped_x - origin[0];
        let dy = snapped_y - origin[1];
        if dx.hypot(dy) > 1e-6 {
            for pt in ring.iter_mut() {
                pt[0] += dx;
                pt[1] += dy;
            }
            moved_count += 1;
        }
    }
    moved_count
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TrenchConfig {
    pub standard_length: TrenchStandardLength,
    pub width_m: f64,
    pub coverage_ratio: f64,
    pub angle_deg: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrenchBox {
    pub name: String,
    pub polygon: Polygon<f64>,
    pub width_m: f64,
    pub length_m: f64,
}

pub fn generate_standard_trenches(area: &Polygon<f64>, cfg: &TrenchConfig) -> Vec<TrenchBox> {
    let Some(bbox) = area.bounding_rect() else {
        return Vec::new();
    };
    let len = cfg.standard_length.meters();
    let wid = cfg.width_m.max(1.0);
    let single_area = len * wid;
    let target_area = area.unsigned_area() * cfg.coverage_ratio.clamp(0.01, 0.50);
    let max_count = ((target_area / single_area).ceil() as usize).max(1);

    let step_x = len + 6.0;
    let step_y = wid + 6.0;
    let mut out = Vec::new();
    let mut idx = 1usize;

    let mut y = bbox.min().y + 2.0;
    while y + wid <= bbox.max().y - 2.0 && out.len() < max_count {
        let mut x = bbox.min().x + 2.0;
        while x + len <= bbox.max().x - 2.0 && out.len() < max_count {
            let rect = Polygon::new(
                LineString::from(vec![
                    Coord { x, y },
                    Coord { x: x + len, y },
                    Coord {
                        x: x + len,
                        y: y + wid,
                    },
                    Coord { x, y: y + wid },
                    Coord { x, y },
                ]),
                vec![],
            );
            let rotated = if cfg.angle_deg.abs() > 1e-6 {
                rect.rotate_around_centroid(cfg.angle_deg)
            } else {
                rect
            };
            if area.contains(&rotated) {
                out.push(TrenchBox {
                    name: format!("T-{idx}"),
                    polygon: rotated,
                    width_m: wid,
                    length_m: len,
                });
                idx += 1;
            }
            x += step_x;
        }
        y += step_y;
    }
    out
}

pub const HGIS_SQM_TO_PYEONG_FACTOR: f64 = 0.3025;
pub const HGIS_PYEONG_TO_SQM_FACTOR: f64 = 3.305_785;

#[derive(Debug, Clone, PartialEq)]
pub struct TrenchStatRow {
    pub trench_no: String,
    pub area_m2: f64,
    pub area_pyeong: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SurveyAreaStats {
    pub survey_area_m2: f64,
    pub survey_area_pyeong: f64,
    pub trench_area_m2: f64,
    pub trench_area_pyeong: f64,
    pub trench_coverage_ratio_pct: f64,
    pub trench_count: usize,
    pub trench_rows: Vec<TrenchStatRow>,
}

/// C++ `KaSurveyStatisticsEngine::calculateSurveyStatistics` — 조사구역·트렌치 면적(㎡), 평수(`× 0.3025`) 및 시굴 비율(`trenchArea / surveyArea * 100.0`) 산출 (`surveyArea <= 1e-6` 시 0.0% 폴백)
#[must_use]
pub fn calculate_survey_statistics(
    survey_areas_m2: &[f64],
    trenches: &[(&str, f64)],
) -> SurveyAreaStats {
    let survey_area_m2: f64 = survey_areas_m2
        .iter()
        .copied()
        .filter(|v| *v > 0.0 && v.is_finite())
        .sum();
    let survey_area_pyeong = survey_area_m2 * HGIS_SQM_TO_PYEONG_FACTOR;
    let mut trench_area_m2 = 0.0;
    let mut trench_rows = Vec::with_capacity(trenches.len());
    for &(no, area) in trenches {
        let a = if area > 0.0 && area.is_finite() {
            area
        } else {
            0.0
        };
        trench_area_m2 += a;
        trench_rows.push(TrenchStatRow {
            trench_no: no.to_owned(),
            area_m2: a,
            area_pyeong: a * HGIS_SQM_TO_PYEONG_FACTOR,
        });
    }
    let trench_area_pyeong = trench_area_m2 * HGIS_SQM_TO_PYEONG_FACTOR;
    let trench_coverage_ratio_pct = if survey_area_m2 <= 1e-6 {
        0.0
    } else {
        (trench_area_m2 / survey_area_m2) * 100.0
    };
    SurveyAreaStats {
        survey_area_m2,
        survey_area_pyeong,
        trench_area_m2,
        trench_area_pyeong,
        trench_coverage_ratio_pct,
        trench_count: trench_rows.len(),
        trench_rows,
    }
}

/// C++ `KaAreaTableExporter::exportStatsToCsv` — UTF-8 BOM(`\xEF\xBB\xBF`) 프리픽스 포함 측량 통계 요약 CSV 바이트 생성
#[must_use]
pub fn export_survey_stats_to_csv_utf8_bom(stats: &SurveyAreaStats) -> Vec<u8> {
    let mut out = vec![0xEF_u8, 0xBB, 0xBF];
    let body = format!(
        "SurveyAreaM2,SurveyAreaPyeong,TrenchAreaM2,TrenchAreaPyeong,CoverageRatioPct,TrenchCount\n{:.2},{:.2},{:.2},{:.2},{:.2},{}\n",
        stats.survey_area_m2,
        stats.survey_area_pyeong,
        stats.trench_area_m2,
        stats.trench_area_pyeong,
        stats.trench_coverage_ratio_pct,
        stats.trench_count
    );
    out.extend_from_slice(body.as_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo::wkt;

    #[test]
    fn trench_grid_uses_only_selected_standard_length_without_shrinking() {
        let area = wkt!(POLYGON((0.0 0.0, 100.0 0.0, 100.0 80.0, 0.0 80.0, 0.0 0.0)));
        let cfg = TrenchConfig {
            standard_length: TrenchStandardLength::M20,
            width_m: 2.0,
            coverage_ratio: 0.10,
            angle_deg: 0.0,
        };
        let trenches = generate_standard_trenches(&area, &cfg);
        assert!(!trenches.is_empty());
        for t in &trenches {
            assert!((t.length_m - 20.0).abs() < 1e-6);
            assert!((t.width_m - 2.0).abs() < 1e-6);
        }
    }

    #[test]
    fn lshape_trench_all_four_bends_keep_exact_15m_arms_and_snap_to_coordinate_grid() {
        for bend in 0..=3 {
            let ring = build_lshape_ring(102.3, 204.7, 15.0, 2.0, bend, Some(5.0));
            assert_eq!(ring.len(), 7);
            let poly = Polygon::new(
                LineString::from(
                    ring.iter()
                        .map(|&p| Coord { x: p[0], y: p[1] })
                        .collect::<Vec<_>>(),
                ),
                vec![],
            );
            assert!((poly.unsigned_area() - 56.0).abs() < 1e-6);
            assert!((ring[0][0] % 5.0).abs() < 1e-6);
            assert!((ring[0][1] % 5.0).abs() < 1e-6);
        }
    }

    #[test]
    fn test_trench_lshape_origin_1000_2000_area_56_and_invalid_width_guard() {
        let ring = build_lshape_ring(1000.0, 2000.0, 15.0, 2.0, 1, None);
        assert_eq!(ring.len(), 7);
        let poly = Polygon::new(
            LineString::from(
                ring.iter()
                    .map(|&p| Coord { x: p[0], y: p[1] })
                    .collect::<Vec<_>>(),
            ),
            vec![],
        );
        assert!((poly.unsigned_area() - 56.0).abs() < 1e-4);
        assert!(build_lshape_ring(1000.0, 2000.0, 15.0, 15.0, 1, None).is_empty());

        let mut trenches = vec![vec![[1003.4, 2007.8], [1018.4, 2007.8]]];
        let moved = align_trenches_to_grid_intersections(&mut trenches, 10.0);
        assert_eq!(moved, 1);
        assert!((trenches[0][0][0] - 1000.0).abs() < 1e-6);
        assert!((trenches[0][0][1] - 2010.0).abs() < 1e-6);
    }

    #[test]
    fn test_pyeong_conversion_10000m2_to_3025_ratio_10pct_and_zero_survey_area_fallback() {
        let trenches = [
            ("T-1", 200.0),
            ("T-2", 200.0),
            ("T-3", 200.0),
            ("T-4", 200.0),
            ("T-5", 200.0),
        ];
        let stats = calculate_survey_statistics(&[10_000.0], &trenches);
        assert!((stats.survey_area_pyeong - 3025.0).abs() < 1e-9);
        assert!((stats.trench_area_pyeong - 302.5).abs() < 1e-9);
        assert!((stats.trench_coverage_ratio_pct - 10.0).abs() < 1e-9);
        assert_eq!(stats.trench_count, 5);

        let csv_bytes = export_survey_stats_to_csv_utf8_bom(&stats);
        assert!(csv_bytes.starts_with(&[0xEF, 0xBB, 0xBF]));

        let zero_stats = calculate_survey_statistics(&[0.0], &[("T-1", 100.0)]);
        assert_eq!(zero_stats.trench_coverage_ratio_pct, 0.0);
        assert!(!zero_stats.trench_coverage_ratio_pct.is_nan());
    }

    #[test]
    fn test_rectangle_geometry_generation_and_translation_step_with_shift() {
        // TestKaTrenchPlaceTool::testRectangleGeometryGeneration:
        // center (100.0, 200.0), width 2.0m, length 20.0m, rotation 0.0 deg -> area 40.0 m2, bbox [90, 199, 110, 201]
        let (ring, bbox, area) = create_trench_rectangle_polygon(
            [100.0, 200.0],
            HGIS_TRENCH_DEFAULT_WIDTH_M,
            HGIS_TRENCH_DEFAULT_LENGTH_M,
            0.0,
        );
        assert_eq!(ring.len(), 5);
        assert!((area - 40.0).abs() < 1e-6);
        assert!((bbox[0] - 90.0).abs() < 1e-6);
        assert!((bbox[1] - 199.0).abs() < 1e-6);
        assert!((bbox[2] - 110.0).abs() < 1e-6);
        assert!((bbox[3] - 201.0).abs() < 1e-6);

        // Zero/negative dimensions clamp to width >= 1.0m, length >= 5.0m -> area 5.0 m2
        let (_, _, clamped_area) = create_trench_rectangle_polygon([0.0, 0.0], 0.0, -3.0, 0.0);
        assert!((clamped_area - 5.0).abs() < 1e-6);

        // TestKaTrenchMoveToolKeys::testTranslationStepWithShift:
        // center (100.0, 200.0) + Right + Shift -> (101.0, 200.0)
        let moved =
            translate_trench_center_with_key([100.0, 200.0], TrenchArrowKey::Right, true, true)
                .expect("editable trench layer");
        assert!((moved[0] - 101.0).abs() < 1e-6);
        assert!((moved[1] - 200.0).abs() < 1e-6);

        // Read-only layer guard returns 한글 안내 메시지
        let err =
            translate_trench_center_with_key([100.0, 200.0], TrenchArrowKey::Right, false, false)
                .expect_err("read-only must reject");
        assert_eq!(
            err,
            "트렌치 레이어가 편집 모드가 아닙니다. 편집 시작(연필 아이콘)을 먼저 눌러주세요."
        );
    }
}

/// 시굴 트렌치 기본 단폭 (2.0 m, C++ `HGIS_TRENCH_DEFAULT_WIDTH_M`)
pub const HGIS_TRENCH_DEFAULT_WIDTH_M: f64 = 2.0;
/// 시굴 트렌치 기본 장폭 (20.0 m, C++ `HGIS_TRENCH_DEFAULT_LENGTH_M`)
pub const HGIS_TRENCH_DEFAULT_LENGTH_M: f64 = 20.0;
/// 화살표 키 기본 1회 이동 보폭 (0.1 m / 10 cm, C++ `HGIS_TRENCH_KEY_STEP_BASE_M`)
pub const HGIS_TRENCH_KEY_STEP_BASE_M: f64 = 0.1;
/// Shift + 화살표 키 대형 이동 보폭 (1.0 m, C++ `HGIS_TRENCH_KEY_STEP_LARGE_M`)
pub const HGIS_TRENCH_KEY_STEP_LARGE_M: f64 = 1.0;
/// 키 연속 입력 디바운스 간격 (30 ms, C++ `HGIS_KEY_DEBOUNCE_INTERVAL_MS`)
pub const HGIS_KEY_DEBOUNCE_INTERVAL_MS: u64 = 30;

/// 화살표 키 방향 열거형 (C++ `KaTrenchMoveToolKeys`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrenchArrowKey {
    Left,
    Right,
    Up,
    Down,
}

/// 중심점(`center`), 단폭(`width_m`, 최소 1.0m 클램핑), 장폭(`length_m`, 최소 5.0m 클램핑), 회전각(`rotation_deg`)으로부터
/// 2D 아핀 회전된 트렌치 4정점 닫힌 링, 바운딩 박스(`[min_x, min_y, max_x, max_y]`), 면적(㎡) 산출
/// (C++ `KaTrenchPlaceTool::canvasReleaseEvent` 1:1 대응)
#[must_use]
pub fn create_trench_rectangle_polygon(
    center: [f64; 2],
    width_m: f64,
    length_m: f64,
    rotation_deg: f64,
) -> (Vec<[f64; 2]>, [f64; 4], f64) {
    let eff_w = if width_m.is_finite() {
        width_m.max(1.0)
    } else {
        HGIS_TRENCH_DEFAULT_WIDTH_M
    };
    let eff_l = if length_m.is_finite() {
        length_m.max(5.0)
    } else {
        HGIS_TRENCH_DEFAULT_LENGTH_M
    };
    let rad = if rotation_deg.is_finite() {
        rotation_deg.to_radians()
    } else {
        0.0
    };
    let (cos_t, sin_t) = (rad.cos(), rad.sin());
    let half_l = eff_l * 0.5;
    let half_w = eff_w * 0.5;
    let local_corners = [
        [-half_l, -half_w],
        [half_l, -half_w],
        [half_l, half_w],
        [-half_l, half_w],
    ];
    let mut ring = Vec::with_capacity(5);
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for [dx, dy] in local_corners {
        let wx = center[0] + dx * cos_t - dy * sin_t;
        let wy = center[1] + dx * sin_t + dy * cos_t;
        min_x = min_x.min(wx);
        min_y = min_y.min(wy);
        max_x = max_x.max(wx);
        max_y = max_y.max(wy);
        ring.push([wx, wy]);
    }
    if let Some(&first) = ring.first() {
        ring.push(first);
    }
    (ring, [min_x, min_y, max_x, max_y], eff_w * eff_l)
}

/// 화살표 키 및 Shift 수식키에 따른 트렌치 중심 좌표 평행이동 및 비편집 모드 가드
/// (C++ `KaTrenchMoveToolKeys::translateSelectedTrenches` 1:1 대응)
pub fn translate_trench_center_with_key(
    center: [f64; 2],
    key: TrenchArrowKey,
    shift_pressed: bool,
    layer_is_editable: bool,
) -> Result<[f64; 2], &'static str> {
    if !layer_is_editable {
        return Err(
            "트렌치 레이어가 편집 모드가 아닙니다. 편집 시작(연필 아이콘)을 먼저 눌러주세요.",
        );
    }
    let step = if shift_pressed {
        HGIS_TRENCH_KEY_STEP_LARGE_M
    } else {
        HGIS_TRENCH_KEY_STEP_BASE_M
    };
    let (dx, dy) = match key {
        TrenchArrowKey::Left => (-step, 0.0),
        TrenchArrowKey::Right => (step, 0.0),
        TrenchArrowKey::Up => (0.0, step),
        TrenchArrowKey::Down => (0.0, -step),
    };
    Ok([center[0] + dx, center[1] + dy])
}
