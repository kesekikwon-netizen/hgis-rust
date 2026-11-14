//! MAP1c rest-gated tile-request tests (1..4): zero requests while panning, zooming, resizing,
//! or holding a drag in progress; centre-first batch after `VIEW_REST_TIME`; and exact remaining
//! delay passed to `request_repaint_after` with no continuous repaints.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui::{self, Event, Modifiers, PointerButton, Pos2, RawInput, Rect};

use crate::app_state::StrataAppState;
use crate::basemap_cadastral_jobs::{shared_tile_fetchers_with_clock, TexKey, MAX_IN_FLIGHT};
use crate::basemap_tile_key::VIEW_REST_TIME;

use super::async_tests_a::{map_app_at, one_px, run_frame, visible_tiles, CHUNCHEON};
use super::async_tests_c::flush_ctx;

pub(super) fn install_clock(app: &mut StrataAppState) -> (Instant, Arc<Mutex<Instant>>) {
    let t0 = Instant::now();
    let now_cell = Arc::new(Mutex::new(t0));
    let reader = Arc::clone(&now_cell);
    let (bg, cad) = shared_tile_fetchers_with_clock(Arc::new(move || *reader.lock().unwrap()));
    app.basemap_fetch = bg;
    app.cadastral_fetch = cad;
    (t0, now_cell)
}

pub(super) fn run_frame_events(
    ctx: &egui::Context,
    rect: Rect,
    events: Vec<Event>,
    f: impl FnOnce(&egui::Context, &egui::Painter),
) -> egui::FullOutput {
    let mut f = Some(f);
    let raw = RawInput { screen_rect: Some(rect), events, predicted_dt: 0.0, ..Default::default() };
    ctx.run(raw, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(cb) = f.take() {
                cb(ctx, &ui.painter_at(rect));
            }
        });
    })
}

pub(super) fn trigger_context_menu_refresh(app: &mut StrataAppState, ctx: &egui::Context) {
    let screen = Rect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 600.0));
    app.map_context_menu_open = true;
    app.map_context_menu_pos = [40.0, 40.0];
    // Row 4 (0=prev, 1=next, 2=layerZoom, 3=full, 4=map.refresh), each 22 px high inside popup margin.
    let at = Pos2::new(80.0, 40.0 + 6.0 + 4.0 * 22.0 + 11.0);
    for events in [
        vec![],
        vec![Event::PointerMoved(at)],
        vec![Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE }],
        vec![Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE }],
    ] {
        let _ = ctx.run(RawInput { screen_rect: Some(screen), events, ..Default::default() }, |ctx| {
            app.render_map_context_menu(ctx);
        });
    }
    assert!(!app.map_context_menu_open);
}

/// (1) Frames with a changing view (centre, metres per pixel, or canvas size) start zero requests.
#[test]
fn test_map1c_1_moving_view_starts_zero_requests() {
    let (mut app, mut rect) = map_app_at(CHUNCHEON.0, CHUNCHEON.1, 1.0, [900.0, 600.0]);
    let (t0, now_cell) = install_clock(&mut app);
    let ctx = egui::Context::default();

    for step in 0..9_u64 {
        *now_cell.lock().unwrap() = t0 + Duration::from_millis(step * 200);
        match step % 3 {
            0 => app.home_map.viewport.center_xy[0] += 800.0,
            1 => app.home_map.viewport.meters_per_pixel *= 0.9,
            _ => rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(900.0 + step as f32 * 8.0, 600.0)),
        }
        run_frame(&ctx, rect, |ctx, p| {
            app.paint_basemap_layer_with(ctx, p, rect, |_k, _key, _z, _x, _y| move || Some(one_px()));
        });
        app.basemap_fetch.wait_for_idle();
    }
    assert_eq!(app.basemap_fetch.requests_started(), 0, "moving view frames must start zero requests");
}

/// (2) Holding a view for less than `VIEW_REST_TIME` starts zero requests; holding for `VIEW_REST_TIME`
/// starts at most 6, nearest the view centre first.
#[test]
fn test_map1c_2_view_held_below_rest_time_starts_zero_then_at_rest_starts_six_nearest_centre() {
    let (mut app, rect) = map_app_at(CHUNCHEON.0, CHUNCHEON.1, 1.0, [1480.0, 625.0]);
    let expected_first6: Vec<TexKey> =
        visible_tiles(&app, rect).into_iter().take(MAX_IN_FLIGHT).map(|(_, k)| k).collect();
    let (t0, now_cell) = install_clock(&mut app);
    let ctx = egui::Context::default();

    for sub_ms in [0_u64, 60, 119] {
        *now_cell.lock().unwrap() = t0 + Duration::from_millis(sub_ms);
        run_frame(&ctx, rect, |ctx, p| {
            app.paint_basemap_layer_with(ctx, p, rect, |_k, _key, _z, _x, _y| move || Some(one_px()));
        });
        assert_eq!(app.basemap_fetch.requests_started(), 0, "must start zero at {sub_ms} ms < 120 ms");
    }

    *now_cell.lock().unwrap() = t0 + VIEW_REST_TIME;
    let (stx, srx) = mpsc::channel::<TexKey>();
    let (rtx, rrx) = mpsc::channel::<()>();
    let rrx = Arc::new(Mutex::new(rrx));
    run_frame(&ctx, rect, |ctx, p| {
        app.paint_basemap_layer_with(ctx, p, rect, |kind, _key, z, tx, ty| {
            let (stx, rrx) = (stx.clone(), Arc::clone(&rrx));
            move || {
                let _ = stx.send((kind, z, tx, ty));
                let _ = rrx.lock().unwrap().recv();
                Some(one_px())
            }
        });
    });
    assert_eq!(app.basemap_fetch.requests_started(), MAX_IN_FLIGHT);
    let mut got: Vec<TexKey> = (0..MAX_IN_FLIGHT).map(|_| srx.recv().unwrap()).collect();
    got.sort_by_key(|k| expected_first6.iter().position(|e| e == k).unwrap_or(usize::MAX));
    assert_eq!(got, expected_first6, "at rest must start the 6 tiles nearest to centre first");
    for _ in 0..MAX_IN_FLIGHT {
        rtx.send(()).unwrap();
    }
    app.basemap_fetch.wait_for_idle();
}

/// (3) A drag in progress starts zero requests even when the view is equal for two frames (> 120 ms apart).
#[test]
fn test_map1c_3_drag_in_progress_starts_zero_even_when_view_is_equal_for_two_frames() {
    let (mut app, rect) = map_app_at(CHUNCHEON.0, CHUNCHEON.1, 1.0, [900.0, 600.0]);
    let (t0, now_cell) = install_clock(&mut app);
    let ctx = egui::Context::default();
    let (p0, p1) = (Pos2::new(200.0, 200.0), Pos2::new(260.0, 230.0));

    *now_cell.lock().unwrap() = t0;
    run_frame_events(
        &ctx,
        rect,
        vec![
            Event::PointerMoved(p0),
            Event::PointerButton { pos: p0, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE },
        ],
        |ctx, p| app.paint_basemap_layer_with(ctx, p, rect, |_k, _key, _z, _x, _y| move || Some(one_px())),
    );

    app.home_map.viewport.center_xy[0] += 1_200.0;
    *now_cell.lock().unwrap() = t0 + Duration::from_millis(20);
    run_frame_events(&ctx, rect, vec![Event::PointerMoved(p1)], |ctx, p| {
        app.paint_basemap_layer_with(ctx, p, rect, |_k, _key, _z, _x, _y| move || Some(one_px()));
    });
    // Hold button down at p1 without moving: equal view for a second frame 250 ms (> 120 ms) later.
    *now_cell.lock().unwrap() = t0 + Duration::from_millis(270);
    run_frame_events(&ctx, rect, vec![], |ctx, p| {
        app.paint_basemap_layer_with(ctx, p, rect, |_k, _key, _z, _x, _y| move || Some(one_px()));
    });
    assert_eq!(app.basemap_fetch.requests_started(), 0, "drag in progress must start zero requests");

    *now_cell.lock().unwrap() = t0 + Duration::from_millis(300);
    run_frame_events(
        &ctx,
        rect,
        vec![Event::PointerButton { pos: p1, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE }],
        |ctx, p| app.paint_basemap_layer_with(ctx, p, rect, |_k, _key, _z, _x, _y| move || Some(one_px())),
    );
    assert_eq!(app.basemap_fetch.requests_started(), 0, "release frame starts rest timer, not requests");

    *now_cell.lock().unwrap() = t0 + Duration::from_millis(300) + VIEW_REST_TIME;
    let (rtx, rrx) = mpsc::channel::<()>();
    let rrx = Arc::new(Mutex::new(rrx));
    run_frame_events(&ctx, rect, vec![], |ctx, p| {
        app.paint_basemap_layer_with(ctx, p, rect, |_k, _key, _z, _x, _y| {
            let rx = Arc::clone(&rrx);
            move || { let _ = rx.lock().unwrap().recv(); Some(one_px()) }
        });
    });
    assert_eq!(app.basemap_fetch.requests_started(), MAX_IN_FLIGHT);
    drop(rtx);
    app.basemap_fetch.wait_for_idle();
}

/// (4) `request_repaint_after` receives the exact remaining rest time, and never repaints continuously.
#[test]
fn test_map1c_4_request_repaint_after_receives_exact_remaining_rest_time_and_not_after_rest() {
    let (mut app, rect) = map_app_at(CHUNCHEON.0, CHUNCHEON.1, 1.0, [140.0, 140.0]);
    let vis_len = visible_tiles(&app, rect).len();
    assert!(vis_len > 0 && vis_len <= MAX_IN_FLIGHT);
    let (t0, now_cell) = install_clock(&mut app);
    let ctx = egui::Context::default();
    flush_ctx(&ctx, rect);

    let delays = Arc::new(Mutex::new(Vec::<Duration>::new()));
    let delays_cb = Arc::clone(&delays);
    ctx.set_request_repaint_callback(move |info| delays_cb.lock().unwrap().push(info.delay));

    *now_cell.lock().unwrap() = t0;
    let out0 = run_frame_events(&ctx, rect, vec![], |ctx, p| {
        app.paint_basemap_layer_with(ctx, p, rect, |_k, _key, _z, _x, _y| move || Some(one_px()));
    });
    assert_eq!(*delays.lock().unwrap(), vec![Duration::from_millis(120)]);
    assert_eq!(out0.viewport_output[&egui::ViewportId::ROOT].repaint_delay, Duration::from_millis(120));

    flush_ctx(&ctx, rect);
    delays.lock().unwrap().clear();
    *now_cell.lock().unwrap() = t0 + Duration::from_millis(45);
    let out45 = run_frame_events(&ctx, rect, vec![], |ctx, p| {
        app.paint_basemap_layer_with(ctx, p, rect, |_k, _key, _z, _x, _y| move || Some(one_px()));
    });
    assert_eq!(*delays.lock().unwrap(), vec![Duration::from_millis(75)]);
    assert_eq!(out45.viewport_output[&egui::ViewportId::ROOT].repaint_delay, Duration::from_millis(75));

    *now_cell.lock().unwrap() = t0 + VIEW_REST_TIME;
    run_frame_events(&ctx, rect, vec![], |ctx, p| {
        app.paint_basemap_layer_with(ctx, p, rect, |_k, _key, _z, _x, _y| move || Some(one_px()));
    });
    app.basemap_fetch.wait_for_idle();
    run_frame_events(&ctx, rect, vec![], |ctx, p| {
        app.paint_basemap_layer_with(ctx, p, rect, |_k, _key, _z, _x, _y| move || Some(one_px()));
    });
    assert_eq!(app.basemap_tiles_painted, vis_len);

    flush_ctx(&ctx, rect);
    delays.lock().unwrap().clear();
    *now_cell.lock().unwrap() = t0 + Duration::from_millis(300);
    let out_idle = run_frame_events(&ctx, rect, vec![], |ctx, p| {
        app.paint_basemap_layer_with(ctx, p, rect, |_k, _key, _z, _x, _y| move || Some(one_px()));
    });
    assert_eq!(out_idle.viewport_output[&egui::ViewportId::ROOT].repaint_delay, Duration::MAX);
    assert!(delays.lock().unwrap().is_empty(), "must not request continuous repaints after rest");
}
