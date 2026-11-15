//! Pictures of the user read through GDAL: what the file is (size, where its pixels lie, its CRS) and the part a
//! view needs, read at the size it is shown at.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use gdal::raster::Buffer;
use gdal::spatial_ref::SpatialRef;
use gdal::DriverManager;

use super::{picture_info, read_picture};

/// A 4 × 2 picture of known colours: red, green, blue, white on the top row; black, grey, yellow, white below.
const PIXELS: [[u8; 3]; 8] = [[255, 0, 0], [0, 255, 0], [0, 0, 255], [255, 255, 255], [0, 0, 0], [128, 128, 128], [255, 255, 0], [255, 255, 255]];

/// The picture in memory, three bands.
fn memory_picture(transform: Option<[f64; 6]>, epsg: Option<u32>) -> gdal::Dataset {
    let mut picture = DriverManager::get_driver_by_name("MEM").unwrap().create_with_band_type::<u8, _>("", 4, 2, 3).unwrap();
    for band in 0..3 {
        let values: Vec<u8> = PIXELS.iter().map(|pixel| pixel[band]).collect();
        picture.rasterband(band + 1).unwrap().write((0, 0), (4, 2), &mut Buffer::new((4, 2), values)).unwrap();
    }
    if let Some(transform) = transform {
        picture.set_geo_transform(&transform).unwrap();
    }
    if let Some(code) = epsg {
        picture.set_spatial_ref(&SpatialRef::from_epsg(code).unwrap()).unwrap();
    }
    picture
}

fn copy_as(picture: &gdal::Dataset, driver: &str, path: &Path) -> PathBuf {
    picture.create_copy(&DriverManager::get_driver_by_name(driver).unwrap(), path, &Default::default()).unwrap();
    path.to_path_buf()
}

/// A GeoTIFF names its own transform and CRS.
#[test]
fn a_geotiff_names_where_its_pixels_lie_and_its_crs() {
    let dir = tempfile::tempdir().unwrap();
    let gt = [219_000.0, 0.5, 0.0, 360_000.0, 0.0, -0.5];
    let tif = copy_as(&memory_picture(Some(gt), Some(5187)), "GTiff", &dir.path().join("plan.tif"));
    let info = picture_info(&tif).unwrap();
    assert_eq!((info.width, info.height, info.bands), (4, 2, 3));
    assert_eq!(info.transform, Some(gt));
    assert_eq!(info.epsg, Some(5187));
}

/// A JPEG with a world file beside it: the world file says where its pixels lie (a turned picture too); the
/// world file gives the centre of the first pixel, the transform its corner. A PNG with none has no transform.
#[test]
fn a_world_file_places_a_picture_and_none_leaves_it_unplaced() {
    let dir = tempfile::tempdir().unwrap();
    let jpg = copy_as(&memory_picture(None, None), "JPEG", &dir.path().join("scan.jpg"));
    // 0.5 m pixels turned by 30 degrees; the first pixel's centre at (219 100, 360 200).
    let (a, d, b, e) = (0.5 * 30_f64.to_radians().cos(), 0.5 * 30_f64.to_radians().sin(), 0.5 * 30_f64.to_radians().sin(), -0.5 * 30_f64.to_radians().cos());
    std::fs::write(dir.path().join("scan.jgw"), format!("{a}\n{d}\n{b}\n{e}\n219100\n360200\n")).unwrap();
    let info = picture_info(&jpg).unwrap();
    let gt = info.transform.expect("the world file");
    let want = [219_100.0 - a / 2.0 - b / 2.0, a, b, 360_200.0 - d / 2.0 - e / 2.0, d, e];
    assert!((0..6).all(|i| (gt[i] - want[i]).abs() < 1e-6), "{gt:?} against {want:?}");
    assert_eq!(info.epsg, None);

    let png = copy_as(&memory_picture(None, None), "PNG", &dir.path().join("photo.png"));
    assert_eq!(picture_info(&png).unwrap().transform, None);
}

/// The whole picture at its size is its pixels, opaque; at half its size the pixel under the middle of each
/// shown pixel (GDAL's nearest neighbour, as QGIS draws a raster by default).
#[test]
fn a_window_is_read_at_the_size_it_is_shown_at() {
    let dir = tempfile::tempdir().unwrap();
    let tif = copy_as(&memory_picture(Some([0.0, 1.0, 0.0, 2.0, 0.0, -1.0]), None), "GTiff", &dir.path().join("plan.tif"));
    let whole = read_picture(&tif, [0, 0, 4, 2], [4, 2]).unwrap();
    let want: Vec<u8> = PIXELS.iter().flat_map(|[r, g, b]| [*r, *g, *b, 255]).collect();
    assert_eq!(whole, want);
    let half = read_picture(&tif, [0, 0, 4, 2], [2, 1]).unwrap();
    assert_eq!(half, [128, 128, 128, 255, 255, 255, 255, 255]);
    let corner = read_picture(&tif, [2, 1, 2, 1], [2, 1]).unwrap();
    assert_eq!(corner, [255, 255, 0, 255, 255, 255, 255, 255]);
}

/// A file GDAL cannot read as a picture is an error.
#[test]
fn a_broken_picture_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("broken.jpg");
    std::fs::write(&path, b"no picture").unwrap();
    assert!(picture_info(&path).is_err());
}
