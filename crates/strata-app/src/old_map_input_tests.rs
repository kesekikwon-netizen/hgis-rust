//! Whole-icon InstantPopup input, target/cpp-ref/src/app/MainWindowRibbon.cpp:485-500.
use super::popup_input_harness::Harness;
use crate::app_state::{MainTab, StrataAppState};
use eframe::egui::{PointerButton, Pos2, Shape};

fn start() -> Harness {
    let directory = tempfile::tempdir().expect("temporary survey");
    let path = directory.path().join("demo.gpkg");
    std::fs::copy(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg"), &path).expect("tracked demo copy");
    let mut app = StrataAppState::new_startup();
    app.vworld_api_key = crate::basemap_cadastral_jobs::CachedApiKey::with_reader_and_clock(std::sync::Arc::new(String::new), std::sync::Arc::new(std::time::Instant::now));
    app.history_gis_key_reader = || "OFFLINE-TEST-ONLY".to_owned();
    assert!(app.open_survey_gpkg(&path));
    app.active_tab = MainTab::Map;
    let mut h = Harness::new(app, directory);
    for _ in 0..3 {
        h.frame(vec![]);
    }
    h
}

fn icon(h: &mut Harness) -> Pos2 {
    let output = h.frame(vec![]);
    let label = output
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            Shape::Text(t) if t.galley.text() == "옛 지도" => Some(t.galley.rect.translate(t.pos.to_vec2()).center()),
            _ => None,
        })
        .expect("painted old-map caption");
    output
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            Shape::Rect(r) if (r.rect.center().x - label.x).abs() < 0.1 && r.rect.width() > 20.0 && r.rect.height() > 20.0 && r.rect.top() < 100.0 && r.rect.bottom() < label.y => Some(r.rect.center()),
            _ => None,
        })
        .expect("actual painted old-map icon tile")
}

fn choices(h: &mut Harness, name: &str) {
    h.save(name);
    assert!(h.app.ribbon.old_map_menu_open, "the opening icon click immediately dismissed its own menu");
    h.point_at("▤  대동여지도");
    h.point_at("▤  1919 조선지형도 1:5만");
}

#[test]
fn old_map_input_primary_icon_opens_both_choices() {
    for width in [1680, 2560, 1280] {
        let mut h = start();
        h.size[0] = width;
        let at = icon(&mut h);
        h.press(at, PointerButton::Primary);
        choices(&mut h, &format!("old-map-primary-{width}"));
    }
}

#[test]
fn old_map_input_secondary_icon_opens_both_choices() {
    for width in [1680, 2560, 1280] {
        let mut h = start();
        h.size[0] = width;
        let at = icon(&mut h);
        h.press(at, PointerButton::Secondary);
        choices(&mut h, &format!("old-map-secondary-{width}"));
    }
}

#[test]
fn old_map_input_label_is_a_working_control() {
    let mut h = start();
    let at = h.point_at("옛 지도");
    h.press(at, PointerButton::Primary);
    choices(&mut h, "old-map-label");
    let at = icon(&mut h);
    h.press(at, PointerButton::Primary);
    assert!(!h.app.ribbon.old_map_menu_open, "pressing the initiating control again must dismiss the menu");
}

fn open_icon(h: &mut Harness) {
    let at = icon(h);
    h.press(at, PointerButton::Primary);
    assert!(h.app.ribbon.old_map_menu_open);
}

fn choose(h: &mut Harness, text: &str) {
    let at = h.point_at(text);
    h.press(at, PointerButton::Primary);
    assert!(!h.app.ribbon.old_map_menu_open);
}

#[test]
fn old_map_input_daedong_choice_installs_once_without_writing_survey() {
    let mut h = start();
    let path = h.app.survey_path.clone().expect("temporary survey");
    let bytes = std::fs::read(&path).expect("before");
    let original = super::vertex_crs_input_tests::records(&h);
    h.app.home_map.viewport.center_xy = [0.0, 0.0]; // Prepared view in work CRS EPSG:5187, outside the survey.
    for _ in 0..2 {
        open_icon(&mut h);
        choose(&mut h, "▤  대동여지도");
    }
    assert_eq!(h.app.left_layers.entries.iter().filter(|e| e.layer_key == "ref_daedongyeojido" && e.visible).count(), 1);
    assert!(!h.app.left_layers.entries.iter().any(|e| e.layer_key == "ref_history_1919"));
    assert!(h.app.home_map.real_survey.is_layer_visible("대동여지도") && h.app.old_map_shown() && h.app.is_dirty);
    assert_ne!(h.app.home_map.viewport.center_xy, [0.0, 0.0], "actual choice did not frame the survey");
    assert_eq!(super::vertex_crs_input_tests::records(&h), original);
    assert_eq!(std::fs::read(path).expect("after"), bytes);
    h.save("old-map-daedong-selected");
}

#[test]
fn old_map_input_1919_choice_uses_history_key_and_installs_once() {
    let mut h = start();
    let path = h.app.survey_path.clone().expect("temporary survey");
    let bytes = std::fs::read(&path).expect("before");
    let original = super::vertex_crs_input_tests::records(&h);
    for _ in 0..2 {
        open_icon(&mut h);
        choose(&mut h, "▤  1919 조선지형도 1:5만");
    }
    assert_eq!(h.app.tile_key_for(crate::basemap_tiles::TileKind::History1919), "OFFLINE-TEST-ONLY");
    assert_eq!(h.app.left_layers.entries.iter().filter(|e| e.layer_key == "ref_history_1919" && e.visible).count(), 1);
    assert!(!h.app.left_layers.entries.iter().any(|e| e.layer_key == "ref_daedongyeojido"));
    assert!(h.app.home_map.real_survey.is_layer_visible("1919 조선지형도 1:5만") && h.app.old_map_shown() && h.app.is_dirty);
    assert_eq!(super::vertex_crs_input_tests::records(&h), original);
    assert_eq!(std::fs::read(path).expect("after"), bytes);
    h.save("old-map-1919-selected-offline");
}

#[test]
fn old_map_input_1919_missing_key_stays_in_footer_without_changing_data() {
    let mut h = start();
    h.app.history_gis_key_reader = String::new;
    let original = super::vertex_crs_input_tests::records(&h);
    let rows = h.app.left_layers.entries.clone();
    open_icon(&mut h);
    choose(&mut h, "▤  1919 조선지형도 1:5만");
    assert!(h.app.status_left_message().contains("역사지리정보DB API 키가 필요합니다"));
    assert!(!h.app.is_dirty && h.app.modals.active.is_none());
    assert_eq!(h.app.left_layers.entries, rows);
    assert_eq!(super::vertex_crs_input_tests::records(&h), original);
    h.save("old-map-1919-missing-key");
}

#[test]
fn old_map_input_compact_entry_keeps_both_choices() {
    let mut h = start();
    h.size[0] = 800;
    let at = h.point_at("배경 지도");
    h.press(at, PointerButton::Primary);
    let at = h.point_at("옛 지도");
    h.press(at, PointerButton::Primary);
    choices(&mut h, "old-map-compact-800");
    choose(&mut h, "▤  대동여지도");
    assert!(h.app.old_map_shown());
}

#[test]
fn old_map_input_escape_and_outside_dismissal_keep_pending_capture() {
    let mut h = start();
    h.app.handle_ribbon_click("그리기");
    h.app.start_edit_feature_poly();
    h.frame(vec![]);
    for at in [Pos2::new(700.0, 450.0), Pos2::new(820.0, 450.0), Pos2::new(820.0, 570.0)] {
        h.press(at, PointerButton::Primary);
    }
    assert_eq!(h.app.draw_sketch.points.len(), 3);
    let points = h.app.draw_sketch.points.clone();
    let records = super::vertex_crs_input_tests::records(&h);
    let history = format!("{:?}|{:?}", h.app.undo_tables, h.app.redo_tables);
    let path = h.app.survey_path.clone().expect("temporary survey");
    let bytes = std::fs::read(&path).expect("before");
    open_icon(&mut h);
    h.close_popup_without_losing_capture("old-map-pending");
    open_icon(&mut h);
    h.press(Pos2::new(700.0, 700.0), PointerButton::Primary);
    assert!(!h.app.ribbon.old_map_menu_open);
    assert_eq!(h.app.draw_sketch.points, points, "menu-dismissal click leaked into the pending drawing");
    assert_eq!(super::vertex_crs_input_tests::records(&h), records);
    assert_eq!(format!("{:?}|{:?}", h.app.undo_tables, h.app.redo_tables), history);
    assert_eq!(std::fs::read(path).expect("after"), bytes);
    h.press(Pos2::new(760.0, 700.0), PointerButton::Primary);
    assert_eq!(h.app.draw_sketch.points.len(), 4, "the next deliberate canvas press did not return to capture");
}
