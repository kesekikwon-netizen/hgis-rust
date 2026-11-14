//! Regression tests for `MAP3`: map wheel zoom factor, multi-frame smoothing invariance,
//! cursor-anchored zoom, `Ctrl` fine wheel zoom, and `+` / `−` overlay buttons.
//! Ported from `target/cpp-ref/tests/test_layer_state.cpp:657-668`,
//! `target/cpp-ref/tests/test_save_open.cpp:1517-1549`,
//! `target/cpp-ref/src/core/LayerOps.h:44`, `target/cpp-ref/src/app/KaDrawingTools.cpp:111-115`,
//! and `target/cpp-ref/src/app/KaMapControls.cpp:87-88`.

use eframe::egui::{self, epaint::Shape, Event, Modifiers, MouseWheelUnit, PointerButton, Pos2, RawInput, Rect, Vec2};

use super::wheel_zoom_mpp_factor;
use crate::app_state::{MainTab, StrataAppState};

fn new_map_app() -> (StrataAppState, egui::Context) {
    let mut app = StrataAppState::new_startup();
    app.active_tab = MainTab::Map;
    app.home_map.active_basemap = strata_net::tile_fetcher::BasemapSource::Daedongyeojido;
    app.vworld_api_key = crate::basemap_cadastral_jobs::CachedApiKey::with_reader_and_clock(
        std::sync::Arc::new(String::new),
        std::sync::Arc::new(std::time::Instant::now),
    );
    let ctx = egui::Context::default();
    let _ = run_frame(&mut app, &ctx, vec![]);
    (app, ctx)
}

fn run_frame(app: &mut StrataAppState, ctx: &egui::Context, events: Vec<Event>) -> Vec<egui::epaint::ClippedShape> {
    let raw = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1680.0, 960.0))),
        predicted_dt: 1.0 / 60.0,
        events,
        ..Default::default()
    };
    ctx.run(raw, |ctx| {
        let _ = app.render_workspace_egui(ctx);
    }).shapes
}

/// Windows mouse wheel notch as delivered by `winit 0.30.13` (`src/platform_impl/windows/event_loop.rs:1727-1742`,
/// `src/event.rs:953-959`) -> `egui-winit 0.31.1` (`src/lib.rs:694-711`) -> `egui 0.31.1` (`src/data/input.rs:503-519, 1161-1170`).
fn windows_wheel_notch(notches: f32, modifiers: Modifiers) -> Event {
    Event::MouseWheel {
        unit: MouseWheelUnit::Line,
        delta: Vec2::new(0.0, notches),
        modifiers,
    }
}

fn walk_shapes(shape: &Shape, f: &mut impl FnMut(&Shape)) {
    f(shape);
    if let Shape::Vec(v) = shape {
        v.iter().for_each(|s| walk_shapes(s, f));
    }
}

fn find_zoom_button(shapes: &[egui::epaint::ClippedShape], glyph: &str) -> Pos2 {
    let mut found = None;
    for c in shapes {
        walk_shapes(&c.shape, &mut |s| {
            if let Shape::Text(t) = s {
                if t.galley.text() == glyph && t.pos.x > 1000.0 && t.pos.y < 300.0 {
                    found = Some(Rect::from_min_size(t.pos, t.galley.size()).center());
                }
            }
        });
    }
    found.unwrap_or_else(|| panic!("missing overlay zoom button {glyph:?}"))
}

/// Pure table test: `(wheel_notches, ctrl) -> mpp_factor` (`LayerOps.h:44`, `KaDrawingTools.cpp:111-115`).
#[test]
fn table_driven_wheel_zoom_factor_matches_cpp_and_ctrl_rule() {
    // Hand-derived expected multipliers for meters_per_pixel:
    // base = 1.2 without Ctrl; 1.0 + (1.2 - 1.0) * 0.35 = 1.07 with Ctrl.
    let cases: &[(f64, bool, f64, &str)] = &[
        (0.0, false, 1.0, "zero delta"),
        (1.0, false, 1.0 / 1.2, "one notch in"),
        (-1.0, false, 1.2, "one notch out"),
        (0.5, false, 1.0 / 1.2_f64.sqrt(), "half notch in"),
        (-0.5, false, 1.2_f64.sqrt(), "half notch out"),
        (3.0, false, 1.0 / 1.728, "three notches in (1.2^3)"),
        (-3.0, false, 1.728, "three notches out (1.2^3)"),
        (1.0, true, 1.0 / 1.07, "one notch in with Ctrl (1 + 0.2 * 0.35)"),
        (-1.0, true, 1.07, "one notch out with Ctrl"),
        (0.5, true, 1.0 / 1.07_f64.sqrt(), "half notch in with Ctrl"),
        (3.0, true, 1.0 / (1.07 * 1.07 * 1.07), "three notches in with Ctrl"),
    ];
    for &(notches, ctrl, want, label) in cases {
        let got = wheel_zoom_mpp_factor(notches, ctrl);
        assert!(
            (got - want).abs() < 1e-12,
            "{label}: notches={notches}, ctrl={ctrl}: got {got:.15}, want {want:.15}"
        );
    }
    // Two half-notches multiply to one full notch.
    let half = wheel_zoom_mpp_factor(0.5, false);
    assert!((half * half - wheel_zoom_mpp_factor(1.0, false)).abs() < 1e-15);
}

/// One Windows mouse wheel notch over the map zooms `meters_per_pixel` by `1.2` (`1e-9`) after all egui smoothing frames finish.
#[test]
fn one_wheel_notch_zooms_by_exact_1_2_after_egui_smoothing_ends() {
    let (mut app, ctx) = new_map_app();
    let hover = Pos2::new(850.0, 480.0);
    let _ = run_frame(&mut app, &ctx, vec![Event::PointerMoved(hover)]);
    app.home_map.viewport.meters_per_pixel = 12.0;
    let mpp_before = app.home_map.viewport.meters_per_pixel;
    let denom_before = app.map_scale_denominator();

    let _ = run_frame(&mut app, &ctx, vec![Event::PointerMoved(hover), windows_wheel_notch(1.0, Modifiers::NONE)]);
    for _ in 0..40 {
        let _ = run_frame(&mut app, &ctx, vec![Event::PointerMoved(hover)]);
    }
    let smooth_left = ctx.input(|i| i.smooth_scroll_delta.y);
    assert_eq!(smooth_left, 0.0, "40 frames must finish egui scroll smoothing");

    let mpp_after_in = app.home_map.viewport.meters_per_pixel;
    let zoom_in_ratio = mpp_before / mpp_after_in;
    assert!(
        (zoom_in_ratio - 1.2).abs() < 1e-9,
        "one notch in must change scale by 1.2 within 1e-9, measured ratio={zoom_in_ratio:.12} (mpp {mpp_before} -> {mpp_after_in})"
    );
    let denom_after = app.map_scale_denominator();
    assert_eq!(denom_after, (10.0 * crate::status_bar_text::POINTS_PER_METER).round() as u64);
    assert!(denom_after < denom_before, "status-bar scale denominator follows 1.2x zoom in");

    // One notch backward restores exact original meters_per_pixel.
    let _ = run_frame(&mut app, &ctx, vec![Event::PointerMoved(hover), windows_wheel_notch(-1.0, Modifiers::NONE)]);
    for _ in 0..40 {
        let _ = run_frame(&mut app, &ctx, vec![Event::PointerMoved(hover)]);
    }
    let mpp_after_out = app.home_map.viewport.meters_per_pixel;
    let zoom_out_ratio = mpp_after_out / mpp_after_in;
    assert!(
        (zoom_out_ratio - 1.2).abs() < 1e-9,
        "one notch out must change scale by 1.2 within 1e-9, measured ratio={zoom_out_ratio:.12}"
    );
    assert!((mpp_after_out - mpp_before).abs() < 1e-9);
    assert_eq!(app.map_scale_denominator(), denom_before);

    // Three notches in one gesture multiply by 1.2^3 = 1.728, and zoom_at clamps to 0.01..=5000.0.
    let _ = run_frame(&mut app, &ctx, vec![Event::PointerMoved(hover), windows_wheel_notch(3.0, Modifiers::NONE)]);
    assert!((mpp_before / app.home_map.viewport.meters_per_pixel - 1.728).abs() < 1e-9);
    app.home_map.viewport.meters_per_pixel = 0.011;
    let _ = run_frame(&mut app, &ctx, vec![Event::PointerMoved(hover), windows_wheel_notch(2.0, Modifiers::NONE)]);
    assert_eq!(app.home_map.viewport.meters_per_pixel, 0.01);
    app.home_map.viewport.meters_per_pixel = 4800.0;
    let _ = run_frame(&mut app, &ctx, vec![Event::PointerMoved(hover), windows_wheel_notch(-2.0, Modifiers::NONE)]);
    assert_eq!(app.home_map.viewport.meters_per_pixel, 5000.0);
}

/// One notch split by the toolkit across many frames (`Line` fractional events and `Point` touchpad events) zooms by `1.2` (`1e-9`).
#[test]
fn wheel_notch_split_across_frames_multiplies_to_exact_1_2() {
    // 1) High-resolution wheel: 8 frames of 0.125 lines = 1.0 line notch.
    let (mut app, ctx) = new_map_app();
    let hover = Pos2::new(850.0, 480.0);
    let _ = run_frame(&mut app, &ctx, vec![Event::PointerMoved(hover)]);
    app.home_map.viewport.meters_per_pixel = 12.0;
    let mpp_before = app.home_map.viewport.meters_per_pixel;

    for _ in 0..8 {
        let _ = run_frame(&mut app, &ctx, vec![Event::PointerMoved(hover), windows_wheel_notch(0.125, Modifiers::NONE)]);
    }
    for _ in 0..40 {
        let _ = run_frame(&mut app, &ctx, vec![Event::PointerMoved(hover)]);
    }
    let ratio_line = mpp_before / app.home_map.viewport.meters_per_pixel;
    assert!(
        (ratio_line - 1.2).abs() < 1e-9,
        "8 x 0.125 line wheel frames must zoom by 1.2 within 1e-9, got {ratio_line:.12}"
    );

    // 2) Precision touchpad (`MouseWheelUnit::Point`): 8 frames of 5.0 pt = 40.0 pt (`options.line_scroll_speed = 40.0`).
    app.home_map.viewport.meters_per_pixel = 12.0;
    for _ in 0..8 {
        let ev = Event::MouseWheel { unit: MouseWheelUnit::Point, delta: Vec2::new(0.0, 5.0), modifiers: Modifiers::NONE };
        let _ = run_frame(&mut app, &ctx, vec![Event::PointerMoved(hover), ev]);
    }
    for _ in 0..40 {
        let _ = run_frame(&mut app, &ctx, vec![Event::PointerMoved(hover)]);
    }
    let ratio_pt = 12.0 / app.home_map.viewport.meters_per_pixel;
    assert!(
        (ratio_pt - 1.2).abs() < 1e-9,
        "8 x 5.0 pt touchpad frames must zoom by 1.2 within 1e-9, got {ratio_pt:.12}"
    );
}

/// Off-center mouse cursor keeps the exact world point under the mouse fixed (`< 1e-6 px`).
#[test]
fn wheel_zoom_keeps_world_point_under_off_center_mouse_fixed() {
    let (mut app, ctx) = new_map_app();
    let off_center = Pos2::new(620.0, 310.0);
    app.home_map.viewport.meters_per_pixel = 5.0;
    let _ = run_frame(&mut app, &ctx, vec![Event::PointerMoved(off_center)]);
    let world_before = app.home_map.cursor_world_xy;
    let center_before = app.home_map.viewport.center_xy;

    let _ = run_frame(&mut app, &ctx, vec![Event::PointerMoved(off_center), windows_wheel_notch(1.0, Modifiers::NONE)]);
    for _ in 0..10 {
        let _ = run_frame(&mut app, &ctx, vec![Event::PointerMoved(off_center)]);
    }
    let world_after = app.home_map.cursor_world_xy;
    let mpp_after = app.home_map.viewport.meters_per_pixel;
    let drift_px = (world_after[0] - world_before[0]).hypot(world_after[1] - world_before[1]) / mpp_after;
    assert!(
        drift_px < 1e-6,
        "world point under off-center cursor moved by {drift_px:.9} px ({world_before:?} -> {world_after:?})"
    );
    assert!(
        (app.home_map.viewport.center_xy[0] - center_before[0]).abs() > 1.0,
        "off-center cursor zoom must shift viewport center toward the cursor"
    );
}

/// `Ctrl` + mouse wheel zooms on the canvas by `1.07` per notch (`KaDrawingTools.cpp:113-114`) around the cursor.
#[test]
fn ctrl_wheel_notch_zooms_by_finer_cpp_step_on_canvas() {
    let (mut app, ctx) = new_map_app();
    let hover = Pos2::new(640.0, 340.0);
    app.home_map.viewport.meters_per_pixel = 10.7;
    let _ = run_frame(&mut app, &ctx, vec![Event::PointerMoved(hover)]);
    let world_before = app.home_map.cursor_world_xy;

    let ctrl = Modifiers { ctrl: true, ..Modifiers::NONE };
    let _ = run_frame(&mut app, &ctx, vec![Event::PointerMoved(hover), windows_wheel_notch(1.0, ctrl)]);
    for _ in 0..40 {
        let _ = run_frame(&mut app, &ctx, vec![Event::PointerMoved(hover)]);
    }
    let ratio = 10.7 / app.home_map.viewport.meters_per_pixel;
    assert!((ratio - 1.07).abs() < 1e-9, "Ctrl + 1 notch must zoom by 1.07, got {ratio:.12}");
    let world_after = app.home_map.cursor_world_xy;
    let drift_px = (world_after[0] - world_before[0]).hypot(world_after[1] - world_before[1]) / app.home_map.viewport.meters_per_pixel;
    assert!(drift_px < 1e-6, "Ctrl + wheel must also keep cursor world point fixed, drift={drift_px}");
}

/// Overlay `+` and `−` buttons (`KaMapControls.cpp:87-88` -> `QgsMapCanvas::zoomIn/zoomOut`) zoom by `1.2` around map center.
#[test]
fn overlay_plus_and_minus_buttons_zoom_by_cpp_wheel_factor() {
    let (mut app, ctx) = new_map_app();
    app.home_map.viewport.meters_per_pixel = 12.0;
    let center_before = app.home_map.viewport.center_xy;
    let shapes = run_frame(&mut app, &ctx, vec![]);
    let plus_pos = find_zoom_button(&shapes, "+");
    let minus_pos = find_zoom_button(&shapes, "−");

    let click = |app: &mut StrataAppState, pos: Pos2| {
        let _ = run_frame(app, &ctx, vec![Event::PointerMoved(pos)]);
        let _ = run_frame(app, &ctx, vec![Event::PointerButton { pos, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE }]);
        let _ = run_frame(app, &ctx, vec![Event::PointerButton { pos, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE }]);
    };

    click(&mut app, plus_pos);
    let after_plus = app.home_map.viewport.meters_per_pixel;
    assert!(
        (12.0 / after_plus - 1.2).abs() < 1e-9,
        "+ button must zoom in by 1.2, got ratio {:.12} (mpp {after_plus})",
        12.0 / after_plus
    );
    assert!((app.home_map.viewport.center_xy[0] - center_before[0]).abs() < 1e-9);
    assert!((app.home_map.viewport.center_xy[1] - center_before[1]).abs() < 1e-9);

    click(&mut app, minus_pos);
    let after_minus = app.home_map.viewport.meters_per_pixel;
    assert!(
        (after_minus / after_plus - 1.2).abs() < 1e-9,
        "− button must zoom out by 1.2, got ratio {:.12}",
        after_minus / after_plus
    );
    assert!((after_minus - 12.0).abs() < 1e-9);
}
