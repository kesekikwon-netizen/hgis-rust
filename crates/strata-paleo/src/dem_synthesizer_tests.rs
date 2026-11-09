
use crate::terrain_edt_tpi::GridTransform;

use crate::dem_synthesizer::*;

#[test]
fn contour_dem_synthesis_and_multi_sheet_merge_produce_continuous_elevation_surface() {
    let tf = GridTransform {
        origin_x: 0.0,
        origin_y: 100.0,
        cell_size: 10.0,
        width: 10,
        height: 10,
    };
    let contours = vec![
        TopographicContourFeature {
            vertices_xy: vec![[0.0, 90.0], [100.0, 90.0]],
            elevation_m: 40.0,
        },
        TopographicContourFeature {
            vertices_xy: vec![[0.0, 10.0], [100.0, 10.0]],
            elevation_m: 10.0,
        },
    ];
    let spots = vec![[50.0, 50.0, 25.0]];
    let dem = synthesize_dem_from_contours_and_spots(&tf, &contours, &spots);
    assert_eq!(dem.len(), 100);
    // North row (near y=90) should be higher than south row (near y=10)
    assert!(dem[5] > dem[85]);

    // Merge two adjacent 5x10 sheet tiles
    let left_tf = GridTransform {
        origin_x: 0.0,
        origin_y: 100.0,
        cell_size: 10.0,
        width: 5,
        height: 10,
    };
    let right_tf = GridTransform {
        origin_x: 50.0,
        origin_y: 100.0,
        cell_size: 10.0,
        width: 5,
        height: 10,
    };
    let merged = merge_multi_sheet_dem_grids(
        &tf,
        &[
            DemTilePatch {
                sheet_code: "359121".to_owned(),
                transform: left_tf,
                elevations_m: vec![20.0; 50],
            },
            DemTilePatch {
                sheet_code: "359122".to_owned(),
                transform: right_tf,
                elevations_m: vec![30.0; 50],
            },
        ],
        15.0,
    );
    assert_eq!(merged.len(), 100);
    assert!((merged[0] - 20.0).abs() < 1e-6);
    assert!((merged[9] - 30.0).abs() < 1e-6);

    let tmp = std::env::temp_dir().join(format!("ka_dem_lock_test_{}.tif", std::process::id()));
    let written = atomic_write_dem_avoiding_file_lock(&tmp, b"GTIFF_DEM_BYTES")
        .expect("atomic DEM write should succeed");
    assert!(written.exists());
    let _ = std::fs::remove_file(written);
}

#[test]
fn test_ngii_contour_interpolation_1m_resolution_and_valid_elevation_bounds() {
    use std::sync::atomic::AtomicBool;

    let tf_1m = GridTransform {
        origin_x: 219_800.0,
        origin_y: 361_620.0,
        cell_size: 1.0,
        width: 20,
        height: 20,
    };
    let contours = vec![
        TopographicContourFeature {
            vertices_xy: vec![[219_800.0, 361_618.0], [219_820.0, 361_618.0]],
            elevation_m: 35.0,
        },
        TopographicContourFeature {
            vertices_xy: vec![[219_800.0, 361_610.0], [219_820.0, 361_610.0]],
            elevation_m: 30.0,
        },
        TopographicContourFeature {
            vertices_xy: vec![[219_800.0, 361_602.0], [219_820.0, 361_602.0]],
            elevation_m: 25.0,
        },
    ];
    let spots = vec![[219_810.0, 361_614.0, 32.5]];
    let cancel = AtomicBool::new(false);
    let dem = interpolate_dem_from_contours_validated(&tf_1m, &contours, &spots, Some(&cancel))
        .expect("1.0m contour DEM interpolation must succeed");
    assert_eq!(dem.len(), 400);
    assert!((tf_1m.cell_size - 1.0).abs() < 1e-9);
    assert!(dem.iter().all(|&z| (-50.0..=2000.0).contains(&z)));
}

#[test]
fn test_feather_seam_linear_midpoint_51m_and_nodata_preservation_50m() {
    // 1. TestKaDemSynthesizer::testFeatherSeamLinearMidpoint:
    // Tile A = 50.0m, Tile B = 52.0m, d = 10.0m over 20.0m blend width (alpha = 0.5) -> 51.0f
    let blended = feather_seam_overlap_cell(50.0, 52.0, 10.0, HGIS_SEAM_BLEND_WIDTH_M);
    assert!((blended - 51.0).abs() < 1e-6);

    // 2. TestKaDemSynthesizer::testFeatherSeamNoDataPreservation:
    // Tile A = 50.0m, Tile B = -9999.0m (NoData), d = 14.0m (alpha = 0.7) -> 50.0f preserved
    let preserved_a =
        feather_seam_overlap_cell(50.0, HGIS_DEM_NODATA_VAL, 14.0, HGIS_SEAM_BLEND_WIDTH_M);
    assert!((preserved_a - 50.0).abs() < 1e-6);

    let preserved_b =
        feather_seam_overlap_cell(HGIS_DEM_NODATA_VAL, 52.0, 6.0, HGIS_SEAM_BLEND_WIDTH_M);
    assert!((preserved_b - 52.0).abs() < 1e-6);

    let both_nodata = feather_seam_overlap_cell(
        HGIS_DEM_NODATA_VAL,
        HGIS_DEM_NODATA_VAL,
        10.0,
        HGIS_SEAM_BLEND_WIDTH_M,
    );
    assert!((both_nodata - HGIS_DEM_NODATA_VAL).abs() < 1e-5);

    // 3. TestKaContourGenerator & TestKaPaleoStreamExtractor
    assert!((interpolate_contour_edge_fraction(15.0, 10.0, 20.0) - 0.5).abs() < 1e-9);
    assert_eq!(classify_contour_type(8.0, 2.0), ContourType::Intermediate);
    assert_eq!(classify_contour_type(10.0, 2.0), ContourType::Index);
    assert_eq!(classify_contour_type(12.0, 2.0), ContourType::Intermediate);

    // Center 50.0m, South(idx 2) = 40.0m (slope 1.0), Southeast(idx 1) = 30.0m (slope 20 / 14.142 = 1.414) -> SE (code 2)
    let neighbors = [50.0_f32, 30.0, 40.0, 50.0, 50.0, 50.0, 50.0, 50.0];
    assert_eq!(
        compute_d8_cell_flow_direction(50.0, &neighbors, 10.0),
        HGIS_D8_SOUTHEAST
    );
    assert_eq!(filter_stream_cells_by_threshold(&[300, 600], 500), vec![1]);
}
