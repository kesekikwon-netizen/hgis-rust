
use crate::pipeline::CrossSectionProfile;

use crate::section_renderer::*;

#[test]
fn cross_section_rgba_renderer_draws_three_stratigraphic_horizons() {
    let profile = CrossSectionProfile {
        dist_m: vec![0.0, 50.0, 100.0],
        z_surface_curr_m: vec![45.0, 44.5, 44.0],
        z_paleo_top_m: vec![43.0, 42.5, 42.0],
        z_bedrock_base_m: vec![38.0, 37.5, 37.0],
    };
    let rgba = render_cross_section_rgba(&profile, 240, 120);
    assert_eq!(rgba.len(), 240 * 120 * 4);
}

#[test]
fn test_flat_surface_min_range_10m_and_four_stratum_palette_constants() {
    // TestKaSectionRenderer2D::testFlatSurfaceMinRange:
    // Flat surface at 30.0m -> expanded to [25.0m, 35.0m] (10.0m span), no NaN or division by zero
    let flat_profile = [30.0_f32, 30.0, 30.0, 30.0];
    let (z_min, z_max) = compute_section_vertical_bounds(&flat_profile);
    assert!((z_max - z_min - HGIS_MIN_SECTION_VERT_RANGE_M).abs() < 1e-6);
    assert!((z_min - 25.0).abs() < 1e-6);
    assert!((z_max - 35.0).abs() < 1e-6);

    let y_mid = section_elevation_to_pixel_y(30.0, z_min, z_max, 200.0, 10.0);
    assert!(y_mid.is_finite());
    assert!((y_mid - 110.0).abs() < 1e-5);

    assert_eq!(HGIS_STRATUM_FILL_COLOR, [194, 158, 120]);
    assert_eq!(HGIS_STRATUM_ALLUVIUM_COLOR, [222, 203, 166]);
    assert_eq!(HGIS_STRATUM_WEATHERED_COLOR, [168, 160, 144]);
    assert_eq!(HGIS_STRATUM_BEDROCK_COLOR, [120, 115, 108]);
    assert_eq!(HGIS_WATER_TABLE_COLOR, [41, 128, 185]);

    // TestKaCrossViewCursorSync::testSectionProjectionDistance & TestKaElevationTooltipProbe::testFillDepthCalculation
    let probe = probe_multi_elevation_at_world_coord(
        [45.0, 20.0],
        [0.0, 0.0],
        [100.0, 0.0],
        Some(32.50),
        Some(29.30),
    );
    assert!(probe.has_valid_sample);
    assert!((probe.section_distance_d - 45.0).abs() < 1e-6);
    assert!((probe.fill_depth_m - 3.20).abs() < 1e-6);

    // TestKaArtifactSectionProjector::testBufferWidthFilteringAndProjection
    let artifacts = vec![
        ArtifactPoint3D {
            fid: 1,
            x: 50.0,
            y: 1.5,
            z: 35.0,
            material: ArtifactMaterial::Pottery,
            artifact_no: "토기-1".to_string(),
        },
        ArtifactPoint3D {
            fid: 2,
            x: 50.0,
            y: 2.5,
            z: 36.0,
            material: ArtifactMaterial::Stone,
            artifact_no: "석기-2".to_string(),
        },
    ];
    let projected = project_artifacts_to_section(
        &artifacts,
        [0.0, 0.0],
        [100.0, 0.0],
        HGIS_DEFAULT_SECTION_BUFFER_M,
    );
    assert_eq!(projected.len(), 1);
    assert_eq!(projected[0].fid, 1);
    assert!((projected[0].d_proj_m - 50.0).abs() < 1e-6);
    assert!((projected[0].elevation_z - 35.0).abs() < 1e-6);
    assert!((projected[0].d_perp_m - 1.5).abs() < 1e-6);

    // TestKaHomographySolver::testIdentityQuadHomography & testCollinearPinsRejection
    let pins = [
        TargetPinPair {
            photo_pixel: [0.0, 0.0],
            section_world: [0.0, 0.0],
        },
        TargetPinPair {
            photo_pixel: [100.0, 0.0],
            section_world: [1.0, 0.0],
        },
        TargetPinPair {
            photo_pixel: [100.0, 100.0],
            section_world: [1.0, 1.0],
        },
        TargetPinPair {
            photo_pixel: [0.0, 100.0],
            section_world: [0.0, 1.0],
        },
    ];
    let h = solve_homography_4point(&pins).expect("convex quad homography should succeed");
    let p_world = project_homography_point(&h, [50.0, 50.0]);
    assert!((p_world[0] - 0.5).abs() <= 1e-6);
    assert!((p_world[1] - 0.5).abs() <= 1e-6);

    let collinear_pins = [
        TargetPinPair {
            photo_pixel: [0.0, 10.0],
            section_world: [0.0, 0.0],
        },
        TargetPinPair {
            photo_pixel: [50.0, 10.0],
            section_world: [0.5, 0.0],
        },
        TargetPinPair {
            photo_pixel: [100.0, 10.0],
            section_world: [1.0, 0.0],
        },
        TargetPinPair {
            photo_pixel: [0.0, 100.0],
            section_world: [0.0, 1.0],
        },
    ];
    assert_eq!(
        solve_homography_4point(&collinear_pins),
        Err(HomographyError::CollinearOrConcavePins)
    );

    // TestKaRadiocarbonCalibrator::testStandardNeolithicCalibration
    let curve = default_neolithic_intcal20_segment();
    let cal = calibrate_c14_date(4500, 30, &curve).expect("4500 BP calibration should succeed");
    assert!(!cal.range_2sigma.is_empty());
    let total_prob: f64 = cal.range_2sigma.iter().map(|r| r.probability_pct).sum();
    assert!((95.3..=95.5).contains(&total_prob));
    for r in &cal.range_2sigma {
        assert!(r.start_year_cal >= -3350 && r.end_year_cal <= -3090);
    }
    assert_eq!(
        calibrate_c14_date(60_000, 30, &curve),
        Err(RadiocarbonError::ExceedsIntCalBounds)
    );
}
