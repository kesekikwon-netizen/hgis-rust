//! `checklist_engine` 시험.
//!
//! `checklist_engine` 에서 분리했다.

use strata_layout::sheet_composer::{DrawingSheet, SheetOrientation, SheetPaperSize};
use strata_types::crs::SubmitCrs;

use crate::checklist_engine::*;

#[test]
fn test_topology_checklist_protruding_feature_triggers_rule9_error_1_0_25() {
    use std::sync::atomic::AtomicBool;

    let sheet = DrawingSheet::new(SheetPaperSize::A3, SheetOrientation::Landscape);
    let survey_bbox = Some([200_000.0, 350_000.0, 200_100.0, 350_100.0]);
    // Protrudes 1.0 m outside survey east boundary (200_101.0 > 200_100.0)
    let features = [[200_050.0, 350_050.0, 200_101.0, 350_080.0]];
    let cancel = AtomicBool::new(false);
    let findings = ChecklistEngine::evaluate_sheet_with_topology(
        &sheet,
        SubmitCrs::Epsg5179,
        survey_bbox,
        &features,
        Some(&cancel),
    )
    .expect("topology checklist should complete");
    let (errors, warnings, passes) = ChecklistEngine::count_by_severity(&findings);
    assert_eq!((errors, warnings, passes), (1, 0, 25));
}

#[test]
fn test_r06_min_area_r07_sliver_r18_feature_no_r20_period_and_r22_date_thresholds() {
    assert_eq!(
        ChecklistEngine::check_r06_min_polygon_area(0.03),
        ChecklistSeverity::Error
    );
    assert_eq!(
        ChecklistEngine::check_r06_min_polygon_area(0.05),
        ChecklistSeverity::Pass
    );

    // 10m x 0.01m needle sliver -> area = 0.1, perim = 20.02 -> thinness = 0.0031 < 0.015
    assert_eq!(
        ChecklistEngine::check_r07_sliver_thinness(0.10, 20.02),
        ChecklistSeverity::Warning
    );
    assert_eq!(
        ChecklistEngine::check_r07_sliver_thinness(100.0, 40.0),
        ChecklistSeverity::Pass
    );

    assert_eq!(
        ChecklistEngine::check_r18_feature_no_format("집자리_1호"),
        ChecklistSeverity::Error
    );
    assert_eq!(
        ChecklistEngine::check_r18_feature_no_format("주거1"),
        ChecklistSeverity::Pass
    );
    assert_eq!(
        ChecklistEngine::check_r18_feature_no_format("수혈12-1"),
        ChecklistSeverity::Pass
    );

    assert_eq!(
        ChecklistEngine::check_r20_period_code_valid("삼국"),
        ChecklistSeverity::Pass
    );
    assert_eq!(
        ChecklistEngine::check_r20_period_code_valid("철기시대"),
        ChecklistSeverity::Error
    );

    assert_eq!(
        ChecklistEngine::check_r22_date_format_valid("2026-11-07"),
        ChecklistSeverity::Pass
    );
    assert_eq!(
        ChecklistEngine::check_r22_date_format_valid("2026/11/07"),
        ChecklistSeverity::Warning
    );

    let bbox =
        ChecklistEngine::compute_error_zoom_bbox(200_000.0, 500_000.0, 200_000.0, 500_000.0);
    assert_eq!(bbox, [199_980.0, 499_980.0, 200_020.0, 500_020.0]);
    assert!((bbox[2] - bbox[0] - 40.0).abs() < 1e-9);
    assert!((bbox[3] - bbox[1] - 40.0).abs() < 1e-9);
}

#[test]
fn test_r04_ring_close_r08_bowtie_5_5_r10_duplicate_r11_spike_057deg_and_r21_gcp_count() {
    // R04 unclosed ring
    let unclosed = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 0.01]];
    let (r04_sev, r04_pt, r04_msg) = ChecklistEngine::check_r04_ring_closure(&unclosed);
    assert_eq!(r04_sev, ChecklistSeverity::Error);
    assert_eq!(r04_pt, Some([0.0, 0.01]));
    assert!(r04_msg.contains("10.00 mm"));

    // R08 Bowtie self-intersection at (5.0, 5.0)
    let bowtie = [
        [0.0, 0.0],
        [10.0, 10.0],
        [0.0, 10.0],
        [10.0, 0.0],
        [0.0, 0.0],
    ];
    let (r08_sev, r08_pt, _) = ChecklistEngine::check_r08_self_intersection(&bowtie);
    assert_eq!(r08_sev, ChecklistSeverity::Error);
    let ix = r08_pt.expect("intersection point");
    assert!((ix[0] - 5.0).abs() < 1e-6);
    assert!((ix[1] - 5.0).abs() < 1e-6);

    // R10 duplicate feature
    let (r10_sev, r10_pt, r10_msg) = ChecklistEngine::check_r10_duplicate_feature(
        [100.0, 200.0],
        50.0,
        [100.0005, 200.0],
        50.0002,
        42,
    );
    assert_eq!(r10_sev, ChecklistSeverity::Error);
    assert_eq!(r10_pt, Some([100.0, 200.0]));
    assert!(r10_msg.contains("FID 42"));

    // R11 needle spike at P1(100.0, 0.5) with angle ~0.57 deg < 2.0 deg
    let needle = [[0.0, 0.0], [100.0, 0.5], [0.0, 1.0], [0.0, 0.0]];
    let (r11_sev, r11_pt, r11_msg) = ChecklistEngine::check_r11_spike_vertex(&needle);
    assert_eq!(r11_sev, ChecklistSeverity::Warning);
    assert_eq!(r11_pt, Some([100.0, 0.5]));
    assert!(r11_msg.contains("0.57도"));

    // R21 control points count < 2
    let (r21_sev, r21_pt, r21_msg) = ChecklistEngine::check_r21_control_points_count(
        &[[200_010.0, 500_020.0]],
        [200_000.0, 500_000.0],
    );
    assert_eq!(r21_sev, ChecklistSeverity::Error);
    assert_eq!(r21_pt, Some([200_010.0, 500_020.0]));
    assert!(r21_msg.contains("현재: 1개"));
}

#[test]
fn test_duplicate_feat_no_detection_and_period_domain_validation() {
    let features = [(10_i64, "1호 주거지"), (25_i64, "1호 주거지")];
    let issues = validate_unique_feature_ids(&features);
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].fid, 25);
    assert_eq!(
        issues[0].message_ko,
        "동일한 유구번호 '1호 주거지'이(가) 이미 존재합니다 (FID: 10와 중복)."
    );

    assert!(validate_period_domain(10, "삼국").is_none());
    let typo_issue = validate_period_domain(10, "삼국시디").expect("invalid period domain");
    assert_eq!(typo_issue.severity, ChecklistSeverity::Warning);
    assert!(typo_issue.message_ko.contains("삼국시디"));

    // TestKaStratigraphicCycleDetector::testAcyclicLinearStratigraphy & testCycleLoopDetection
    let linear_edges = vec![
        StratigraphicEdge {
            younger_feature_id: "1호".to_string(),
            older_feature_id: "2호".to_string(),
            relation: StratigraphicRelation::Cuts,
        },
        StratigraphicEdge {
            younger_feature_id: "2호".to_string(),
            older_feature_id: "3호".to_string(),
            relation: StratigraphicRelation::Cuts,
        },
    ];
    let acyclic_res = build_harris_matrix(&["1호", "2호", "3호"], &linear_edges);
    assert!(acyclic_res.is_acyclic);
    assert_eq!(acyclic_res.topological_order, vec!["1호", "2호", "3호"]);
    assert!(acyclic_res.detected_cycle_path.is_empty());

    let mut cycle_edges = linear_edges;
    cycle_edges.push(StratigraphicEdge {
        younger_feature_id: "3호".to_string(),
        older_feature_id: "1호".to_string(),
        relation: StratigraphicRelation::Cuts,
    });
    let cycle_res = build_harris_matrix(&["1호", "2호", "3호"], &cycle_edges);
    assert!(!cycle_res.is_acyclic);
    assert_eq!(
        cycle_res.detected_cycle_path,
        vec!["1호", "2호", "3호", "1호"]
    );
}

#[test]
fn test_drawing_checklist_v1_json_loads_all_27_cpp_rules_in_exact_order() {
    let engine = ChecklistEngine::with_default_rules();
    assert_eq!(engine.rule_count(), 27);
    assert_eq!(engine.title(), "발굴 위치도면 유의사항 기반 검수 v1");

    let expected_ids = [
        "REQ_DRAWING_SURVEY_AREA",
        "REQ_DRAWING_SITE_LOCATION",
        "REQ_DRAWING_FEATURE_PLAN",
        "REQ_DRAWING_FEATURE_DETAIL",
        "GEOMETRY_VALID",
        "GEOMETRY_NOT_EMPTY",
        "GEOMETRY_NONZERO_AREA",
        "SURVEY_POLYGON_ONLY",
        "SURVEY_NO_ABSTRACT_MARKER",
        "GCP_MIN_TWO",
        "GCP_DATUM_META",
        "GCP_ELLIPSOID_META",
        "GCP_PROJECTION_META",
        "GCP_ORIGIN_META",
        "GCP_ACCURACY_NOTE",
        "FEATURE_NO_SYMBOL_ONLY",
        "FEATURE_LEGEND_FIELDS",
        "EXTENT_MATCH",
        "CRS_PROJECT_SET",
        "EXPORT_SHP_READY",
        "CRS_WORK_BELT",
        "SHEET_COVERS_TARGETS",
        "SHEET_ELEMENTS",
        "REQUIRED_FIELDS",
        "GCP_POINT_ID_UNIQUE",
        "FEATURE_NO_UNIQUE",
        "HERITAGE_PLEDGE_SCOPE",
    ];
    let actual_ids: Vec<&str> = engine.rules().iter().map(|r| r.id.as_str()).collect();
    assert_eq!(actual_ids, expected_ids);

    let sheet = DrawingSheet::new(SheetPaperSize::A3, SheetOrientation::Landscape);
    let results = ChecklistEngine::evaluate_sheet_results(&sheet, SubmitCrs::Epsg5179, true);
    assert_eq!(results.len(), 27);
    let errs = ChecklistEngine::failed_count(&results, "error");
    let warns = ChecklistEngine::failed_count(&results, "warn");
    let passes = ChecklistEngine::passed_count(&results);
    assert_eq!(errs + warns + passes, 27);
    assert_eq!(
        ChecklistEngine::status_bar_text(&results),
        format!("검수: 오류 {errs} · 주의 {warns}")
    );
}
