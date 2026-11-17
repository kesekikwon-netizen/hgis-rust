//! Capture key ownership, target/cpp-ref/src/app/KaCaptureMapTool.cpp:379-391.
//! Menu isolation follows the owner's data-preservation order, not a claim about Qt shortcut internals.
use super::popup_input_harness::Harness;
use eframe::egui::{Event, Key, PointerButton};

fn key(h: &mut Harness, key: Key) {
    for pressed in [true, false] {
        h.frame(vec![Event::Key { key, physical_key: None, pressed, repeat: false, modifiers: Default::default() }]);
    }
}

fn polygon_count(h: &Harness) -> usize {
    h.app.survey_tables.iter().find(|table| table.table == "feature_poly").expect("actual polygon table").layer.features.len()
}

fn native_menu_preserves_capture(key_code: Key) {
    for (menu, action) in [("레이어", "파일함"), ("기록", "그리기")] {
        let mut h = Harness::drawing();
        h.size = [1024, 768];
        h.frame(vec![]);
        let points = h.app.draw_sketch.points.clone();
        let count = polygon_count(&h);
        let dirty = h.app.is_dirty;
        let tool = h.app.active_tool;
        let at = h.point_at(menu);
        h.press(at, PointerButton::Primary);
        h.point_at(action);
        key(&mut h, key_code);
        h.save(&format!("menu-capture-{menu}-{key_code:?}"));
        assert_eq!(h.app.draw_sketch.points, points, "{menu} {key_code:?} changed pending EPSG:5187 vertices");
        assert_eq!(polygon_count(&h), count, "{menu} {key_code:?} committed capture");
        assert_eq!(h.app.is_dirty, dirty, "{menu} {key_code:?} changed dirty state");
        assert_eq!(h.app.active_tool, tool);
        h.escape();
        assert_eq!(h.app.draw_sketch.points, points, "closing menu discarded capture");
        // Same keys must work again when the actual menu has closed.
        key(&mut h, key_code);
        if key_code == Key::Enter {
            assert!(h.app.draw_sketch.points.is_empty());
            assert_eq!(polygon_count(&h), count + 1, "canvas Enter did not commit after menu close");
        } else {
            assert_eq!(h.app.draw_sketch.points.len(), 2, "canvas undo-vertex did not resume after menu close");
        }
    }
}

#[test]
fn menu_capture_delete_preserves_draft_until_actual_menu_closes() {
    native_menu_preserves_capture(Key::Delete);
}

#[test]
fn menu_capture_backspace_preserves_draft_until_actual_menu_closes() {
    native_menu_preserves_capture(Key::Backspace);
}

#[test]
fn menu_capture_enter_preserves_draft_until_actual_menu_closes() {
    native_menu_preserves_capture(Key::Enter);
}

#[test]
fn menu_capture_target_dropdown_owns_capture_keys() {
    for key_code in [Key::Delete, Key::Backspace, Key::Enter] {
        let mut h = Harness::drawing();
        let points = h.app.draw_sketch.points.clone();
        let count = polygon_count(&h);
        let at = h.point_at("모든 조사 레이어");
        h.press(at, PointerButton::Primary);
        h.point_at("현재 레이어");
        key(&mut h, key_code);
        h.save(&format!("menu-capture-target-{key_code:?}"));
        assert_eq!(h.app.draw_sketch.points, points, "target dropdown {key_code:?} changed draft");
        assert_eq!(polygon_count(&h), count);
        h.escape();
        assert_eq!(h.app.draw_sketch.points, points);
    }
}

#[test]
fn menu_capture_opening_click_and_delete_in_one_batch_preserves_draft() {
    let mut h = Harness::drawing();
    h.size = [1024, 768];
    h.frame(vec![]);
    let points = h.app.draw_sketch.points.clone();
    let at = h.point_at("레이어");
    h.frame(vec![Event::PointerMoved(at)]);
    h.frame(vec![Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: true, modifiers: Default::default() }]);
    h.frame(vec![
        Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: false, modifiers: Default::default() },
        Event::Key { key: Key::Delete, physical_key: None, pressed: true, repeat: false, modifiers: Default::default() },
    ]);
    assert_eq!(h.app.draw_sketch.points, points, "menu opening batch changed draft");
}

#[test]
fn menu_capture_actual_layer_search_owns_text_edit_keys() {
    for key_code in [Key::Backspace, Key::Delete, Key::Enter] {
        let mut h = Harness::drawing();
        let points = h.app.draw_sketch.points.clone();
        let count = polygon_count(&h);
        let at = h.point_at("     레이어 찾기");
        h.press(at, PointerButton::Primary);
        h.frame(vec![Event::Text("ab".to_owned())]);
        assert_eq!(h.app.home_map.layer_filter, "ab", "actual search field did not receive text");
        assert!(h.ctx.wants_keyboard_input(), "actual text control lacks focus");
        key(&mut h, key_code);
        h.save(&format!("menu-capture-search-{key_code:?}"));
        assert_eq!(h.app.draw_sketch.points, points, "search {key_code:?} changed draft");
        assert_eq!(polygon_count(&h), count, "search {key_code:?} committed capture");
        assert_eq!(h.app.home_map.layer_filter, if key_code == Key::Backspace { "a" } else { "ab" });
    }
}
