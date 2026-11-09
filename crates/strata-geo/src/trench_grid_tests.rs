//! `trench_grid` 회귀 시험. 원본 `mod tests` 를 그대로 옮겼다.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use geo::{Area, Coord, LineString, Polygon};

use crate::trench_edit::*;
use crate::trench_generate::*;
use crate::trench_lshape::*;
use crate::trench_stats::*;

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
