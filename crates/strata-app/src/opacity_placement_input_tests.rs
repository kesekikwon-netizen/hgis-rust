//! Real map rail interactions. target/cpp-ref/tests/test_shell_focus.cpp:144-172.
use super::popup_input_harness::Harness;
use crate::app_state::{MainTab, StrataAppState};
use eframe::egui::{Color32, Event, PointerButton, Pos2, Rect, Shape, Vec2};

#[path = "opacity_placement_input_tests_a.rs"]
mod ownership;

fn start() -> Harness {
    let directory = tempfile::tempdir().expect("owned temporary survey");
    let file = directory.path().join("demo.gpkg");
    std::fs::copy(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg"), &file).expect("tracked demo copy");
    let mut app = StrataAppState::new_startup();
    app.vworld_api_key = crate::basemap_cadastral_jobs::CachedApiKey::with_reader_and_clock(std::sync::Arc::new(String::new), std::sync::Arc::new(std::time::Instant::now));
    app.history_gis_key_reader = String::new;
    assert!(app.open_survey_gpkg(&file));
    app.active_tab = MainTab::Map;
    app.active_tool = strata_canvas2d::map_tools::MapToolKind::Pan;
    app.right_inspector.selected_layer_key = None;
    let mut h = Harness::new(app, directory);
    for _ in 0..4 {
        h.frame(vec![]);
    }
    h
}

fn rectangles(h: &mut Harness) -> (Rect, Rect) {
    let output = h.frame(vec![]);
    let card = output
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            Shape::Rect(r) if r.fill == Color32::from_rgba_unmultiplied(252, 251, 247, 248) => Some(r.rect),
            _ => None,
        })
        .expect("actual painted opacity rail");
    let size = h.app.home_map.viewport.screen_size_px;
    let host = output
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            Shape::Rect(r) if r.fill == Color32::WHITE && (f64::from(r.rect.width()) - size[0]).abs() < 0.5 && (f64::from(r.rect.height()) - size[1]).abs() < 0.5 => Some(r.rect),
            _ => None,
        })
        .expect("actual map background matches viewport");
    (host, card)
}

fn close(a: Vec2, b: Vec2) {
    // 0.5 GUI points allows a single physical-pixel rounding at native DPI2; no world-coordinate tolerance.
    assert!((a - b).length() <= 0.5, "screen placement: got {a:?}, expected {b:?}");
}
fn button(at: Pos2, button: PointerButton, pressed: bool) -> Event {
    Event::PointerButton {
        pos: at,
        button,
        pressed,
        modifiers: Default::default(),
    }
}
fn drag(h: &mut Harness, from: Pos2, delta: Vec2) {
    h.frame(vec![Event::PointerMoved(from)]);
    h.frame(vec![button(from, PointerButton::Primary, true)]);
    h.frame(vec![Event::PointerMoved(from + delta)]);
    h.frame(vec![button(from + delta, PointerButton::Primary, false)]);
    h.frame(vec![]);
}
fn snapshot(h: &Harness) -> impl PartialEq {
    (
        super::vertex_crs_input_tests::records(h),
        format!("{:?}|{:?}", h.app.undo_tables, h.app.redo_tables),
        std::fs::read(h.app.survey_path.as_ref().expect("owned survey")).expect("survey bytes"),
        h.app.is_dirty,
    )
}

#[test]
fn rail_default_is_lower_left_without_covering_scale_bar_at_wide_and_compact_sizes() {
    let mut h = start();
    let before = snapshot(&h);
    for size in [[1680, 960], [800, 600], [1280, 720]] {
        h.size = size;
        for selected in [None, Some("조사구역".to_owned())] {
            h.app.right_inspector.selected_layer_key = selected;
            let (host, card) = rectangles(&mut h);
            assert!(host.contains_rect(card));
            close(Vec2::new(card.left() - host.left(), host.bottom() - card.bottom()), Vec2::new(12.0, 56.0));
            h.save(&format!("opacity-lower-left-{}-{}-{}", size[0], size[1], card.height()));
        }
    }
    assert!(snapshot(&h) == before, "layout changed survey/history/bytes");
}

#[test]
fn rail_blank_drag_moves_card_preserves_map_focus_and_returns_to_same_session_place() {
    let mut h = start();
    h.app.right_inspector.selected_layer_key = Some("조사구역".to_owned());
    h.app.home_map.legend_focus = true;
    let before = snapshot(&h);
    let center = h.app.home_map.viewport.center_xy;
    let (_, card) = rectangles(&mut h);
    drag(&mut h, card.min + Vec2::new(5.0, 5.0), Vec2::new(110.0, -90.0));
    close(rectangles(&mut h).1.min - card.min, Vec2::new(110.0, -90.0));
    assert_eq!(h.app.home_map.viewport.center_xy, center, "rail drag panned map");
    assert!(h.app.home_map.legend_focus, "rail stole legend focus");
    assert!(h.ctx.memory(|m| m.focused().is_none()), "rail requested keyboard focus");
    let moved = rectangles(&mut h).1;
    let home = h.point_at("홈");
    h.press(home, PointerButton::Primary);
    assert_eq!(h.app.active_tab, MainTab::Home);
    let map = h.point_at("지도");
    h.press(map, PointerButton::Primary);
    assert_eq!(h.app.active_tab, MainTab::Map);
    close(rectangles(&mut h).1.min - moved.min, Vec2::ZERO);
    assert!(snapshot(&h) == before, "rail moved survey/history/bytes");
    h.save("opacity-moved-and-returned");
}

#[test]
fn rail_manual_place_clamps_whole_card_after_drag_outside_and_window_resize() {
    let mut h = start();
    h.app.right_inspector.selected_layer_key = Some("조사구역".to_owned());
    let (host, card) = rectangles(&mut h);
    drag(&mut h, card.min + Vec2::splat(5.0), Vec2::new(300.0, -200.0));
    close(rectangles(&mut h).1.min - card.min, Vec2::new(300.0, -200.0));
    let at = rectangles(&mut h).1.min + Vec2::splat(5.0);
    drag(&mut h, at, host.size());
    for size in [[800, 600], [1680, 960]] {
        h.size = size;
        let (host, card) = rectangles(&mut h);
        assert!(host.contains_rect(card), "resized rail outside host: {host:?} {card:?}");
        assert!(card.bottom() <= host.bottom() - 44.0, "rail covers fixed bottom controls");
        h.save(&format!("opacity-clamped-{}", size[0]));
    }
}
