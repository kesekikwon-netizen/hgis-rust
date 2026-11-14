//! R2-1a — submit SHP through GDAL equals the cpp package byte for byte (except the DBF date, bytes 1..3).
//! Ported from target/cpp-ref/tests/test_submit_package.cpp (layerWithoutCrsStopsThePackage,
//! koreanFieldNamesCutOnCodePoints, verifyWrittenCatchesAWrongCount) and test_schema_version.cpp
//! (schemaTable_isShapefileSafeAndOptional). Tempdir copies only. Shared helpers for submit_shp_types_tests.rs.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use gdal::spatial_ref::SpatialRef;
use gdal::vector::{LayerAccess, LayerOptions, OGRwkbGeometryType};
use gdal::DriverManager;

use super::{verify_written, write_submit_shapefiles, ShpLayerReport, SubmitEncoding, SubmitSource};
use crate::submit_shp_names::{field_name_for, left_utf8};

pub(super) const KEYS: [&str; 7] =
    ["survey_area", "feature_poly", "feature_line", "control_points", "section_line", "artifact_point", "trial_trench"];

/// Feature count per key in the cpp demo package (README_submit.txt shp_layers).
pub(super) const DEMO_COUNTS: [u64; 7] = [1, 2, 1, 2, 1, 1, 1];

/// Field notes of the cpp demo package (README_submit.txt shp_field_names).
pub(super) const DEMO_NOTES: [&str; 2] = ["survey_area survey_name=surv_name", "artifact_point artifact_no=artif_no"];

pub(super) fn manifest(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)
}

/// Copies `samples/<rel>` to `dir/<file name>`.
pub(super) fn sample_copy(dir: &Path, rel: &str) -> PathBuf {
    let from = manifest(&format!("../../samples/{rel}"));
    let to = dir.join(from.file_name().expect("file name"));
    std::fs::copy(&from, &to).expect("copy sample");
    to
}

/// One source per key, table = key, title = key.
pub(super) fn key_sources(gpkg: &Path, keys: &[&'static str]) -> Vec<(&'static str, Vec<SubmitSource>)> {
    keys.iter()
        .map(|k| (*k, vec![SubmitSource { gpkg: gpkg.to_path_buf(), table: (*k).to_owned(), title: (*k).to_owned() }]))
        .collect()
}

/// A GPKG with one polygon layer `t` titled `조사구역` (optionally without CRS) holding `features` squares.
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
        layer.create_feature(gdal::vector::Geometry::from_wkt(&wkt).expect("wkt")).expect("feature");
    }
    SubmitSource { gpkg: path.to_path_buf(), table: "t".to_owned(), title: "조사구역".to_owned() }
}

/// The PROJ folder is a process-wide GDAL setting: tests that use GDAL run one at a time.
static GDAL: Mutex<()> = Mutex::new(());

pub(super) fn gdal_lock() -> MutexGuard<'static, ()> {
    GDAL.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Writes `sources` into `dir/out` and lists every difference from the cpp package in
/// `tests/fixtures/<fixture>`: the README layer lines (`counts`) and field notes, then each differing
/// file with its length, first differing byte and the bytes there (text files also show both texts),
/// then the file count.
pub(super) fn package_differences(
    dir: &Path,
    sources: &[(&'static str, Vec<SubmitSource>)],
    enc: SubmitEncoding,
    fixture: &str,
    counts: &[(&'static str, u64)],
    notes_want: &[&str],
) -> Vec<String> {
    let out = dir.join("out");
    std::fs::create_dir(&out).expect("out dir");
    let (reports, notes) = match write_submit_shapefiles(sources, &out, enc) {
        Ok(written) => written,
        Err(e) => return vec![format!("write: {e}")],
    };
    let mut diffs = Vec::new();
    let want: Vec<ShpLayerReport> = counts.iter().map(|(k, n)| ShpLayerReport { key: k, features: *n }).collect();
    if reports != want {
        diffs.push(format!("reports {reports:?}"));
    }
    if notes != notes_want {
        diffs.push(format!("notes {notes:?}"));
    }
    let fixture = manifest(&format!("tests/fixtures/{fixture}"));
    for (key, _) in counts {
        for ext in ["shp", "shx", "dbf", "prj", "cpg"] {
            let file = format!("{key}.{ext}");
            let mut got = std::fs::read(out.join(&file)).unwrap_or_default();
            let mut cpp = std::fs::read(fixture.join(&file)).expect(&file);
            if ext == "dbf" && got.len() > 3 && cpp.len() > 3 {
                // DBF header bytes 1..3 = last-update date (YY MM DD).
                got[1..4].fill(0);
                cpp[1..4].fill(0);
            }
            if got != cpp {
                let first = got.iter().zip(&cpp).position(|(a, b)| a != b);
                let mut line = format!("{file}: len {} vs cpp {}, first diff {first:?}", got.len(), cpp.len());
                if let Some(at) = first {
                    let end = |v: &[u8]| (at + 24).min(v.len());
                    line += &format!("\n  got {:?}\n  cpp {:?}", &got[at..end(&got)], &cpp[at..end(&cpp)]);
                }
                if matches!(ext, "prj" | "cpg") {
                    line += &format!("\n  got {}\n  cpp {}", String::from_utf8_lossy(&got), String::from_utf8_lossy(&cpp));
                }
                diffs.push(line);
            }
        }
    }
    let written = std::fs::read_dir(&out).expect("list").count();
    if written != counts.len() * 5 {
        diffs.push(format!("{written} files written, cpp has {}", counts.len() * 5));
    }
    diffs
}

/// The demo copy against tests/fixtures/cpp_submit_demo.
fn demo_differences(dir: &Path) -> Vec<String> {
    let gpkg = sample_copy(dir, "demo_survey/demo.gpkg");
    let counts: Vec<_> = KEYS.iter().copied().zip(DEMO_COUNTS).collect();
    package_differences(dir, &key_sources(&gpkg, &KEYS), SubmitEncoding::Utf8, "cpp_submit_demo", &counts, &DEMO_NOTES)
}

#[test]
fn demo_shapefiles_equal_the_cpp_package() {
    let _gdal = gdal_lock();
    let dir = tempfile::tempdir().expect("temp dir");
    let diffs = demo_differences(dir.path());
    assert!(diffs.is_empty(), "differs from the cpp package:\n{}", diffs.join("\n"));
}

/// ExportService.cpp:147-153 — a key whose layers have no feature writes no file and no README line;
/// an empty layer without CRS is skipped before the CRS check.
#[test]
fn key_without_features_writes_nothing() {
    let _gdal = gdal_lock();
    let dir = tempfile::tempdir().expect("temp dir");
    let gpkg = sample_copy(dir.path(), "demo_survey/demo.gpkg");
    let empty = small_gpkg(&dir.path().join("empty.gpkg"), Some(5187), 0);
    let empty_no_crs = small_gpkg(&dir.path().join("empty_no_crs.gpkg"), None, 0);
    let mut sources = key_sources(&gpkg, &["survey_area"]);
    sources.push(("section_line", vec![empty, empty_no_crs]));
    let out = dir.path().join("out");
    std::fs::create_dir(&out).expect("out dir");
    let (reports, _) = write_submit_shapefiles(&sources, &out, SubmitEncoding::Utf8).expect("write");
    assert_eq!(reports, vec![ShpLayerReport { key: "survey_area", features: 1 }]);
    assert!(std::fs::read_dir(&out).expect("list").all(|e| !e.expect("entry").file_name().to_string_lossy().starts_with("section_line")));
}

/// test_submit_package.cpp:78-94 — a layer without CRS stops the package with the cpp text, naming the
/// layer by its display name (ExportService.cpp:150-152 `vl->name()`).
#[test]
fn layer_without_crs_stops_with_cpp_text() {
    let _gdal = gdal_lock();
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
        "「조사구역」(survey_area) 레이어에 좌표계가 없어 EPSG:5179로 바꿀 수 없습니다. 레이어 좌표계를 지정한 뒤 다시 만드세요."
    );
    assert_eq!(std::fs::read_dir(&out).expect("list").count(), 0);
}

/// test_submit_package.cpp:271-286 verifyWrittenCatchesAWrongCount.
#[test]
fn verify_written_catches_a_wrong_count() {
    let _gdal = gdal_lock();
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

/// The `.prj` text follows the proj.db version: with GDAL's PROJ left on the bundled PROJ 9.6.2's own database the
/// datum is `GCS_KGD2002`; with the shipped PROJ 9.8.1 database (the one the cpp packages were made with) it is
/// `KGD2002`, and the demo package equals the cpp one. The shipped database works from a folder with Korean letters,
/// handed to GDAL through its `PROJ_DATA` setting (no env var, no unsafe). Runs in a child process so static GDAL's
/// open sqlite handle on `proj.db` closes on exit before the tempdir is removed.
#[test]
fn shipped_proj_db_gives_the_cpp_prj_text() {
    const SUB_DIR: &str = "STRATA_TEST_SHIPPED_PROJ_SUBDIR";
    if let Some(sub) = std::env::var_os(SUB_DIR) {
        let esri_wkt = || {
            std::thread::spawn(|| {
                let srs = SpatialRef::from_epsg(5179).expect("EPSG:5179");
                srs.morph_to_esri().expect("ESRI");
                srs.to_wkt().expect("WKT")
            })
            .join()
            .expect("thread")
        };
        let root = PathBuf::from(sub);
        let korean = root.join("조사 자료").join("proj");
        let work = root.join("work");
        std::fs::create_dir(&work).expect("work dir");
        let bundled = esri_wkt();
        assert!(bundled.contains(r#"GEOGCS["GCS_KGD2002""#), "{bundled}");
        super::install_submit_proj_db(&korean).expect("install proj.db");
        let shipped = esri_wkt();
        assert!(shipped.contains(r#"GEOGCS["KGD2002""#), "{shipped}");
        let diffs = std::thread::spawn(move || demo_differences(&work)).join().expect("thread");
        assert!(diffs.is_empty(), "differs from the cpp package:\n{}", diffs.join("\n"));
        return;
    }
    let dir = tempfile::tempdir().expect("temp dir");
    let out = std::process::Command::new(std::env::current_exe().expect("test exe"))
        .args(["--exact", "submit_shp::tests::shipped_proj_db_gives_the_cpp_prj_text", "--nocapture"])
        .env(SUB_DIR, dir.path())
        .env_remove("PROJ_DATA")
        .env_remove("PROJ_LIB")
        .output()
        .expect("run child");
    assert!(
        out.status.success(),
        "child failed:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(std::fs::read(dir.path().join("조사 자료/proj/proj.db")).expect("proj.db").len(), 10_280_960);
    dir.close().expect("remove test tempdir cleanly");
}

/// The GDAL inside is the bundled 3.12.1 built with only the drivers the submit files need, and without GEOS
/// (an OSGeo4W GDAL is 3.14 and brings PDF, MySQL and PostgreSQL drivers and GEOS).
#[test]
fn bundled_gdal_is_3_12_1_with_the_needed_drivers_only() {
    assert_eq!(gdal::version::version_info("RELEASE_NAME"), "3.12.1");
    let drivers: Vec<String> =
        (0..DriverManager::count()).map(|i| DriverManager::get_driver(i).expect("driver").short_name()).collect();
    for need in ["ESRI Shapefile", "GPKG", "SQLite"] {
        assert!(drivers.iter().any(|d| d == need), "{need} missing: {drivers:?}");
    }
    for never in ["PDF", "MySQL", "PostgreSQL", "netCDF", "HDF5"] {
        assert!(!drivers.iter().any(|d| d == never), "{never} built in: {drivers:?}");
    }
    let build = gdal::version::version_info("BUILD_INFO");
    assert!(!gdal::version::VersionInfo::has_geos(), "{build}");
    assert!(!build.contains("GEOS_ENABLED") && !build.contains("CURL_ENABLED"), "{build}");
    let square = gdal::vector::Geometry::from_wkt("POLYGON((0 0,1 0,1 1,0 1,0 0))").expect("wkt");
    assert!(!square.is_valid(), "OGR geometry validity check succeeded, so GEOS was linked into GDAL");
}
