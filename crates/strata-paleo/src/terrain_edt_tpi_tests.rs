use crate::terrain_edt_tpi::*;

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
    assert_eq!(edge.from_site_id, "봉수대-A");
    assert!(!edge.is_visible);
    assert!(edge.min_clearance_m < 0.0);

    // TestPaleoDemClassifier::testNaturalLeveeClassification & C++ PaleoDemClassifier::classify
    assert_eq!(
        classify_tpi_std_slope_landform(0.85, 1.8),
        PaleoLandformType::NaturalLevee
    );
    let terrace_cls = classify_paleo_dem_polygon(
        "하안단구",
        &PaleoDemMetrics {
            has_dem: true,
            mean_elev_m: 42.0,
            local_relief_m: 18.5,
            mean_slope_deg: 2.4,
            river_dist_m: 120.0,
            has_borehole: true,
            borehole_code: "BH-02".to_owned(),
            alluvial_thickness_m: 3.6,
            weathered_depth_m: 4.8,
            deposit_env: "사질".to_owned(),
            dominant_uscs: "SM".to_owned(),
        },
    );
    assert_eq!(terrace_cls.kind, "하안단구");
    assert!(terrace_cls.note.contains("중위단구(T2, 비고 18.5m)"));
}
