//! Project-scoped persistence, target/cpp-ref/src/app/KaDrawingLayerSets.cpp:29-59 and KaDrawingStudioSheets.cpp:99-169.
use super::persistence;
use crate::app_state::StrataAppState;
use rusqlite::Connection;
use std::path::{Path, PathBuf};

fn copy_demo(folder: &Path, name: &str) -> PathBuf {
    let file = folder.join(name);
    std::fs::copy(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg"), &file).expect("tracked demo copy");
    file
}
fn opened(file: &Path) -> StrataAppState {
    let mut app = StrataAppState::new_startup();
    assert!(app.open_survey_gpkg(file));
    app
}
fn inject(file: &Path, payload: &str) {
    let conn = Connection::open(file).expect("temporary GPKG");
    conn.execute_batch("CREATE TABLE ka_hgis_layer_sets (id INTEGER PRIMARY KEY CHECK(id=1), payload TEXT NOT NULL)").expect("metadata table");
    conn.execute("INSERT INTO ka_hgis_layer_sets VALUES (1,?1)", [payload]).expect("recorded metadata");
}

#[test]
fn layer_sets_metadata_roundtrip_keeps_real_geometry_and_does_not_register_a_layer() {
    let dir = tempfile::tempdir().expect("temp");
    let file = copy_demo(dir.path(), "roundtrip.gpkg");
    let mut app = opened(&file);
    assert!(app.store_layer_set("유구 배치"));
    persistence::write(&app.layer_sets, &file).expect("metadata write");
    app.layer_sets = persistence::read(&file).expect("metadata read");
    assert_eq!(app.layer_set_names(), ["유구 배치"]);
    app.left_layers.entries[0].visible = false;
    assert!(app.apply_layer_set("유구 배치"));
    assert!(app.left_layers.entries[0].visible);
    let conn = Connection::open(&file).expect("verify actual GPKG");
    assert_eq!(conn.query_row("SELECT count(*) FROM feature_poly", [], |r| r.get::<_, i64>(0)).expect("real features"), 2);
    assert_eq!(conn.query_row("SELECT count(*) FROM gpkg_contents WHERE table_name='ka_hgis_layer_sets'", [], |r| r.get::<_, i64>(0)).expect("registered layers"), 0);
}

#[test]
fn layer_sets_metadata_reader_never_writes_legacy_or_creates_missing_files() {
    let dir = tempfile::tempdir().expect("temp");
    let file = copy_demo(dir.path(), "legacy.gpkg");
    let before = std::fs::read(&file).expect("before");
    let mut app = StrataAppState::new_startup();
    app.layer_sets = persistence::read(&file).expect("legacy accepted");
    assert!(app.layer_set_names().is_empty());
    assert_eq!(std::fs::read(&file).expect("after"), before);
    let missing = dir.path().join("missing.gpkg");
    assert!(persistence::read(&missing).is_err());
    assert!(!missing.exists());
}

#[test]
fn layer_sets_metadata_reader_rejects_corrupt_unknown_and_unrecognized_payloads() {
    let dir = tempfile::tempdir().expect("temp");
    let cases = [r#"{"version":99,"sets":{}}"#, "{broken", r#"{"version":1,"sets":{},"future":true}"#];
    let actual = cases
        .iter()
        .enumerate()
        .map(|(i, text)| {
            let file = copy_demo(dir.path(), &format!("invalid-{i}.gpkg"));
            inject(&file, text);
            persistence::read(&file).is_err()
        })
        .collect::<Vec<_>>();
    assert_eq!(actual, [true, true, true]);
}

#[test]
fn layer_sets_metadata_writer_preserves_unknown_data_and_refuses_to_create_databases() {
    let dir = tempfile::tempdir().expect("temp");
    let file = copy_demo(dir.path(), "future.gpkg");
    inject(&file, r#"{"version":99,"sets":{"future":{}}}"#);
    let before = std::fs::read(&file).expect("before");
    let app = StrataAppState::new_startup();
    assert!(persistence::write(&app.layer_sets, &file).is_err());
    assert_eq!(std::fs::read(&file).expect("after"), before);
    let missing = dir.path().join("missing.gpkg");
    assert!(persistence::write(&app.layer_sets, &missing).is_err());
    assert!(!missing.exists());
}

#[test]
fn layer_sets_metadata_failed_transaction_keeps_the_previous_catalog() {
    let dir = tempfile::tempdir().expect("temp");
    let file = copy_demo(dir.path(), "rollback.gpkg");
    let old = r#"{"version":1,"sets":{"original":{}}}"#;
    inject(&file, old);
    let conn = Connection::open(&file).expect("temp GPKG");
    conn.execute_batch("CREATE TRIGGER refuse_sets BEFORE INSERT ON ka_hgis_layer_sets BEGIN SELECT RAISE(ABORT,'recorded rejection'); END").expect("controlled failure");
    let mut app = StrataAppState::new_startup();
    assert!(app.store_layer_set("replacement"));
    assert!(persistence::write(&app.layer_sets, &file).is_err());
    assert_eq!(conn.query_row("SELECT payload FROM ka_hgis_layer_sets WHERE id=1", [], |r| r.get::<_, String>(0)).expect("previous payload"), old);
}

#[test]
fn layer_sets_survive_explicit_survey_save_and_reopen_without_crossing_projects() {
    let dir = tempfile::tempdir().expect("temp");
    let first = copy_demo(dir.path(), "first.gpkg");
    let second = copy_demo(dir.path(), "second.gpkg");
    let mut app = opened(&first);
    assert!(app.store_layer_set("kept"));
    app.handle_ribbon_click("저장");
    assert!(!app.is_dirty, "explicit save did not finish");
    let mut reopened = opened(&first);
    assert_eq!(reopened.layer_set_names(), ["kept"]);
    assert!(reopened.open_survey_gpkg(&second));
    assert!(reopened.layer_set_names().is_empty(), "previous survey's sets leaked");
}

#[test]
fn layer_sets_bad_metadata_refuses_open_before_replacing_current_work() {
    let dir = tempfile::tempdir().expect("temp");
    let good = copy_demo(dir.path(), "good.gpkg");
    let bad = copy_demo(dir.path(), "bad-metadata.gpkg");
    inject(&bad, r#"{"version":99,"sets":{}}"#);
    let mut app = opened(&good);
    assert!(app.store_layer_set("pending"));
    let rows = app.left_layers.entries.clone();
    assert!(!app.open_survey_gpkg(&bad));
    assert_eq!(app.survey_path.as_deref(), Some(good.as_path()));
    assert_eq!(app.layer_set_names(), ["pending"]);
    assert_eq!(app.left_layers.entries, rows);
    assert!(app.is_dirty);
}

#[test]
fn layer_sets_opening_another_survey_drops_old_kept_sheets_and_questions() {
    let dir = tempfile::tempdir().expect("temp");
    let first = copy_demo(dir.path(), "first.gpkg");
    let second = copy_demo(dir.path(), "second.gpkg");
    let mut app = opened(&first);
    app.store_sheet("first survey sheet");
    app.drawing.sheet_ask = Some(crate::gui_tab_drawing::gui_tab_drawing_sheets::SheetAsk::Remove("first survey sheet".to_owned()));
    assert!(app.open_survey_gpkg(&second));
    assert!(app.sheet_set_rows().is_empty(), "previous project's sheet can still select its old layer combination");
    assert!(app.drawing.sheet_ask.is_none());
}
