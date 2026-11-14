//! `MAP1d` wheel-zoom pinning and modal-blocking tests: pointer leaving the map between two
//! notches, wheel notch while a modal window is open, ten-notch bursts (continuous and with one
//! `>= 120 ms` pause inside the burst), and `Shift` + wheel notch.
//!
//! C++ references: `target/cpp-ref/src/app/MainWindow.cpp:510, 2258-2265`,
//! `target/cpp-ref/src/core/LayerOps.cpp:2255-2258`, `target/cpp-ref/src/core/LayerOps.h:44`,
//! `target/cpp-ref/src/app/KaDrawingTools.cpp:102-122`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui::{self, Event, Modifiers, Pos2, RawInput, Rect, Sense};
use strata_net::tile_fetcher::BasemapSource;
use strata_types::crs::WorkCrs;

use super::apply_map_wheel_zoom;
use super::tests::{new_map_app, run_frame, windows_wheel_notch};
use crate::app_state::StrataAppState;
use crate::basemap_cadastral_jobs::{shared_tile_fetchers_with_clock, CachedApiKey, TexKey, MAX_IN_FLIGHT};
use crate::basemap_tile_key::VIEW_REST_TIME;
use crate::basemap_tiles::TileKind;
use crate::dialogs_modal::ActiveModalDialog;
use crate::survey_area_setup::SurveyAreaSetup;

/// EPSG:5186 (Chuncheon central belt, metres: easting, northing) test fixture with injected clock
/// and fake non-empty key, never contacting external services or reading stored user keys.
fn new_burst_app() -> (StrataAppState, egui::Context, Rect, Instant, Arc<Mutex<Instant>>) {
    let mut app = StrataAppState::new_startup();
    app.work_crs = WorkCrs::Epsg5186;
    app.left_layers.apply_default_basemaps();
    app.home_map.active_basemap = BasemapSource::VworldSatellite;
    app.home_map.viewport.center_xy = [200_000.0, 580_000.0];
    app.home_map.viewport.meters_per_pixel = 8.0;
    app.home_map.viewport.screen_size_px = [1200.0, 800.0];
    let t0 = Instant::now();
    let now_cell = Arc::new(Mutex::new(t0));
    let reader = Arc::clone(&now_cell);
    let clock: Arc<dyn Fn() -> Instant + Send + Sync> = Arc::new(move || *reader.lock().unwrap());
    app.vworld_api_key = CachedApiKey::with_reader_and_clock(Arc::new(|| "FAKE-KEY".to_owned()), Arc::clone(&clock));
    let (bg, cad) = shared_tile_fetchers_with_clock(clock);
    app.basemap_fetch = bg;
    app.cadastral_fetch = cad;
    let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(1200.0, 800.0));
    let ctx = egui::Context::default();
    run_burst_frame(&mut app, &ctx, rect, vec![], |_k, _key, _z, _x, _y| move || None);
    (app, ctx, rect, t0, now_cell)
}

fn one_px() -> egui::ColorImage {
    egui::ColorImage::from_rgba_unmultiplied([1, 1], &[40, 80, 120, 255])
}

fn run_burst_frame<M, F>(app: &mut StrataAppState, ctx: &egui::Context, rect: Rect, events: Vec<Event>, make_job: M)
where
    M: FnMut(TileKind, &str, u8, u32, u32) -> F,
    F: FnOnce() -> Option<egui::ColorImage> + Send + 'static,
{
    let mut make_job = Some(make_job);
    let raw = RawInput { screen_rect: Some(rect), predicted_dt: 1.0 / 60.0, events, ..Default::default() };
    let _ = ctx.run(raw, |ctx| {
        egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| {
            let (r, response) = ui.allocate_exact_size(rect.size(), Sense::click_and_drag());
            app.home_map.viewport.screen_size_px = [f64::from(r.width()), f64::from(r.height())];
            if response.hovered() {
                apply_map_wheel_zoom(ui, r, response.hover_pos(), app.map_wheel_block_reason(), &mut app.home_map.viewport);
            }
            if let Some(mj) = make_job.take() {
                app.paint_basemap_layer_with(ctx, &ui.painter_at(r), r, mj);
            }
        });
    });
}

/// First notch inside the map zooms by `1.2`; after the pointer leaves the map (onto a side panel
/// or out of the OS window via `Event::PointerGone`), a second notch leaves the viewport untouched.
#[test]
fn pointer_leaving_map_between_two_notches_ignores_second_notch() {
    let (mut app, ctx) = new_map_app();
    let (inside_map, left_panel) = (Pos2::new(850.0, 480.0), Pos2::new(60.0, 480.0));
    app.home_map.viewport.meters_per_pixel = 12.0;
    let _ = run_frame(&mut app, &ctx, vec![Event::PointerMoved(inside_map)]);
    let _ = run_frame(&mut app, &ctx, vec![Event::PointerMoved(inside_map), windows_wheel_notch(1.0, Modifiers::NONE)]);
    let (mpp_after_first, center_after_first) = (app.home_map.viewport.meters_per_pixel, app.home_map.viewport.center_xy);
    assert!((12.0 / mpp_after_first - 1.2).abs() < 1e-9, "first notch over map must zoom by 1.2");

    // 1) Pointer moves onto the left layer panel outside the map canvas, then rolls a second notch.
    let _ = run_frame(&mut app, &ctx, vec![Event::PointerMoved(left_panel)]);
    let _ = run_frame(&mut app, &ctx, vec![Event::PointerMoved(left_panel), windows_wheel_notch(1.0, Modifiers::NONE)]);
    for _ in 0..40 {
        let _ = run_frame(&mut app, &ctx, vec![Event::PointerMoved(left_panel)]);
    }
    assert!((app.home_map.viewport.meters_per_pixel - mpp_after_first).abs() < 1e-12);
    assert_eq!(app.home_map.viewport.center_xy, center_after_first);

    // 2) Pointer returns to map, leaves the window (`Event::PointerGone`), then a wheel notch arrives.
    let _ = run_frame(&mut app, &ctx, vec![Event::PointerMoved(inside_map)]);
    let _ = run_frame(&mut app, &ctx, vec![Event::PointerGone]);
    let _ = run_frame(&mut app, &ctx, vec![windows_wheel_notch(1.0, Modifiers::NONE)]);
    assert!((app.home_map.viewport.meters_per_pixel - mpp_after_first).abs() < 1e-12);
    assert_eq!(app.home_map.viewport.center_xy, center_after_first);
}

/// Rolling a wheel notch while a modal window is open (`ActiveModalDialog::NewSurvey` or
/// `survey_area_setup`) does nothing whether the pointer is over the modal or over the map beside it.
#[test]
fn wheel_notch_while_modal_window_is_open_does_nothing() {
    let (mut app, ctx) = new_map_app();
    let (over_modal, over_uncovered_map) = (Pos2::new(840.0, 480.0), Pos2::new(450.0, 220.0));
    app.home_map.viewport.meters_per_pixel = 12.0;
    let center0 = app.home_map.viewport.center_xy;

    app.modals.active = Some(ActiveModalDialog::NewSurvey);
    let _ = run_frame(&mut app, &ctx, vec![Event::PointerMoved(over_modal)]);
    let _ = run_frame(&mut app, &ctx, vec![Event::PointerMoved(over_modal), windows_wheel_notch(1.0, Modifiers::NONE)]);
    assert_eq!(app.home_map.viewport.meters_per_pixel, 12.0, "notch over modal window must not zoom map");
    assert_eq!(app.home_map.viewport.center_xy, center0);

    let _ = run_frame(&mut app, &ctx, vec![Event::PointerMoved(over_uncovered_map)]);
    let _ = run_frame(&mut app, &ctx, vec![Event::PointerMoved(over_uncovered_map), windows_wheel_notch(1.0, Modifiers::NONE)]);
    assert_eq!(app.home_map.viewport.meters_per_pixel, 12.0, "notch over uncovered map canvas while NewSurvey modal is open must not zoom map");
    assert_eq!(app.home_map.viewport.center_xy, center0);

    app.modals.active = None;
    app.home_map.survey_area_setup = Some(SurveyAreaSetup::default());
    let _ = run_frame(&mut app, &ctx, vec![Event::PointerMoved(over_uncovered_map)]);
    let _ = run_frame(&mut app, &ctx, vec![Event::PointerMoved(over_uncovered_map), windows_wheel_notch(1.0, Modifiers::NONE)]);
    assert_eq!(app.home_map.viewport.meters_per_pixel, 12.0, "notch while survey_area_setup modal is open must not zoom map");

    app.home_map.survey_area_setup = None;
    let _ = run_frame(&mut app, &ctx, vec![Event::PointerMoved(over_uncovered_map)]);
    let _ = run_frame(&mut app, &ctx, vec![Event::PointerMoved(over_uncovered_map), windows_wheel_notch(1.0, Modifiers::NONE)]);
    assert!((12.0 / app.home_map.viewport.meters_per_pixel - 1.2).abs() < 1e-9);
}

/// Ten wheel notches within 1 s (`50 ms` apart, no gap `>= 120 ms`) start zero tile requests during
/// the burst and request only final-zoom (`z = 17`, `8.0 / 1.2^10` m/px) tiles after `VIEW_REST_TIME`.
#[test]
fn ten_wheel_notches_in_burst_without_pause_request_only_final_zoom_after_rest() {
    let (mut app, ctx, rect, t0, now_cell) = new_burst_app();
    let hover = Pos2::new(480.0, 310.0);
    // Hand-derived EPSG:5186 world point at (480, 310) on 1200x800 (centre (600, 400) = (200_000, 580_000), mpp = 8.0):
    let anchor_world = [199_040.0_f64, 580_720.0_f64];
    let requested = Arc::new(Mutex::new(Vec::<TexKey>::new()));

    for step in 0..10_u64 {
        *now_cell.lock().unwrap() = t0 + Duration::from_millis(step * 50);
        let req = Arc::clone(&requested);
        run_burst_frame(&mut app, &ctx, rect, vec![Event::PointerMoved(hover), windows_wheel_notch(1.0, Modifiers::NONE)], move |kind, _key, z, tx, ty| {
            let req = Arc::clone(&req);
            move || { req.lock().unwrap().push((kind, z, tx, ty)); Some(one_px()) }
        });
        assert_eq!(app.basemap_fetch.requests_started(), 0, "notch {} must start zero requests", step + 1);
    }
    // Hand-derived: 8.0 / 1.2^10 = 8.0 / 6.1917364224; effective mpp * cos(36°) = 1.0453 m -> z = 17.
    let mpp_final = app.home_map.viewport.meters_per_pixel;
    assert!((8.0 / mpp_final - 6.191_736_422_4).abs() < 1e-9);
    let after_world = app.home_map.viewport.screen_to_world([480.0, 310.0]);
    let drift_px = (after_world[0] - anchor_world[0]).hypot(after_world[1] - anchor_world[1]) / mpp_final;
    assert!(drift_px < 1e-6, "10-notch burst must keep cursor world point fixed within 1e-6 px, got {drift_px}");

    let last_notch = t0 + Duration::from_millis(9 * 50);
    *now_cell.lock().unwrap() = last_notch + Duration::from_millis(119);
    run_burst_frame(&mut app, &ctx, rect, vec![Event::PointerMoved(hover)], |_k, _key, _z, _x, _y| move || Some(one_px()));
    assert_eq!(app.basemap_fetch.requests_started(), 0, "119 ms after last notch is still < VIEW_REST_TIME");

    *now_cell.lock().unwrap() = last_notch + VIEW_REST_TIME;
    let (rtx, rrx) = mpsc::channel::<()>();
    let rrx = Arc::new(Mutex::new(rrx));
    let req = Arc::clone(&requested);
    run_burst_frame(&mut app, &ctx, rect, vec![Event::PointerMoved(hover)], move |kind, _key, z, tx, ty| {
        let (req, rrx) = (Arc::clone(&req), Arc::clone(&rrx));
        move || { req.lock().unwrap().push((kind, z, tx, ty)); let _ = rrx.lock().unwrap().recv(); Some(one_px()) }
    });
    assert_eq!(app.basemap_fetch.requests_started(), MAX_IN_FLIGHT, "rest after burst starts ceiling of 6");
    drop(rtx);
    app.basemap_fetch.wait_for_idle();
    let got = requested.lock().unwrap().clone();
    assert_eq!(got.len(), MAX_IN_FLIGHT);
    assert!(got.iter().all(|&(_, z, _, _)| z == 17), "all requested tiles must be at final z=17, got {got:?}");
}

/// Ten notches within 1 s with one `>= 120 ms` pause after notch 5: at the pause, `MAX_IN_FLIGHT`
/// (`6`) requests start at intermediate `z = 16` and are NOT cancelled when notches 6..10 resume;
/// zero new requests start during notches 6..10; and `120 ms` after notch 10, `z = 17` requests
/// start within the shared `MAX_IN_FLIGHT = 6` ceiling alongside any still-running `z = 16` jobs.
#[test]
fn ten_wheel_notches_with_one_pause_starts_six_uncancelled_intermediate_requests_then_final_zoom() {
    let (mut app, ctx, rect, t0, now_cell) = new_burst_app();
    let hover = Pos2::new(600.0, 400.0);
    let started_zs = Arc::new(Mutex::new(Vec::<u8>::new()));

    for step in 0..5_u64 {
        *now_cell.lock().unwrap() = t0 + Duration::from_millis(step * 40);
        run_burst_frame(&mut app, &ctx, rect, vec![Event::PointerMoved(hover), windows_wheel_notch(1.0, Modifiers::NONE)], |_k, _key, _z, _x, _y| move || Some(one_px()));
    }
    assert_eq!(app.basemap_fetch.requests_started(), 0);

    // Pause 150 ms (>= 120 ms) at t = 310 ms: hold all 6 z=16 workers before asserting exact count,
    // then release 4 while keeping 2 in flight so remaining z=17 work shares the 6-job ceiling cleanly.
    let (mid_tx, mid_rx) = mpsc::channel::<TexKey>();
    let ((hold_tx, hold_rx), (rel4_tx, rel4_rx)) = (mpsc::channel::<()>(), mpsc::channel::<()>());
    let (hold_rx, rel4_rx, hold_cnt) = (Arc::new(Mutex::new(hold_rx)), Arc::new(Mutex::new(rel4_rx)), Arc::new(Mutex::new(0_usize)));
    *now_cell.lock().unwrap() = t0 + Duration::from_millis(310);
    let zs = Arc::clone(&started_zs);
    run_burst_frame(&mut app, &ctx, rect, vec![Event::PointerMoved(hover)], move |kind, _key, z, tx, ty| {
        let (zs, mid_tx, hold_rx, rel4_rx, cnt) = (Arc::clone(&zs), mid_tx.clone(), Arc::clone(&hold_rx), Arc::clone(&rel4_rx), Arc::clone(&hold_cnt));
        move || {
            zs.lock().unwrap().push(z);
            let idx = { let mut g = cnt.lock().unwrap(); *g += 1; *g };
            let _ = mid_tx.send((kind, z, tx, ty));
            let _ = if idx <= 2 { hold_rx.lock().unwrap().recv() } else { rel4_rx.lock().unwrap().recv() };
            Some(one_px())
        }
    });
    assert_eq!(app.basemap_fetch.requests_started(), MAX_IN_FLIGHT);
    let mid_keys: Vec<TexKey> = (0..MAX_IN_FLIGHT).map(|_| mid_rx.recv().unwrap()).collect();
    assert!(mid_keys.iter().all(|&(_, z, _, _)| z == 16), "pause after notch 5 must request z=16");
    drop(rel4_tx);
    while app.basemap_fetch.in_flight_count() > 2 {
        std::thread::yield_now();
    }

    // Notches 6..=10 at t = 350..510 ms (< 1.0 s): zero new requests start, 2 in-flight z=16 jobs stay uncancelled.
    for step in 0..5_u64 {
        *now_cell.lock().unwrap() = t0 + Duration::from_millis(350 + step * 40);
        run_burst_frame(&mut app, &ctx, rect, vec![Event::PointerMoved(hover), windows_wheel_notch(1.0, Modifiers::NONE)], |_k, _key, _z, _x, _y| move || Some(one_px()));
        assert_eq!(app.basemap_fetch.requests_started(), MAX_IN_FLIGHT);
        assert_eq!(app.basemap_fetch.in_flight_count(), 2, "in-flight z=16 jobs must not be cancelled");
    }

    // Rest at t = 630 ms (510 + 120 ms < 1.0 s): hold all 4 z=17 workers so none frees a slot early.
    let ((fin_tx, fin_rx), (fin_hold_tx, fin_hold_rx)) = (mpsc::channel::<u8>(), mpsc::channel::<()>());
    let fin_hold_rx = Arc::new(Mutex::new(fin_hold_rx));
    *now_cell.lock().unwrap() = t0 + Duration::from_millis(510) + VIEW_REST_TIME;
    let zs = Arc::clone(&started_zs);
    run_burst_frame(&mut app, &ctx, rect, vec![Event::PointerMoved(hover)], move |_k, _key, z, _x, _y| {
        let (zs, fin_tx, fin_hold_rx) = (Arc::clone(&zs), fin_tx.clone(), Arc::clone(&fin_hold_rx));
        move || { zs.lock().unwrap().push(z); let _ = fin_tx.send(z); let _ = fin_hold_rx.lock().unwrap().recv(); Some(one_px()) }
    });
    assert_eq!(app.basemap_fetch.requests_started(), MAX_IN_FLIGHT + 4);
    for _ in 0..4 {
        assert_eq!(fin_rx.recv().unwrap(), 17);
    }
    drop(hold_tx);
    drop(fin_hold_tx);
    app.basemap_fetch.wait_for_idle();
    // `MAX_UPLOADS_PER_FRAME` (8) caps each frame while 10 tiles (6 z=16 + 4 z=17) finished, so drain two frames.
    for _ in 0..2 {
        run_burst_frame(&mut app, &ctx, rect, vec![Event::PointerMoved(hover)], |_k, _key, _z, _x, _y| move || None);
        app.basemap_fetch.wait_for_idle();
    }
    assert!(app.basemap_textures.get(&mid_keys[0]).is_some(), "completed z=16 job uploads for stand-in use");
    let log = started_zs.lock().unwrap().clone();
    assert_eq!((&log[..6], &log[6..]), (&[16; 6][..], &[17; 4][..]));
}

/// `Shift` + wheel notch zooms by `1.2` around the mouse cursor just like a plain notch
/// (C++ `MainWindow.cpp:510, 2258-2265` and `LayerOps.cpp:2255-2258` define no `Shift` rule on
/// `m_canvas`, while `egui-0.31.1/src/input_state/mod.rs:327-331` turns `raw_scroll_delta` sideways).
#[test]
fn shift_plus_wheel_notch_zooms_by_1_2_around_cursor_like_plain_notch() {
    let (mut app, ctx) = new_map_app();
    let hover = Pos2::new(640.0, 340.0);
    app.home_map.viewport.meters_per_pixel = 12.0;
    let _ = run_frame(&mut app, &ctx, vec![Event::PointerMoved(hover)]);
    let world_before = app.home_map.cursor_world_xy;

    let shift = Modifiers { shift: true, ..Modifiers::NONE };
    let _ = run_frame(&mut app, &ctx, vec![Event::PointerMoved(hover), windows_wheel_notch(1.0, shift)]);
    let raw_scroll = ctx.input(|i| i.raw_scroll_delta);
    assert_eq!(raw_scroll.y, 0.0, "egui turns Shift + wheel into horizontal raw_scroll_delta");
    assert!(raw_scroll.x > 0.0);

    for _ in 0..40 {
        let _ = run_frame(&mut app, &ctx, vec![Event::PointerMoved(hover)]);
    }
    let mpp_in = app.home_map.viewport.meters_per_pixel;
    assert!((12.0 / mpp_in - 1.2).abs() < 1e-9, "Shift + 1 notch must zoom by 1.2, got {:.12}", 12.0 / mpp_in);
    let world_after = app.home_map.cursor_world_xy;
    let drift_px = (world_after[0] - world_before[0]).hypot(world_after[1] - world_before[1]) / mpp_in;
    assert!(drift_px < 1e-6, "Shift + wheel must keep cursor world point fixed within 1e-6 px, got {drift_px}");

    let _ = run_frame(&mut app, &ctx, vec![Event::PointerMoved(hover), windows_wheel_notch(-1.0, shift)]);
    assert!((app.home_map.viewport.meters_per_pixel - 12.0).abs() < 1e-9);
}
