use crate::trench_profile3d::*;

#[test]
fn test_trench_profile_3d_slope_angle_45deg_and_zero_length_fallback() {
    // TestKaTrenchProfile3D::testSlopeAngle45Deg: (0,0,100) -> (10,0,110) => 45.0 deg
    let p = TrenchProfile3d::from_endpoints([0.0, 0.0], [10.0, 0.0], 100.0, 110.0);
    assert!((p.length_m - 10.0).abs() < 1e-6);
    assert!((p.slope_deg - 45.0).abs() < 1e-6);
    assert!((p.azimuth_deg - 90.0).abs() < 1e-6);

    // Zero horizontal length fallback -> 0.0 deg slope (no NaN / 90 deg spike)
    let zero = TrenchProfile3d::from_endpoints([5.0, 5.0], [5.0, 5.0], 100.0, 125.0);
    assert_eq!(zero.length_m, 0.0);
    assert_eq!(zero.slope_deg, 0.0);

    // TestKaTerrainMeshExporter::testLocalOffsetCenteringAndNormals
    let dem = [50.0_f32, 50.0, 50.0, 50.0];
    let mesh = generate_terrain_mesh(
        &dem,
        2,
        2,
        200_000.0,
        500_000.0,
        5.0,
        HGIS_DEFAULT_Z_EXAGGERATION,
    );
    assert_eq!(mesh.vertices.len(), 4);
    assert_eq!(mesh.indices.len(), 6);
    assert!((mesh.vertices[0].x - (-2.5)).abs() <= 1e-6);
    assert!((mesh.vertices[0].y - (-2.5)).abs() <= 1e-6);
    assert!((mesh.vertices[1].x - 2.5).abs() <= 1e-6);
    assert!((mesh.vertices[1].y - (-2.5)).abs() <= 1e-6);
    assert!((mesh.vertices[2].x - (-2.5)).abs() <= 1e-6);
    assert!((mesh.vertices[2].y - 2.5).abs() <= 1e-6);
    assert!((mesh.vertices[3].x - 2.5).abs() <= 1e-6);
    assert!((mesh.vertices[3].y - 2.5).abs() <= 1e-6);
    for v in &mesh.vertices {
        assert!((v.nx - 0.0).abs() <= 1e-6);
        assert!((v.ny - 0.0).abs() <= 1e-6);
        assert!((v.nz - 1.0).abs() <= 1e-6);
    }
    assert_eq!((mesh.vertices[0].u, mesh.vertices[0].v), (0.0, 0.0));
    assert_eq!((mesh.vertices[1].u, mesh.vertices[1].v), (1.0, 0.0));
    assert_eq!((mesh.vertices[2].u, mesh.vertices[2].v), (0.0, 1.0));
    assert_eq!((mesh.vertices[3].u, mesh.vertices[3].v), (1.0, 1.0));
}

#[test]
fn test_suspect_point_filtering_and_survey_contour_tin_builder() {
    let mut pts = Vec::new();
    for i in 0..10 {
        pts.push(SurveyPoint3D {
            fid: i + 1,
            x: (i % 5) as f64 * 5.0,
            y: (i / 5) as f64 * 5.0,
            z: 30.0,
            code: "G".to_owned(),
            is_suspect: false,
        });
    }
    let bad_idx = 5usize;
    pts[bad_idx].z = 55.0;

    filter_suspect_points(&mut pts, HGIS_SUSPECT_ELEVATION_DIFF_M);
    assert!(pts[bad_idx].is_suspect);
    assert!(pts
        .iter()
        .enumerate()
        .filter(|&(idx, _)| idx != bad_idx)
        .all(|(_, p)| !p.is_suspect));

    assert_eq!(
        survey_contour_rejection(&pts[..2]),
        Some("등고선을 만들려면 점이 3개 이상 필요합니다.")
    );
    assert_eq!(survey_contour_rejection(&pts), None);

    let line = vec![
        SurveyPoint3D {
            fid: 100,
            x: 0.0,
            y: 0.0,
            z: 30.0,
            code: "T".to_owned(),
            is_suspect: false,
        },
        SurveyPoint3D {
            fid: 101,
            x: 2.0,
            y: 0.0,
            z: 31.0,
            code: "T".to_owned(),
            is_suspect: false,
        },
    ];
    let densified = densify_survey_breaklines(&[line], 0.5, &pts);
    assert_eq!(densified.len(), 4);
    let fitted = fit_contour_cell_size(0.1, 1000.0, 1000.0, HGIS_MAX_CONTOUR_GRID_CELLS);
    assert!(fitted >= 0.5);
}

#[test]
fn test_profile_length_and_elevation_sampling_and_dem_legend_ticks() {
    let mut grid = Vec::with_capacity(22);
    for _r in 0..2 {
        for c in 0..=10 {
            grid.push(10.0_f32 + (c as f32));
        }
    }
    let pts = sample_elevation_profile_bilinear(
        &grid,
        11,
        2,
        1.0,
        [0.0, 0.0],
        &[[0.0, 0.5], [10.0, 0.5]],
        1.0,
    );
    assert_eq!(pts.len(), 11);
    assert!((pts[0].chainage_m - 0.0).abs() < 1e-6);
    assert!((pts[0].elevation_m - 10.0).abs() < 1e-5);
    assert!((pts[5].chainage_m - 5.0).abs() < 1e-6);
    assert!((pts[5].elevation_m - 15.0).abs() < 1e-5);
    assert!((pts[10].chainage_m - 10.0).abs() < 1e-6);
    assert!((pts[10].elevation_m - 20.0).abs() < 1e-5);

    let ticks = build_dem_legend_ticks(-12.0, 1850.0);
    assert!((5..=7).contains(&ticks.len()));
    assert!((ticks[0] - (-12.0)).abs() < 1e-6);
    assert!((*ticks.last().unwrap_or(&0.0) - 1850.0).abs() < 1e-6);

    let (depth, caption) =
        compute_pit_elevation_depth_and_caption(Some(48.50), Some(47.20), None);
    assert!((depth.unwrap_or(0.0) - 1.30).abs() < 1e-6);
    assert_eq!(caption, "상면 표고 48.50, 바닥 표고 47.20, 깊이 1.30 미터");
}
