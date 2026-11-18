//! Ordered physical input, target/cpp-ref/src/app/KaLayerOpacityRail.cpp:229-260.
use super::*;

#[test]
fn stationary_same_frame_blank_press_keeps_default_lower_left_after_resize() {
    let mut h = start();
    let (host, card) = rectangles(&mut h);
    h.frame(vec![Event::PointerMoved(host.center())]);
    let at = card.min + Vec2::splat(5.0);
    h.frame(vec![Event::PointerMoved(at), button(at, PointerButton::Primary, true)]);
    h.frame(vec![button(at, PointerButton::Primary, false)]);
    h.size = [1680, 1080];
    let (host, card) = rectangles(&mut h);
    close(Vec2::new(card.left() - host.left(), host.bottom() - card.bottom()), Vec2::new(12.0, 56.0));
}

#[test]
fn batched_blank_drag_claims_press_origin_and_release_outside_card() {
    let mut h = start();
    h.app.start_edit_feature_poly();
    let before = snapshot(&h);
    let center = h.app.home_map.viewport.center_xy;
    let (_, card) = rectangles(&mut h);
    let at = card.min + Vec2::splat(5.0);
    let delta = Vec2::new(100.0, -60.0);
    h.frame(vec![
        Event::PointerMoved(at),
        button(at, PointerButton::Primary, true),
        Event::PointerMoved(at + delta),
        button(at + delta, PointerButton::Primary, false),
    ]);
    close(rectangles(&mut h).1.min - card.min, delta);
    assert!(h.app.draw_sketch.points.is_empty(), "batched rail drag leaked EPSG5187 capture");
    assert_eq!(h.app.home_map.viewport.center_xy, center);
    assert!(snapshot(&h) == before);
}
