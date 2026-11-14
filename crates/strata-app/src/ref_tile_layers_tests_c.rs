//! Reference tile layers, third part: the terrain map (리본 「지형」 → 지형맵, OpenTopoMap).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use strata_types::schema::LegendGroup;

use super::tests::{draw_survey_area, map_app, paint_at_rest, work_xy, GYEONGJU};
use super::{DAEDONGYEOJIDO, TERRAIN_MAP};
use crate::basemap_tiles::{tile_url, TileKind};

fn rows(app: &crate::app_state::StrataAppState) -> Vec<(&str, &str, LegendGroup)> {
    app.left_layers.entries.iter().map(|e| (e.layer_key.as_str(), e.label_ko.as_str(), e.group)).collect()
}

/// MainWindowOverlay.cpp:239-267 toggleTerrainMap — the first click adds 「지형맵」 (:263), later clicks hide
/// and show it with the two texts of toggleExistingOverlay (:227, :232). The view is left alone.
#[test]
fn test_f2_terrain_button_adds_the_row_then_hides_and_shows_it_with_the_cpp_texts() {
    let centre = work_xy(GYEONGJU.0, GYEONGJU.1);
    let (mut app, _rect) = map_app([centre[0] + 400.0, centre[1] - 250.0], 2.0);
    draw_survey_area(&mut app, centre);

    app.handle_ribbon_click("지형");
    assert_eq!(
        rows(&app),
        vec![
            ("survey_area", "조사구역", LegendGroup::SurveyData),
            ("ref_terrain_map", "지형맵", LegendGroup::ReferenceMap),
            ("vworld_satellite", "위성지도 (VWorld)", LegendGroup::Basemap),
            ("vworld_cadastral_wms", "연속지적도 (VWorld WMS)", LegendGroup::Cadastral),
        ]
    );
    assert!(app.reference_tile_layer_shown(&TERRAIN_MAP));
    assert_eq!(app.home_map.draw_guide_message.as_deref(), Some("지형맵을 올렸습니다. 다시 누르면 숨깁니다."));
    assert_eq!(app.home_map.viewport.center_xy, [centre[0] + 400.0, centre[1] - 250.0], "the view stays");
    assert!((app.home_map.viewport.meters_per_pixel - 2.0).abs() < f64::EPSILON);

    app.handle_ribbon_click("지형");
    assert!(!app.reference_tile_layer_shown(&TERRAIN_MAP));
    assert!(!app.home_map.real_survey.is_layer_visible("지형맵"));
    assert_eq!(app.home_map.draw_guide_message.as_deref(), Some("지형맵를 껐습니다."));
    assert!(app.left_layers.entries.iter().any(|e| e.layer_key == "ref_terrain_map"), "hidden, not removed");

    app.handle_ribbon_click("지형");
    assert!(app.reference_tile_layer_shown(&TERRAIN_MAP));
    assert_eq!(app.home_map.draw_guide_message.as_deref(), Some("지형맵를 다시 켰습니다."));
    assert_eq!(app.left_layers.entries.iter().filter(|e| e.layer_key == "ref_terrain_map").count(), 1);
}

#[test]
fn test_f2_terrain_button_without_a_survey_area_adds_nothing_and_says_why() {
    let centre = work_xy(GYEONGJU.0, GYEONGJU.1);
    let (mut app, rect) = map_app(centre, 30.0);
    let before = app.left_layers.entries.clone();
    app.handle_ribbon_click("지형");
    assert_eq!(app.left_layers.entries, before);
    assert!(!app.reference_tile_layer_shown(&TERRAIN_MAP));
    assert_eq!(
        app.home_map.draw_guide_message.as_deref(),
        Some("조사구역을 먼저 그려 주세요. 조사구역에서 반경 100km 안의 지형맵을 올립니다.")
    );
    let (asked, _) = paint_at_rest(&mut app, rect, None);
    assert!(asked.is_empty());
}

/// From a view centred outside the circle nothing of the new map would show: the view goes to the survey
/// area and keeps its scale.
#[test]
fn test_f2_terrain_button_from_a_view_outside_the_circle_goes_to_the_survey_area_at_the_same_scale() {
    let centre = work_xy(GYEONGJU.0, GYEONGJU.1);
    let (mut app, _rect) = map_app([centre[0] - 150_000.0, centre[1]], 30.0);
    draw_survey_area(&mut app, centre);
    app.handle_ribbon_click("지형");
    let view = &app.home_map.viewport;
    assert!((view.center_xy[0] - centre[0]).abs() < 1e-6 && (view.center_xy[1] - centre[1]).abs() < 1e-6, "{:?}", view.center_xy);
    assert!((view.meters_per_pixel - 30.0).abs() < f64::EPSILON);
}

/// BasemapOps.cpp:1583-1586 — zmin 5, zmax 17. Only tiles that reach into the 100 km circle are asked for.
#[test]
fn test_f2_terrain_tiles_are_asked_for_up_to_zoom_17_and_only_inside_the_circle() {
    let centre = work_xy(GYEONGJU.0, GYEONGJU.1);
    // 0.2 m per pixel is zoom 19 for the background.
    let (mut app, rect) = map_app(centre, 0.2);
    draw_survey_area(&mut app, centre);
    app.handle_ribbon_click("지형");
    let (asked, _) = paint_at_rest(&mut app, rect, None);
    assert!(!asked.is_empty());
    assert!(asked.iter().all(|k| k.0 == TileKind::Terrain && k.1 == 17), "{asked:?}");

    app.home_map.viewport.center_xy = [centre[0], centre[1] + 150_000.0];
    app.home_map.viewport.meters_per_pixel = 30.0;
    let (asked, _) = paint_at_rest(&mut app, rect, None);
    assert!(asked.is_empty(), "{asked:?}");
}

/// A newer tile map lies above an older one, both above the satellite, and the higher row is painted last:
/// what was just asked for is seen. (C++ keeps the older tile layer on top, BasemapOps.cpp:922 and :362-401;
/// this order is the director's choice, docs/EVOLUTION.md E-13.)
#[test]
fn test_f2_a_newer_tile_map_goes_above_the_older_one_and_is_painted_over_it() {
    let centre = work_xy(GYEONGJU.0, GYEONGJU.1);
    let (mut app, rect) = map_app(centre, 30.0);
    draw_survey_area(&mut app, centre);
    assert!(app.add_reference_tile_layer(&DAEDONGYEOJIDO));
    app.handle_ribbon_click("지형");
    app.home_map.viewport.center_xy = centre;
    app.home_map.viewport.meters_per_pixel = 30.0;
    let keys: Vec<&str> = app.left_layers.entries.iter().map(|e| e.layer_key.as_str()).collect();
    assert_eq!(keys, vec!["survey_area", "ref_terrain_map", "ref_daedongyeojido", "vworld_satellite", "vworld_cadastral_wms"]);
    let (asked, _) = paint_at_rest(&mut app, rect, None);
    // Each frame handles the lower row (the old map) before the higher one, so the higher is painted over it.
    let first_old_map = asked.iter().position(|k| k.0 == TileKind::Daedongyeojido).unwrap();
    let first_terrain = asked.iter().position(|k| k.0 == TileKind::Terrain).unwrap();
    assert!(first_old_map < first_terrain, "{asked:?}");
}

#[test]
fn test_f2_terrain_tile_address_and_cache_folder() {
    assert_eq!(tile_url(TileKind::Terrain, "FAKE-TEST-KEY", 17, 112_345, 51_234), "https://tile.opentopomap.org/17/112345/51234.png");
    let cache = crate::basemap_fetch::tile_cache_path(TileKind::Terrain, 12, 3518, 1608);
    assert!(cache.ends_with("opentopomap/12/3518/1608.png") || cache.ends_with(r"opentopomap\12\3518\1608.png"), "{}", cache.display());
}

/// The volunteer-run terrain tile service is told which program asks (plan 2.8: a request says honestly what
/// it is); the VWorld command with its Referer stays as it was (net0c_tests pins it).
#[test]
fn test_f2_terrain_requests_name_the_app_and_send_no_made_up_referer() {
    let dest = std::path::Path::new("tile.png");
    let args = |kind| -> Vec<String> {
        crate::basemap_fetch::tile_curl_for(kind, dest).get_args().map(|a| a.to_string_lossy().into_owned()).collect()
    };
    let terrain = args(TileKind::Terrain);
    assert!(terrain.contains(&format!("User-Agent: Strata/{} (desktop GIS)", env!("CARGO_PKG_VERSION"))), "{terrain:?}");
    assert!(terrain.iter().all(|a| !a.starts_with("Referer:")), "{terrain:?}");
    assert_eq!(terrain.last().map(String::as_str), Some("tile.png"));
    for kind in [TileKind::Satellite, TileKind::Base, TileKind::Daedongyeojido] {
        let other = args(kind);
        assert!(other.contains(&"Referer: https://localhost".to_owned()) && other.contains(&"User-Agent: Mozilla/5.0".to_owned()), "{kind:?}");
    }
}

/// The same rule in the real fetch path, not only in the helper: with the network switch on and no cached
/// tile, the one process started for a terrain tile names the app and carries no Referer; the address goes
/// in on standard input.
#[test]
fn test_f2_terrain_fetch_starts_curl_with_the_app_name_and_without_a_referer() {
    use strata_net::network::probe_dry_run;
    let dir = tempfile::tempdir().expect("tempdir");
    let (z, x, y) = (12, 3518, 1610);
    let (launches, cmds) = probe_dry_run(true, || {
        assert_eq!(crate::basemap_fetch::fetch_tile_in(dir.path(), TileKind::Terrain, "", z, x, y), None);
    });
    assert_eq!(launches, 1);
    assert_eq!(cmds[0].program, "curl");
    assert!(cmds[0].args.contains(&format!("User-Agent: Strata/{} (desktop GIS)", env!("CARGO_PKG_VERSION"))), "{:?}", cmds[0].args);
    assert!(cmds[0].args.iter().all(|a| !a.starts_with("Referer:")), "{:?}", cmds[0].args);
    assert!(String::from_utf8_lossy(&cmds[0].stdin).contains("https://tile.opentopomap.org/12/3518/1610.png"));
}

/// Owner 2026-11-15: both tile maps end 100 km from the survey area. A screen 900 m wide is asked for tiles
/// when it lies 99 km from the survey area and for nothing when it lies 101 km away.
#[test]
fn test_f2_both_tile_maps_end_100_km_from_the_survey_area() {
    let centre = work_xy(GYEONGJU.0, GYEONGJU.1);
    for layer in [&DAEDONGYEOJIDO, &TERRAIN_MAP] {
        let (mut app, rect) = map_app(centre, 1.0);
        draw_survey_area(&mut app, centre);
        assert!(app.add_reference_tile_layer(layer));
        app.home_map.viewport.meters_per_pixel = 1.0;
        app.home_map.viewport.center_xy = [centre[0] - 99_000.0, centre[1]];
        let (asked, _) = paint_at_rest(&mut app, rect, None);
        assert!(!asked.is_empty(), "{}: 99 km", layer.title);
        app.home_map.viewport.center_xy = [centre[0] - 101_000.0, centre[1]];
        let (asked, _) = paint_at_rest(&mut app, rect, None);
        assert!(asked.is_empty(), "{}: 101 km: {asked:?}", layer.title);
    }
}

/// Showing the hidden row again from a view centred outside the circle would show nothing either: the view
/// goes to the survey area at its scale, as on the first click, and the move is one step of the view history.
#[test]
fn test_f2_showing_the_row_again_from_outside_the_circle_goes_to_the_survey_area() {
    let centre = work_xy(GYEONGJU.0, GYEONGJU.1);
    let (mut app, _rect) = map_app(centre, 30.0);
    draw_survey_area(&mut app, centre);
    app.handle_ribbon_click("지형");
    app.handle_ribbon_click("지형");
    assert!(!app.reference_tile_layer_shown(&TERRAIN_MAP));
    app.home_map.viewport.center_xy = [centre[0], centre[1] - 150_000.0];
    let far = app.home_map.viewport.world_bbox();
    app.home_map.extent_history.seed(far);
    app.handle_ribbon_click("지형");
    assert!(app.reference_tile_layer_shown(&TERRAIN_MAP));
    let view = &app.home_map.viewport;
    assert!((view.center_xy[0] - centre[0]).abs() < 1e-6 && (view.center_xy[1] - centre[1]).abs() < 1e-6, "{:?}", view.center_xy);
    assert!((view.meters_per_pixel - 30.0).abs() < f64::EPSILON);
    assert!(app.home_map.extent_history.can_previous(), "the move can be stepped back");
    assert_eq!(app.home_map.extent_history.previous(), Some(far));

    // Hiding never moves the view.
    app.home_map.viewport.center_xy = [centre[0], centre[1] - 150_000.0];
    app.handle_ribbon_click("지형");
    assert_eq!(app.home_map.viewport.center_xy, [centre[0], centre[1] - 150_000.0]);
}
