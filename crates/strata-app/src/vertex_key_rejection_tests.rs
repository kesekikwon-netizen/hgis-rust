//! Rejected picked-vertex keys, target/cpp-ref/src/app/KaFeatureSelectTool.cpp:71-82.
use super::menu_selection_keys_tests::{key, selected};
use super::popup_input_harness::Harness;
use eframe::egui::{Key, Modifiers};
use strata_storage::edit_buffer::FeatureRecord;

fn records(h: &Harness) -> Vec<(String, Vec<FeatureRecord>)> {
    h.app.survey_tables.iter().map(|t| (t.table.clone(), t.layer.features.to_vec())).collect()
}

fn minimal(table: &str) -> Harness {
    let mut h = selected(false);
    let layer = h.app.survey_tables.iter().find(|t| t.table == table).expect("tracked demo layer");
    // target/cpp-ref/tests/test_editing_tools.cpp:228-248: one valid removal, then re-pick the same target.
    // WKT stays in the demo table CRS EPSG:5186; the existing Harness uses work CRS EPSG:5187.
    assert_eq!(layer.srs_id, 5186);
    let fid = layer.layer.features[0].fid;
    let count = layer.layer.features.len();
    h.app.home_map.selected_shape = Some((table.to_owned(), fid));
    h.app.sync_card_with_selection();
    let card = h.app.home_map.card.as_ref().expect("selected feature card");
    assert_eq!(card.table, table);
    assert_eq!(card.fid, fid);
    h.app.pick_vertex(0);
    key(&mut h, Key::Delete, Modifiers::default());
    let layer = h.app.survey_tables.iter().find(|t| t.table == table).expect("selected layer remains");
    assert_eq!(layer.layer.features.len(), count, "fixture removed a whole feature");
    let shape = layer.layer.features.iter().find(|f| f.fid == fid).expect("same feature");
    assert_eq!(shape.wkt_geom.matches(',').count(), if table == "feature_poly" { 3 } else { 1 }, "fixture did not reach triangle or two-point line");
    assert!(h.app.home_map.active_vertex.is_none());
    h.app.pick_vertex(0);
    h
}

fn rejected_ui_preserves_saved_feature(table: &str) {
    let mut h = minimal(table);
    assert!(h.app.save_survey_project(), "save only the temporary demo copy");
    let before = records(&h);
    let original = std::fs::read(h.app.survey_path.as_ref().expect("temp path")).expect("read temp copy");
    let undo = h.app.undo_tables.len();
    let redo = h.app.redo_tables.clone();
    assert!(!h.app.is_dirty);
    h.app.pick_vertex(0);
    for _ in 0..2 {
        key(&mut h, Key::Delete, Modifiers::default());
        assert_eq!(records(&h), before, "rejected vertex Delete fell through to whole-feature deletion");
        assert_eq!(h.app.home_map.active_vertex, Some(0));
        assert_eq!(h.app.undo_tables.len(), undo);
        assert_eq!(h.app.redo_tables, redo);
        assert!(!h.app.is_dirty, "rejected edit dirtied the saved survey");
        // target/cpp-ref/src/app/KaVertexEditTool.cpp:368-373: retain the real rejection, never a success message.
        assert_eq!(h.app.status_left_message(), if table == "feature_poly" { "면은 꼭짓점 3개보다 줄일 수 없습니다." } else { "선은 꼭짓점 2개보다 줄일 수 없습니다." });
    }
    h.save(if table == "feature_poly" { "vertex-key-rejected-triangle" } else { "vertex-key-rejected-min-line" });
    h.escape();
    assert!(h.app.home_map.active_vertex.is_none());
    assert_eq!(records(&h), before);
    key(&mut h, Key::Delete, Modifiers::default());
    let after = records(&h);
    assert_eq!(after.iter().find(|t| t.0 == table).expect("target table").1.len(), before.iter().find(|t| t.0 == table).expect("target table").1.len() - 1, "intentional whole-feature Delete stopped working");
    key(&mut h, Key::Z, Modifiers::CTRL);
    assert_eq!(records(&h), before, "undo did not restore exact IDs, WKT and attributes");
    assert_eq!(std::fs::read(h.app.survey_path.as_ref().expect("temp path")).expect("read temp copy"), original);
}

#[test]
fn vertex_key_rejection_triangle_delete_preserves_feature_and_history() {
    rejected_ui_preserves_saved_feature("feature_poly");
}

#[test]
fn vertex_key_rejection_min_line_delete_preserves_feature_and_history() {
    rejected_ui_preserves_saved_feature("feature_line");
}

#[test]
fn vertex_key_rejection_owned_error_consumes_key_without_mutating_unsaved_records() {
    for table in ["feature_poly", "feature_line"] {
        let mut h = minimal(table);
        let before = records(&h);
        let undo = h.app.undo_tables.len();
        assert!(h.app.is_dirty);
        assert!(h.app.handle_vertex_delete_key(), "a rejected owned-vertex key must still be consumed");
        assert_eq!(records(&h), before);
        assert_eq!(h.app.undo_tables.len(), undo);
        key(&mut h, Key::Backspace, Modifiers::default());
        assert_eq!(records(&h), before);
        assert_eq!(h.app.home_map.active_vertex, Some(0));
    }
}
