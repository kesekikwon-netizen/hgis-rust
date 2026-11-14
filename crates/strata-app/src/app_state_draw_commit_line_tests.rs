//! Line capture commit, CRS transform, undo/redo, and save/reopen round-trip tests
//! (`target/cpp-ref/src/app/KaCaptureMapTool.cpp:393-456`,
//! `target/cpp-ref/src/app/MainWindowEditing.cpp:782-920`).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use strata_engine::Transformer;
use strata_storage::gpkg_wkt::wkt_to_parts;
use strata_storage::wkb_codec::WkbGeometry;
use strata_types::crs::CrsCode;

use super::*;

fn opened_demo(dir: &Path) -> (StrataAppState, PathBuf) {
    let path = dir.join("demo.gpkg");
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg"),
        &path,
    )
    .expect("copy demo");
    let mut app = StrataAppState::new_startup();
    app.open_survey_gpkg(&path);
    app.active_tab = MainTab::Map;
    app.home_map.viewport.meters_per_pixel = 0.05;
    (app, path)
}

fn work_line_points(app: &StrataAppState) -> Vec<[f64; 2]> {
    let to_work = Transformer::new(5186, CrsCode::from(app.work_crs).epsg_number()).expect("PROJ");
    [[200_100.0, 450_100.0], [200_120.0, 450_100.0], [200_120.0, 450_130.0]]
        .iter()
        .map(|p| to_work.transform(*p).expect("transform"))
        .collect()
}

fn table_features<'a>(
    app: &'a StrataAppState,
    table: &str,
) -> &'a [strata_storage::edit_buffer::FeatureRecord] {
    &app.survey_tables.iter().find(|t| t.table == table).expect("table").layer.features
}

// Finish with >= 2 points in a 5187 view into a 5186 feature_line table: WKT LINESTRING in 5186,
// stamped uid/created_at/updated_at, last_drawn_shape untouched, form offered, card updated,
// and no missing-attribute counter suffix (target/cpp-ref/src/app/MainWindowEditing.cpp:845-914).
#[test]
fn finished_line_transforms_from_5187_to_5186_and_skips_last_drawn_shape() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut app, _) = opened_demo(dir.path());
    assert_eq!(CrsCode::from(app.work_crs).epsg_number(), 5187);
    app.start_edit_feature_line();
    for p in work_line_points(&app) {
        app.sketch_press(p);
    }
    app.sketch_finish();

    let feats = table_features(&app, "feature_line");
    assert_eq!(feats.len(), 2);
    let new = feats.last().unwrap();
    let new_fid = new.fid;
    assert!(new_fid < 0);
    assert!(new.wkt_geom.starts_with("LINESTRING ("), "{}", new.wkt_geom);
    let parts = wkt_to_parts(&new.wkt_geom).expect("wkt");
    let WkbGeometry::LineString(pts) = &parts[0] else { panic!("linestring") };
    let want = [[200_100.0, 450_100.0], [200_120.0, 450_100.0], [200_120.0, 450_130.0]];
    assert_eq!(pts.len(), 3);
    for (g, w) in pts.iter().zip(want) {
        assert!((g[0] - w[0]).hypot(g[1] - w[1]) < 1e-6, "{g:?} vs {w:?}");
    }
    assert_eq!(app.home_map.last_drawn_shape, None, "MainWindowEditing.cpp:845-849 polygon only");
    assert!(app.is_dirty);
    assert_eq!(app.unsaved_chip_text().as_deref(), Some("저장 안 됨 1건"));
    let entry = app.left_layers.entries.iter().find(|e| e.layer_key == "feature_line").unwrap();
    assert_eq!(entry.feature_count, 2);
    let name = entry.label_ko.clone();
    assert_eq!(app.status_left_message(), "도형은 넣었습니다. 이름·번호를 적거나 Esc로 건너뛰세요.");
    app.skip_feature_form();
    assert_eq!(app.home_map.card.as_ref().map(|c| (c.table.as_str(), c.fid, c.follows_selection)), Some(("feature_line", new_fid, false)));
    let msg = app.home_map.draw_guide_message.clone().unwrap();
    assert_eq!(
        msg,
        format!("도형을 넣었습니다 ({name}, 2개). Ctrl+Z로 되돌리기 · 「저장」(Ctrl+S)으로 파일에 씁니다")
    );
}

// Section line with continuous drawing skips the modal form and supports undo/redo
// (target/cpp-ref/src/app/MainWindowEditing.cpp:853-911).
#[test]
fn section_line_continuous_draw_and_undo_redo_follow_edit_buffer() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut app, _) = opened_demo(dir.path());
    app.ribbon.continuous_draw = true;
    app.start_edit_section_line();
    let pts = work_line_points(&app);
    app.sketch_press(pts[0]);
    app.sketch_press(pts[1]);
    app.sketch_finish();

    assert!(app.home_map.feature_form.is_none());
    assert_eq!(table_features(&app, "section_line").len(), 2);
    let msg = app.home_map.draw_guide_message.clone().unwrap();
    assert!(msg.ends_with(" · 이름·번호는 나중에 속성에서"), "{msg}");
    app.undo_map_action();
    assert_eq!(table_features(&app, "section_line").len(), 1);
    app.redo_map_action();
    assert_eq!(table_features(&app, "section_line").len(), 2);
}

// Empty finish does nothing; 1-point finish emits TOO_FEW_TEXT and keeps the vertex
// (target/cpp-ref/src/app/KaCaptureMapTool.cpp:398-402, target/cpp-ref/tests/test_editing_tools.cpp:135-146).
#[test]
fn one_point_line_finish_keeps_vertex_and_empty_finish_is_silent() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut app, _) = opened_demo(dir.path());
    app.start_edit_feature_line();
    let initial_msg = app.home_map.draw_guide_message.clone();
    app.sketch_finish();
    assert_eq!(app.home_map.draw_guide_message, initial_msg);
    assert!(!app.is_dirty);

    let pts = work_line_points(&app);
    app.sketch_press(pts[0]);
    app.sketch_finish();
    assert_eq!(app.home_map.draw_guide_message.as_deref(), Some(TOO_FEW_TEXT));
    assert_eq!(app.draw_sketch.points.len(), 1);
    assert_eq!(table_features(&app, "feature_line").len(), 1);
    assert!(!app.is_dirty);
}

// Save, close, reopen on a tempdir demo copy: feature_line has one more feature with matching coordinates.
#[test]
fn drawn_line_saves_to_gpkg_and_reopens_with_same_coordinates() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut app, path) = opened_demo(dir.path());
    app.start_edit_feature_line();
    for p in work_line_points(&app) {
        app.sketch_press(p);
    }
    app.sketch_finish();
    app.skip_feature_form();
    assert!(app.save_survey_project());
    assert!(!app.is_dirty);

    let conn = rusqlite::Connection::open(&path).expect("open");
    let count: i64 = conn.query_row("SELECT count(*) FROM feature_line", [], |r| r.get(0)).expect("count");
    let rtree: i64 = conn.query_row("SELECT count(*) FROM rtree_feature_line_geom", [], |r| r.get(0)).expect("rtree");
    assert_eq!((count, rtree), (2, 2));
    drop(conn);

    let mut reopened = StrataAppState::new_startup();
    reopened.open_survey_gpkg(&path);
    let feats = table_features(&reopened, "feature_line");
    assert_eq!(feats.len(), 2);
    let parts = wkt_to_parts(&feats.last().unwrap().wkt_geom).expect("wkt");
    let WkbGeometry::LineString(pts) = &parts[0] else { panic!("linestring") };
    let want = [[200_100.0, 450_100.0], [200_120.0, 450_100.0], [200_120.0, 450_130.0]];
    assert_eq!(pts.len(), 3);
    for (g, w) in pts.iter().zip(want) {
        assert!((g[0] - w[0]).hypot(g[1] - w[1]) < 1e-6, "{g:?} vs {w:?}");
    }

    // New survey schema has uid/created_at/updated_at on feature_line: verify stamps round-trip too.
    let new_path = dir.path().join("new_line.gpkg");
    let crs = strata_types::crs::WorkCrs::Epsg5187;
    strata_storage::gpkg::GpkgRepository::create_new_survey(&new_path, crs).expect("create");
    let mut fresh = StrataAppState::new_startup();
    fresh.start_new_survey_project_with_path("new_line", &new_path, crs);
    fresh.active_tab = MainTab::Map;
    fresh.home_map.viewport.meters_per_pixel = 0.05;
    fresh.start_edit_feature_line();
    fresh.sketch_press([21_100.0, 451_800.0]);
    fresh.sketch_press([21_125.0, 451_810.0]);
    fresh.sketch_finish();
    fresh.skip_feature_form();
    let stamped = &table_features(&fresh, "feature_line")[0].attributes;
    assert_eq!(stamped["uid"].len(), 36);
    assert!(!stamped["created_at"].is_empty());
    assert_eq!(stamped["created_at"], stamped["updated_at"]);
    assert!(fresh.save_survey_project());
}

// Ported from target/cpp-ref/tests/test_workflow.cpp:3310-3365 (captureClickOnSavedVertex_startsNewSketch):
// clicking on a saved line vertex starts a new sketch without altering the buffered feature.
#[test]
fn clicking_saved_line_vertex_starts_new_sketch_without_touching_existing_line() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut app, _) = opened_demo(dir.path());
    let fid = table_features(&app, "feature_line")[0].fid;
    let orig_wkt = table_features(&app, "feature_line")[0].wkt_geom.clone();
    let WkbGeometry::LineString(orig_pts) = &wkt_to_parts(&orig_wkt).unwrap()[0] else { panic!("line") };
    let to_work = Transformer::new(5186, CrsCode::from(app.work_crs).epsg_number()).expect("PROJ");
    let saved_vertex = to_work.transform(orig_pts[0]).expect("transform");

    let pending = [("note".to_owned(), Some("pending note".to_owned()))];
    let idx = app.survey_tables.iter().position(|t| t.table == "feature_line").unwrap();
    assert!(app.survey_tables[idx].layer.change_attributes(fid, &pending));

    app.start_edit_feature_line();
    app.sketch_press(saved_vertex);
    assert_eq!(app.draw_sketch.points.len(), 1);
    assert_eq!(table_features(&app, "feature_line").len(), 1);
    assert_eq!(table_features(&app, "feature_line")[0].wkt_geom, orig_wkt);
    assert_eq!(table_features(&app, "feature_line")[0].attributes.get("note").map(String::as_str), Some("pending note"));
}
