use crate::viewport::ViewportTransform;

use crate::canvas_painter::*;

#[test]
fn demo_gyeongju_scene_projects_survey_area_and_trenches_to_screen_space() {
    let scene = MapSceneModel::new_demo_gyeongju_scene();
    let vp = ViewportTransform::new([219_850.0, 360_120.0], 0.5, [1200.0, 800.0]);
    let prims = scene.project_polygons(&vp);
    assert!(prims.len() >= 2);
    assert!(prims.iter().any(|p| p.layer_key == "survey_area"));
    assert!(prims.iter().any(|p| p.layer_key == "trial_trench"));
}

#[test]
fn test_subpixel_culling_under_1_5px_and_selection_outline_width_doubled() {
    // 1. TestKaCanvasPainter::testSubpixelCullingUnder1_5px: width = 0.8px, height = 0.7px -> false
    let tiny_poly = [[10.0_f32, 20.0], [10.8, 20.0], [10.8, 20.7], [10.0, 20.7]];
    assert!(!should_draw_polygon_feature(&tiny_poly));

    // Degenerate polygon (< 3 vertices) -> false
    assert!(!should_draw_polygon_feature(&[[10.0, 20.0], [30.0, 40.0]]));

    // Normal polygon (10px x 10px) -> true
    let normal_poly = [[10.0_f32, 20.0], [20.0, 20.0], [20.0, 30.0], [10.0, 30.0]];
    assert!(should_draw_polygon_feature(&normal_poly));

    // 2. TestKaFeatureStyleRenderer::testSelectionOutlineWidthDoubled: 0.35mm selected -> 0.70mm & #00E5FF
    let (w_sel, col_sel) = compute_selection_outline_style(0.35, [139, 69, 19, 255], true);
    assert!((w_sel - 0.70).abs() < 1e-6);
    assert_eq!(col_sel, HGIS_SELECTION_OUTLINE_COLOR);

    // 3. TestKaSpatialHitTester::testPointOverPolygonPriority & testMarqueeLeftRightMode
    let candidates = vec![
        PickableFeatureCandidate {
            fid: 200, // Polygon B
            dim: HitGeometryDim::Polygon,
            visible: true,
            locked: false,
            bbox_min_xy: [90.0, 90.0],
            bbox_max_xy: [120.0, 120.0],
            distance_to_click_m: 0.0,
        },
        PickableFeatureCandidate {
            fid: 101, // Artifact Point A at (100.0, 100.0)
            dim: HitGeometryDim::Point,
            visible: true,
            locked: false,
            bbox_min_xy: [100.0, 100.0],
            bbox_max_xy: [100.0, 100.0],
            distance_to_click_m: 0.2,
        },
    ];
    assert_eq!(pick_feature_at_with_priority(&candidates, 1.0), Some(101));

    // Marquee [80..110, 80..110] only partially overlaps Polygon B [90..120, 90..120]
    let only_poly = &candidates[0..1];
    let contain_res = select_by_marquee_directional(only_poly, [80.0, 80.0], [110.0, 110.0]);
    assert!(contain_res.is_empty());
    let crossing_res = select_by_marquee_directional(only_poly, [110.0, 110.0], [80.0, 80.0]);
    assert_eq!(crossing_res, vec![200]);

    // 4. TestKaHatchPatternRenderer::testSpacingPixelScaling: 2.54mm at 300 DPI -> 30.0px
    assert!((compute_hatch_spacing_px(2.54, 300.0) - 30.0).abs() < 1e-9);
    assert!((compute_hatch_spacing_px(0.01, 254.0) - 5.0).abs() < 1e-9);

    // 5. TestKaArchaeologicalMarkerRenderer::testDeclinationRotationAngle:
    // mapRotation = 15.0°, gridConvergence = 1.5°, magneticDeclination = -7.0° -> (16.5°, 9.5°)
    let (theta_grid, theta_mag) = compute_north_arrow_declination_angles_deg(15.0, 1.5, -7.0);
    assert!((theta_grid - 16.5).abs() < 1e-9);
    assert!((theta_mag - 9.5).abs() < 1e-9);
    assert_eq!(
        resolve_artifact_svg_resource_path("pottery"),
        Some(":/symbols/artifacts/pottery_vessel.svg")
    );
    assert!(
        (compute_marker_size_px(4.0, SizingMode::ScreenFixedMm, 254.0, 1.0) - 40.0).abs()
            < 1e-9
    );
    assert!(
        (compute_marker_size_px(10.0, SizingMode::WorldProportionalM, 96.0, 0.5) - 20.0).abs()
            < 1e-9
    );

    // 6. TestKaCanvasHitTester::testPolygonPointInPolygon & testLineTolerance10px
    let poly_hit = perform_canvas_hit_test_10px(
        Some(([45.0, 45.0], [55.0, 55.0], 1)),
        None,
        [52.0, 48.0],
        0.05,
        HGIS_SELECTION_TOLERANCE_PX,
    );
    assert!(poly_hit.has_hit);
    assert!((poly_hit.hit_distance_pixels - 0.0).abs() < 1e-9);

    let line_hit = perform_canvas_hit_test_10px(
        None,
        Some((0.40, 2)),
        [0.0, 0.0],
        0.05,
        HGIS_SELECTION_TOLERANCE_PX,
    );
    assert!(line_hit.has_hit);
    assert!((line_hit.hit_distance_pixels - 8.0).abs() < 1e-9);
}
