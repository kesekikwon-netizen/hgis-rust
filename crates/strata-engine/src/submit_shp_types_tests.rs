//! R2-1b — every GPKG field type written as in the cpp packages, EUC-KR (CP949), and several layers per key
//! merged (SubmitShp.cpp:82-133). Truth = cpp packages in tests/fixtures (dbf bytes 1..3 masked).
//! Ported from target/cpp-ref/tests/test_submit_package.cpp:168 mergeKeepsFieldsOfEveryLayerAndReadmeRecordsProvenance.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use gdal::spatial_ref::SpatialRef;
use gdal::vector::sql::Dialect;
use gdal::vector::{Feature, FieldDefn, Geometry, LayerAccess, LayerOptions, OGRFieldType, OGRwkbGeometryType};
use gdal::{Dataset, DatasetOptions, DriverManager, GdalOpenFlags};

use super::tests::{gdal_lock, key_sources, package_differences, sample_copy, DEMO_COUNTS, DEMO_NOTES, KEYS};
use super::{write_submit_shapefiles, ShpLayerReport, SubmitEncoding, SubmitSource};

/// The demo copy whose feature_poly gains the typed columns of the cpp fixture recipe
/// (docs/senior/ref-files/cpp-submit-package-types-UTF-8-5187/make-fixture.ps1), same SQL; `long` = c_long.
fn types_gpkg(dir: &Path, long: &str) -> PathBuf {
    let gpkg = sample_copy(dir, "demo_survey/demo.gpkg");
    let flags = GdalOpenFlags::GDAL_OF_UPDATE | GdalOpenFlags::GDAL_OF_VECTOR;
    let ds = Dataset::open_ex(&gpkg, DatasetOptions { open_flags: flags, ..DatasetOptions::default() }).expect("open");
    for c in ["c_int MEDIUMINT", "c_i64 INTEGER", "c_real REAL", "c_date DATE", "c_dt DATETIME", "c_bool BOOLEAN", "c_t20 TEXT(20)", "c_long TEXT"] {
        ds.execute_sql(format!("ALTER TABLE feature_poly ADD COLUMN {c}"), None, Dialect::SQLITE).expect("alter");
    }
    let update = format!(
        "UPDATE feature_poly SET c_int=42, c_i64=9000000000, c_real=1.25, c_date='2026-11-14', \
         c_dt='2026-11-14T08:30:15Z', c_bool=1, c_t20='짧은글', c_long='{long}'"
    );
    ds.execute_sql(update, None, Dialect::SQLITE).expect("update");
    gpkg
}

fn types_differences(enc: SubmitEncoding, fixture: &str, long: &str) -> Vec<String> {
    let dir = tempfile::tempdir().expect("temp dir");
    let gpkg = types_gpkg(dir.path(), long);
    let counts: Vec<_> = KEYS.iter().copied().zip(DEMO_COUNTS).collect();
    package_differences(dir.path(), &key_sources(&gpkg, &KEYS), enc, fixture, &counts, &DEMO_NOTES)
}

/// Integer N(10,0), Integer64 N(20,0), Real N(24,15), Date D(8), DateTime C(24), Boolean L(1), TEXT(20) C(20), TEXT C(254).
#[test]
fn field_types_equal_the_cpp_package_utf8() {
    let _gdal = gdal_lock();
    let diffs = types_differences(SubmitEncoding::Utf8, "cpp_submit_types_utf8", &"가".repeat(120));
    assert!(diffs.is_empty(), "differs from the cpp package:\n{}", diffs.join("\n"));
}

#[test]
fn field_types_equal_the_cpp_package_euc_kr() {
    let _gdal = gdal_lock();
    let diffs = types_differences(SubmitEncoding::EucKr, "cpp_submit_types_euckr", &"가".repeat(120));
    assert!(diffs.is_empty(), "differs from the cpp package:\n{}", diffs.join("\n"));
}

/// cpp-submit-package-long-EUC-KR-5187: c_long = 'a' + '가' x130 (261 CP949 bytes) in C(254) is cut at the
/// byte width, the last 2-byte character in half: 61 + (b0a1 x126) + b0.
#[test]
fn long_euc_kr_text_is_cut_at_the_byte_width() {
    let _gdal = gdal_lock();
    let long = format!("a{}", "가".repeat(130));
    let diffs = types_differences(SubmitEncoding::EucKr, "cpp_submit_long_euckr", &long);
    assert!(diffs.is_empty(), "differs from the cpp package:\n{}", diffs.join("\n"));
}

/// samples/bad_survey/bad.gpkg, work EPSG:5186, EUC-KR: only survey_area and feature_poly have features.
#[test]
fn bad_survey_euc_kr_5186_equals_the_cpp_package() {
    let _gdal = gdal_lock();
    let dir = tempfile::tempdir().expect("temp dir");
    let gpkg = sample_copy(dir.path(), "bad_survey/bad.gpkg");
    let counts = [("survey_area", 1), ("feature_poly", 1)];
    let notes = ["survey_area survey_name=surv_name"];
    let diffs = package_differences(
        dir.path(),
        &key_sources(&gpkg, &KEYS),
        SubmitEncoding::EucKr,
        "cpp_submit_bad_euckr_5186",
        &counts,
        &notes,
    );
    assert!(diffs.is_empty(), "differs from the cpp package:\n{}", diffs.join("\n"));
}

/// A GPKG in EPSG:5187 with one polygon layer `t` of TEXT(40) `fields`; each row = (x, y, values by field).
fn layer_gpkg(path: &Path, title: &str, fields: &[&str], rows: &[(f64, f64, &[&str])]) -> SubmitSource {
    let driver = DriverManager::get_driver_by_name("GPKG").expect("GPKG driver");
    let mut ds = driver.create_vector_only(path).expect("create gpkg");
    let srs = SpatialRef::from_epsg(5187).expect("srs");
    let layer = ds
        .create_layer(LayerOptions { name: "t", srs: Some(&srs), ty: OGRwkbGeometryType::wkbPolygon, options: None })
        .expect("layer");
    for name in fields {
        let fd = FieldDefn::new(name, OGRFieldType::OFTString).expect("field");
        fd.set_width(40);
        fd.add_to_layer(&layer).expect("add field");
    }
    for (x, y, values) in rows {
        let mut f = Feature::new(layer.defn()).expect("feature");
        for (i, v) in values.iter().enumerate() {
            f.set_field_string(i, v).expect("value");
        }
        let wkt = format!("POLYGON(({x} {y},{} {y},{} {},{x} {},{x} {y}))", x + 10.0, x + 10.0, y + 10.0, y + 10.0);
        f.set_geometry(Geometry::from_wkt(&wkt).expect("wkt")).expect("geometry");
        f.create(&layer).expect("create");
    }
    SubmitSource { gpkg: path.to_path_buf(), table: "t".to_owned(), title: title.to_owned() }
}

/// test_submit_package.cpp:168-214 — two survey areas under one key: one SHP with 2 features and the field
/// only the second layer has; README line features=2. Features in layer order, values by field name.
#[test]
fn merge_keeps_fields_of_every_layer() {
    let _gdal = gdal_lock();
    let dir = tempfile::tempdir().expect("temp dir");
    let first = layer_gpkg(&dir.path().join("a.gpkg"), "조사구역", &["name"], &[(200_000.0, 450_000.0, &["1구역"])]);
    let second = layer_gpkg(
        &dir.path().join("b.gpkg"),
        "조사구역 2",
        &["name", "memo"],
        &[(200_500.0, 450_500.0, &["2구역", "둘째만"])],
    );
    let out = dir.path().join("out");
    std::fs::create_dir(&out).expect("out dir");
    let (reports, notes) =
        write_submit_shapefiles(&[("survey_area", vec![first, second])], &out, SubmitEncoding::Utf8).expect("write");
    assert_eq!(reports, vec![ShpLayerReport { key: "survey_area", features: 2 }]);
    assert!(notes.is_empty(), "{notes:?}");
    let ds = Dataset::open(out.join("survey_area.shp")).expect("open written");
    let mut layer = ds.layer(0).expect("layer");
    let names: Vec<String> = layer.defn().fields().map(|f| f.name()).collect();
    assert_eq!(names, ["fid", "name", "memo"], "a field only the second layer has was dropped");
    let rows: Vec<(Option<String>, Option<String>)> =
        layer.features().map(|f| (f.field_as_string(1).expect("name"), f.field_as_string(2).expect("memo"))).collect();
    assert_eq!(
        rows,
        [(Some("1구역".to_owned()), None), (Some("2구역".to_owned()), Some("둘째만".to_owned()))]
    );
}
