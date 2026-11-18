//! C++ frame NoFocus and slider ClickFocus, target/cpp-ref/src/app/KaLayerOpacityRail.cpp:18-69.
use super::*;
use eframe::egui::Key;

#[path = "opacity_control_input_tests_a.rs"]
mod pointer;

fn key(h: &mut Harness, key: Key) {
    for pressed in [true, false] {
        h.frame(vec![Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: Default::default(),
        }]);
    }
    h.frame(vec![]);
}
fn click_slider(h: &mut Harness, row: usize) {
    let (_, card) = rectangles(h);
    h.press(Pos2::new(card.left() + 135.0, card.top() + 40.0 + 24.0 * row as f32), PointerButton::Primary);
}
fn raster() -> Harness {
    let mut h = start();
    h.app.right_inspector.selected_layer_key = Some("위성".to_owned());
    h.frame(vec![]);
    h
}
fn opacity(h: &Harness, want: f32) {
    // 1e-6 is >8 f32 epsilons on the normalized0..1 range and <0.001 percent; not a world-coordinate tolerance.
    let got = h.app.rail_opacity("위성");
    assert!((got - want).abs() < 1e-6, "slider value {got} expected {want}");
}
fn brightness(h: &Harness) -> i32 {
    ((h.app.home_map.real_survey.layer_brightness - 0.5) * 510.0).round() as i32
}

#[test]
fn slider_opacity_keys_use_integer_steps_pages_and_endpoints_without_moving_map() {
    let mut h = raster();
    for size in [[1680, 960], [800, 600]] {
        h.size = size;
        h.frame(vec![]);
        click_slider(&mut h, 0);
        opacity(&h, 0.5);
        let center = h.app.home_map.viewport.center_xy;
        let before = snapshot(&h);
        // Public Qt6.11 QSlider/QAbstractSlider contracts: horizontal step1, page10, integer bounds.
        for (pressed, want) in [
            (Key::ArrowRight, 0.51),
            (Key::ArrowLeft, 0.5),
            (Key::PageUp, 0.6),
            (Key::PageDown, 0.5),
            (Key::End, 1.0),
            (Key::ArrowRight, 1.0),
            (Key::Home, 0.0),
            (Key::ArrowLeft, 0.0),
        ] {
            key(&mut h, pressed);
            opacity(&h, want);
        }
        assert_eq!(h.app.home_map.viewport.center_xy, center);
        assert!(snapshot(&h) == before, "slider keys changed survey/history/bytes");
        h.save(&format!("opacity-keyboard-boundary-{}", size[0]));
    }
}

#[test]
fn slider_brightness_keys_retain_signed_integer_range_and_event_order() {
    let mut h = raster();
    click_slider(&mut h, 1);
    assert_eq!(brightness(&h), 0);
    for (pressed, want) in [
        (Key::ArrowRight, 1),
        (Key::PageUp, 11),
        (Key::PageDown, 1),
        (Key::ArrowLeft, 0),
        (Key::Home, -255),
        (Key::ArrowLeft, -255),
        (Key::End, 255),
        (Key::ArrowRight, 255),
    ] {
        key(&mut h, pressed);
        assert_eq!(brightness(&h), want, "brightness key {pressed:?}");
    }
    key(&mut h, Key::Home);
    h.frame(vec![
        Event::Key {
            key: Key::ArrowRight,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Default::default(),
        },
        Event::Key {
            key: Key::ArrowRight,
            physical_key: None,
            pressed: true,
            repeat: true,
            modifiers: Default::default(),
        },
    ]);
    assert_eq!(brightness(&h), -253, "repeat events were dropped");
    h.frame(vec![
        Event::Key {
            key: Key::End,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Default::default(),
        },
        Event::Key {
            key: Key::ArrowLeft,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Default::default(),
        },
    ]);
    assert_eq!(brightness(&h), 254, "key event ordering was collapsed");
    h.save("brightness-keyboard-focus");
}

#[test]
fn slider_enter_and_space_preserve_pending_capture_and_ignore_stale_pointer_position() {
    let mut h = raster();
    h.app.start_edit_feature_poly();
    let (host, _) = rectangles(&mut h);
    for delta in [Vec2::new(-60.0, -50.0), Vec2::new(60.0, -50.0), Vec2::new(60.0, 30.0)] {
        h.press(host.center() + delta, PointerButton::Primary);
    }
    assert_eq!(h.app.draw_sketch.points.len(), 3, "real EPSG5187 draft setup");
    click_slider(&mut h, 0);
    opacity(&h, 0.5);
    let points = h.app.draw_sketch.points.clone();
    let before = snapshot(&h);
    h.frame(vec![Event::PointerMoved(host.center())]);
    for pressed in [Key::Enter, Key::Space] {
        key(&mut h, pressed);
        opacity(&h, 0.5);
        assert_eq!(h.app.draw_sketch.points, points, "slider key committed pending capture");
    }
    assert!(snapshot(&h) == before, "slider key changed original data/history/bytes");
}

#[test]
fn slider_focus_outline_is_visible_and_click_focus_does_not_add_tab_stops() {
    let mut h = raster();
    click_slider(&mut h, 1);
    let bright = h.ctx.memory(|m| m.focused()).expect("brightness ClickFocus");
    click_slider(&mut h, 0);
    let op = h.ctx.memory(|m| m.focused()).expect("opacity ClickFocus");
    assert_ne!(bright, op);
    let (_, card) = rectangles(&mut h);
    let out = h.frame(vec![]);
    assert!(
        out.shapes
            .iter()
            .any(|s| matches!(&s.shape,Shape::Rect(r) if card.contains_rect(r.rect) && r.rect.contains(Pos2::new(card.left()+135.0,card.top()+40.0)) && r.stroke.color==Color32::from_rgb(181,87,58))),
        "focused slider lacks visible outline"
    );
    key(&mut h, Key::Tab);
    assert!(![Some(bright), Some(op)].contains(&h.ctx.memory(|m| m.focused())), "ClickFocus-only sliders became Tab stops");
    h.save("opacity-focus-tab-return");
}

#[test]
fn disabled_vector_brightness_keeps_search_owner_and_does_not_change_display() {
    let mut h = start();
    h.app.right_inspector.selected_layer_key = Some("조사구역".to_owned());
    let at = h.point_at("     레이어 찾기");
    h.press(at, PointerButton::Primary);
    let owner = h.ctx.memory(|m| m.focused());
    let before = h.app.home_map.real_survey.layer_brightness;
    click_slider(&mut h, 1);
    assert_eq!(h.ctx.memory(|m| m.focused()), owner, "disabled brightness took focus");
    h.frame(vec![Event::Text("ab".to_owned())]);
    assert_eq!(h.app.home_map.layer_filter, "ab");
    assert_eq!(h.app.home_map.real_survey.layer_brightness, before);
}
