//! Reference tile layers (대동여지도): a map is on the canvas only after its command, as a legend row the user
//! can uncheck, fade and remove, and only inside the circle around the survey area (owner 2026-11-15).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::{Arc, Mutex};
use std::time::Instant;

use eframe::egui::{self, epaint::Shape, vec2, Color32, ColorImage, Pos2, RawInput, Rect};
use strata_geo::crs_transform::lonlat_to_tm;
use strata_net::tile_fetcher::BasemapSource;
use strata_storage::edit_buffer::{FeatureRecord, LayerEditBuffer};
use strata_storage::survey_gpkg_read::SurveyFileTable;
use strata_types::crs::WorkCrs;
use strata_types::schema::{DomainLayerKey, LegendGroup};

use super::{ref_tile_layer, DAEDONGYEOJIDO};
use crate::app_state::StrataAppState;
use crate::basemap_cadastral_jobs::{shared_tile_fetchers_with_clock, CachedApiKey, TexKey};
use crate::basemap_paint::tm_params_for;
use crate::basemap_tile_key::VIEW_REST_TIME;
use crate::basemap_tiles::TileKind;

/// 경주 (lon, lat) — the survey area of these tests sits here.
pub(super) const GYEONGJU: (f64, f64) = (129.22, 35.84);

pub(super) fn work_xy(lon: f64, lat: f64) -> [f64; 2] {
    lonlat_to_tm(lon, lat, &tm_params_for(WorkCrs::Epsg5187).unwrap())
}

/// A map app looking at `centre` (work CRS metres) with the default two background rows.
pub(super) fn map_app(centre: [f64; 2], mpp: f64) -> (StrataAppState, Rect) {
    let size = [900.0_f32, 600.0];
    let mut app = StrataAppState::new_startup();
    app.left_layers.apply_default_basemaps();
    app.home_map.active_basemap = BasemapSource::VworldSatellite;
    app.vworld_api_key = CachedApiKey::with_reader_and_clock(Arc::new(|| "FAKE-TEST-KEY".to_owned()), Arc::new(Instant::now));
    app.home_map.viewport =
        strata_canvas2d::viewport::ViewportTransform::new(centre, mpp, [f64::from(size[0]), f64::from(size[1])]);
    (app, Rect::from_min_size(Pos2::ZERO, vec2(size[0], size[1])))
}

/// Puts a 100 m square survey area around `centre` into the open survey.
pub(super) fn draw_survey_area(app: &mut StrataAppState, centre: [f64; 2]) {
    let (x, y) = (centre[0], centre[1]);
    let wkt = format!(
        "POLYGON (({} {}, {} {}, {} {}, {} {}, {} {}))",
        x - 50.0, y - 50.0, x + 50.0, y - 50.0, x + 50.0, y + 50.0, x - 50.0, y + 50.0, x - 50.0, y - 50.0
    );
    let mut layer = LayerEditBuffer::new(DomainLayerKey::SurveyArea);
    layer.features.push(FeatureRecord { fid: 1, wkt_geom: wkt, attributes: Default::default() });
    app.survey_tables.push(SurveyFileTable {
        table: "survey_area".to_owned(),
        srs_id: 5187,
        geometry_type: "POLYGON".to_owned(),
        columns: Vec::new(),
        baseline: layer.features.clone(),
        layer,
    });
    // A drawn layer goes to the top of the legend (app_state_draw_entry.rs begin_edit_layer).
    app.left_layers.ensure_domain_layer(DomainLayerKey::SurveyArea);
    let at = app.left_layers.entries.iter().position(|e| e.layer_key == "survey_area").unwrap();
    let row = app.left_layers.entries.remove(at);
    app.left_layers.entries.insert(0, row);
}

/// The command, then the view the test is about (the command itself frames the whole circle).
pub(super) fn add_old_map_and_look_at(app: &mut StrataAppState, centre: [f64; 2], mpp: f64) {
    assert!(app.add_reference_tile_layer(&DAEDONGYEOJIDO));
    app.home_map.viewport.center_xy = centre;
    app.home_map.viewport.meters_per_pixel = mpp;
}

pub(super) fn install_clock(app: &mut StrataAppState) -> Arc<Mutex<Instant>> {
    let now = Arc::new(Mutex::new(Instant::now()));
    let reader = Arc::clone(&now);
    let (bg, cad) = shared_tile_fetchers_with_clock(Arc::new(move || *reader.lock().unwrap()));
    app.basemap_fetch = bg;
    app.cadastral_fetch = cad;
    now
}

pub(super) fn run_frame(ctx: &egui::Context, rect: Rect, f: impl FnOnce(&egui::Context, &egui::Painter)) -> egui::FullOutput {
    let mut f = Some(f);
    ctx.run(RawInput { screen_rect: Some(rect), ..RawInput::default() }, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(cb) = f.take() {
                cb(ctx, &ui.painter_at(rect));
            }
        });
    })
}

/// Paints the reference tile layers for a view that has been still long enough, with a job that records the
/// tile it was made for and answers `image`. Returns the tiles asked for and the frame's shapes.
pub(super) fn paint_at_rest(app: &mut StrataAppState, rect: Rect, image: Option<ColorImage>) -> (Vec<TexKey>, Vec<egui::epaint::ClippedShape>) {
    let ctx = egui::Context::default();
    let now = install_clock(app);
    let _ = run_frame(&ctx, rect, |ctx, p| app.paint_reference_tile_layers_with(ctx, p, rect, |_k, _z, _x, _y| || None));
    app.basemap_fetch.wait_for_idle();
    app.basemap_fetch.clear_failed();
    let later = *now.lock().unwrap() + VIEW_REST_TIME;
    *now.lock().unwrap() = later;
    let asked: Arc<Mutex<Vec<TexKey>>> = Arc::default();
    let mut shapes = Vec::new();
    // Two passes: the first starts the jobs, the second (after they finished) paints what they answered.
    for _ in 0..2 {
        let sink = Arc::clone(&asked);
        let img = image.clone();
        let out = run_frame(&ctx, rect, |ctx, p| {
            app.paint_reference_tile_layers_with(ctx, p, rect, move |k, z, x, y| {
                sink.lock().unwrap().push((k, z, x, y));
                let img = img.clone();
                move || img
            });
        });
        app.basemap_fetch.wait_for_idle();
        shapes = out.shapes;
    }
    let asked = asked.lock().unwrap().clone();
    (asked, shapes)
}

pub(super) fn mesh_colours(shapes: &[egui::epaint::ClippedShape]) -> Vec<Color32> {
    let mut out = Vec::new();
    for s in shapes {
        if let Shape::Mesh(m) = &s.shape {
            out.extend(m.vertices.iter().map(|v| v.color));
        }
    }
    out
}

/// The owner's defect of 2026-11-15: an unmerged build painted 대동여지도 and 지형맵 on every survey without a
/// click and without a legend row, so they could not be removed. A map is painted only for a legend row.
#[test]
fn test_f1_no_reference_tile_is_asked_for_without_its_legend_row() {
    let centre = work_xy(GYEONGJU.0, GYEONGJU.1);
    let (mut app, rect) = map_app(centre, 30.0);
    draw_survey_area(&mut app, centre);
    assert!(app.left_layers.entries.iter().all(|e| ref_tile_layer(&e.layer_key).is_none()));
    // The title merely counting as "visible" (the default for a title that was never set, or a stale mark of
    // an earlier survey) must not be enough.
    app.home_map.real_survey.layer_visible.insert("대동여지도".to_owned(), true);
    let (asked, shapes) = paint_at_rest(&mut app, rect, Some(ColorImage::new([1, 1], Color32::RED)));
    assert!(asked.is_empty(), "{asked:?}");
    assert!(mesh_colours(&shapes).is_empty());
}

#[test]
fn test_f1_old_map_command_adds_one_reference_row_above_the_background_rows() {
    let centre = work_xy(GYEONGJU.0, GYEONGJU.1);
    let (mut app, _rect) = map_app(centre, 30.0);
    draw_survey_area(&mut app, centre);
    assert!(app.add_reference_tile_layer(&DAEDONGYEOJIDO));
    assert!(app.add_reference_tile_layer(&DAEDONGYEOJIDO), "a second click keeps the one row");
    let rows: Vec<(&str, &str, LegendGroup)> =
        app.left_layers.entries.iter().map(|e| (e.layer_key.as_str(), e.label_ko.as_str(), e.group)).collect();
    assert_eq!(
        rows,
        vec![
            ("survey_area", "조사구역", LegendGroup::SurveyData),
            ("ref_daedongyeojido", "대동여지도", LegendGroup::ReferenceMap),
            ("vworld_satellite", "위성지도 (VWorld)", LegendGroup::Basemap),
            ("vworld_cadastral_wms", "연속지적도 (VWorld WMS)", LegendGroup::Cadastral),
        ]
    );
    let row = app.left_layers.entries.iter().find(|e| e.layer_key == "ref_daedongyeojido").unwrap();
    assert!(row.read_only && !row.snap_enabled, "a picture: not edited, not snapped to");
    assert!((row.opacity - 1.0).abs() < f32::EPSILON);
    assert!(app.home_map.real_survey.is_layer_visible("대동여지도"));
    assert_eq!(
        app.home_map.draw_guide_message.as_deref(),
        Some("대동여지도를 올렸습니다. 조사구역에서 반경 100km 안에 보입니다.")
    );
}

/// A second click after the user unchecked the row shows it again (BasemapOps.cpp:877-895 replaces the app's
/// own earlier copy and :924 checks the new node).
#[test]
fn test_f1_old_map_command_shows_an_unchecked_row_again() {
    let centre = work_xy(GYEONGJU.0, GYEONGJU.1);
    let (mut app, _rect) = map_app(centre, 30.0);
    draw_survey_area(&mut app, centre);
    assert!(app.add_reference_tile_layer(&DAEDONGYEOJIDO));
    app.home_map.real_survey.toggle_layer_visible("대동여지도");
    assert!(!app.home_map.real_survey.is_layer_visible("대동여지도"));
    assert!(app.add_reference_tile_layer(&DAEDONGYEOJIDO));
    assert!(app.home_map.real_survey.is_layer_visible("대동여지도"));
}

/// The command frames the circle: the survey area in the middle of the map and the whole radius in view,
/// from wherever the user was looking.
#[test]
fn test_f1_old_map_command_frames_the_circle_around_the_survey_area() {
    let centre = work_xy(GYEONGJU.0, GYEONGJU.1);
    for (from, mpp) in [([centre[0] - 150_000.0, centre[1]], 30.0), (centre, 0.1), ([centre[0] + 3_000.0, centre[1] - 9_000.0], 2_000.0)] {
        let (mut app, _rect) = map_app(from, mpp);
        draw_survey_area(&mut app, centre);
        assert!(app.add_reference_tile_layer(&DAEDONGYEOJIDO));
        let view = &app.home_map.viewport;
        assert!((view.center_xy[0] - centre[0]).abs() < 1e-6 && (view.center_xy[1] - centre[1]).abs() < 1e-6, "{:?}", view.center_xy);
        // The 900 x 600 map shows 110 km above and below the survey area: 220 km over 600 px.
        assert!((view.meters_per_pixel - 220_000.0 / 600.0).abs() < 1e-9, "{}", view.meters_per_pixel);
    }
}

#[test]
fn test_f1_old_map_command_without_a_survey_area_adds_nothing_and_says_why() {
    let centre = work_xy(GYEONGJU.0, GYEONGJU.1);
    let (mut app, rect) = map_app(centre, 30.0);
    let before = app.left_layers.entries.clone();
    assert!(!app.add_reference_tile_layer(&DAEDONGYEOJIDO));
    assert_eq!(app.left_layers.entries, before);
    assert_eq!(app.home_map.viewport.center_xy, centre, "the view stays");
    assert!((app.home_map.viewport.meters_per_pixel - 30.0).abs() < f64::EPSILON);
    assert_eq!(
        app.home_map.draw_guide_message.as_deref(),
        Some("조사구역을 먼저 그려 주세요. 조사구역에서 반경 100km 안의 대동여지도를 올립니다.")
    );
    let (asked, _) = paint_at_rest(&mut app, rect, None);
    assert!(asked.is_empty());
}

#[test]
fn test_f1_view_on_the_survey_area_asks_for_old_map_tiles_only_up_to_its_deepest_zoom() {
    let centre = work_xy(GYEONGJU.0, GYEONGJU.1);
    // 1 m per pixel is zoom 17 for the background; the old map stops at its own deepest zoom.
    let (mut app, rect) = map_app(centre, 1.0);
    draw_survey_area(&mut app, centre);
    add_old_map_and_look_at(&mut app, centre, 1.0);
    let (asked, _) = paint_at_rest(&mut app, rect, None);
    assert!(!asked.is_empty());
    assert!(asked.iter().all(|k| k.0 == TileKind::Daedongyeojido && k.1 == DAEDONGYEOJIDO.max_zoom), "{asked:?}");
}

#[test]
fn test_f1_view_beyond_the_circle_asks_for_nothing() {
    let centre = work_xy(GYEONGJU.0, GYEONGJU.1);
    // 150 km west of the survey area, a screen 27 km wide: every tile is outside the 100 km circle.
    let (mut app, rect) = map_app([centre[0] - 150_000.0, centre[1]], 30.0);
    draw_survey_area(&mut app, centre);
    add_old_map_and_look_at(&mut app, [centre[0] - 150_000.0, centre[1]], 30.0);
    let (asked, shapes) = paint_at_rest(&mut app, rect, Some(ColorImage::new([1, 1], Color32::RED)));
    assert!(asked.is_empty(), "{asked:?}");
    assert!(mesh_colours(&shapes).is_empty());
}

#[test]
fn test_f1_view_across_the_rim_asks_only_for_tiles_that_reach_into_the_circle() {
    let centre = work_xy(GYEONGJU.0, GYEONGJU.1);
    // The screen centre is on the rim (100 km east); 100 m per pixel: 90 km wide, tiles about 10 km.
    let (mut app, rect) = map_app(centre, 100.0);
    draw_survey_area(&mut app, centre);
    add_old_map_and_look_at(&mut app, [centre[0] + 100_000.0, centre[1]], 100.0);
    let (asked, _) = paint_at_rest(&mut app, rect, None);
    assert!(!asked.is_empty());
    let params = tm_params_for(WorkCrs::Epsg5187).unwrap();
    let mut beyond = 0;
    for &(_, z, x, y) in &asked {
        // Nearest corner of the tile to the survey area, in work CRS metres.
        let nearest = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)]
            .iter()
            .map(|&(dx, dy)| {
                let (lon, lat) = crate::basemap_tiles::tile_point_lon_lat(z, f64::from(x) + dx, f64::from(y) + dy);
                let m = lonlat_to_tm(lon, lat, &params);
                (m[0] - centre[0]).hypot(m[1] - centre[1])
            })
            .fold(f64::INFINITY, f64::min);
        if nearest > 100_000.0 {
            beyond += 1;
        }
    }
    // A tile whose nearest corner is beyond the rim can still be asked for only when its screen box (the
    // tile is a slightly rotated square in the work CRS) reaches the rim: at most the one column on the rim.
    assert!(beyond <= 12, "{beyond} of {} tiles lie wholly beyond the rim", asked.len());
    let east_most = asked.iter().map(|k| k.2).max().unwrap();
    let west_most = asked.iter().map(|k| k.2).min().unwrap();
    assert!(east_most - west_most <= 6, "about half of the 9 tile columns of the screen: {west_most}..{east_most}");
}

#[test]
fn test_f1_unchecked_row_and_removed_row_ask_for_nothing() {
    let centre = work_xy(GYEONGJU.0, GYEONGJU.1);
    let (mut app, rect) = map_app(centre, 30.0);
    draw_survey_area(&mut app, centre);
    add_old_map_and_look_at(&mut app, centre, 30.0);
    app.home_map.real_survey.toggle_layer_visible("대동여지도");
    let (asked, _) = paint_at_rest(&mut app, rect, None);
    assert!(asked.is_empty(), "unchecked: {asked:?}");

    app.home_map.real_survey.toggle_layer_visible("대동여지도");
    assert!(app.remove_layers(&["ref_daedongyeojido".to_owned()]));
    assert!(app.left_layers.entries.iter().all(|e| e.layer_key != "ref_daedongyeojido"));
    let (asked, _) = paint_at_rest(&mut app, rect, None);
    assert!(asked.is_empty(), "removed: {asked:?}");
}

