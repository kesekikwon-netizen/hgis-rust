//! WU-06 — the package folder is the survey's EPSG:5179 shapefiles, not the placeholder square,
//! checklist errors stay in the README, and the map does not gain the package.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use gdal::vector::LayerAccess;
use strata_engine::{read_shapefile, Shapes, SubmitEncoding};

use super::{folder_name_part, package_folder_name, SubmitPhase};
use crate::app_state::StrataAppState;

fn demo(dir: &Path) -> std::path::PathBuf {
    let path = dir.join("demo.gpkg");
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg"),
        &path,
    )
    .expect("tracked demo copy");
    path
}

fn opened(dir: &Path) -> StrataAppState {
    let path = demo(dir);
    let mut app = StrataAppState::new_startup();
    assert!(app.open_survey_gpkg(&path));
    app
}

/// `SubmitReadme.cpp:106-110` and `KaSubmitFlow.cpp:124-130`.
#[test]
fn folder_name_keeps_korean_and_skips_a_taken_name() {
    assert_eq!(folder_name_part("  광령리/조사:1. "), "광령리_조사_1");
    assert_eq!(folder_name_part(&"가".repeat(41)).chars().count(), 40);
    let name = package_folder_name("demo", "20261118_120000", |n| {
        n == "제출_demo_20261118_120000"
    });
    assert_eq!(name, "제출_demo_20261118_120000_2");
    assert_eq!(
        package_folder_name("", "20261118_120000", |_| false),
        "제출_20261118_120000"
    );
}

/// `KaSubmitFlow.cpp:90-94` — declining the one question writes nothing.
#[test]
fn declining_the_checklist_question_writes_nothing() {
    let dir = tempfile::tempdir().expect("temp");
    let mut app = opened(dir.path());
    let parent = dir.path().join("out");
    std::fs::create_dir(&parent).expect("parent");
    app.submit_parent_override = Some(parent.clone());
    app.begin_submit_package();
    assert!(
        matches!(app.submit_phase, Some(SubmitPhase::Errors { .. })),
        "{:?}",
        app.submit_phase
    );
    app.cancel_submit_package();
    assert!(app.submit_phase.is_none());
    let names: Vec<_> = std::fs::read_dir(&parent)
        .expect("list")
        .map(|e| e.expect("entry").file_name())
        .collect();
    assert!(names.is_empty(), "{names:?}");
}

/// Re-open the written SHP: feature count and EPSG:5179, not the 0–100 square. The map is unchanged.
#[test]
fn package_reopens_as_5179_and_stays_off_the_map() {
    let dir = tempfile::tempdir().expect("temp");
    let mut app = opened(dir.path());
    let before: Vec<_> = app
        .left_layers
        .entries
        .iter()
        .map(|e| e.layer_key.clone())
        .collect();
    let survey = app.survey_path.clone().expect("survey");
    let original = std::fs::read(&survey).expect("bytes");
    std::fs::write(
        survey.parent().expect("dir").join("plan.dxf"),
        b"not a submit file",
    )
    .expect("dxf");
    app.is_dirty = true;
    let parent = dir.path().join("out");
    std::fs::create_dir(&parent).expect("parent");
    app.submit_parent_override = Some(parent.clone());
    app.begin_submit_package();
    app.accept_submit_errors();
    assert!(matches!(
        app.submit_phase,
        Some(SubmitPhase::Unsaved { .. })
    ));
    app.answer_submit_unsaved(false);
    let out = app
        .choose_submit_encoding(SubmitEncoding::Utf8)
        .expect("package");
    assert!(
        app.is_dirty,
        "저장하지 않고 만들기 leaves the survey unsaved"
    );
    assert_eq!(
        std::fs::read(&survey).expect("bytes"),
        original,
        "the survey file is not written"
    );
    assert_eq!(
        app.left_layers
            .entries
            .iter()
            .map(|e| e.layer_key.clone())
            .collect::<Vec<_>>(),
        before
    );
    assert!(app
        .left_layers
        .entries
        .iter()
        .all(|e| !e.layer_key.contains("제출_")));

    let names: Vec<String> = std::fs::read_dir(&out)
        .expect("pkg")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    assert!(names.iter().any(|n| n == "MANIFEST.sha256"), "{names:?}");
    assert!(names.iter().all(|n| !n.ends_with(".dxf")), "{names:?}");
    assert!(!out.join("조사도면.pdf").exists());
    assert!(!out.join("site_boundary_5179.shp").exists());
    let readme = String::from_utf8(std::fs::read(out.join("README_submit.txt")).expect("readme"))
        .expect("utf-8");
    assert!(readme.contains("checklist_errors_remain: yes"));
    assert!(readme.contains("조사도면.pdf 없음: 도면만들기에서 용지를 만든 뒤 다시 보내기 하세요."));
    assert!(readme.contains("saved_state: unsaved_edits_included"));
    let msg = app.home_map.draw_guide_message.clone().unwrap_or_default();
    assert!(
        msg.contains("도면 용지가 없어 조사도면.pdf는 빠졌습니다"),
        "{msg}"
    );

    let shp = out.join("survey_area.shp");
    let want = readme
        .lines()
        .find_map(|line| {
            line.strip_prefix("survey_area.shp features=")
                .and_then(|rest| rest.split_whitespace().next())
                .and_then(|n| n.parse::<u64>().ok())
        })
        .expect("readme count");
    let (count, outside) = reopen(&shp);
    assert_eq!(count, want);
    assert!(want > 0);
    assert!(outside, "the ring is the placeholder square");
}

fn reopen(shp: &Path) -> (u64, bool) {
    let count = {
        let ds = gdal::Dataset::open(shp).expect("shp");
        let layer = ds.layer(0).expect("layer");
        let srs = layer.spatial_ref().expect("srs");
        let code = if srs.auth_name().as_deref() == Some("EPSG") {
            srs.auth_code().ok()
        } else {
            let mut probe = srs.clone();
            probe.auto_identify_epsg().expect("identify");
            probe.auth_code().ok()
        };
        assert_eq!(code, Some(5179), "SubmitShp.cpp:193-196");
        layer.feature_count()
    };
    let mut outside = false;
    read_shapefile(shp, "UTF-8", &[], &mut |row| {
        if let Shapes::Polygons(polys) = &row.shapes {
            for poly in polys {
                for ring in poly {
                    if ring
                        .iter()
                        .any(|p| p[0] < 0.0 || p[1] < 0.0 || p[0] > 100.0 || p[1] > 100.0)
                    {
                        outside = true;
                    }
                }
            }
        }
        false
    })
    .expect("reopen rows");
    (count, outside)
}
