//! R2-1a — submit SHP through GDAL equals the cpp package byte for byte (except the DBF date, bytes 1..3).
//! Ported from target/cpp-ref/tests/test_submit_package.cpp (layerWithoutCrsStopsThePackage,
//! koreanFieldNamesCutOnCodePoints, verifyWrittenCatchesAWrongCount) and test_schema_version.cpp
//! (schemaTable_isShapefileSafeAndOptional). Tempdir copies only.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use gdal::spatial_ref::SpatialRef;
use gdal::vector::{Geometry, LayerAccess, LayerOptions, OGRwkbGeometryType};
use gdal::DriverManager;

use super::{verify_written, write_submit_shapefiles, ShpLayerReport, SubmitEncoding, SubmitSource};
use crate::submit_shp_names::{field_name_for, left_utf8};

const KEYS: [&str; 7] =
    ["survey_area", "feature_poly", "feature_line", "control_points", "section_line", "artifact_point", "trial_trench"];

fn manifest(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)
}

fn demo_copy(dir: &Path) -> PathBuf {
    let gpkg = dir.join("demo.gpkg");
    std::fs::copy(manifest("../../samples/demo_survey/demo.gpkg"), &gpkg).expect("copy demo");
    gpkg
}

fn demo_sources(gpkg: &Path) -> Vec<(&'static str, Vec<SubmitSource>)> {
    KEYS.iter().map(|k| (*k, vec![SubmitSource { gpkg: gpkg.to_path_buf(), table: (*k).to_owned() }])).collect()
}

/// A GPKG with one polygon layer `t` (optionally without CRS) holding `features` squares.
fn small_gpkg(path: &Path, epsg: Option<u32>, features: usize) -> SubmitSource {
    let driver = DriverManager::get_driver_by_name("GPKG").expect("GPKG driver");
    let mut ds = driver.create_vector_only(path).expect("create gpkg");
    let srs = epsg.map(|c| SpatialRef::from_epsg(c).expect("srs"));
    let mut layer = ds
        .create_layer(LayerOptions { name: "t", srs: srs.as_ref(), ty: OGRwkbGeometryType::wkbPolygon, options: None })
        .expect("layer");
    for i in 0..features {
        let x = 200_000.0 + 20.0 * i as f64;
        let wkt = format!("POLYGON(({x} 450000,{} 450000,{} 450010,{x} 450010,{x} 450000))", x + 10.0, x + 10.0);
        layer.create_feature(Geometry::from_wkt(&wkt).expect("wkt")).expect("feature");
    }
    SubmitSource { gpkg: path.to_path_buf(), table: "t".to_owned() }
}

#[test]
fn demo_shapefiles_equal_the_cpp_package() {
    let dir = tempfile::tempdir().expect("temp dir");
    let gpkg = demo_copy(dir.path());
    let out = dir.path().join("out");
    std::fs::create_dir(&out).expect("out dir");
    let (reports, notes) = write_submit_shapefiles(&demo_sources(&gpkg), &out, SubmitEncoding::Utf8).expect("write");
    // README_submit.txt of the cpp package: shp_layers and shp_field_names.
    let counts = [1, 2, 1, 2, 1, 1, 1];
    let want: Vec<ShpLayerReport> = KEYS.iter().zip(counts).map(|(k, n)| ShpLayerReport { key: k, features: n }).collect();
    assert_eq!(reports, want);
    assert_eq!(notes, ["survey_area survey_name=surv_name", "artifact_point artifact_no=artif_no"]);
    let fixture = manifest("tests/fixtures/cpp_submit_demo");
    // Every differing file with its length and first differing byte; text files also show both texts.
    let mut diffs = Vec::new();
    for key in KEYS {
        for ext in ["shp", "shx", "dbf", "prj", "cpg"] {
            let file = format!("{key}.{ext}");
            let mut got = std::fs::read(out.join(&file)).expect(&file);
            let mut cpp = std::fs::read(fixture.join(&file)).expect(&file);
            if ext == "dbf" && got.len() > 3 && cpp.len() > 3 {
                // DBF header bytes 1..3 = last-update date (YY MM DD).
                got[1..4].fill(0);
                cpp[1..4].fill(0);
            }
            if got != cpp {
                let first = got.iter().zip(&cpp).position(|(a, b)| a != b);
                let mut line = format!("{file}: len {} vs cpp {}, first diff {first:?}", got.len(), cpp.len());
                if matches!(ext, "prj" | "cpg") {
                    line += &format!("\n  got {}\n  cpp {}", String::from_utf8_lossy(&got), String::from_utf8_lossy(&cpp));
                }
                diffs.push(line);
            }
        }
    }
    assert!(diffs.is_empty(), "differs from the cpp package:\n{}", diffs.join("\n"));
    let mut written: Vec<_> = std::fs::read_dir(&out).expect("list").map(|e| e.expect("entry").file_name()).collect();
    written.sort();
    assert_eq!(written.len(), 35, "{written:?}");
}

/// ExportService.cpp:147-153 — a key whose layers have no feature writes no file and no README line;
/// an empty layer without CRS is skipped before the CRS check.
#[test]
fn key_without_features_writes_nothing() {
    let dir = tempfile::tempdir().expect("temp dir");
    let gpkg = demo_copy(dir.path());
    let empty = small_gpkg(&dir.path().join("empty.gpkg"), Some(5187), 0);
    let empty_no_crs = small_gpkg(&dir.path().join("empty_no_crs.gpkg"), None, 0);
    let sources = vec![
        ("survey_area", vec![SubmitSource { gpkg: gpkg.clone(), table: "survey_area".to_owned() }]),
        ("section_line", vec![empty, empty_no_crs]),
    ];
    let out = dir.path().join("out");
    std::fs::create_dir(&out).expect("out dir");
    let (reports, _) = write_submit_shapefiles(&sources, &out, SubmitEncoding::Utf8).expect("write");
    assert_eq!(reports, vec![ShpLayerReport { key: "survey_area", features: 1 }]);
    assert!(std::fs::read_dir(&out).expect("list").all(|e| !e.expect("entry").file_name().to_string_lossy().starts_with("section_line")));
}

/// test_submit_package.cpp:78-94 — a layer without CRS stops the package with the cpp text.
#[test]
fn layer_without_crs_stops_with_cpp_text() {
    let dir = tempfile::tempdir().expect("temp dir");
    let src = small_gpkg(&dir.path().join("no_crs.gpkg"), None, 1);
    let ds = gdal::Dataset::open(&src.gpkg).expect("open");
    assert!(ds.layer_by_name("t").expect("t").spatial_ref().is_none(), "the test needs a layer that really has no CRS");
    drop(ds);
    let out = dir.path().join("out");
    std::fs::create_dir(&out).expect("out dir");
    let err = write_submit_shapefiles(&[("survey_area", vec![src])], &out, SubmitEncoding::Utf8).expect_err("no CRS");
    assert_eq!(
        err.0,
        "「t」(survey_area) 레이어에 좌표계가 없어 EPSG:5179로 바꿀 수 없습니다. 레이어 좌표계를 지정한 뒤 다시 만드세요."
    );
    assert_eq!(std::fs::read_dir(&out).expect("list").count(), 0);
}

/// Several non-empty layers per key and EUC-KR wait for R2-1b.
#[test]
fn merge_and_euc_kr_wait_for_r2_1b() {
    let dir = tempfile::tempdir().expect("temp dir");
    let a = small_gpkg(&dir.path().join("a.gpkg"), Some(5187), 1);
    let b = small_gpkg(&dir.path().join("b.gpkg"), Some(5187), 1);
    let out = dir.path().join("out");
    std::fs::create_dir(&out).expect("out dir");
    assert!(write_submit_shapefiles(&[("survey_area", vec![a.clone(), b])], &out, SubmitEncoding::Utf8).is_err());
    assert!(write_submit_shapefiles(&[("survey_area", vec![a])], &out, SubmitEncoding::EucKr).is_err());
}

/// test_submit_package.cpp:271-286 verifyWrittenCatchesAWrongCount.
#[test]
fn verify_written_catches_a_wrong_count() {
    let fixture = manifest("tests/fixtures/cpp_submit_demo/survey_area.shp");
    verify_written(&fixture, 1).expect("cpp file is EPSG:5179 with one feature");
    let err = verify_written(&fixture, 2).expect_err("wrong count");
    assert_eq!(err.0, "survey_area.shp에 도형이 2개 기록돼야 하는데 1개입니다.");
    let missing = verify_written(Path::new("Z:/없는/survey_area.shp"), 1).expect_err("missing");
    assert_eq!(missing.0, "방금 쓴 survey_area.shp을(를) 다시 열지 못했습니다.");
}

/// test_submit_package.cpp:259-269 koreanFieldNamesCutOnCodePoints.
#[test]
fn korean_field_names_cut_on_code_points() {
    assert_eq!(left_utf8("조사구역이름", 10), "조사구");
    let mut used = HashSet::new();
    let a = field_name_for("유구종류메모", &mut used);
    let b = field_name_for("유구종류메모2", &mut used);
    for name in [&a, &b] {
        assert!(name.len() <= 10, "{name}");
    }
    assert_ne!(a, b);
}

/// test_schema_version.cpp:113-131 — every schema field fits DBF's 10 bytes; only the two legacy names need an alias.
#[test]
fn schema_fields_are_shapefile_safe() {
    let yaml = include_str!("../../../data/schemas/ka_hgis_layers.yaml");
    let mut layers = 0;
    let mut used = HashSet::new();
    for line in yaml.lines() {
        if line.trim_start().starts_with("- id:") {
            layers += 1;
            used.clear();
        } else if let Some(rest) = line.trim_start().strip_prefix("- {name: ") {
            let name = rest.split(',').next().expect("name");
            let dbf = field_name_for(name, &mut used);
            assert!(dbf.len() <= 10, "{name} -> {dbf}");
            assert!(dbf == name || name == "survey_name" || name == "artifact_no", "{name}: no alias but over 10 bytes");
        }
    }
    assert_eq!(layers, 7);
}

/// The PROJ folder reaches GDAL through OSRSetPROJSearchPaths (no environment variable): re-setting the
/// folders GDAL already uses keeps EPSG:5179 resolvable.
#[test]
#[allow(unsafe_code)] // reads GDAL's current PROJ folders to hand them back unchanged
fn proj_search_paths_go_through_the_gdal_api() {
    let mut current: Vec<PathBuf> = Vec::new();
    // SAFETY: OSRGetPROJSearchPaths returns a NULL-terminated string list we own and free with CSLDestroy.
    unsafe {
        let list = gdal_sys::OSRGetPROJSearchPaths();
        let mut i = 0;
        while !list.is_null() && !(*list.add(i)).is_null() {
            current.push(PathBuf::from(std::ffi::CStr::from_ptr(*list.add(i)).to_string_lossy().into_owned()));
            i += 1;
        }
        gdal_sys::CSLDestroy(list);
    }
    let with_db: Vec<&Path> = current.iter().map(PathBuf::as_path).filter(|p| p.join("proj.db").is_file()).collect();
    assert!(!with_db.is_empty(), "GDAL PROJ folders: {current:?}");
    super::set_gdal_proj_search_paths(&with_db).expect("set");
    SpatialRef::from_epsg(5179).expect("EPSG:5179 after re-setting the PROJ folders");
}
