//! Snap target dropdown input, target/cpp-ref/src/app/KaSnapSettingsWidget.cpp:24-28.
use super::popup_input_harness::Harness;
use eframe::egui::{Event, Key, PointerButton};

#[test]
fn popup_escape_snap_target_dropdown_keeps_pending_polygon() {
    let mut h = Harness::drawing();
    let at = h.point_at("모든 조사 레이어");
    h.press(at, PointerButton::Primary);
    h.point_at("현재 레이어");
    h.close_popup_without_losing_capture("snap-target-popup");
    assert!(!h.ctx.memory(|memory| memory.any_popup_open()), "actual target dropdown stayed open");
    h.escape();
    assert!(h.app.draw_sketch.points.is_empty(), "canvas Escape blocked after dropdown close");
}

#[test]
fn popup_escape_snap_target_choices_survive_reopen_without_changing_capture() {
    let mut h = Harness::drawing();
    let points = h.app.draw_sketch.points.clone();
    for (choice, expected) in [("현재 레이어", true), ("모든 조사 레이어", false)] {
        let current = if h.app.home_map.cad.snap_target_current_only { "현재 레이어" } else { "모든 조사 레이어" };
        let at = h.point_at(current);
        h.press(at, PointerButton::Primary);
        let at = h.point_at(choice);
        h.press(at, PointerButton::Primary);
        assert_eq!(h.app.home_map.cad.snap_target_current_only, expected, "actual dropdown chose the wrong snapping scope");
        assert_eq!(h.app.draw_sketch.points, points, "selecting snapping scope changed unfinished points");
        let at = h.point_at(choice);
        h.press(at, PointerButton::Primary);
        h.point_at(if expected { "모든 조사 레이어" } else { "현재 레이어" });
        h.close_popup_without_losing_capture(if expected { "snap-target-current-reopen" } else { "snap-target-all-reopen" });
        assert_eq!(h.app.home_map.cad.snap_target_current_only, expected, "Escape changed the chosen snapping scope");
    }
}

#[test]
fn popup_escape_snap_target_batched_click_does_not_discard_capture() {
    let mut h = Harness::drawing();
    let at = h.point_at("모든 조사 레이어");
    let points = h.app.draw_sketch.points.clone();
    h.frame(vec![Event::PointerMoved(at)]);
    h.frame(vec![Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: true, modifiers: Default::default() }]);
    h.frame(vec![
        Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: false, modifiers: Default::default() },
        Event::Key { key: Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: Default::default() },
    ]);
    assert_eq!(h.app.draw_sketch.points, points, "batched target dropdown click/Escape discarded capture");
    assert!(!h.ctx.memory(|memory| memory.any_popup_open()), "batched Escape left target dropdown open");
    h.frame(vec![Event::Key { key: Key::Escape, physical_key: None, pressed: false, repeat: false, modifiers: Default::default() }]);
    h.escape();
    assert!(h.app.draw_sketch.points.is_empty());
}
