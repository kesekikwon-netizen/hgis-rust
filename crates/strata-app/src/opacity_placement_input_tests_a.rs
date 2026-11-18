//! Pointer owners, target/cpp-ref/src/app/KaLayerOpacityRail.cpp:229-260.
use super::*;

#[test]
fn rail_blank_drag_keeps_search_focus_before_slider_takes_it() {
    let mut h = start();
    h.app.right_inspector.selected_layer_key = Some("위성".to_owned());
    let search = h.point_at("     레이어 찾기");
    h.press(search, PointerButton::Primary);
    assert!(h.ctx.wants_keyboard_input(), "actual search lacks focus");
    let owner = h.ctx.memory(|m| m.focused());
    let (_, card) = rectangles(&mut h);
    drag(&mut h, card.min + Vec2::splat(5.0), Vec2::new(80.0, -40.0));
    assert_eq!(h.ctx.memory(|m| m.focused()), owner, "rail stole real text focus");
    h.frame(vec![Event::Text("ab".to_owned())]);
    assert_eq!(h.app.home_map.layer_filter, "ab", "blank card stole subsequent search input");
    h.press(Pos2::new(card.left() + 80.0 + 80.0, card.top() - 40.0 + 40.0), PointerButton::Primary);
    assert!(h.ctx.memory(|m| m.focused()).is_some_and(|id| Some(id) != owner), "C++ ClickFocus slider did not take focus");
    h.frame(vec![Event::Text("cd".to_owned())]);
    assert_eq!(h.app.home_map.layer_filter, "ab", "slider click left search owning subsequent text");
}

#[test]
fn rail_claims_newly_expanded_card_on_its_first_capture_frame() {
    let mut h = start();
    let (host, _) = rectangles(&mut h);
    h.app.start_edit_feature_poly();
    h.app.right_inspector.selected_layer_key = Some("조사구역".to_owned());
    let before = snapshot(&h);
    // Independent active-card top: 86-point card above the 44-point footer and 12-point gap.
    // Press in its new upper margin before egui has seen the expanded card's hit rectangle.
    let at = Pos2::new(host.left() + 17.0, host.bottom() - 86.0 - 56.0 + 5.0);
    h.frame(vec![Event::PointerMoved(at), button(at, PointerButton::Primary, true)]);
    assert!(h.app.draw_sketch.points.is_empty(), "first-frame expanded card leaked a capture press");
    h.frame(vec![button(at, PointerButton::Primary, false)]);
    assert!(snapshot(&h) == before);
}

#[test]
fn rail_primary_drag_survives_an_unrelated_secondary_button_release() {
    let mut h = start();
    let (_, card) = rectangles(&mut h);
    let at = card.min + Vec2::splat(5.0);
    h.frame(vec![Event::PointerMoved(at)]);
    h.frame(vec![button(at, PointerButton::Primary, true)]);
    h.frame(vec![Event::PointerMoved(at + Vec2::new(50.0, -30.0))]);
    h.frame(vec![button(at + Vec2::new(50.0, -30.0), PointerButton::Secondary, true)]);
    h.frame(vec![button(at + Vec2::new(50.0, -30.0), PointerButton::Secondary, false)]);
    h.frame(vec![Event::PointerMoved(at + Vec2::new(100.0, -60.0))]);
    h.frame(vec![button(at + Vec2::new(100.0, -60.0), PointerButton::Primary, false)]);
    close(rectangles(&mut h).1.min - card.min, Vec2::new(100.0, -60.0));
    assert!(!h.app.map_context_menu_open, "rail's secondary sequence opened canvas context menu");
}

#[test]
fn rail_owns_blank_and_slider_presses_during_real_polygon_capture() {
    let mut h = start();
    h.app.start_edit_feature_poly();
    h.app.right_inspector.selected_layer_key = Some("조사구역".to_owned());
    let before = snapshot(&h);
    let center = h.app.home_map.viewport.center_xy;
    let (_, card) = rectangles(&mut h);
    h.press(card.min + Vec2::splat(5.0), PointerButton::Primary);
    assert!(h.app.draw_sketch.points.is_empty(), "card press added EPSG:5187 capture vertex");
    let track = Pos2::new(card.left() + 58.0 + 154.0 * 0.25, card.top() + 40.0);
    h.press(track, PointerButton::Primary);
    assert!((h.app.rail_opacity("조사구역") - 0.25).abs() < 0.01, "slider did not set opacity");
    assert!(h.app.draw_sketch.points.is_empty(), "slider added EPSG:5187 capture vertex");
    close(rectangles(&mut h).1.min - card.min, Vec2::ZERO);
    drag(&mut h, card.min + Vec2::splat(5.0), Vec2::new(100.0, -60.0));
    assert!(h.app.draw_sketch.points.is_empty());
    close(rectangles(&mut h).1.min - card.min, Vec2::new(100.0, -60.0));
    assert_eq!(h.app.home_map.viewport.center_xy, center);
    assert!(snapshot(&h) == before, "opacity UI changed survey/history/bytes");
    let at = rectangles(&mut h).0.center();
    h.press(at, PointerButton::Primary);
    assert_eq!(h.app.draw_sketch.points.len(), 1, "canvas did not recover after rail release");
}

#[test]
fn rail_slider_and_interrupted_blank_drag_keep_separate_owners() {
    let mut h = start();
    h.app.right_inspector.selected_layer_key = Some("위성".to_owned());
    let (_, card) = rectangles(&mut h);
    let center = h.app.home_map.viewport.center_xy;
    let from = Pos2::new(card.left() + 80.0, card.top() + 40.0);
    drag(&mut h, from, Vec2::new(70.0, -6.0));
    close(rectangles(&mut h).1.min - card.min, Vec2::ZERO);
    assert!(h.app.rail_opacity("위성") > 0.5);
    let at = card.min + Vec2::splat(5.0);
    h.frame(vec![Event::PointerMoved(at)]);
    h.frame(vec![button(at, PointerButton::Primary, true)]);
    h.frame(vec![Event::PointerMoved(at + Vec2::new(90.0, -40.0))]);
    let moved = rectangles(&mut h).1;
    close(moved.min - card.min, Vec2::new(90.0, -40.0));
    h.frame(vec![Event::WindowFocused(false), Event::PointerGone]);
    h.frame(vec![Event::WindowFocused(true), Event::PointerMoved(moved.center() + Vec2::new(30.0, 30.0))]);
    close(rectangles(&mut h).1.min - moved.min, Vec2::ZERO);
    h.frame(vec![button(moved.center(), PointerButton::Primary, false)]);
    assert_eq!(h.app.home_map.viewport.center_xy, center, "interrupted rail gesture panned canvas");
    h.app.right_inspector.selected_layer_key = Some("조사구역".to_owned());
    let (_, vector) = rectangles(&mut h);
    let brightness = h.app.home_map.real_survey.layer_brightness;
    h.press(Pos2::new(vector.left() + 130.0, vector.top() + 64.0), PointerButton::Primary);
    assert_eq!(h.app.home_map.real_survey.layer_brightness, brightness, "vector brightness became enabled");
    close(rectangles(&mut h).1.min - vector.min, Vec2::ZERO);
}
