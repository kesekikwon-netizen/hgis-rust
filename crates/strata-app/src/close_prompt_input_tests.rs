//! Close question ownership, target/cpp-ref/src/app/MainWindow.cpp:333-352 and KaClosePrompt.cpp:24-28,52-56.
use super::menu_selection_keys_tests::{key, selected};
use super::popup_input_harness::Harness;
use crate::app_state::MainTab;
use eframe::egui::{Event, FullOutput, Key, Modifiers, PointerButton, ViewportCommand};
use strata_storage::edit_buffer::FeatureRecord;

fn records(h: &Harness) -> Vec<(String, Vec<FeatureRecord>)> {
    assert!(h.app.survey_tables.iter().all(|t| t.srs_id == 5186), "tracked demo table CRS");
    h.app.survey_tables.iter().map(|t| (t.table.clone(), t.layer.features.to_vec())).collect()
}

fn command(out: &FullOutput, want: ViewportCommand) -> bool {
    out.viewport_output.values().any(|v| v.commands.contains(&want))
}

fn ask(h: &mut Harness) {
    let out = h.frame_with_close_request(vec![], Modifiers::default());
    assert!(h.app.close_prompt_open && command(&out, ViewportCommand::CancelClose), "unsaved root close must wait for an answer");
    assert!(!command(&out, ViewportCommand::Close));
    for _ in 0..2 {
        h.frame(vec![]);
    }
}

fn answer(h: &mut Harness, label: &str) -> FullOutput {
    let at = h.point_at(label);
    h.press_output(at, PointerButton::Primary)
}

fn stored(h: &Harness) -> Vec<(String, Vec<FeatureRecord>)> {
    let tables = strata_storage::survey_gpkg_read::read_survey_tables(h.app.survey_path.as_ref().expect("temp path")).expect("read temp copy");
    tables.iter().map(|t| (t.table.clone(), t.layer.features.to_vec())).collect()
}

#[test]
fn close_prompt_input_delete_preserves_records_until_cancel() {
    let mut h = selected(false);
    let before = records(&h);
    let source = std::fs::read(h.app.survey_path.as_ref().expect("temp path")).expect("read");
    let undo = h.app.undo_tables.len();
    ask(&mut h);
    for _ in 0..2 {
        key(&mut h, Key::Delete, Modifiers::default());
        assert_eq!(records(&h), before, "Delete edited records behind close question");
        assert_eq!(h.app.undo_tables.len(), undo);
    }
    h.save("close-question-owned-delete");
    let out = answer(&mut h, "취소");
    assert!(!h.app.close_prompt_open && !command(&out, ViewportCommand::Close));
    h.frame(vec![]);
    key(&mut h, Key::Delete, Modifiers::default());
    assert_ne!(records(&h), before, "normal Delete did not resume");
    key(&mut h, Key::Z, Modifiers::CTRL);
    assert_eq!(records(&h), before);
    assert_eq!(std::fs::read(h.app.survey_path.as_ref().expect("temp path")).expect("read"), source);
}

#[test]
fn close_prompt_input_escape_cancels_only_question_and_repeated_close_waits() {
    let mut h = selected(false);
    let before = records(&h);
    let tool = h.app.active_tool;
    let selection = h.app.home_map.selected_shape.clone();
    ask(&mut h);
    h.escape();
    assert!(!h.app.close_prompt_open, "Escape left close question open");
    assert_eq!(h.app.active_tool, tool);
    assert_eq!(h.app.home_map.selected_shape, selection);
    assert_eq!(records(&h), before);
    ask(&mut h);
    assert_eq!(records(&h), before);
    assert!(h.app.is_dirty);
}

fn capture() -> Harness {
    let mut h = Harness::drawing();
    let table = h.app.survey_tables.iter_mut().find(|t| t.table == "feature_poly").expect("demo table");
    assert!(table.layer.change_attributes(table.layer.features[0].fid, &[("note".to_owned(), Some("저장 전 기록".to_owned()))]));
    h.app.is_dirty = true;
    h
}

#[test]
fn close_prompt_input_escape_preserves_pending_capture() {
    let mut h = capture();
    let points = h.app.draw_sketch.points.clone();
    let tool = h.app.active_tool;
    ask(&mut h);
    h.escape();
    assert_eq!(h.app.draw_sketch.points, points, "close question Escape discarded EPSG:5187 capture");
    assert_eq!(h.app.active_tool, tool);
    assert!(!h.app.close_prompt_open);
}

fn history_is_owned(redo: bool) {
    let mut h = selected(false);
    key(&mut h, Key::Delete, Modifiers::default());
    if redo {
        key(&mut h, Key::Z, Modifiers::CTRL);
    }
    let before = records(&h);
    let undo = h.app.undo_tables.len();
    let redo_before = h.app.redo_tables.clone();
    ask(&mut h);
    let action = if redo { Key::Y } else { Key::Z };
    key(&mut h, action, Modifiers::CTRL);
    assert_eq!(records(&h), before, "history changed records behind close question");
    assert_eq!(h.app.undo_tables.len(), undo);
    assert_eq!(h.app.redo_tables, redo_before);
    answer(&mut h, "취소");
    h.frame(vec![]);
    key(&mut h, action, Modifiers::CTRL);
    assert_ne!(records(&h), before, "normal history did not resume");
}

#[test]
fn close_prompt_input_undo_preserves_pending_records() {
    history_is_owned(false);
}

#[test]
fn close_prompt_input_redo_preserves_pending_records() {
    history_is_owned(true);
}

#[test]
fn close_prompt_input_background_home_click_is_blocked_at_480() {
    let mut h = selected(false);
    h.size = [480, 640];
    ask(&mut h);
    h.point_at("저장하고 끝내기");
    h.point_at("저장하지 않고 끝내기");
    h.point_at("취소");
    let at = h.point_at("홈");
    h.press(at, PointerButton::Primary);
    assert_eq!(h.app.active_tab, MainTab::Map, "background Home activated behind close question");
    assert!(h.app.close_prompt_open);
    h.save("close-question-background-blocked");
}

#[test]
fn close_prompt_input_discard_sends_close_and_repeated_close_bypasses_question() {
    let mut h = selected(false);
    let before = records(&h);
    let source = std::fs::read(h.app.survey_path.as_ref().expect("temp path")).expect("read");
    ask(&mut h);
    let out = answer(&mut h, "저장하지 않고 끝내기");
    assert!(command(&out, ViewportCommand::Close));
    assert!(h.app.closing_unsaved && !h.app.close_prompt_open && h.app.is_dirty);
    let again = h.frame_with_close_request(vec![], Modifiers::default());
    assert!(!command(&again, ViewportCommand::CancelClose) && !h.app.close_prompt_open);
    assert_eq!(records(&h), before);
    assert_eq!(std::fs::read(h.app.survey_path.as_ref().expect("temp path")).expect("read"), source);
}

#[test]
fn close_prompt_input_failed_save_never_sends_close() {
    let mut h = selected(false);
    h.app.survey_tables.iter_mut().find(|t| t.table == "feature_poly").expect("demo table").layer.features[0].wkt_geom = "POLYGON Z ((0 0 1,1 0 1,1 1 1,0 0 1))".to_owned();
    let before = records(&h);
    let source = std::fs::read(h.app.survey_path.as_ref().expect("temp path")).expect("read");
    ask(&mut h);
    let out = answer(&mut h, "저장하고 끝내기");
    assert!(!command(&out, ViewportCommand::Close));
    assert!(h.app.close_prompt_open && h.app.is_dirty && !h.app.closing_unsaved);
    assert_eq!(records(&h), before);
    assert!(h.app.status_left_message().starts_with("저장 실패"));
    assert_eq!(std::fs::read(h.app.survey_path.as_ref().expect("temp path")).expect("read"), source);
}

fn event(key: Key, modifiers: Modifiers) -> Event {
    Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }
}

#[test]
fn close_prompt_input_default_enter_saves_before_sending_close() {
    let mut h = selected(false);
    let before = records(&h);
    ask(&mut h);
    let out = h.frame(vec![event(Key::Enter, Modifiers::default())]);
    assert!(command(&out, ViewportCommand::Close), "default Enter did not request close after Save");
    assert!(!h.app.close_prompt_open && !h.app.is_dirty);
    assert_eq!(stored(&h), before);
    let next = h.frame_with_close_request(vec![], Modifiers::default());
    assert!(!command(&next, ViewportCommand::CancelClose) && !h.app.close_prompt_open);
}

#[path = "close_prompt_input_tests_a.rs"]
mod opening;
