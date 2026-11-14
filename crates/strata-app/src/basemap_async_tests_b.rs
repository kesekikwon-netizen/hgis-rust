//! MAP1 async background-tile tests (5..8): parent stand-in UV mapping, texture-cache eviction limit,
//! shared 6-in-flight ceiling across cadastral and basemap fetchers, and 2-second key-store caching
//! (target/cpp-ref/src/app/MainWindow.cpp:514,526).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui::{self, Rect, Shape};

use crate::basemap_cadastral_jobs::{CachedApiKey, KEY_REFRESH_INTERVAL, MAX_IN_FLIGHT};
use crate::basemap_mesh::tile_mesh_points;
use crate::basemap_paint::{tm_params_for, TileView, BASEMAP_TEXTURE_LIMIT};
use crate::basemap_tiles::TileKind;

use super::async_tests_a::{map_app_at, one_px, run_frame, visible_tiles, CHUNCHEON};

fn collect_meshes(shape: &Shape, out: &mut Vec<Arc<egui::Mesh>>) {
    match shape {
        Shape::Mesh(m) => out.push(Arc::clone(m)),
        Shape::Vec(v) => {
            for s in v {
                collect_meshes(s, out);
            }
        }
        _ => {}
    }
}

fn uv_bounds(mesh: &egui::Mesh) -> Rect {
    let mut r = Rect::NOTHING;
    for v in &mesh.vertices {
        r.extend_with(v.uv);
    }
    r
}

/// (5) With only a parent texture in memory, missing child tiles draw from the matching quadrant UV of the parent,
/// and stand-ins do not count toward `basemap_tiles_painted`.
#[test]
fn test_map1_5_parent_stand_in_draws_matching_quadrant_uv_and_is_not_counted() {
    let (mut app, rect) = map_app_at(CHUNCHEON.0, CHUNCHEON.1, 0.5, [220.0, 220.0]);
    let vis = visible_tiles(&app, rect);
    assert!(!vis.is_empty());
    let (_, (_, z, tx, ty)) = vis[0];
    let (pz, px, py) = (z - 1, tx >> 1, ty >> 1);

    let ctx = egui::Context::default();
    let parent_tex = ctx.load_texture("test_parent_sat", one_px(), egui::TextureOptions::LINEAR);
    let parent_id = parent_tex.id();
    app.basemap_textures
        .0
        .insert((TileKind::Satellite, pz, px, py), parent_tex);

    let view = TileView::from_viewport(&app.home_map.viewport, rect, app.work_crs);
    let params = tm_params_for(app.work_crs).unwrap();
    let child_pts = tile_mesh_points(&view, &params, rect, z, tx, ty).unwrap();

    let caller = std::thread::current().id();
    let (rel_tx, rel_rx) = mpsc::channel::<()>();
    let rel_rx = Arc::new(Mutex::new(rel_rx));
    let out = run_frame(&ctx, rect, |ctx, p| {
        app.paint_basemap_layer_with(ctx, p, rect, |_k, _key, _z, _x, _y| {
            let rx = Arc::clone(&rel_rx);
            move || {
                if std::thread::current().id() != caller {
                    let _ = rx.lock().unwrap().recv();
                }
                None
            }
        });
    });

    assert_eq!(app.basemap_tiles_painted, 0, "stand-ins must not increment basemap_tiles_painted");
    let mut meshes = Vec::new();
    for clipped in &out.shapes {
        collect_meshes(&clipped.shape, &mut meshes);
    }
    let child_mesh = meshes
        .iter()
        .find(|m| {
            m.texture_id == parent_id
                && m.vertices.first().is_some_and(|v| (v.pos - child_pts[0]).length() < 1e-3)
        })
        .expect("missing child tile must be drawn with parent stand-in mesh");

    let u0 = ((tx & 1) as f32) * 0.5;
    let v0 = ((ty & 1) as f32) * 0.5;
    let got_uv = uv_bounds(child_mesh);
    assert!((got_uv.min.x - u0).abs() < 1e-6 && (got_uv.min.y - v0).abs() < 1e-6, "got {got_uv:?}");
    assert!(
        (got_uv.max.x - (u0 + 0.5)).abs() < 1e-6 && (got_uv.max.y - (v0 + 0.5)).abs() < 1e-6,
        "got {got_uv:?}"
    );

    for _ in 0..MAX_IN_FLIGHT {
        let _ = rel_tx.send(());
    }
    app.basemap_fetch.wait_for_idle();
}

/// (6) After views across many different places, `basemap_textures` stays within `BASEMAP_TEXTURE_LIMIT` + current view.
#[test]
fn test_map1_6_texture_cache_stays_bounded_after_visiting_many_places() {
    let (mut app, rect) = map_app_at(CHUNCHEON.0, CHUNCHEON.1, 1.0, [900.0, 600.0]);
    let ctx = egui::Context::default();
    let base_x = app.home_map.viewport.center_xy[0];
    let mut total_distinct_inserted = 0usize;

    // Seed and visit 35 disjoint places (~20 tiles per view -> ~700 tiles > BASEMAP_TEXTURE_LIMIT = 512).
    for step in 0..35_usize {
        app.home_map.viewport.center_xy[0] = base_x + (step as f64) * 4_500.0;
        let vis: Vec<_> = visible_tiles(&app, rect).into_iter().map(|(_, k)| k).collect();
        for &k in &vis {
            if let std::collections::hash_map::Entry::Vacant(e) = app.basemap_textures.0.entry(k) {
                let tex = ctx.load_texture(format!("t_{step}_{}_{}", k.2, k.3), one_px(), egui::TextureOptions::LINEAR);
                e.insert(tex);
                total_distinct_inserted += 1;
            }
        }
        run_frame(&ctx, rect, |ctx, p| {
            app.paint_basemap_layer_with(ctx, p, rect, |_k, _key, _z, _x, _y| move || Some(one_px()));
        });
        assert!(
            app.basemap_textures.len() <= BASEMAP_TEXTURE_LIMIT + vis.len(),
            "cache grew unbounded at step {step}: len={} limit={BASEMAP_TEXTURE_LIMIT} view={}",
            app.basemap_textures.len(),
            vis.len()
        );
    }
    assert!(total_distinct_inserted > BASEMAP_TEXTURE_LIMIT + 100);
    assert!(app.basemap_textures.len() <= BASEMAP_TEXTURE_LIMIT);
}

/// (7) With 6 cadastral jobs in flight, a background tile request is refused, and vice versa.
#[test]
fn test_map1_7_cadastral_and_basemap_share_six_in_flight_ceiling() {
    // mpp = 0.5 => representative scale ~ 1:1,890, where both satellite and cadastral WMS are active.
    let (mut app, rect) = map_app_at(CHUNCHEON.0, CHUNCHEON.1, 0.5, [1480.0, 625.0]);
    let ctx = egui::Context::default();

    // Direction A: 6 cadastral jobs in flight -> background request refused.
    let (cad_started_tx, cad_started_rx) = mpsc::channel::<()>();
    let (cad_rel_tx, cad_rel_rx) = mpsc::channel::<()>();
    let cad_rel_rx = Arc::new(Mutex::new(cad_rel_rx));
    run_frame(&ctx, rect, |ctx, p| {
        app.paint_cadastral_layer_with(ctx, p, rect, |_key, _z, _tx, _ty| {
            let stx = cad_started_tx.clone();
            let rrx = Arc::clone(&cad_rel_rx);
            move || {
                let _ = stx.send(());
                let _ = rrx.lock().unwrap().recv();
                None
            }
        });
    });
    for _ in 0..MAX_IN_FLIGHT {
        cad_started_rx.recv().unwrap();
    }

    let bg_started = Arc::new(AtomicUsize::new(0));
    run_frame(&ctx, rect, |ctx, p| {
        app.paint_basemap_layer_with(ctx, p, rect, |_k, _key, _z, _x, _y| {
            let cnt = Arc::clone(&bg_started);
            move || {
                cnt.fetch_add(1, Ordering::SeqCst);
                None
            }
        });
    });
    assert_eq!(app.basemap_fetch.in_flight_count(), MAX_IN_FLIGHT);
    assert_eq!(bg_started.load(Ordering::SeqCst), 0, "background job must be refused while 6 cadastral jobs run");

    for _ in 0..MAX_IN_FLIGHT {
        cad_rel_tx.send(()).unwrap();
    }
    app.cadastral_fetch.wait_for_idle();
    app.basemap_fetch.wait_for_idle();
    assert_eq!(bg_started.load(Ordering::SeqCst), 0);
    assert_eq!(app.basemap_fetch.in_flight_count(), 0);

    // Direction B: 6 background jobs in flight -> cadastral request refused.
    let (bg_started_tx, bg_started_rx) = mpsc::channel::<()>();
    let (bg_rel_tx, bg_rel_rx) = mpsc::channel::<()>();
    let bg_rel_rx = Arc::new(Mutex::new(bg_rel_rx));
    run_frame(&ctx, rect, |ctx, p| {
        app.paint_basemap_layer_with(ctx, p, rect, |_k, _key, _z, _x, _y| {
            let stx = bg_started_tx.clone();
            let rrx = Arc::clone(&bg_rel_rx);
            move || {
                let _ = stx.send(());
                let _ = rrx.lock().unwrap().recv();
                None
            }
        });
    });
    for _ in 0..MAX_IN_FLIGHT {
        bg_started_rx.recv().unwrap();
    }

    app.home_map.viewport.center_xy[0] += 8_000.0; // fresh cadastral keys not in failed backoff
    let cad_started = Arc::new(AtomicUsize::new(0));
    run_frame(&ctx, rect, |ctx, p| {
        app.paint_cadastral_layer_with(ctx, p, rect, |_key, _z, _tx, _ty| {
            let cnt = Arc::clone(&cad_started);
            move || {
                cnt.fetch_add(1, Ordering::SeqCst);
                None
            }
        });
    });
    assert_eq!(app.cadastral_fetch.in_flight_count(), MAX_IN_FLIGHT);
    assert_eq!(cad_started.load(Ordering::SeqCst), 0, "cadastral job must be refused while 6 background jobs run");

    for _ in 0..MAX_IN_FLIGHT {
        bg_rel_tx.send(()).unwrap();
    }
    app.basemap_fetch.wait_for_idle();
    app.cadastral_fetch.wait_for_idle();
    assert_eq!(cad_started.load(Ordering::SeqCst), 0);
    assert_eq!(app.cadastral_fetch.in_flight_count(), 0);
}

/// (8) Key stores are read once across frames for both paths, and refreshed after 2 seconds.
#[test]
fn test_map1_8_api_key_store_is_not_read_on_every_frame() {
    let (mut app, rect) = map_app_at(CHUNCHEON.0, CHUNCHEON.1, 0.5, [300.0, 300.0]);
    let t0 = Instant::now();
    let now_cell = Arc::new(Mutex::new(t0));
    let key_value = Arc::new(Mutex::new("FAKE-KEY-1".to_owned()));
    let reads = Arc::new(AtomicUsize::new(0));

    let reads_r = Arc::clone(&reads);
    let key_r = Arc::clone(&key_value);
    let now_r = Arc::clone(&now_cell);
    app.vworld_api_key = CachedApiKey::with_reader_and_clock(
        Arc::new(move || {
            reads_r.fetch_add(1, Ordering::SeqCst);
            key_r.lock().unwrap().clone()
        }),
        Arc::new(move || *now_r.lock().unwrap()),
    );

    let ctx = egui::Context::default();
    let seen_keys = Arc::new(Mutex::new(Vec::<String>::new()));

    for step in 0..10_u64 {
        *now_cell.lock().unwrap() = t0 + Duration::from_millis(step * 150); // 0..1.35 s (< 2 s)
        run_frame(&ctx, rect, |ctx, p| {
            app.paint_basemap_layer_with(ctx, p, rect, |_k, api_key, _z, _x, _y| {
                let seen = Arc::clone(&seen_keys);
                let owned = api_key.to_owned();
                move || {
                    seen.lock().unwrap().push(owned);
                    Some(one_px())
                }
            });
            app.paint_cadastral_layer_with(ctx, p, rect, |_api_key, _z, _x, _y| move || Some(one_px()));
        });
        app.basemap_fetch.wait_for_idle();
        app.cadastral_fetch.wait_for_idle();
    }
    assert_eq!(reads.load(Ordering::SeqCst), 1, "key store must not be read on every frame");

    // Save a new key and advance clock to 2 s: next frame picks up the new key.
    *key_value.lock().unwrap() = "FAKE-KEY-2".to_owned();
    *now_cell.lock().unwrap() = t0 + KEY_REFRESH_INTERVAL;
    app.home_map.viewport.center_xy[0] += 5_000.0;
    seen_keys.lock().unwrap().clear();
    run_frame(&ctx, rect, |ctx, p| {
        app.paint_basemap_layer_with(ctx, p, rect, |_k, api_key, _z, _x, _y| {
            let seen = Arc::clone(&seen_keys);
            let owned = api_key.to_owned();
            move || {
                seen.lock().unwrap().push(owned);
                Some(one_px())
            }
        });
    });
    app.basemap_fetch.wait_for_idle();
    assert_eq!(reads.load(Ordering::SeqCst), 2);
    assert!(seen_keys.lock().unwrap().iter().all(|k| k == "FAKE-KEY-2"));
}
