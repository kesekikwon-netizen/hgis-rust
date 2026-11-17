//! Menu isolation and source preservation, target/cpp-ref/tests/test_save_open.cpp:3023-3087.
use super::popup_input_harness::Harness;
use eframe::egui::{Event, Key, Modifiers, PointerButton};
use strata_canvas2d::map_tools::MapToolKind;
use strata_storage::edit_buffer::FeatureRecord;

pub(super) fn key(h: &mut Harness, key: Key, modifiers: Modifiers) {
    for pressed in [true, false] {
        h.frame_with_modifiers(vec![Event::Key { key, physical_key: None, pressed, repeat: false, modifiers }], modifiers);
    }
}

fn records(h: &Harness) -> Vec<FeatureRecord> {
    h.app.survey_tables.iter().find(|t| t.table == "feature_poly").expect("real survey polygon table").layer.features.to_vec()
}

pub(super) fn selected(legend: bool) -> Harness {
    let mut h = Harness::drawing();
    h.app.sketch_cancel();
    h.app.active_tool = MapToolKind::Select;
    h.app.ribbon.sub_toolbar_visible = false;
    // target/cpp-ref/tests/test_save_open.cpp:3038-3057 keeps unsaved attributes through deletion/undo.
    let table = h.app.survey_tables.iter_mut().find(|t| t.table == "feature_poly").expect("real polygon table");
    let fid = table.layer.features[0].fid;
    assert!(table.layer.change_attributes(fid, &[("note".to_owned(), Some("저장 전 기록".to_owned()))]));
    h.app.is_dirty = true;
    h.app.home_map.selected_shape = (!legend).then(|| ("feature_poly".to_owned(), records(&h)[0].fid));
    h.app.right_inspector.selected_layer_key = None;
    for _ in 0..3 {
        h.frame(vec![]);
    }
    if legend {
        let name = h.app.left_layers.entries.iter().find(|r| r.layer_key == "feature_poly").expect("actual legend row").label_ko.clone();
        let at = h.point_at(&name);
        h.press(at, PointerButton::Primary);
        assert!(h.app.home_map.legend_focus, "actual legend click did not take focus");
        assert_eq!(h.app.right_inspector.selected_layer_key.as_deref(), Some(name.as_str()));
    } else {
        assert!(h.app.home_map.selected_shape.is_some());
        assert!(!h.app.home_map.legend_focus);
    }
    assert!(!h.ctx.wants_keyboard_input());
    h
}

fn native_menu_delete(legend: bool) {
    let mut h = selected(legend);
    let original_file = std::fs::read(h.app.survey_path.as_ref().expect("temporary survey path")).expect("read temp file");
    let before = records(&h);
    let dirty = h.app.is_dirty;
    h.size = [1024, 768];
    for _ in 0..2 {
        let at = h.point_at("레이어");
        h.press(at, PointerButton::Primary);
        h.point_at("파일함");
        key(&mut h, Key::Delete, Modifiers::default());
        h.save(if legend { "menu-selection-legend-delete" } else { "menu-selection-feature-delete" });
        assert_eq!(records(&h), before, "native menu Delete changed selected survey records");
        assert_eq!(h.app.is_dirty, dirty, "native menu Delete changed survey dirty state");
        h.escape();
        assert_eq!(records(&h), before, "closing native menu changed selected records");
    }
    key(&mut h, Key::Delete, Modifiers::default());
    if legend {
        assert!(!h.app.survey_tables.iter().any(|t| t.table == "feature_poly"), "legend Delete did not remove layer after menu closed");
    } else {
        assert_eq!(records(&h).len(), before.len() - 1, "map Delete did not resume after menu closed");
    }
    key(&mut h, Key::Z, Modifiers::CTRL);
    assert_eq!(records(&h), before, "CtrlZ did not restore exact original IDs, geometry and attributes");
    assert_eq!(std::fs::read(h.app.survey_path.as_ref().expect("temporary path")).expect("read temp file"), original_file, "Delete/undo wrote the source GPKG");
}

#[test]
fn menu_selection_native_menu_delete_keeps_existing_feature() {
    native_menu_delete(false);
}

#[test]
fn menu_selection_native_menu_delete_keeps_focused_legend_layer() {
    native_menu_delete(true);
}

#[test]
fn menu_selection_layer_context_delete_keeps_focused_layer() {
    let mut h = selected(true);
    let before = records(&h);
    let name = h.app.right_inspector.selected_layer_key.clone().expect("actual selected layer");
    let at = h.point_at(&name);
    h.press(at, PointerButton::Secondary);
    assert!(h.app.layer_context_menu_open, "actual context menu did not open");
    key(&mut h, Key::Delete, Modifiers::default());
    assert_eq!(records(&h), before, "layer context Delete changed survey layer");
    h.escape();
    assert!(!h.app.layer_context_menu_open);
    assert_eq!(records(&h), before);
}

#[test]
fn menu_selection_native_menu_backspace_keeps_picked_vertex() {
    let mut h = selected(false);
    h.app.pick_vertex(0);
    let before = records(&h);
    h.size = [1024, 768];
    let at = h.point_at("레이어");
    h.press(at, PointerButton::Primary);
    h.point_at("파일함");
    key(&mut h, Key::Backspace, Modifiers::default());
    h.save("menu-selection-vertex-backspace");
    assert_eq!(records(&h), before, "menu Backspace removed a picked vertex");
    assert_eq!(h.app.home_map.active_vertex, Some(0));
    h.escape();
    key(&mut h, Key::Backspace, Modifiers::default());
    assert_eq!(records(&h).len(), before.len(), "Backspace removed a whole feature");
    assert_ne!(records(&h), before, "canvas Backspace did not resume");
    key(&mut h, Key::Z, Modifiers::CTRL);
    assert_eq!(records(&h), before, "CtrlZ did not restore picked-vertex edit");
}

#[test]
fn menu_selection_batched_delete_backspace_edits_picked_vertex_once() {
    let mut h = selected(false);
    h.app.pick_vertex(0);
    let before = records(&h);
    let steps = h.app.undo_tables.len();
    h.frame([Key::Delete, Key::Backspace].into_iter().map(|key| Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::default() }).collect());
    assert_eq!(records(&h).len(), before.len(), "batched edit keys removed the feature after removing its vertex");
    assert_eq!(h.app.undo_tables.len(), steps + 1, "one input batch applied two edits");
    key(&mut h, Key::Z, Modifiers::CTRL);
    assert_eq!(records(&h), before);
}

#[test]
fn menu_selection_batched_escape_backspace_releases_vertex_without_edit() {
    let mut h = selected(false);
    h.app.pick_vertex(0);
    let before = records(&h);
    let steps = h.app.undo_tables.len();
    h.frame([Key::Escape, Key::Backspace].into_iter().map(|key| Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::default() }).collect());
    assert_eq!(records(&h), before, "Escape/Backspace edited before releasing the picked vertex");
    assert_eq!(h.app.undo_tables.len(), steps);
    assert!(h.app.home_map.active_vertex.is_none());
}

fn menu_history_keeps_records(redo: bool) {
    let mut h = selected(false);
    let original = records(&h);
    key(&mut h, Key::Delete, Modifiers::default());
    let deleted = records(&h);
    assert_eq!(deleted.len(), original.len() - 1);
    if redo {
        key(&mut h, Key::Z, Modifiers::CTRL);
    }
    let before = records(&h);
    h.size = [1024, 768];
    let at = h.point_at("레이어");
    h.press(at, PointerButton::Primary);
    h.point_at("파일함");
    let command = if redo { Key::Y } else { Key::Z };
    key(&mut h, command, Modifiers::CTRL);
    h.save(if redo { "menu-selection-history-redo" } else { "menu-selection-history-undo" });
    assert_eq!(records(&h), before, "menu history key changed survey behind menu");
    h.escape();
    key(&mut h, command, Modifiers::CTRL);
    assert_eq!(records(&h), if redo { deleted } else { original }, "history did not resume after menu close");
}

#[test]
fn menu_selection_native_menu_ctrl_z_keeps_selected_history() {
    menu_history_keeps_records(false);
}

#[test]
fn menu_selection_native_menu_ctrl_y_keeps_selected_history() {
    menu_history_keeps_records(true);
}

#[test]
fn menu_selection_delete_then_ctrl_z_batch_restores_selected_record() {
    let mut h = selected(false);
    let before = records(&h);
    h.frame_with_modifiers([Key::Delete, Key::Z].into_iter().map(|key| Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::CTRL }).collect(), Modifiers::CTRL);
    assert_eq!(records(&h), before, "Delete/CtrlZ reversed deletion and restoration");
}
