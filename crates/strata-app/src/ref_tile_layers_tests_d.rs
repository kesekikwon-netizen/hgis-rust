//! Reference tile layers, fourth part: what the independent review of F1 found (2026-11-15).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::{Arc, Mutex};

use eframe::egui::{self, Color32, ColorImage};
use strata_storage::edit_buffer::FeatureRecord;

use super::tests::{add_old_map_and_look_at, draw_survey_area, install_clock, map_app, paint_at_rest, run_frame, work_xy, GYEONGJU};
use super::{ref_tile_layer, DAEDONGYEOJIDO};
use crate::basemap_cadastral_jobs::TexKey;
use crate::basemap_tiles::TileKind;

/// Remove the row, ask for the map again, then Ctrl+Z: the removed row must not come back beside the new one
/// (two rows painted the picture twice).
#[test]
fn test_f1_undo_of_a_removal_does_not_bring_back_a_second_row() {
    let centre = work_xy(GYEONGJU.0, GYEONGJU.1);
    let (mut app, _rect) = map_app(centre, 30.0);
    draw_survey_area(&mut app, centre);
    assert!(app.add_reference_tile_layer(&DAEDONGYEOJIDO));
    assert!(app.remove_layers(&["ref_daedongyeojido".to_owned()]));
    assert!(app.add_reference_tile_layer(&DAEDONGYEOJIDO));
    app.undo_map_action();
    assert_eq!(app.left_layers.entries.iter().filter(|e| e.layer_key == "ref_daedongyeojido").count(), 1);

    // Without the second command the undo brings the one row back, as for every layer.
    assert!(app.remove_layers(&["ref_daedongyeojido".to_owned()]));
    assert!(app.left_layers.entries.iter().all(|e| ref_tile_layer(&e.layer_key).is_none()));
    app.undo_map_action();
    assert_eq!(app.left_layers.entries.iter().filter(|e| e.layer_key == "ref_daedongyeojido").count(), 1);
}

/// The centre of the circle needs a coordinate transformer for the survey-area table. It is worked out once
/// and again only when the survey area or the work CRS changes, not on every frame (5.9 ms per frame measured).
#[test]
fn test_f1_the_circle_centre_is_worked_out_once_until_the_survey_area_changes() {
    let centre = work_xy(GYEONGJU.0, GYEONGJU.1);
    let (mut app, rect) = map_app(centre, 30.0);
    draw_survey_area(&mut app, centre);
    add_old_map_and_look_at(&mut app, centre, 30.0);
    let before = app.ref_circle.times_worked_out();
    for _ in 0..3 {
        let _ = paint_at_rest(&mut app, rect, None);
    }
    assert_eq!(app.ref_circle.times_worked_out(), before + 1, "three paints (nine frames), one computation");

    // A second shape 40 km to the east moves the centre 20 km: worked out once more, and the tiles follow.
    let (x, y) = (centre[0] + 40_000.0, centre[1]);
    let wkt = format!("POLYGON (({} {}, {} {}, {} {}, {} {}))", x, y, x + 10.0, y, x + 10.0, y + 10.0, x, y);
    let table = app.survey_tables.iter_mut().find(|t| t.table == "survey_area").unwrap();
    table.layer.features.push(FeatureRecord { fid: 2, wkt_geom: wkt, attributes: Default::default() });
    app.home_map.viewport.center_xy = [centre[0] + 115_000.0, centre[1]];
    let (asked, _) = paint_at_rest(&mut app, rect, None);
    assert_eq!(app.ref_circle.times_worked_out(), before + 2);
    assert!(!asked.is_empty(), "115 km east of the first shape is 95 km from the new centre");
}

/// Like the background tiles, reference tiles are asked for only once the view has come to rest.
#[test]
fn test_f1_no_reference_tile_is_asked_for_before_the_view_rests() {
    let centre = work_xy(GYEONGJU.0, GYEONGJU.1);
    let (mut app, rect) = map_app(centre, 30.0);
    draw_survey_area(&mut app, centre);
    add_old_map_and_look_at(&mut app, centre, 30.0);
    let _now = install_clock(&mut app);
    let ctx = egui::Context::default();
    let asked: Arc<Mutex<Vec<TexKey>>> = Arc::default();
    let sink = Arc::clone(&asked);
    let _ = run_frame(&ctx, rect, |ctx, p| {
        app.paint_reference_tile_layers_with(ctx, p, rect, move |k, z, x, y| {
            sink.lock().unwrap().push((k, z, x, y));
            || None
        });
    });
    assert!(asked.lock().unwrap().is_empty(), "the clock has not moved: the view is not at rest yet");
}

/// The pictures of a row that was removed (or unchecked) do not stay in memory.
#[test]
fn test_f1_textures_of_a_removed_row_are_dropped() {
    let centre = work_xy(GYEONGJU.0, GYEONGJU.1);
    let (mut app, rect) = map_app(centre, 30.0);
    draw_survey_area(&mut app, centre);
    add_old_map_and_look_at(&mut app, centre, 30.0);
    let _ = paint_at_rest(&mut app, rect, Some(ColorImage::new([1, 1], Color32::RED)));
    let held = |app: &crate::app_state::StrataAppState| app.basemap_textures.0.keys().filter(|k| k.0 == TileKind::Daedongyeojido).count();
    assert!(held(&app) > 0);
    assert!(app.remove_layers(&["ref_daedongyeojido".to_owned()]));
    let _ = paint_at_rest(&mut app, rect, None);
    assert_eq!(held(&app), 0);
}

/// The ribbon label itself opens the menu (the chip does: gui_ribbon_top.rs); it never claims a map was applied.
#[test]
fn test_f1_old_map_label_opens_its_menu_and_claims_nothing() {
    let mut app = crate::app_state::StrataAppState::new_startup();
    app.handle_ribbon_click("옛 지도");
    assert!(app.ribbon.old_map_menu_open);
    assert!(app.active_tab == crate::app_state::MainTab::Map);
    assert_eq!(app.home_map.draw_guide_message, None);
    assert!(app.left_layers.entries.iter().all(|e| ref_tile_layer(&e.layer_key).is_none()));
}
