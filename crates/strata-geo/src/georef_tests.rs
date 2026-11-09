//! polygon_ops 회귀 시험 2/2 — GCP·DBSCAN·트래버스·정합 품질.

#![allow(clippy::unwrap_used, clippy::expect_used)]


use crate::cadastral_parcel::*;
use crate::cluster_traverse::*;
use crate::gcp_solve::*;
use crate::georef_quality::*;

#[test]
fn test_perfect_affine_transformation_and_insufficient_gcp_error() {
    // TestKaGcpTransformSolver::testPerfectAffineTransformation
    let gcps = vec![
        GcpPointPair {
            source_pixel: [0.0, 0.0],
            target_world: [100.0, 200.0],
            is_enabled: true,
        },
        GcpPointPair {
            source_pixel: [100.0, 0.0],
            target_world: [200.0, 200.0],
            is_enabled: true,
        },
        GcpPointPair {
            source_pixel: [0.0, 100.0],
            target_world: [100.0, 300.0],
            is_enabled: true,
        },
        GcpPointPair {
            source_pixel: [100.0, 100.0],
            target_world: [200.0, 300.0],
            is_enabled: true,
        },
    ];
    let res = solve_gcp_transform(&gcps, TransformType::Affine6Param)
        .expect("4 non-collinear GCPs should succeed");
    assert!(res.is_success);
    let expected = [1.0, 0.0, 100.0, 0.0, 1.0, 200.0];
    for (got, exp) in res.transform_coeffs.iter().zip(expected.iter()) {
        assert!((got - exp).abs() < 1e-9);
    }
    assert!(res.rmse_m <= 1e-9);
    assert!(!res.exceeds_allowable_rmse);

    // TestKaGcpTransformSolver::testInsufficientGcpError
    let err = solve_gcp_transform(&gcps[..2], TransformType::Affine6Param);
    assert_eq!(err, Err(GcpSolverError::InsufficientGcpCount));
}

#[test]
fn test_dbscan_cluster_formation_and_closed_traverse_precision() {
    // TestKaArtifactClusterAnalyzer::testDbscanClusterFormation:
    // 4 points within 0.5m radius + 1 isolated point 10m away
    let pts = vec![
        (1, [0.0, 0.0], "Pottery"),
        (2, [0.3, 0.0], "Pottery"),
        (3, [0.0, 0.3], "Stone"),
        (4, [0.3, 0.3], "Pottery"),
        (5, [10.0, 10.0], "Iron"),
    ];
    let (clusters, noise_count) = compute_dbscan_artifact_clusters(
        &pts,
        HGIS_DBSCAN_DEFAULT_EPSILON_M,
        HGIS_DBSCAN_DEFAULT_MIN_PTS,
    );
    assert_eq!(clusters.len(), 1);
    assert_eq!(clusters[0].cluster_id, 0);
    assert_eq!(clusters[0].member_fids.len(), 4);
    assert_eq!(noise_count, 1);

    let co = compute_material_co_occurrence(
        &[vec!["Pottery", "Stone"], vec!["Pottery", "Iron"]],
        &["Pottery", "Stone", "Iron"],
    );
    assert_eq!(co.matrix[0][0], 2);
    assert_eq!(co.matrix[0][1], 1);
    assert_eq!(co.matrix[0][2], 1);

    // TestKaSurveyNetworkAdjuster::testClosedTraversePrecision:
    // 4 legs of 100.0m each (total 400.0m), net fx = +0.03m, fy = -0.04m -> fs = 0.05m, ratio = 1/8000
    let legs = vec![
        TraverseLeg {
            from_station: "CP-1".to_string(),
            to_station: "CP-2".to_string(),
            delta_x_m: 100.0,
            delta_y_m: 0.0,
            measured_distance_m: 100.0,
        },
        TraverseLeg {
            from_station: "CP-2".to_string(),
            to_station: "CP-3".to_string(),
            delta_x_m: 0.0,
            delta_y_m: 100.0,
            measured_distance_m: 100.0,
        },
        TraverseLeg {
            from_station: "CP-3".to_string(),
            to_station: "CP-4".to_string(),
            delta_x_m: -99.97,
            delta_y_m: 0.0,
            measured_distance_m: 100.0,
        },
        TraverseLeg {
            from_station: "CP-4".to_string(),
            to_station: "CP-1".to_string(),
            delta_x_m: 0.0,
            delta_y_m: -100.04,
            measured_distance_m: 100.0,
        },
    ];
    let adj = adjust_closed_traverse(&legs, [1000.0, 2000.0], 10.0);
    assert!((adj.linear_closing_error_m - 0.05).abs() < 1e-6);
    assert_eq!(adj.closing_ratio_denominator, 8000);
    assert!(adj.is_acceptable);
    let closed_end = adj.adjusted_coords.last().expect("adjusted end coord");
    assert!((closed_end[0] - 1000.0).abs() < 1e-9);
    assert!((closed_end[1] - 2000.0).abs() < 1e-9);

    // TestKaCadastralOverlapAnalyzer::testParcelOverlapAndRounding & testSliverFilteringUnder0_1m2
    let survey_ring = [
        [0.0, 0.0],
        [20.0, 0.0],
        [20.0, 25.0],
        [0.0, 25.0],
        [0.0, 0.0],
    ];
    let parcels = vec![
        CadastralParcelInput {
            pnu: "4713010300101050001".to_string(),
            jibun: "105-1 대".to_string(),
            jimok: "대".to_string(),
            official_area_m2: 1000.0,
            exterior_ring: vec![
                [0.0, 0.0],
                [40.0, 0.0],
                [40.0, 25.0],
                [0.0, 25.0],
                [0.0, 0.0],
            ],
        },
        // Sliver parcel with 0.2m x 0.2m = 0.04 m² overlap (< 0.1 m²)
        CadastralParcelInput {
            pnu: "4713010300101050002".to_string(),
            jibun: "105-2 전".to_string(),
            jimok: "전".to_string(),
            official_area_m2: 500.0,
            exterior_ring: vec![
                [19.8, 0.0],
                [30.0, 0.0],
                [30.0, 0.2],
                [19.8, 0.2],
                [19.8, 0.0],
            ],
        },
    ];
    let summary =
        analyze_cadastral_inclusion(&survey_ring, &parcels, HGIS_MIN_PARCEL_OVERLAP_M2);
    assert_eq!(summary.total_parcels_count, 1);
    assert_eq!(summary.parcel_records.len(), 1);
    let rec = &summary.parcel_records[0];
    assert_eq!(rec.jibun, "105-1 대");
    assert!((rec.overlap_area_m2 - 500.0).abs() < 1e-6);
    assert!((rec.inclusion_ratio_pct - 50.0).abs() < 1e-6);
}

#[test]
fn test_rmse_grade_evaluation_and_subpixel_centering_on_crosshair() {
    // TestGeorefQuality::testRmseGradeEvaluation:
    // 0.08m -> GradeA, 0.18m -> GradeB, 0.35m -> GradeC, 0.65m -> GradeD
    assert_eq!(
        evaluate_rmse_grade(0.08),
        GeorefQualityGrade::GradeAExcellent
    );
    assert_eq!(evaluate_rmse_grade(0.18), GeorefQualityGrade::GradeBGood);
    assert_eq!(evaluate_rmse_grade(0.35), GeorefQualityGrade::GradeCCaution);
    assert_eq!(
        evaluate_rmse_grade(0.65),
        GeorefQualityGrade::GradeDCritical
    );
    assert!(
        georef_quality_summary_ko(2, 0.0, 0.0).contains("한 점 더 찍으면 기울기도 맞습니다")
    );
    assert!(
        georef_quality_summary_ko(4, 0.08, 0.12).contains("평균 어긋남 0.08 m · 최대 0.12 m")
    );

    // TestGeorefPixelWindow::testSubPixelCenteringOnCrosshair:
    // 15x15 white background (255) with a dark crosshair centered at offset (+1, -1) from center (7, 7), i.e., col=8, row=6
    let mut patch = vec![255_u8; 15 * 15];
    for i in 0..15 {
        patch[6 * 15 + i] = 20;
        patch[i * 15 + 8] = 20;
    }
    patch[6 * 15 + 8] = 0;
    let sub = find_subpixel_crosshair_center_15x15(&patch, 100, 200);
    assert!(sub.is_confident);
    assert!((sub.refined_u - 101.0).abs() <= 0.15);
    assert!((sub.refined_v - 199.0).abs() <= 0.15);

    // Flat low-contrast patch falls back to integer click
    let flat = vec![200_u8; 15 * 15];
    let fallback = find_subpixel_crosshair_center_15x15(&flat, 100, 200);
    assert!(!fallback.is_confident);
    assert!((fallback.refined_u - 100.0).abs() < 1e-9);
    assert!((fallback.refined_v - 200.0).abs() < 1e-9);

    let mut taken = std::collections::BTreeSet::new();
    let base = std::path::PathBuf::from("/tmp/plan_georef.tif");
    taken.insert(base.clone());
    let uniq = unique_georef_output_path(&base, &taken);
    assert!(uniq.ends_with("plan_georef_2.tif"));

    // TestSurveyScopeClip::testLineIntersectionClipping:
    // 100m x 100m survey box [0, 0, 100, 100], contour line (-50, 50) -> (150, 50) (200m total) -> clipped length = 100.0m
    let (c0, c1) =
        clip_line_segment_by_bbox([-50.0, 50.0], [150.0, 50.0], [0.0, 0.0, 100.0, 100.0])
            .expect("line intersects survey box");
    let clipped_len = (c1[0] - c0[0]).hypot(c1[1] - c0[1]);
    assert!((clipped_len - 100.0).abs() <= 1e-4);
}
