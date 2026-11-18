//! A north slope written the way the survey contour package is written.
//! Four lines at 2, 4, 6 and 8 m, four survey points, a north-up GeoTIFF in EPSG:5187.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use gdal::vector::{FieldValue, LayerAccess};
use gdal::{Dataset, DriverManager};

use super::write_contour_package;

fn slope() -> [[f64; 3]; 4] {
    [
        [0.0, 0.0, 0.0],
        [100.0, 0.0, 0.0],
        [0.0, 100.0, 10.0],
        [100.0, 100.0, 10.0],
    ]
}

#[test]
fn the_package_has_the_slope_levels_and_a_real_grid() {
    let dir = tempfile::tempdir().unwrap();
    let out = write_contour_package(dir.path(), 5187, &slope(), 2.0, 10.0, 0.0, &[]).unwrap();
    assert_eq!((out.lines, out.points), (4, 4));

    let gpkg = Dataset::open(dir.path().join("contours.gpkg")).unwrap();
    assert_eq!(
        gpkg.layer_by_name("survey_points").unwrap().feature_count(),
        4
    );
    let mut elevs = Vec::new();
    for feature in gpkg.layer_by_name("contour_lines").unwrap().features() {
        match feature.field(0).unwrap() {
            Some(FieldValue::RealValue(z)) => elevs.push((z * 100.0).round() as i32),
            other => panic!("elev field: {other:?}"),
        }
    }
    elevs.sort();
    assert_eq!(elevs, vec![200, 400, 600, 800]);

    let tif = Dataset::open(dir.path().join("surface.tif")).unwrap();
    assert_eq!(tif.spatial_ref().unwrap().auth_code().unwrap(), 5187);
    assert_eq!(tif.rasterband(1).unwrap().no_data_value(), Some(-9999.0));
    assert!(tif.raster_size().0 >= 2 && tif.raster_size().1 >= 2);
    let _ = DriverManager::get_driver_by_name("GTiff");
}

#[test]
fn two_points_write_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let err = write_contour_package(
        dir.path(),
        5187,
        &[[0.0, 0.0, 1.0], [1.0, 1.0, 2.0]],
        1.0,
        1.0,
        0.0,
        &[],
    )
    .unwrap_err();
    assert!(err.0.contains("3개"), "{err}");
    assert!(!dir.path().join("surface.tif").exists());
    assert!(!dir.path().join("contours.gpkg").exists());
}
