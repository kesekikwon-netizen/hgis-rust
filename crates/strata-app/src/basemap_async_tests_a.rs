//! MAP1 async background-tile tests (1..4): non-blocking UI paint, 8-per-frame texture upload cap,
//! centre-first request ordering, and RETRY_AFTER failure backoff
//! (target/cpp-ref/src/app/MainWindow.cpp:514,526).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui::{self, vec2, ColorImage, Pos2, RawInput, Rect};
use strata_geo::crs_transform::lonlat_to_tm;
use strata_net::tile_fetcher::BasemapSource;
use strata_types::crs::WorkCrs;

use crate::app_state::StrataAppState;
use crate::basemap_cadastral_jobs::{
    shared_tile_fetchers_with_clock, CachedApiKey, TexKey, MAX_IN_FLIGHT, MAX_UPLOADS_PER_FRAME,
    RETRY_AFTER,
};
use crate::basemap_mesh::{screen_tile_range, tile_mesh_points};
use crate::basemap_paint::{tm_params_for, TileView};
use crate::basemap_tile_key::VIEW_REST_TIME;
use crate::basemap_tiles::{basemap_zoom_for, TileKind};

pub(super) const CHUNCHEON: (f64, f64) = (127.73, 37.88);

pub(super) fn map_app_at(lon: f64, lat: f64, mpp: f64, size: [f32; 2]) -> (StrataAppState, Rect) {
    let mut app = StrataAppState::new_startup();
    app.left_layers.apply_default_basemaps();
    app.home_map.active_basemap = BasemapSource::VworldSatellite;
    app.vworld_api_key = CachedApiKey::with_reader_and_clock(
        Arc::new(|| "FAKE-TEST-KEY".to_owned()),
        Arc::new(Instant::now),
    );
    let params = tm_params_for(WorkCrs::Epsg5187).unwrap();
    let center = lonlat_to_tm(lon, lat, &params);
    app.home_map.viewport = strata_canvas2d::viewport::ViewportTransform::new(
        center, mpp, [f64::from(size[0]), f64::from(size[1])],
    );
    (app, Rect::from_min_size(Pos2::ZERO, vec2(size[0], size[1])))
}

pub(super) fn one_px() -> ColorImage { ColorImage::new([1, 1], egui::Color32::from_rgb(40, 90, 140)) }

pub(super) fn visible_tiles(app: &StrataAppState, rect: Rect) -> Vec<(f32, TexKey)> {
    let z = basemap_zoom_for(app.home_map.viewport.meters_per_pixel, rect);
    let view = TileView::from_viewport(&app.home_map.viewport, rect, app.work_crs);
    let params = tm_params_for(app.work_crs).unwrap();
    let (x0, y0, x1, y1) = screen_tile_range(&view, &params, rect, z).unwrap();
    let mut out = Vec::new();
    for ty in y0..=y1 {
        for tx in x0..=x1 {
            let Some(pts) = tile_mesh_points(&view, &params, rect, z, tx, ty) else { continue };
            if !Rect::from_points(&pts).intersects(rect) {
                continue;
            }
            let dist_sq = (pts[pts.len() / 2] - rect.center()).length_sq();
            out.push((dist_sq, (TileKind::Satellite, z, tx, ty)));
        }
    }
    out.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| (a.1 .3, a.1 .2).cmp(&(b.1 .3, b.1 .2))));
    out
}

pub(super) fn run_frame(
    ctx: &egui::Context,
    rect: Rect,
    f: impl FnOnce(&egui::Context, &egui::Painter),
) -> egui::FullOutput {
    let mut f = Some(f);
    let raw = RawInput { screen_rect: Some(rect), ..RawInput::default() };
    ctx.run(raw, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(cb) = f.take() {
                cb(ctx, &ui.painter_at(rect));
            }
        });
    })
}

pub(super) fn settle_view_at(app: &mut StrataAppState, ctx: &egui::Context, rect: Rect, now: &Mutex<Instant>) {
    run_frame(ctx, rect, |ctx, p| app.paint_basemap_layer_with(ctx, p, rect, |_k, _key, _z, _x, _y| move || None));
    let next = *now.lock().unwrap() + VIEW_REST_TIME;
    *now.lock().unwrap() = next;
}

pub(super) fn settle_initial_view(app: &mut StrataAppState, ctx: &egui::Context, rect: Rect) -> (Instant, Arc<Mutex<Instant>>) {
    let t0 = Instant::now();
    let now = Arc::new(Mutex::new(t0));
    let r = Arc::clone(&now);
    let (bg, cad) = shared_tile_fetchers_with_clock(Arc::new(move || *r.lock().unwrap()));
    app.basemap_fetch = bg;
    app.cadastral_fetch = cad;
    settle_view_at(app, ctx, rect, &now);
    (t0, now)
}

/// (1) Missing tiles run on background worker threads, never blocking the UI paint call, capped at 6 in flight.
#[test]
fn test_map1_1_paint_never_blocks_ui_thread_and_caps_in_flight_at_six() {
    let (mut app, rect) = map_app_at(CHUNCHEON.0, CHUNCHEON.1, 1.0, [1480.0, 625.0]);
    assert!(visible_tiles(&app, rect).len() > 12);
    let ctx = egui::Context::default();
    settle_initial_view(&mut app, &ctx, rect);
    let caller = std::thread::current().id();
    let on_caller = Arc::new(AtomicUsize::new(0));
    let (started_tx, started_rx) = mpsc::channel::<(TexKey, std::thread::ThreadId)>();
    let (release_tx, release_rx) = mpsc::channel::<()>();
    let release_rx = Arc::new(Mutex::new(release_rx));

    run_frame(&ctx, rect, |ctx, p| {
        app.paint_basemap_layer_with(ctx, p, rect, |kind, _key, z, tx, ty| {
            let (on_caller, started_tx, release_rx) = (Arc::clone(&on_caller), started_tx.clone(), Arc::clone(&release_rx));
            move || {
                let tid = std::thread::current().id();
                if tid == caller { on_caller.fetch_add(1, Ordering::SeqCst); return None; }
                let _ = started_tx.send(((kind, z, tx, ty), tid));
                let _ = release_rx.lock().unwrap().recv();
                Some(one_px())
            }
        });
    });

    assert_eq!(on_caller.load(Ordering::SeqCst), 0, "tile jobs must not run on the UI thread");
    assert_eq!(app.basemap_tiles_painted, 0);
    for _ in 0..MAX_IN_FLIGHT { assert_ne!(started_rx.recv().unwrap().1, caller); }
    assert_eq!(app.basemap_fetch.in_flight_count(), MAX_IN_FLIGHT);
    assert!(started_rx.try_recv().is_err(), "no 7th request may be in flight");
    for _ in 0..MAX_IN_FLIGHT { release_tx.send(()).unwrap(); }
    app.basemap_fetch.wait_for_idle();
}

/// (2) After workers finish, each frame turns at most 8 finished pictures into textures until the view is painted.
#[test]
fn test_map1_2_uploads_at_most_eight_textures_per_frame_until_all_painted() {
    let (mut app, rect) = map_app_at(CHUNCHEON.0, CHUNCHEON.1, 1.0, [420.0, 320.0]);
    let vis: Vec<TexKey> = visible_tiles(&app, rect).into_iter().map(|(_, k)| k).collect();
    assert!(vis.len() >= 10 && vis.len() <= 24, "expected 10..=24 tiles, got {}", vis.len());
    let ctx = egui::Context::default();
    settle_initial_view(&mut app, &ctx, rect);

    for chunk in vis[..10].chunks(MAX_IN_FLIGHT) {
        let (rel_tx, rel_rx) = mpsc::channel::<()>();
        let rel_rx = Arc::new(Mutex::new(rel_rx));
        for &k in chunk {
            let rx = Arc::clone(&rel_rx);
            assert!(app.basemap_fetch.request_with(k, None, move || { let _ = rx.lock().unwrap().recv(); Some(one_px()) }));
        }
        for _ in 0..chunk.len() { rel_tx.send(()).unwrap(); }
        app.basemap_fetch.wait_for_idle();
    }
    assert_eq!(app.basemap_textures.len(), 0);

    let caller = std::thread::current().id();
    let (rel_tx, rel_rx) = mpsc::channel::<()>();
    let rel_rx = Arc::new(Mutex::new(rel_rx));
    let remaining = vis.len() - 10;
    let out_a = run_frame(&ctx, rect, |ctx, p| {
        app.paint_basemap_layer_with(ctx, p, rect, |_kind, _key, _z, _tx, _ty| {
            let rx = Arc::clone(&rel_rx);
            move || { if std::thread::current().id() != caller { let _ = rx.lock().unwrap().recv(); } Some(one_px()) }
        });
    });
    assert_eq!(app.basemap_textures.len(), MAX_UPLOADS_PER_FRAME);
    assert_eq!(app.basemap_tiles_painted, MAX_UPLOADS_PER_FRAME);
    let delay = out_a.viewport_output[&egui::ViewportId::ROOT].repaint_delay;
    assert!(delay.is_zero(), "remaining finished images must request an immediate repaint");

    for _ in 0..remaining.min(MAX_IN_FLIGHT) { let _ = rel_tx.send(()); }
    app.basemap_fetch.wait_for_idle();

    for _ in 0..4 {
        let prev = app.basemap_textures.len();
        run_frame(&ctx, rect, |ctx, p| {
            app.paint_basemap_layer_with(ctx, p, rect, |_k, _key, _z, _x, _y| move || Some(one_px()));
        });
        assert!(app.basemap_textures.len() - prev <= MAX_UPLOADS_PER_FRAME);
        app.basemap_fetch.wait_for_idle();
        if app.basemap_tiles_painted == vis.len() {
            break;
        }
    }
    assert_eq!(app.basemap_tiles_painted, vis.len());
}

/// (3) The first 6 requests are the 6 tiles nearest to the view centre; off-screen unstarted tiles are skipped after pan.
#[test]
fn test_map1_3_requests_six_tiles_nearest_to_view_centre_first() {
    let (mut app, rect) = map_app_at(CHUNCHEON.0, CHUNCHEON.1, 1.0, [1480.0, 625.0]);
    let expected_all: Vec<TexKey> = visible_tiles(&app, rect).into_iter().map(|(_, k)| k).collect();
    assert!(expected_all.len() > 14);
    let expected_first6 = expected_all[..MAX_IN_FLIGHT].to_vec();
    let unstarted_from_view1: std::collections::HashSet<TexKey> =
        expected_all[MAX_IN_FLIGHT..].iter().copied().collect();

    let ctx = egui::Context::default();
    let (_, now_cell) = settle_initial_view(&mut app, &ctx, rect);
    let caller = std::thread::current().id();
    let requested = Arc::new(Mutex::new(Vec::<TexKey>::new()));
    let (started_tx, started_rx) = mpsc::channel::<TexKey>();
    let (rel_tx, rel_rx) = mpsc::channel::<()>();
    let rel_rx = Arc::new(Mutex::new(rel_rx));

    run_frame(&ctx, rect, |ctx, p| {
        app.paint_basemap_layer_with(ctx, p, rect, |kind, _key, z, tx, ty| {
            let requested = Arc::clone(&requested);
            let started_tx = started_tx.clone();
            let rel_rx = Arc::clone(&rel_rx);
            move || {
                let k = (kind, z, tx, ty);
                requested.lock().unwrap().push(k);
                if std::thread::current().id() == caller {
                    return None;
                }
                let _ = started_tx.send(k);
                let _ = rel_rx.lock().unwrap().recv();
                Some(one_px())
            }
        });
    });

    assert_eq!(app.basemap_fetch.in_flight_count(), MAX_IN_FLIGHT, "must start at most 6 requests on first frame");
    for _ in 0..MAX_IN_FLIGHT {
        started_rx.recv().unwrap();
    }
    assert!(started_rx.try_recv().is_err(), "no 7th worker may start");
    let mut got: Vec<TexKey> = requested.lock().unwrap().clone();
    assert_eq!(got.len(), MAX_IN_FLIGHT);
    got.sort_by_key(|k| expected_first6.iter().position(|e| e == k).unwrap_or(usize::MAX));
    assert_eq!(got, expected_first6, "first 6 in-flight tiles must be the 6 nearest to view centre");

    app.home_map.viewport.center_xy[0] += 50_000.0;
    for _ in 0..MAX_IN_FLIGHT { rel_tx.send(()).unwrap(); }
    app.basemap_fetch.wait_for_idle();
    settle_view_at(&mut app, &ctx, rect, &now_cell);

    let (started2_tx, started2_rx) = mpsc::channel::<TexKey>();
    let (rel2_tx, rel2_rx) = mpsc::channel::<()>();
    let rel2_rx = Arc::new(Mutex::new(rel2_rx));
    run_frame(&ctx, rect, |ctx, p| {
        app.paint_basemap_layer_with(ctx, p, rect, |kind, _key, z, tx, ty| {
            let started2_tx = started2_tx.clone();
            let rel2_rx = Arc::clone(&rel2_rx);
            move || {
                let k = (kind, z, tx, ty);
                let _ = started2_tx.send(k);
                let _ = rel2_rx.lock().unwrap().recv();
                Some(one_px())
            }
        });
    });
    for _ in 0..MAX_IN_FLIGHT {
        let k = started2_rx.recv().unwrap();
        assert!(!unstarted_from_view1.contains(&k), "off-screen tile from old view was fetched: {k:?}");
    }
    for _ in 0..MAX_IN_FLIGHT {
        rel2_tx.send(()).unwrap();
    }
    app.basemap_fetch.wait_for_idle();
}

/// (4) A tile whose job failed is not requested again before RETRY_AFTER (30 s), and is requested after it.
#[test]
fn test_map1_4_failed_tile_waits_for_retry_after_before_refetch() {
    let (mut app, rect) = map_app_at(CHUNCHEON.0, CHUNCHEON.1, 1.0, [180.0, 180.0]);
    let vis: Vec<TexKey> = visible_tiles(&app, rect).into_iter().map(|(_, k)| k).collect();
    assert!(!vis.is_empty() && vis.len() <= MAX_IN_FLIGHT);
    let target = vis[0];
    let ctx = egui::Context::default();
    let (_, now_cell) = settle_initial_view(&mut app, &ctx, rect);
    let t_base = *now_cell.lock().unwrap();
    let target_calls = Arc::new(AtomicUsize::new(0));
    for (dt, fail) in [(Duration::ZERO, true), (Duration::from_secs(29), false), (RETRY_AFTER, false)] {
        *now_cell.lock().unwrap() = t_base + dt;
        run_frame(&ctx, rect, |ctx, p| {
            app.paint_basemap_layer_with(ctx, p, rect, |kind, _key, z, tx, ty| {
                let calls = Arc::clone(&target_calls);
                move || {
                    if (kind, z, tx, ty) == target {
                        calls.fetch_add(1, Ordering::SeqCst);
                        if fail { return None; }
                    }
                    Some(one_px())
                }
            });
        });
        app.basemap_fetch.wait_for_idle();
        let expected = if dt < RETRY_AFTER { 1 } else { 2 };
        assert_eq!(target_calls.load(Ordering::SeqCst), expected, "at dt={dt:?}");
    }
    run_frame(&ctx, rect, |ctx, p| {
        app.paint_basemap_layer_with(ctx, p, rect, |_k, _key, _z, _x, _y| move || Some(one_px()));
    });
    assert_eq!(app.basemap_tiles_painted, vis.len());
}
