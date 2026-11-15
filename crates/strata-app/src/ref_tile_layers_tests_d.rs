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

    // A second, larger shape 40 km to the east is now the survey area the circle lies around: worked out
    // once more, and the tiles follow.
    let (x, y) = (centre[0] + 40_000.0, centre[1]);
    let wkt = format!("POLYGON (({} {}, {} {}, {} {}, {} {}, {} {}))", x, y, x + 200.0, y, x + 200.0, y + 200.0, x, y + 200.0, x, y);
    let table = app.survey_tables.iter_mut().find(|t| t.table == "survey_area").unwrap();
    table.layer.features.push(FeatureRecord { fid: 2, wkt_geom: wkt, attributes: Default::default() });
    app.home_map.viewport.center_xy = [centre[0] + 115_000.0, centre[1]];
    let (asked, _) = paint_at_rest(&mut app, rect, None);
    assert_eq!(app.ref_circle.times_worked_out(), before + 2);
    assert!(!asked.is_empty(), "115 km east of the first shape is 75 km from the new centre");
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

/// A press on the ribbon button is the command itself since the owner's word of 2026-11-15 (「이것은 위의
/// ui버튼이 눌려야 나오게하라」; until then it opened the menu, as the C++ button does). Without a drawn
/// survey area it says what to do first, opens no menu, and still claims no map.
#[test]
fn test_f1_old_map_press_is_the_command_and_claims_nothing_without_an_area() {
    let mut app = crate::app_state::StrataAppState::new_startup();
    app.handle_ribbon_click("옛 지도");
    assert!(!app.ribbon.old_map_menu_open);
    assert!(app.active_tab == crate::app_state::MainTab::Map);
    assert_eq!(app.home_map.draw_guide_message.as_deref(), Some(DAEDONGYEOJIDO.needs_area_text));
    assert!(app.left_layers.entries.iter().all(|e| ref_tile_layer(&e.layer_key).is_none()));
}

/// With survey-area shapes far apart the circle lies around the survey area itself, which is the largest
/// shape, and not around the middle of their common box, which is empty land (owner 2026-11-15 with a
/// picture of it: 「어떤도면이던지 조사구역을 중심으로 반경 10km가 보이게하라」). The order of the shapes
/// does not matter.
#[test]
fn the_circle_lies_around_the_largest_survey_area_shape() {
    let centre = work_xy(GYEONGJU.0, GYEONGJU.1);
    let (mut app, _rect) = map_app(centre, 30.0);
    draw_survey_area(&mut app, centre);
    let (x, y) = (centre[0] + 6_000.0, centre[1] - 5_000.0);
    let small = format!("POLYGON (({} {}, {} {}, {} {}, {} {}))", x, y, x + 20.0, y, x + 20.0, y + 30.0, x, y);
    let table = app.survey_tables.iter_mut().find(|t| t.table == "survey_area").unwrap();
    table.layer.features.insert(0, FeatureRecord { fid: 2, wkt_geom: small, attributes: Default::default() });

    let found = app.reference_circle_centre().unwrap();
    assert!((found[0] - centre[0]).abs() < 1e-6 && (found[1] - centre[1]).abs() < 1e-6, "{found:?} for {centre:?}");
    assert!(app.add_reference_tile_layer(&DAEDONGYEOJIDO));
    let view = app.home_map.viewport.center_xy;
    assert!((view[0] - centre[0]).abs() < 1e-6 && (view[1] - centre[1]).abs() < 1e-6, "the view goes to the survey area: {view:?}");
}
