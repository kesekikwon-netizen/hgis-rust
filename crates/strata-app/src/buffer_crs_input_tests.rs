//! Buffer admission, target/cpp-ref/src/core/BufferAnalysis.cpp:54-87,177-297.
use super::menu_selection_keys_tests::key;
use super::popup_input_harness::Harness;
use super::vertex_crs_input_tests::{fixture_table, records};
use eframe::egui::{Key, Modifiers, PointerButton};

fn select(h: &mut Harness, table: &str) {
    let name = h.app.left_layers.entries.iter().find(|e| e.layer_key == table).expect("real legend row").label_ko.clone();
    let at = h.point_at(&name);
    h.press(at, PointerButton::Primary);
    assert_eq!(h.app.right_inspector.selected_layer_key.as_deref(), Some(name.as_str()));
}

fn start(source: i32) -> Harness {
    let mut h = fixture_table("feature_poly", source);
    h.app.home_map.viewport.meters_per_pixel = 4.0;
    h.frame(vec![]);
    select(&mut h, "feature_poly");
    let at = h.point_at("버퍼");
    h.press(at, PointerButton::Primary);
    assert!(h.app.ribbon.sub_toolbar_visible);
    assert_eq!(h.app.work_crs.auth_id(), "EPSG:5187");
    h
}

fn buffer(h: &mut Harness, meters: u32) {
    let at = h.point_at(&format!("{meters}m"));
    h.press(at, PointerButton::Primary);
}

fn source_data(h: &Harness) -> impl PartialEq + std::fmt::Debug {
    (
        records(h),
        format!("{:?}|{:?}", h.app.undo_tables, h.app.redo_tables),
        std::fs::read(h.app.survey_path.as_ref().expect("temporary copy")).expect("source bytes"),
    )
}

fn state(h: &Harness) -> impl PartialEq + std::fmt::Debug {
    (
        source_data(h),
        h.app.is_dirty,
        h.app.home_map.real_survey.distance_rings.clone(),
        h.app.left_layers.entries.clone(),
        h.app.home_map.real_survey.layer_visible.clone(),
    )
}

fn rejected(source: i32, meters: u32, prior: bool) {
    let mut h = start(source);
    if source == 0 {
        // target/cpp-ref/src/app/MainWindowSession.cpp:1225-1228: prepare unrelated pending work and both history directions.
        let t = h.app.survey_tables.iter_mut().find(|t| t.table == "feature_line").expect("other table");
        let fid = t.layer.features[0].fid;
        for value in ["first pending", "second pending"] {
            assert!(t.layer.change_attributes(fid, &[("note".to_owned(), Some(value.to_owned()))]));
            h.app.undo_tables.push(crate::app_state_layer_remove::UndoStep::Table("feature_line".to_owned()));
        }
        h.app.is_dirty = true;
        key(&mut h, Key::Z, Modifiers { ctrl: true, ..Default::default() });
        assert_eq!((h.app.undo_tables.len(), h.app.redo_tables.len()), (1, 1));
    }
    if prior {
        select(&mut h, "survey_area");
        for radius in [500, 1000] {
            buffer(&mut h, radius);
            assert!(h.app.home_map.real_survey.distance_rings.contains_key(&radius), "valid preparation failed");
        }
        select(&mut h, "feature_poly");
        h.app.home_map.viewport.center_xy = strata_engine::Transformer::new(5186, 5187).expect("known area CRS").transform([200_050.0, 450_050.0]).expect("local view");
        h.frame(vec![]);
    }
    let before = state(&h);
    let originals = source_data(&h);
    let was_dirty = h.app.is_dirty;
    let old_rings = h.app.home_map.real_survey.distance_rings.clone();
    for _ in 0..2 {
        h.app.show_status_message("retry pending");
        buffer(&mut h, meters);
        h.save(&format!("buffer-crs-{source}-{meters}-{prior}-rejected"));
        assert_eq!(state(&h), before, "unknown source CRS created or replaced a work-CRS buffer");
        assert_eq!(h.app.status_left_message(), format!("{meters}m 경계를 그리지 못했습니다. 선택한 레이어에 도형이 없습니다"));
        assert!(h.app.modals.active.is_none() && h.app.home_map.feature_form.is_none());
    }
    let close = h.point_at("닫기");
    h.press(close, PointerButton::Primary);
    assert!(!h.app.ribbon.sub_toolbar_visible);
    assert_eq!(state(&h), before, "closing buffer tools changed retained data");
    let reopen = h.point_at("버퍼");
    h.press(reopen, PointerButton::Primary);
    h.app.show_status_message("reopened retry pending");
    buffer(&mut h, meters);
    assert_eq!(state(&h), before, "reopened buffer tools bypassed source CRS rejection");
    assert_eq!(h.app.status_left_message(), format!("{meters}m 경계를 그리지 못했습니다. 선택한 레이어에 도형이 없습니다"));
    if source == 0 {
        key(&mut h, Key::Y, Modifiers { ctrl: true, ..Default::default() });
        let t = h.app.survey_tables.iter().find(|t| t.table == "feature_line").expect("other table");
        assert_eq!(t.layer.features[0].attributes.get("note").map(String::as_str), Some("second pending"));
        key(&mut h, Key::Z, Modifiers { ctrl: true, ..Default::default() });
        assert_eq!(state(&h), before, "rejection damaged earlier Undo or Redo");
    }
    select(&mut h, "survey_area");
    h.app.show_status_message("recovery pending");
    buffer(&mut h, meters);
    assert_eq!(h.app.status_left_message(), format!("주변 {meters}m 경계를 그렸습니다"));
    assert!(h.app.home_map.real_survey.distance_rings.contains_key(&meters));
    assert_eq!(source_data(&h), originals, "successful recovery changed source records, history or source file");
    assert!(!was_dirty || h.app.is_dirty, "successful recovery cleared earlier unsaved work");
    if prior {
        assert_eq!(h.app.home_map.real_survey.distance_rings, old_rings, "recovery changed the other retained radius");
    }
}

#[test]
fn buffer_crs_input_undefined_500_never_creates_raw_ring() {
    rejected(0, 500, false);
}
#[test]
fn buffer_crs_input_negative_500_never_creates_raw_ring() {
    rejected(-1, 500, false);
}
#[test]
fn buffer_crs_input_unknown_500_never_creates_raw_ring() {
    rejected(999_999, 500, false);
}
#[test]
fn buffer_crs_input_undefined_1000_never_creates_raw_ring() {
    rejected(0, 1000, false);
}
#[test]
fn buffer_crs_input_negative_1000_never_creates_raw_ring() {
    rejected(-1, 1000, false);
}
#[test]
fn buffer_crs_input_unknown_1000_never_creates_raw_ring() {
    rejected(999_999, 1000, false);
}
#[test]
fn buffer_crs_input_undefined_500_preserves_existing_rings() {
    rejected(0, 500, true);
}
#[test]
fn buffer_crs_input_negative_500_preserves_existing_rings() {
    rejected(-1, 500, true);
}
#[test]
fn buffer_crs_input_unknown_500_preserves_existing_rings() {
    rejected(999_999, 500, true);
}
#[test]
fn buffer_crs_input_undefined_1000_preserves_existing_rings() {
    rejected(0, 1000, true);
}
#[test]
fn buffer_crs_input_negative_1000_preserves_existing_rings() {
    rejected(-1, 1000, true);
}
#[test]
fn buffer_crs_input_unknown_1000_preserves_existing_rings() {
    rejected(999_999, 1000, true);
}

#[path = "buffer_crs_input_tests_a.rs"]
mod buffer_crs_input_tests_a;
