//! Expected checks from target/cpp-ref/tests/test_drawing_studio.cpp:135-162.
use crate::app_state::StrataAppState;

fn app_with_two_rows() -> StrataAppState {
    let mut app = StrataAppState::new_startup();
    app.left_layers.add_satellite_basemap_layer();
    app.left_layers.add_cadastral_wms_layer();
    app.left_layers.entries[1].visible = false;
    app.home_map.real_survey.layer_visible.insert(app.left_layers.entries[1].label_ko.clone(), false);
    app
}

#[test]
fn layer_sets_save_records_checks_and_applies_only_when_picked() {
    let mut app = app_with_two_rows();
    assert!(app.store_layer_set("유구 배치"));
    assert!(app.left_layers.entries[0].visible && !app.left_layers.entries[1].visible, "saving must not apply another combination");
    app.left_layers.entries[0].visible = false;
    app.left_layers.entries[1].visible = true;
    assert!(app.apply_layer_set("유구 배치"));
    assert!(app.left_layers.entries[0].visible);
    assert!(!app.left_layers.entries[1].visible);
    for row in &app.left_layers.entries {
        assert_eq!(app.home_map.real_survey.is_layer_visible(&row.label_ko), row.visible, "map visibility and actual check disagree");
    }
    assert!(!app.apply_layer_set("없는 세트"));
    assert!(!app.store_layer_set("  "));
    assert!(app.remove_layer_set("유구 배치"));
    assert!(app.layer_set_names().is_empty());
}

#[test]
fn layer_sets_use_stable_keys_after_rename_and_removal_never_changes_layers() {
    let mut app = app_with_two_rows();
    assert!(app.store_layer_set(" b "));
    assert!(app.store_layer_set("A"));
    assert_eq!(app.layer_set_names(), ["A", "b"]);
    app.left_layers.entries[0].label_ko = "renamed satellite".to_owned();
    app.left_layers.entries[0].visible = false;
    assert!(app.apply_layer_set("b"));
    assert!(app.left_layers.entries[0].visible);
    let before = app.left_layers.entries.clone();
    assert!(app.remove_layer_set("b"));
    assert_eq!(app.left_layers.entries, before);
    assert!(!app.remove_layer_set("b"));
    assert_eq!(app.layer_set_names(), ["A"]);
}

#[test]
fn layer_sets_stored_sheet_captures_and_removes_the_same_name_without_changing_layers() {
    let mut app = app_with_two_rows();
    app.store_sheet("kept sheet");
    assert_eq!(app.layer_set_names(), ["kept sheet"]);
    let before = app.left_layers.entries.clone();
    app.remove_stored_sheet("kept sheet");
    assert!(app.layer_set_names().is_empty());
    assert_eq!(app.left_layers.entries, before);
}

#[test]
fn layer_sets_standalone_set_survives_removing_a_nonexistent_sheet() {
    let mut app = app_with_two_rows();
    assert!(app.store_layer_set("standalone"));
    app.remove_stored_sheet("standalone");
    assert_eq!(app.layer_set_names(), ["standalone"]);
}

#[test]
fn layer_sets_stored_sheet_uses_the_current_same_name_set() {
    let mut app = app_with_two_rows();
    let title = app.left_layers.entries[0].label_ko.clone();
    app.home_map.real_survey.layer_visible.insert(title.clone(), true);
    app.store_sheet("kept");
    app.left_layers.entries[0].visible = false;
    app.home_map.real_survey.layer_visible.insert(title.clone(), false);
    assert!(app.store_layer_set("kept"));
    app.left_layers.entries[0].visible = true;
    app.home_map.real_survey.layer_visible.insert(title.clone(), true);
    app.open_stored_sheet("kept");
    assert!(!app.home_map.real_survey.is_layer_visible(&title), "the sheet applied an old snapshot instead of the named set");
    assert!(!app.left_layers.entries[0].visible);
}

#[test]
fn layer_sets_stored_sheet_without_its_removed_set_does_not_change_checks() {
    let mut app = app_with_two_rows();
    let title = app.left_layers.entries[0].label_ko.clone();
    app.home_map.real_survey.layer_visible.insert(title.clone(), true);
    app.store_sheet("kept");
    assert!(app.store_layer_set("kept"));
    assert!(app.remove_layer_set("kept"));
    app.left_layers.entries[0].visible = false;
    app.home_map.real_survey.layer_visible.insert(title.clone(), false);
    app.open_stored_sheet("kept");
    assert!(!app.home_map.real_survey.is_layer_visible(&title), "the deleted set still changed map visibility");
    assert!(!app.left_layers.entries[0].visible);
    assert_eq!(app.status_left_message(), "도면 「kept」을 열었습니다.");
}
