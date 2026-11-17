//! Opening-frame and repeated-close contracts, target/cpp-ref/src/app/MainWindow.cpp:333-352.
use super::{answer, ask, capture, command, event, key, records, selected, stored};
use eframe::egui::{Event, Key, Modifiers, PointerButton, ViewportCommand};

#[test]
fn close_prompt_input_first_close_delete_enter_never_edits_before_save() {
    let mut h = selected(false);
    let before = records(&h);
    let out = h.frame_with_close_request(vec![event(Key::Delete, Modifiers::default()), event(Key::Enter, Modifiers::default())], Modifiers::default());
    assert_eq!(records(&h), before, "opening close frame edited selected records");
    assert!(command(&out, ViewportCommand::Close));
    assert!(!h.app.close_prompt_open && !h.app.is_dirty);
    assert_eq!(stored(&h), before, "closing saved an unintended edit");
    // target/cpp-ref/src/app/MainWindow.cpp:347-352: successful Save must allow the ensuing close.
    let next = h.frame_with_close_request(vec![], Modifiers::default());
    assert!(!command(&next, ViewportCommand::CancelClose) && !h.app.close_prompt_open);
    assert_eq!(records(&h), before);
}

#[test]
fn close_prompt_input_first_close_blocks_nested_ctrl_n() {
    let mut h = selected(false);
    let before = records(&h);
    let out = h.frame_with_close_request(vec![event(Key::N, Modifiers::CTRL)], Modifiers::CTRL);
    assert!(h.app.save_before.is_none(), "closing frame opened a second save/new question");
    assert!(h.app.close_prompt_open && command(&out, ViewportCommand::CancelClose));
    assert_eq!(records(&h), before);
}

#[test]
fn close_prompt_input_first_close_backspace_preserves_capture() {
    let mut h = capture();
    let points = h.app.draw_sketch.points.clone();
    let out = h.frame_with_close_request(vec![event(Key::Backspace, Modifiers::default())], Modifiers::default());
    assert_eq!(h.app.draw_sketch.points, points, "closing frame Backspace changed EPSG:5187 capture");
    assert!(h.app.close_prompt_open && command(&out, ViewportCommand::CancelClose));
}

#[test]
fn close_prompt_input_repeated_close_keeps_blocking_the_backend() {
    let mut h = selected(false);
    let before = records(&h);
    ask(&mut h);
    for _ in 0..2 {
        let out = h.frame_with_close_request(vec![], Modifiers::default());
        assert!(command(&out, ViewportCommand::CancelClose), "repeated close bypassed unanswered question");
        assert!(!command(&out, ViewportCommand::Close));
        assert!(h.app.close_prompt_open && !h.app.closing_unsaved);
        assert_eq!(records(&h), before);
    }
}

#[test]
fn close_prompt_input_existing_save_before_question_keeps_sole_ownership() {
    let mut h = selected(false);
    let before = records(&h);
    h.app.request_new_survey();
    h.frame(vec![]);
    let request = h.app.save_before.clone();
    let out = h.frame_with_close_request(vec![], Modifiers::default());
    assert!(!h.app.close_prompt_open, "root close stacked a second question behind save-before");
    assert_eq!(h.app.save_before, request);
    assert!(command(&out, ViewportCommand::CancelClose));
    assert_eq!(records(&h), before);
}

#[test]
fn close_prompt_input_clean_close_needs_no_question_or_source_write() {
    let mut h = selected(false);
    assert!(h.app.save_survey_project(), "save only the temp copy");
    let source = std::fs::read(h.app.survey_path.as_ref().expect("temp path")).expect("read");
    let out = h.frame_with_close_request(vec![], Modifiers::default());
    assert!(!h.app.close_prompt_open && !command(&out, ViewportCommand::CancelClose));
    assert!(!h.app.is_dirty);
    assert_eq!(std::fs::read(h.app.survey_path.as_ref().expect("temp path")).expect("read"), source);
}

#[test]
fn close_prompt_input_clean_survey_typing_survives_cancel_and_explicit_save() {
    let mut h = selected(false);
    assert!(h.app.save_survey_project());
    let source = std::fs::read(h.app.survey_path.as_ref().expect("temp path")).expect("read");
    h.frame(vec![]);
    let at = h.point_at("저장 전 기록");
    h.press(at, PointerButton::Primary);
    assert!(h.ctx.wants_keyboard_input());
    key(&mut h, Key::A, Modifiers { ctrl: true, command: true, ..Modifiers::default() });
    h.frame(vec![Event::Text("draft retained across close".to_owned())]);
    assert_eq!(h.app.home_map.card_drafts.get("note").map(String::as_str), Some("draft retained across close"));
    assert!(!h.app.is_dirty, "fixture must still have an uncommitted editor draft");
    let mut expected = records(&h);
    let target = &mut expected.iter_mut().find(|t| t.0 == "feature_poly").expect("polygon table").1[0];
    assert_eq!(target.fid, 1);
    target.attributes.insert("note".to_owned(), "draft retained across close".to_owned());
    // target/cpp-ref/src/app/KaFeatureCardEditors.cpp:220: FocusOut commits text; updated_at is a separate touch field.
    target.attributes.remove("updated_at");
    let out = h.frame_with_close_request(vec![], Modifiers::default());
    assert!(h.app.close_prompt_open && command(&out, ViewportCommand::CancelClose), "close ignored text still being typed");
    let pending = records(&h);
    let mut comparable = pending.clone();
    comparable.iter_mut().find(|t| t.0 == "feature_poly").expect("polygon table").1[0].attributes.remove("updated_at");
    assert_eq!(comparable, expected, "close lost or altered the typed record");
    let cancel = answer(&mut h, "취소");
    assert!(!command(&cancel, ViewportCommand::Close) && !h.app.close_prompt_open && h.app.is_dirty);
    assert_eq!(std::fs::read(h.app.survey_path.as_ref().expect("temp path")).expect("read"), source);
    ask(&mut h);
    let save = answer(&mut h, "저장하고 끝내기");
    assert!(command(&save, ViewportCommand::Close) && !h.app.is_dirty);
    assert_eq!(stored(&h), pending);
}

#[test]
fn close_prompt_input_first_close_blocks_a_pending_home_click() {
    let mut h = selected(false);
    let at = h.point_at("홈");
    h.frame(vec![Event::PointerMoved(at)]);
    h.frame(vec![Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::default() }]);
    let before = records(&h);
    let out = h.frame_with_close_request(vec![Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::default() }], Modifiers::default());
    assert_eq!(h.app.active_tab, crate::app_state::MainTab::Map, "closing frame completed a background tab click");
    assert!(h.app.close_prompt_open && command(&out, ViewportCommand::CancelClose));
    assert_eq!(records(&h), before);
}
