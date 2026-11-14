//! MAP1b regression tests: worker panic slot recovery (f1), API key change + stale-generation +
//! context-menu refresh clearing failed marks (f2), single scheduled repaint at earliest RETRY_AFTER
//! end (f3), child stand-ins, and stand-in retention under eviction (f4).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::RefCell;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui::{self, RawInput, Rect};

use crate::app_state::StrataAppState;
use crate::basemap_cadastral_jobs::{
    shared_tile_fetchers_with_clock, BasemapFetcher, CachedApiKey, MAX_IN_FLIGHT,
};
use crate::basemap_tile_key::KEY_REFRESH_INTERVAL;
use crate::basemap_mesh::tile_mesh_points;
use crate::basemap_paint::{tm_params_for, TileView, FULL_UV};
use crate::basemap_tiles::TileKind;

use super::async_tests_a::{
    map_app_at, one_px, run_frame, settle_initial_view, settle_view_at, visible_tiles, CHUNCHEON,
};
use super::async_tests_b::{collect_meshes, uv_bounds};
use super::async_tests_d::trigger_context_menu_refresh;

struct ThreadExitSignal(Option<mpsc::Sender<()>>);
impl Drop for ThreadExitSignal {
    fn drop(&mut self) {
        if let Some(tx) = self.0.take() { let _ = tx.send(()); }
    }
}
thread_local! { static ON_THREAD_EXIT: RefCell<Option<ThreadExitSignal>> = const { RefCell::new(None) }; }

fn attach_fake_key_clock(app: &mut StrataAppState, initial: &str) -> (Instant, Arc<Mutex<Instant>>, Arc<Mutex<String>>) {
    let t0 = Instant::now();
    let now_cell = Arc::new(Mutex::new(t0));
    let key_cell = Arc::new(Mutex::new(initial.to_owned()));
    let (kr, nr, nf) = (Arc::clone(&key_cell), Arc::clone(&now_cell), Arc::clone(&now_cell));
    app.vworld_api_key = CachedApiKey::with_reader_and_clock(
        Arc::new(move || kr.lock().unwrap().clone()),
        Arc::new(move || *nr.lock().unwrap()),
    );
    let (bg, cad) = shared_tile_fetchers_with_clock(Arc::new(move || *nf.lock().unwrap()));
    app.basemap_fetch = bg;
    app.cadastral_fetch = cad;
    (t0, now_cell, key_cell)
}

pub(super) fn flush_ctx(ctx: &egui::Context, rect: Rect) {
    let _ = ctx.run(RawInput { screen_rect: Some(rect), ..Default::default() }, |_| {});
}

/// (f1) After 6 panicking workers unwind, keys leave `in_flight`, slots return, failures are recorded,
/// repaints are requested, and a 7th request is accepted immediately.
#[test]
fn test_map1b_f1_panicking_workers_release_in_flight_slots_mark_failed_and_repaint() {
    let (mut app, rect) = map_app_at(CHUNCHEON.0, CHUNCHEON.1, 1.0, [1480.0, 625.0]);
    let vis: Vec<_> = visible_tiles(&app, rect).into_iter().map(|(_, k)| k).collect();
    let ctx = egui::Context::default();
    settle_initial_view(&mut app, &ctx, rect);
    flush_ctx(&ctx, rect);
    let repaints = Arc::new(AtomicUsize::new(0));
    let repaints_cb = Arc::clone(&repaints);
    ctx.set_request_repaint_callback(move |_| { repaints_cb.fetch_add(1, Ordering::SeqCst); });

    let (started_tx, started_rx) = mpsc::channel::<()>();
    let (rel_tx, rel_rx) = mpsc::channel::<()>();
    let rel_rx = Arc::new(Mutex::new(rel_rx));
    let (exit_tx, exit_rx) = mpsc::channel::<()>();
    run_frame(&ctx, rect, |ctx, p| {
        app.paint_basemap_layer_with(ctx, p, rect, |_kind, _key, _z, _tx, _ty| {
            let (stx, rrx, etx) = (started_tx.clone(), Arc::clone(&rel_rx), exit_tx.clone());
            move || {
                ON_THREAD_EXIT.with(|slot| *slot.borrow_mut() = Some(ThreadExitSignal(Some(etx))));
                let _ = stx.send(());
                let _ = rrx.lock().unwrap().recv();
                panic!("simulated worker panic");
            }
        });
    });
    for _ in 0..MAX_IN_FLIGHT { started_rx.recv().unwrap(); }
    for _ in 0..MAX_IN_FLIGHT { rel_tx.send(()).unwrap(); }
    for _ in 0..MAX_IN_FLIGHT { exit_rx.recv().unwrap(); }
    app.basemap_fetch.wait_for_idle();
    assert_eq!(app.basemap_fetch.in_flight_count(), 0, "panicked workers must return all 6 slots");
    for &k in &vis[..MAX_IN_FLIGHT] {
        assert!(!app.basemap_fetch.is_in_flight(k));
        assert!(!app.basemap_fetch.request_with(k, None, || Some(one_px())), "panicked key must enter RETRY_AFTER");
    }
    assert!(repaints.load(Ordering::SeqCst) >= 1, "panicked worker must request repaint");
    assert!(app.basemap_fetch.request_with(vis[MAX_IN_FLIGHT], Some(&ctx), || Some(one_px())));
    app.basemap_fetch.wait_for_idle();
}

/// (f2a) When `CachedApiKey` hands out a different key, failed marks of both fetchers clear and a repaint is requested.
#[test]
fn test_map1b_f2_key_change_clears_failed_marks_and_requests_repaint() {
    let (mut app, rect) = map_app_at(CHUNCHEON.0, CHUNCHEON.1, 0.5, [60.0, 60.0]);
    assert!(visible_tiles(&app, rect).len() <= 2);
    let (t0, now_cell, key_cell) = attach_fake_key_clock(&mut app, "BAD-KEY");
    let ctx = egui::Context::default();
    settle_view_at(&mut app, &ctx, rect, &now_cell);
    run_frame(&ctx, rect, |ctx, p| {
        app.paint_basemap_layer_with(ctx, p, rect, |_k, _key, _z, _x, _y| move || None);
        app.paint_cadastral_layer_with(ctx, p, rect, |_key, _z, _x, _y| move || None);
    });
    app.basemap_fetch.wait_for_idle();
    app.cadastral_fetch.wait_for_idle();
    flush_ctx(&ctx, rect);

    *key_cell.lock().unwrap() = "GOOD-KEY".to_owned();
    *now_cell.lock().unwrap() = t0 + KEY_REFRESH_INTERVAL;
    let (bg_good, cad_good) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
    let (rel_tx, rel_rx) = mpsc::channel::<()>();
    let rel_rx = Arc::new(Mutex::new(rel_rx));
    let out = run_frame(&ctx, rect, |ctx, p| {
        app.paint_basemap_layer_with(ctx, p, rect, |_k, api_key, _z, _x, _y| {
            let (cnt, owned, rx) = (Arc::clone(&bg_good), api_key.to_owned(), Arc::clone(&rel_rx));
            move || { assert_eq!(owned, "GOOD-KEY"); cnt.fetch_add(1, Ordering::SeqCst); let _ = rx.lock().unwrap().recv(); Some(one_px()) }
        });
        app.paint_cadastral_layer_with(ctx, p, rect, |api_key, _z, _x, _y| {
            let (cnt, owned, rx) = (Arc::clone(&cad_good), api_key.to_owned(), Arc::clone(&rel_rx));
            move || { assert_eq!(owned, "GOOD-KEY"); cnt.fetch_add(1, Ordering::SeqCst); let _ = rx.lock().unwrap().recv(); Some(one_px()) }
        });
    });
    assert!(out.viewport_output[&egui::ViewportId::ROOT].repaint_delay.is_zero(), "key change must request repaint");
    for _ in 0..MAX_IN_FLIGHT { let _ = rel_tx.send(()); }
    app.basemap_fetch.wait_for_idle();
    app.cadastral_fetch.wait_for_idle();
    assert!(bg_good.load(Ordering::SeqCst) > 0 && cad_good.load(Ordering::SeqCst) > 0);
}

/// (f2b) A job started under an old key that fails after a new key is handed out must not put its tile into RETRY_AFTER.
#[test]
fn test_map1b_f2_stale_old_key_failure_does_not_reenter_thirty_second_wait() {
    let (mut app, rect) = map_app_at(CHUNCHEON.0, CHUNCHEON.1, 1.0, [140.0, 140.0]);
    let target = visible_tiles(&app, rect)[0].1;
    let (t0, now_cell, key_cell) = attach_fake_key_clock(&mut app, "BAD-KEY");
    let ctx = egui::Context::default();
    settle_view_at(&mut app, &ctx, rect, &now_cell);
    let (started_tx, started_rx) = mpsc::channel::<()>();
    let (rel_tx, rel_rx) = mpsc::channel::<()>();
    let rel_rx = Arc::new(Mutex::new(rel_rx));
    run_frame(&ctx, rect, |ctx, p| {
        app.paint_basemap_layer_with(ctx, p, rect, |kind, _key, z, tx, ty| {
            let (stx, rrx) = (started_tx.clone(), Arc::clone(&rel_rx));
            move || if (kind, z, tx, ty) == target { let _ = stx.send(()); let _ = rrx.lock().unwrap().recv(); None } else { Some(one_px()) }
        });
    });
    started_rx.recv().unwrap();

    *key_cell.lock().unwrap() = "GOOD-KEY".to_owned();
    *now_cell.lock().unwrap() = t0 + KEY_REFRESH_INTERVAL;
    run_frame(&ctx, rect, |ctx, p| {
        app.paint_basemap_layer_with(ctx, p, rect, |_k, _key, _z, _x, _y| move || Some(one_px()));
    });
    rel_tx.send(()).unwrap();
    app.basemap_fetch.wait_for_idle();

    let retried = Arc::new(AtomicUsize::new(0));
    run_frame(&ctx, rect, |ctx, p| {
        app.paint_basemap_layer_with(ctx, p, rect, |kind, api_key, z, tx, ty| {
            let (cnt, owned) = (Arc::clone(&retried), api_key.to_owned());
            move || {
                if (kind, z, tx, ty) == target { assert_eq!(owned, "GOOD-KEY"); cnt.fetch_add(1, Ordering::SeqCst); }
                Some(one_px())
            }
        });
    });
    app.basemap_fetch.wait_for_idle();
    assert_eq!(retried.load(Ordering::SeqCst), 1, "stale old-key failure must not block immediate retry");
}

/// (f2c) Context-menu 「지도 새로고침」 clears failed marks of both fetchers in addition to texture caches.
#[test]
fn test_map1b_f2_context_menu_refresh_clears_failed_marks_of_both_fetchers() {
    let (mut app, rect) = map_app_at(CHUNCHEON.0, CHUNCHEON.1, 0.5, [60.0, 60.0]);
    let ctx = egui::Context::default();
    settle_initial_view(&mut app, &ctx, rect);
    run_frame(&ctx, rect, |ctx, p| {
        app.paint_basemap_layer_with(ctx, p, rect, |_k, _key, _z, _x, _y| move || None);
        app.paint_cadastral_layer_with(ctx, p, rect, |_key, _z, _x, _y| move || None);
    });
    app.basemap_fetch.wait_for_idle();
    app.cadastral_fetch.wait_for_idle();

    app.basemap_textures.insert((TileKind::Satellite, 10, 1, 1), ctx.load_texture("dummy_bg", one_px(), egui::TextureOptions::LINEAR));
    app.cadastral_cells.insert((16, 1, 1), ctx.load_texture("dummy_cad", one_px(), egui::TextureOptions::LINEAR));
    trigger_context_menu_refresh(&mut app, &ctx);
    assert!(app.basemap_textures.is_empty() && app.cadastral_cells.len() == 0, "map.refresh must have run");

    let (bg_retried, cad_retried) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
    run_frame(&ctx, rect, |ctx, p| {
        app.paint_basemap_layer_with(ctx, p, rect, |_k, _key, _z, _x, _y| {
            let cnt = Arc::clone(&bg_retried);
            move || { cnt.fetch_add(1, Ordering::SeqCst); Some(one_px()) }
        });
        app.paint_cadastral_layer_with(ctx, p, rect, |_key, _z, _x, _y| {
            let cnt = Arc::clone(&cad_retried);
            move || { cnt.fetch_add(1, Ordering::SeqCst); Some(one_px()) }
        });
    });
    app.basemap_fetch.wait_for_idle();
    app.cadastral_fetch.wait_for_idle();
    assert!(bg_retried.load(Ordering::SeqCst) > 0 && cad_retried.load(Ordering::SeqCst) > 0);
}

/// (f3) Requests ONE repaint at the earliest RETRY_AFTER end among tiles in view, and none when nothing in view waits.
#[test]
fn test_map1b_f3_schedules_single_repaint_at_earliest_retry_end_only_for_view() {
    let (mut app, rect) = map_app_at(CHUNCHEON.0, CHUNCHEON.1, 1.0, [100.0, 100.0]);
    let vis: Vec<_> = visible_tiles(&app, rect).into_iter().map(|(_, k)| k).collect();
    assert!(vis.len() >= 2 && vis.len() <= MAX_IN_FLIGHT);
    let (t_early, t_late) = (vis[0], vis[1]);
    let t0 = Instant::now();
    let now_cell = Arc::new(Mutex::new(t0));
    let now_r = Arc::clone(&now_cell);
    app.basemap_fetch = BasemapFetcher::with_clock(Arc::new(move || *now_r.lock().unwrap()));
    let ctx = egui::Context::default();

    assert!(app.basemap_fetch.request_with((TileKind::Satellite, t_early.1, 1, 1), None, || None));
    app.basemap_fetch.wait_for_idle();
    *now_cell.lock().unwrap() = t0 + Duration::from_secs(10);
    assert!(app.basemap_fetch.request_with(t_early, None, || None));
    app.basemap_fetch.wait_for_idle();
    *now_cell.lock().unwrap() = t0 + Duration::from_secs(14);
    run_frame(&ctx, rect, |ctx, p| app.paint_basemap_layer_with(ctx, p, rect, |_k, _key, _z, _x, _y| move || None));
    *now_cell.lock().unwrap() = t0 + Duration::from_secs(15);
    run_frame(&ctx, rect, |ctx, p| {
        app.paint_basemap_layer_with(ctx, p, rect, |k, _key, z, x, y| move || ((k, z, x, y) != t_late).then(one_px));
    });
    app.basemap_fetch.wait_for_idle();
    run_frame(&ctx, rect, |ctx, p| {
        app.paint_basemap_layer_with(ctx, p, rect, |_k, _key, _z, _x, _y| move || None);
    });
    flush_ctx(&ctx, rect);

    *now_cell.lock().unwrap() = t0 + Duration::from_secs(22);
    let delays = Arc::new(Mutex::new(Vec::<Duration>::new()));
    let delays_cb = Arc::clone(&delays);
    ctx.set_request_repaint_callback(move |info| delays_cb.lock().unwrap().push(info.delay));
    let raw = RawInput { screen_rect: Some(rect), predicted_dt: 0.0, ..Default::default() };
    let out = ctx.run(raw.clone(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            app.paint_basemap_layer_with(ctx, &ui.painter_at(rect), rect, |_k, _key, _z, _x, _y| move || None);
        });
    });
    assert_eq!(*delays.lock().unwrap(), vec![Duration::from_secs(18)]);
    assert_eq!(out.viewport_output[&egui::ViewportId::ROOT].repaint_delay, Duration::from_secs(18));

    app.home_map.viewport.center_xy[0] += 6_000.0;
    for (_, k) in visible_tiles(&app, rect) {
        app.basemap_textures.insert(k, ctx.load_texture(format!("ok_{}_{}", k.2, k.3), one_px(), egui::TextureOptions::LINEAR));
    }
    flush_ctx(&ctx, rect);
    delays.lock().unwrap().clear();
    let out_idle = ctx.run(raw, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            app.paint_basemap_layer_with(ctx, &ui.painter_at(rect), rect, |_k, _key, _z, _x, _y| move || None);
        });
    });
    assert_eq!(out_idle.viewport_output[&egui::ViewportId::ROOT].repaint_delay, Duration::MAX);
    assert!(delays.lock().unwrap().is_empty());
}

/// (f4a) Direct children one level down in memory draw as stand-ins with `FULL_UV`.
#[test]
fn test_map1b_f4_child_textures_one_level_down_draw_as_stand_in() {
    let (mut app, rect) = map_app_at(CHUNCHEON.0, CHUNCHEON.1, 1.0, [220.0, 200.0]);
    let (_, z, tx, ty) = visible_tiles(&app, rect)[0].1;
    let ctx = egui::Context::default();
    let child_key = (TileKind::Satellite, z + 1, tx * 2 + 1, ty * 2);
    let child_tex = ctx.load_texture("child_stand_in", one_px(), egui::TextureOptions::LINEAR);
    let child_id = child_tex.id();
    app.basemap_textures.insert(child_key, child_tex);

    let view = TileView::from_viewport(&app.home_map.viewport, rect, app.work_crs);
    let params = tm_params_for(app.work_crs).unwrap();
    let child_pts = tile_mesh_points(&view, &params, rect, child_key.1, child_key.2, child_key.3).unwrap();
    let out = run_frame(&ctx, rect, |ctx, p| {
        app.paint_basemap_layer_with(ctx, p, rect, |_k, _key, _z, _x, _y| move || None);
    });
    app.basemap_fetch.wait_for_idle();
    assert_eq!(app.basemap_tiles_painted, 0);
    let mut meshes = Vec::new();
    out.shapes.iter().for_each(|c| collect_meshes(&c.shape, &mut meshes));
    let drawn = meshes.iter()
        .find(|m| m.texture_id == child_id && m.vertices.first().is_some_and(|v| (v.pos - child_pts[0]).length() < 1e-3))
        .expect("missing tile must draw direct child stand-in mesh");
    assert_eq!(uv_bounds(drawn), FULL_UV);
}
