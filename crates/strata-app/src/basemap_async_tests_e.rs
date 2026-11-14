//! MAP1c rest-gated tile-request tests (5..8): 20-frame pan gesture request count; cached and
//! stand-in painting plus in-flight completion during movement; cadastral rest and drag gating;
//! and hands-off loading after open, basemap-kind switch, and context-menu refresh.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::HashSet;
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use eframe::egui::{self, Event, Modifiers, PointerButton, Pos2};
use strata_net::tile_fetcher::BasemapSource;

use crate::basemap_cadastral_jobs::{TexKey, MAX_IN_FLIGHT};
use crate::basemap_mesh::tile_mesh_points;
use crate::basemap_paint::{tm_params_for, TileView};
use crate::basemap_tile_key::VIEW_REST_TIME;
use crate::basemap_tiles::TileKind;

use super::async_tests_a::{map_app_at, one_px, run_frame, visible_tiles, CHUNCHEON};
use super::async_tests_b::collect_meshes;
use super::async_tests_c::flush_ctx;
use super::async_tests_d::{install_clock, run_frame_events, trigger_context_menu_refresh};

/// (5) Scripted gesture: 20 frames of panning through views that all miss tiles, then rest — starts
/// requests only for the final view.
#[test]
fn test_map1c_5_twenty_frame_pan_gesture_then_rest_requests_only_final_view() {
    let (mut app, rect) = map_app_at(CHUNCHEON.0, CHUNCHEON.1, 1.0, [900.0, 600.0]);
    let (t0, now_cell) = install_clock(&mut app);
    let ctx = egui::Context::default();
    let base_x = app.home_map.viewport.center_xy[0];
    let requested = Arc::new(Mutex::new(Vec::<TexKey>::new()));

    for step in 0..20_u64 {
        app.home_map.viewport.center_xy[0] = base_x + (step as f64) * 3_500.0;
        *now_cell.lock().unwrap() = t0 + Duration::from_millis(step * 16);
        run_frame(&ctx, rect, |ctx, p| {
            app.paint_basemap_layer_with(ctx, p, rect, |kind, _key, z, tx, ty| {
                let req = Arc::clone(&requested);
                move || {
                    req.lock().unwrap().push((kind, z, tx, ty));
                    Some(one_px())
                }
            });
        });
        app.basemap_fetch.wait_for_idle();
    }
    let during_pan = app.basemap_fetch.requests_started();
    let final_vis: HashSet<TexKey> = visible_tiles(&app, rect).into_iter().map(|(_, k)| k).collect();

    *now_cell.lock().unwrap() = t0 + Duration::from_millis(19 * 16) + VIEW_REST_TIME;
    for _ in 0..14 {
        run_frame(&ctx, rect, |ctx, p| {
            app.paint_basemap_layer_with(ctx, p, rect, |kind, _key, z, tx, ty| {
                let req = Arc::clone(&requested);
                move || {
                    req.lock().unwrap().push((kind, z, tx, ty));
                    Some(one_px())
                }
            });
        });
        app.basemap_fetch.wait_for_idle();
        if app.basemap_tiles_painted == final_vis.len() {
            break;
        }
    }
    let total = app.basemap_fetch.requests_started();
    assert_eq!(
        (during_pan, total),
        (0, final_vis.len()),
        "20-frame pan + rest started {total} requests ({during_pan} during pan) instead of {} final-view tiles",
        final_vis.len()
    );
    assert!(requested.lock().unwrap().iter().all(|k| final_vis.contains(k)));
}

/// (6) Cached tiles and stand-ins paint during movement, and already in-flight jobs finish and are used.
#[test]
fn test_map1c_6_cached_tiles_and_stand_ins_paint_during_movement_and_in_flight_jobs_complete() {
    let (mut app, rect) = map_app_at(CHUNCHEON.0, CHUNCHEON.1, 1.0, [320.0, 240.0]);
    let (t0, now_cell) = install_clock(&mut app);
    let ctx = egui::Context::default();
    let v0_first = visible_tiles(&app, rect)[0].1;

    let (stx, srx) = mpsc::channel::<()>();
    let (rtx, rrx) = mpsc::channel::<()>();
    let rrx = Arc::new(Mutex::new(rrx));
    for dt in [Duration::ZERO, VIEW_REST_TIME] {
        *now_cell.lock().unwrap() = t0 + dt;
        run_frame(&ctx, rect, |ctx, p| {
            app.paint_basemap_layer_with(ctx, p, rect, |kind, _key, z, tx, ty| {
                let (stx, rrx) = (stx.clone(), Arc::clone(&rrx));
                move || {
                    if (kind, z, tx, ty) == v0_first { let _ = stx.send(()); let _ = rrx.lock().unwrap().recv(); }
                    Some(one_px())
                }
            });
        });
    }
    srx.recv().unwrap();
    let started_before_move = app.basemap_fetch.requests_started();

    app.home_map.viewport.center_xy[0] += 4_000.0;
    let v1_vis: Vec<TexKey> = visible_tiles(&app, rect).into_iter().map(|(_, k)| k).collect();
    let (exact_k, stand_child) = (v1_vis[0], v1_vis[1]);
    let anc_k = (TileKind::Satellite, stand_child.1 - 1, stand_child.2 >> 1, stand_child.3 >> 1);
    app.basemap_textures.insert(exact_k, ctx.load_texture("exact_v1", one_px(), egui::TextureOptions::LINEAR));
    let anc_tex = ctx.load_texture("anc_v1", one_px(), egui::TextureOptions::LINEAR);
    let anc_id = anc_tex.id();
    app.basemap_textures.insert(anc_k, anc_tex);

    let view1 = TileView::from_viewport(&app.home_map.viewport, rect, app.work_crs);
    let params = tm_params_for(app.work_crs).unwrap();
    let child_pos0 = tile_mesh_points(&view1, &params, rect, stand_child.1, stand_child.2, stand_child.3).unwrap()[0];

    *now_cell.lock().unwrap() = t0 + VIEW_REST_TIME + Duration::from_millis(16);
    let out_move = run_frame(&ctx, rect, |ctx, p| {
        app.paint_basemap_layer_with(ctx, p, rect, |_k, _key, _z, _x, _y| move || Some(one_px()));
    });
    assert_eq!(app.basemap_fetch.requests_started(), started_before_move, "movement must start zero new requests");
    assert_eq!(app.basemap_tiles_painted, 1, "cached exact-zoom tile must paint during movement");
    let mut meshes = Vec::new();
    out_move.shapes.iter().for_each(|c| collect_meshes(&c.shape, &mut meshes));
    assert!(meshes.iter().any(|m| m.texture_id == anc_id && (m.vertices[0].pos - child_pos0).length() < 1e-3));

    rtx.send(()).unwrap();
    app.basemap_fetch.wait_for_idle();
    app.home_map.viewport.center_xy[0] += 500.0;
    *now_cell.lock().unwrap() = t0 + VIEW_REST_TIME + Duration::from_millis(32);
    run_frame(&ctx, rect, |ctx, p| {
        app.paint_basemap_layer_with(ctx, p, rect, |_k, _key, _z, _x, _y| move || Some(one_px()));
    });
    assert!(app.basemap_textures.get(&v0_first).is_some(), "in-flight job from before movement must be uploaded");
}

/// (7) Cadastral path follows the same rest rule across moving, dragging, sub-rest, and rested frames.
#[test]
fn test_map1c_7_cadastral_layer_waits_for_rest_and_ignores_moving_and_dragging_views() {
    let (mut app, rect) = map_app_at(CHUNCHEON.0, CHUNCHEON.1, 0.5, [900.0, 600.0]);
    let (t0, now_cell) = install_clock(&mut app);
    let ctx = egui::Context::default();
    let (p0, p1) = (Pos2::new(150.0, 150.0), Pos2::new(210.0, 180.0));

    for step in 0..4_u64 {
        app.home_map.viewport.center_xy[0] += 1_500.0;
        *now_cell.lock().unwrap() = t0 + Duration::from_millis(step * 40);
        run_frame(&ctx, rect, |ctx, p| {
            app.paint_cadastral_layer_with(ctx, p, rect, |_key, _z, _x, _y| move || Some(one_px()));
        });
    }
    assert_eq!(app.cadastral_fetch.requests_started(), 0);

    // Drag in progress across two equal frames > 120 ms apart starts zero cadastral requests.
    *now_cell.lock().unwrap() = t0 + Duration::from_millis(200);
    run_frame_events(
        &ctx,
        rect,
        vec![
            Event::PointerMoved(p0),
            Event::PointerButton { pos: p0, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE },
        ],
        |ctx, p| app.paint_cadastral_layer_with(ctx, p, rect, |_key, _z, _x, _y| move || Some(one_px())),
    );
    *now_cell.lock().unwrap() = t0 + Duration::from_millis(220);
    run_frame_events(&ctx, rect, vec![Event::PointerMoved(p1)], |ctx, p| {
        app.paint_cadastral_layer_with(ctx, p, rect, |_key, _z, _x, _y| move || Some(one_px()));
    });
    *now_cell.lock().unwrap() = t0 + Duration::from_millis(450);
    run_frame_events(&ctx, rect, vec![], |ctx, p| {
        app.paint_cadastral_layer_with(ctx, p, rect, |_key, _z, _x, _y| move || Some(one_px()));
    });
    assert_eq!(app.cadastral_fetch.requests_started(), 0);

    let t_rel = t0 + Duration::from_millis(500);
    *now_cell.lock().unwrap() = t_rel;
    run_frame_events(
        &ctx,
        rect,
        vec![Event::PointerButton { pos: p1, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE }],
        |ctx, p| app.paint_cadastral_layer_with(ctx, p, rect, |_key, _z, _x, _y| move || Some(one_px())),
    );
    flush_ctx(&ctx, rect);
    *now_cell.lock().unwrap() = t_rel + Duration::from_millis(80);
    let out80 = run_frame_events(&ctx, rect, vec![], |ctx, p| {
        app.paint_cadastral_layer_with(ctx, p, rect, |_key, _z, _x, _y| move || Some(one_px()));
    });
    assert_eq!(app.cadastral_fetch.requests_started(), 0);
    assert_eq!(out80.viewport_output[&egui::ViewportId::ROOT].repaint_delay, Duration::from_millis(40));

    *now_cell.lock().unwrap() = t_rel + VIEW_REST_TIME;
    run_frame_events(&ctx, rect, vec![], |ctx, p| {
        app.paint_cadastral_layer_with(ctx, p, rect, |_key, _z, _x, _y| move || Some(one_px()));
    });
    app.cadastral_fetch.wait_for_idle();
    assert_eq!(app.cadastral_fetch.requests_started(), MAX_IN_FLIGHT);
}

/// (8) First view after opening a survey, switching basemap kind, and context-menu refresh loads
/// after `VIEW_REST_TIME` without any mouse movement.
#[test]
fn test_map1c_8_first_view_after_open_kind_switch_and_refresh_loads_after_rest_without_mouse() {
    let (mut app, rect) = map_app_at(CHUNCHEON.0, CHUNCHEON.1, 1.0, [200.0, 200.0]);
    let (t0, now_cell) = install_clock(&mut app);
    let ctx = egui::Context::default();
    flush_ctx(&ctx, rect);

    *now_cell.lock().unwrap() = t0;
    let out0 = run_frame_events(&ctx, rect, vec![], |ctx, p| {
        app.paint_basemap_layer_with(ctx, p, rect, |_k, _key, _z, _x, _y| move || Some(one_px()));
    });
    assert_eq!(app.basemap_fetch.requests_started(), 0);
    assert_eq!(out0.viewport_output[&egui::ViewportId::ROOT].repaint_delay, VIEW_REST_TIME);

    *now_cell.lock().unwrap() = t0 + VIEW_REST_TIME;
    run_frame_events(&ctx, rect, vec![], |ctx, p| {
        app.paint_basemap_layer_with(ctx, p, rect, |_k, _key, _z, _x, _y| move || Some(one_px()));
    });
    app.basemap_fetch.wait_for_idle();
    let after_open = app.basemap_fetch.requests_started();
    assert!(after_open > 0);

    app.left_layers.add_vworld_base_layer();
    app.home_map.active_basemap = BasemapSource::VworldBase;
    run_frame_events(&ctx, rect, vec![], |ctx, p| {
        app.paint_basemap_layer_with(ctx, p, rect, |kind, _key, _z, _x, _y| {
            assert_eq!(kind, TileKind::Base);
            move || Some(one_px())
        });
    });
    app.basemap_fetch.wait_for_idle();
    run_frame_events(&ctx, rect, vec![], |ctx, p| {
        app.paint_basemap_layer_with(ctx, p, rect, |_k, _key, _z, _x, _y| move || Some(one_px()));
    });
    let after_switch = app.basemap_fetch.requests_started();
    assert!(after_switch > after_open);

    trigger_context_menu_refresh(&mut app, &ctx);
    run_frame_events(&ctx, rect, vec![], |ctx, p| {
        app.paint_basemap_layer_with(ctx, p, rect, |_k, _key, _z, _x, _y| move || Some(one_px()));
    });
    app.basemap_fetch.wait_for_idle();
    assert!(app.basemap_fetch.requests_started() > after_switch);
}
