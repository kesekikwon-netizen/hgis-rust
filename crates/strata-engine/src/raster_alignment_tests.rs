//! C++ outputs: target/cpp-ref/tests/test_georef.cpp:160-195 and test_georef_backup.cpp.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::*;
use gdal::{raster::Buffer, spatial_ref::SpatialRef, DriverManager};

fn image(dir: &Path, driver: &str, filename: &str, embedded: bool) -> PathBuf {
    let mut mem = DriverManager::get_driver_by_name("MEM").unwrap().create_with_band_type::<u8, _>("", 100, 50, 3).unwrap();
    for band in 1..=3 {
        mem.rasterband(band).unwrap().write((0, 0), (100, 50), &mut Buffer::new((100, 50), vec![120; 5000])).unwrap();
    }
    if embedded {
        mem.set_geo_transform(&[219000.0, 0.5, 0.0, 360000.0, 0.0, -0.5]).unwrap();
        mem.set_spatial_ref(&SpatialRef::from_epsg(5187).unwrap()).unwrap();
    }
    let path = dir.join(filename);
    mem.create_copy(&DriverManager::get_driver_by_name(driver).unwrap(), &path, &Default::default()).unwrap();
    path
}

const FIT: [f64; 6] = [200000.0, 2.0, 0.0, 450100.0, 0.0, -2.0];

#[test]
fn world_file_places_the_real_png_at_the_cpp_extent_without_changing_source_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let source = image(dir.path(), "PNG", "scan.png", false);
    let bytes = std::fs::read(&source).unwrap();
    std::fs::write(source.with_extension("pgw"), b"1\n0\n0\n-1\n5\n5\n").unwrap();
    let out = create_aligned_raster(&source, &dir.path().join("results"), FIT, 5186).expect("real protected-source persistence");
    let values: Vec<f64> = std::fs::read_to_string(out.file.with_extension("pgw")).unwrap().lines().map(|s| s.parse().unwrap()).collect();
    assert_eq!(values, [2.0, 0.0, 0.0, -2.0, 200001.0, 450099.0], "world file uses pixel centres, not corners");
    let actual = crate::picture_info(&out.file).unwrap();
    assert_eq!((actual.width, actual.height, actual.epsg), (100, 50, Some(5186)));
    assert_eq!(actual.transform, Some(FIT));
    assert_eq!(std::fs::read(&source).unwrap(), bytes);
    assert_eq!(std::fs::read(source.with_extension("pgw")).unwrap(), b"1\n0\n0\n-1\n5\n5\n");
    assert_eq!(std::fs::read(out.backup.join("scan.pgw")).unwrap(), b"1\n0\n0\n-1\n5\n5\n");
}

#[test]
fn embedded_geotiff_gets_the_fit_and_work_crs_only_on_the_new_copy() {
    let dir = tempfile::tempdir().unwrap();
    let source = image(dir.path(), "GTiff", "aerial.tif", true);
    let bytes = std::fs::read(&source).unwrap();
    let out = create_aligned_raster(&source, &dir.path().join("results"), FIT, 5186).expect("embedded TIFF must not override the new fit");
    let actual = crate::picture_info(&out.file).unwrap();
    assert_eq!(actual.transform, Some(FIT));
    assert_eq!(actual.epsg, Some(5186));
    assert_eq!(crate::read_picture(&source, [0, 0, 100, 50], [10, 5]).unwrap(), crate::read_picture(&out.file, [0, 0, 100, 50], [10, 5]).unwrap());
    assert_eq!(std::fs::read(&source).unwrap(), bytes, "source metadata must remain byte-for-byte unchanged");
}

#[test]
fn repeated_fits_are_absolute_and_restore_keeps_older_results_and_backup() {
    let dir = tempfile::tempdir().unwrap();
    let source = image(dir.path(), "PNG", "scan.png", false);
    let root = dir.path().join("results");
    let first = create_aligned_raster(&source, &root, FIT, 5187).unwrap();
    let old = std::fs::read(first.file.with_extension("pgw")).unwrap();
    let second = create_aligned_raster(&source, &root, FIT, 5187).unwrap();
    assert_ne!(first.file, second.file);
    let changed = [200030.0, 1.0, 0.5, 450050.0, 0.25, -1.5];
    second.apply(changed, 5187).unwrap();
    second.apply(changed, 5187).unwrap();
    let actual = crate::picture_info(&second.file).unwrap().transform.unwrap();
    // C++ georef tests accept1e-6 map metres. This fixed affine is exactly representable in f64.
    for (got, want) in actual.into_iter().zip(changed) {
        assert!((got - want).abs() < 1e-6);
    }
    second.restore().unwrap();
    assert!(!second.file.exists() && !second.file.with_extension("pgw").exists() && !second.file.with_extension("prj").exists());
    assert!(source.exists() && second.backup.exists());
    assert_eq!(std::fs::read(first.file.with_extension("pgw")).unwrap(), old);
}

#[test]
fn invalid_crs_or_flat_transform_cannot_create_any_result() {
    let dir = tempfile::tempdir().unwrap();
    let source = image(dir.path(), "PNG", "scan.png", false);
    let root = dir.path().join("results");
    for (transform, epsg) in [(FIT, 999999), ([0.0; 6], 5187), ([f64::NAN; 6], 5187)] {
        assert!(create_aligned_raster(&source, &root, transform, epsg).is_err());
    }
    assert!(!root.exists(), "invalid requests must fail before output allocation");
}

#[test]
fn failed_sidecar_write_preserves_the_previously_applied_coordinates() {
    let dir = tempfile::tempdir().unwrap();
    let source = image(dir.path(), "PNG", "scan.png", false);
    let out = create_aligned_raster(&source, &dir.path().join("results"), FIT, 5187).unwrap();
    let world_before = std::fs::read(out.file.with_extension("pgw")).unwrap();
    std::fs::remove_file(out.file.with_extension("prj")).unwrap();
    std::fs::create_dir(out.file.with_extension("prj")).unwrap();
    assert!(out.apply([100.0, 3.0, 0.0, 200.0, 0.0, -3.0], 5187).is_err(), "blocked destination must report failure");
    assert_eq!(std::fs::read(out.file.with_extension("pgw")).unwrap(), world_before, "failed apply changed the previous valid placement");
}
