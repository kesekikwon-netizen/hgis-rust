//! Save-before input ownership, target/cpp-ref/src/app/MainWindowSession.cpp:668-676,1180-1193.
use super::menu_selection_keys_tests::{key, selected};
use super::popup_input_harness::Harness;
use crate::app_state::app_state_save_before::SaveBefore;
use crate::app_state::MainTab;
use crate::dialogs_modal::ActiveModalDialog;
use eframe::egui::{Event, Key, Modifiers, PointerButton};
use strata_storage::edit_buffer::FeatureRecord;

fn records(h: &Harness) -> Vec<(String, Vec<FeatureRecord>)> {
    h.app.survey_tables.iter().map(|t| (t.table.clone(), t.layer.features.to_vec())).collect()
}

fn ask(h: &mut Harness) {
    h.app.request_new_survey();
    for _ in 0..3 {
        h.frame(vec![]);
    }
    assert_eq!(h.app.save_before, Some(SaveBefore::NewSurvey));
}

fn answer(h: &mut Harness, label: &str) {
    let out = h.frame(vec![]);
    // The question is rendered after the ribbon, which also has a Save label.
    let at = out
        .shapes
        .iter()
        .rev()
        .find_map(|s| match &s.shape {
            eframe::egui::Shape::Text(t) if t.galley.text() == label => {
                let rect = t.galley.rect.translate(t.pos.to_vec2());
                (h.ctx.screen_rect().contains_rect(rect) && s.clip_rect.contains_rect(rect)).then_some(rect.center())
            }
            _ => None,
        })
        .expect("actual visible question answer");
    h.press(at, PointerButton::Primary);
}

#[test]
fn save_before_input_delete_preserves_pending_records() {
    let mut h = selected(false);
    let before = records(&h);
    let source = std::fs::read(h.app.survey_path.as_ref().expect("temp path")).expect("read");
    let undo = h.app.undo_tables.len();
    ask(&mut h);
    for _ in 0..2 {
        key(&mut h, Key::Delete, Modifiers::default());
        assert_eq!(records(&h), before, "Delete edited the survey behind its save question");
        assert_eq!(h.app.undo_tables.len(), undo);
        assert!(h.app.is_dirty);
    }
    h.save("save-before-owned-delete");
    answer(&mut h, "취소");
    assert!(h.app.save_before.is_none());
    assert_eq!(records(&h), before);
    key(&mut h, Key::Delete, Modifiers::default());
    assert_ne!(records(&h), before, "normal Delete did not resume after cancel");
    key(&mut h, Key::Z, Modifiers::CTRL);
    assert_eq!(records(&h), before);
    assert_eq!(std::fs::read(h.app.survey_path.as_ref().expect("temp path")).expect("read"), source);
}

#[test]
fn save_before_input_escape_preserves_selection_and_tool() {
    let mut h = selected(false);
    let before = records(&h);
    let selected = h.app.home_map.selected_shape.clone();
    let tool = h.app.active_tool;
    ask(&mut h);
    h.escape();
    assert!(h.app.save_before.is_none());
    assert_eq!(h.app.active_tool, tool, "question Escape changed canvas tool");
    assert_eq!(h.app.home_map.selected_shape, selected, "question Escape cleared selection");
    assert_eq!(records(&h), before);
    assert!(h.app.is_dirty);
}

#[test]
fn save_before_input_escape_preserves_capture() {
    let mut h = Harness::drawing();
    let table = h.app.survey_tables.iter_mut().find(|t| t.table == "feature_poly").expect("demo table");
    assert!(table.layer.change_attributes(table.layer.features[0].fid, &[("note".to_owned(), Some("저장 전 기록".to_owned()))]));
    h.app.is_dirty = true;
    let points = h.app.draw_sketch.points.clone();
    let tool = h.app.active_tool;
    ask(&mut h);
    h.escape();
    assert!(h.app.save_before.is_none());
    assert_eq!(h.app.draw_sketch.points, points, "question Escape discarded pending EPSG:5187 capture");
    assert_eq!(h.app.active_tool, tool);
}

#[test]
fn save_before_input_nested_new_keeps_original_open_request() {
    let mut h = selected(false);
    let next = h.app.survey_path.as_ref().expect("temp path").with_file_name("next.gpkg");
    std::fs::copy(h.app.survey_path.as_ref().expect("temp path"), &next).expect("temp copy");
    let path = h.app.survey_path.clone();
    h.app.request_open_survey(next.clone());
    h.frame(vec![]);
    key(&mut h, Key::N, Modifiers::CTRL);
    assert_eq!(h.app.save_before, Some(SaveBefore::Open(next.clone())), "nested CtrlN replaced the original open request");
    h.app.request_new_survey();
    assert_eq!(h.app.save_before, Some(SaveBefore::Open(next)), "nested request replaced prompt owner");
    answer(&mut h, "취소");
    assert_eq!(h.app.survey_path, path);
    assert!(h.app.is_dirty);
}

#[test]
fn save_before_input_background_home_click_is_blocked() {
    let mut h = selected(false);
    h.size = [480, 640];
    ask(&mut h);
    h.point_at("저장 안 함");
    h.point_at("취소");
    let at = h.point_at("홈");
    h.press(at, PointerButton::Primary);
    assert_eq!(h.app.active_tab, MainTab::Map, "background tab switched behind save question");
    assert_eq!(h.app.save_before, Some(SaveBefore::NewSurvey), "outside click dismissed save question");
    h.save("save-before-background-blocked");
}

#[test]
fn save_before_input_enter_uses_default_save_and_opens_new_dialog() {
    let mut h = selected(false);
    let before = records(&h);
    ask(&mut h);
    key(&mut h, Key::Enter, Modifiers::default());
    assert!(h.app.save_before.is_none(), "Enter did not choose C++ default Save");
    assert_eq!(h.app.modals.active, Some(ActiveModalDialog::NewSurvey));
    assert!(!h.app.is_dirty);
    let stored = strata_storage::survey_gpkg_read::read_survey_tables(h.app.survey_path.as_ref().expect("temp path")).expect("saved temp file");
    let stored: Vec<_> = stored.iter().map(|t| (t.table.clone(), t.layer.features.to_vec())).collect();
    assert_eq!(stored, before, "default Save lost the pending records");
}

#[test]
fn save_before_input_failed_save_keeps_question_and_pending_geometry() {
    let mut h = selected(false);
    // target/cpp-ref/tests/test_save_open.cpp:5772-5823: vetoed save retains current data.
    h.app.survey_tables.iter_mut().find(|t| t.table == "feature_poly").expect("demo table").layer.features[0].wkt_geom = "POLYGON Z ((0 0 1,1 0 1,1 1 1,0 0 1))".to_owned();
    let before = records(&h);
    let source = std::fs::read(h.app.survey_path.as_ref().expect("temp path")).expect("read");
    ask(&mut h);
    answer(&mut h, "저장");
    assert_eq!(h.app.save_before, Some(SaveBefore::NewSurvey));
    assert!(h.app.modals.active.is_none());
    assert_eq!(records(&h), before);
    assert!(h.app.is_dirty);
    assert!(h.app.status_left_message().starts_with("저장 실패"));
    assert_eq!(std::fs::read(h.app.survey_path.as_ref().expect("temp path")).expect("read"), source);
    answer(&mut h, "취소");
    assert!(h.app.save_before.is_none());
}

#[test]
fn save_before_input_discard_opens_only_chosen_temp_copy() {
    let mut h = selected(false);
    let path = h.app.survey_path.clone().expect("temp path");
    let source = std::fs::read(&path).expect("read");
    let next = path.with_file_name("next.gpkg");
    std::fs::copy(&path, &next).expect("temp copy");
    h.app.request_open_survey(next.clone());
    h.frame(vec![]);
    answer(&mut h, "저장 안 함");
    assert_eq!(h.app.survey_path, Some(next));
    assert!(h.app.save_before.is_none());
    assert!(!h.app.is_dirty);
    assert_eq!(std::fs::read(&path).expect("read"), source);
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
    let command = if redo { Key::Y } else { Key::Z };
    key(&mut h, command, Modifiers::CTRL);
    assert_eq!(records(&h), before, "history changed records behind save question");
    assert_eq!(h.app.undo_tables.len(), undo);
    assert_eq!(h.app.redo_tables, redo_before);
    answer(&mut h, "취소");
    key(&mut h, command, Modifiers::CTRL);
    assert_ne!(records(&h), before, "normal history did not resume after cancel");
}

#[test]
fn save_before_input_undo_preserves_pending_records() {
    history_is_owned(false);
}

#[test]
fn save_before_input_redo_preserves_pending_records() {
    history_is_owned(true);
}

#[test]
fn save_before_input_typing_survives_cancel_and_explicit_save() {
    let mut h = selected(false);
    let path = h.app.survey_path.clone().expect("temp path");
    let source = std::fs::read(&path).expect("read");
    let at = h.point_at("저장 전 기록");
    h.press(at, PointerButton::Primary);
    assert!(h.ctx.wants_keyboard_input(), "actual memo click did not focus TextEdit");
    key(&mut h, Key::A, Modifiers { ctrl: true, command: true, ..Modifiers::default() });
    h.frame(vec![Event::Text("draft typed before question".to_owned())]);
    assert_eq!(h.app.home_map.card_drafts.get("note").map(String::as_str), Some("draft typed before question"));
    key(&mut h, Key::N, Modifiers::CTRL);
    assert_eq!(h.app.save_before, Some(SaveBefore::NewSurvey));
    answer(&mut h, "취소");
    let pending = records(&h);
    let poly = &pending.iter().find(|t| t.0 == "feature_poly").expect("polygon table").1[0];
    assert_eq!(poly.attributes.get("note").map(String::as_str), Some("draft typed before question"));
    assert_eq!(std::fs::read(&path).expect("read"), source);
    ask(&mut h);
    answer(&mut h, "저장");
    assert!(h.app.save_before.is_none());
    assert_eq!(h.app.modals.active, Some(ActiveModalDialog::NewSurvey));
    assert!(!h.app.is_dirty);
    let stored = strata_storage::survey_gpkg_read::read_survey_tables(&path).expect("saved temp file");
    let stored: Vec<_> = stored.iter().map(|t| (t.table.clone(), t.layer.features.to_vec())).collect();
    assert_eq!(stored, pending, "explicit Save lost text typed before the question");
}
