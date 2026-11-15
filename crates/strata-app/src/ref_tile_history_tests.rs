//! The 1919 map: its tile grid, its address, its key, and the 옛 지도 button.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::super::tests::{draw_survey_area, map_app, work_xy, GYEONGJU};
use super::super::DAEDONGYEOJIDO;
use super::*;
use crate::basemap_tiles::{grid_point_lon_lat, grid_tile_of, level_for, tile_url, TileGrid, TileKind};

/// One tile is asked for as a WMTS GetTile of the server's own grid (`BasemapOps.cpp:1486-1511`: layer
/// `history:map1919`, tile matrix set `EPSG:5179`, PNG; the matrix names `EPSG:5179:<level>` are the
/// server's, read from its capabilities on 2026-11-15). Without a key nothing is asked.
#[test]
fn a_tile_of_the_1919_map_is_asked_for_in_the_servers_grid() {
    assert_eq!(
        tile_url(TileKind::History1919, " FAKE-KEY ", 11, 692, 1145),
        "https://hgis.history.go.kr/openapi/get.do?Service=WMTS&Request=GetTile&Version=1.0.0&Layer=history:map1919&Style=&Format=image/png&TileMatrixSet=EPSG:5179&TileMatrix=EPSG:5179:11&TileRow=1145&TileCol=692&apiKey=FAKE-KEY"
    );
    assert_eq!(tile_url(TileKind::History1919, "  ", 11, 692, 1145), "");
    assert_eq!((TileKind::History1919.grid(), TileKind::Terrain.grid(), TileKind::Daedongyeojido.grid()), (TileGrid::Korea5179, TileGrid::WebMercator, TileGrid::WebMercator));
}

/// The grid of the server (its capabilities, 2026-11-15): EPSG:5179, the north-west corner at x -200 000,
/// y 4 000 000, 256-pixel tiles, level 0 one tile with a pixel of 55 845 639.682136305 * 0.28 mm, every
/// level twice as fine. The expected longitudes and latitudes are PROJ's (gdaltransform EPSG:5179 ->
/// EPSG:4326) for the corner of tile 692/1145 of level 11 and for a point a quarter and three quarters
/// into it; the city point EPSG:5179 (1 154 436, 1 761 737) lies in that tile.
#[test]
fn the_grid_of_the_1919_map_lies_where_proj_puts_it() {
    let near = |got: (f64, f64), want: (f64, f64)| (got.0 - want.0).abs() < 2e-7 && (got.1 - want.1).abs() < 2e-7;
    let corner = grid_point_lon_lat(TileGrid::Korea5179, 11, 692.0, 1145.0);
    assert!(near(corner, (129.189_518_056_936, 35.842_534_814_725_4)), "{corner:?}");
    let inner = grid_point_lon_lat(TileGrid::Korea5179, 11, 692.25, 1145.75);
    assert!(near(inner, (129.194_645_643_5, 35.829_247_161_648_9)), "{inner:?}");
    assert_eq!(grid_tile_of(TileGrid::Korea5179, 129.209_997_595_797, 35.840_000_443_549_4, 11), (692, 1145));
    assert_eq!(grid_tile_of(TileGrid::Korea5179, inner.0, inner.1, 11), (692, 1145));
    // One level finer the same point lies in one of the four children.
    assert_eq!(grid_tile_of(TileGrid::Korea5179, inner.0, inner.1, 12), (1384, 2291));
    // The web-mercator grid is what it was.
    assert_eq!(grid_tile_of(TileGrid::WebMercator, 129.22, 35.84, 12), crate::basemap_tiles::lon_lat_to_tile(129.22, 35.84, 12));
}

/// The level whose pixel is nearest to the map's: 7.6 m is level 11, a quarter of a metre the finest (16),
/// a whole country the coarsest. The web-mercator kinds keep their zoom rule.
#[test]
fn the_level_follows_the_scale_of_the_map() {
    let rect = eframe::egui::Rect::from_min_size(eframe::egui::Pos2::ZERO, eframe::egui::vec2(900.0, 600.0));
    assert_eq!(level_for(TileGrid::Korea5179, 7.6, rect), 11);
    assert_eq!(level_for(TileGrid::Korea5179, 10.0, rect), 11);
    assert_eq!(level_for(TileGrid::Korea5179, 12.0, rect), 10);
    assert_eq!(level_for(TileGrid::Korea5179, 0.1, rect), 16);
    assert_eq!(level_for(TileGrid::Korea5179, 50_000.0, rect), 0);
    assert_eq!(level_for(TileGrid::WebMercator, 7.6, rect), crate::basemap_tiles::basemap_zoom_for(7.6, rect));
}

/// The 1919 map needs the history-GIS key (`BasemapOps.cpp`, the C++ words) and a drawn survey area; with
/// both it gets its legend row and is seen within 10 km of the survey area (owner 2026-11-15: 「토양 고지형
/// 지질 수계 1919 조선지형도 반경10km」). Its tiles are asked for with that key; the other maps get none.
#[test]
fn the_1919_map_needs_its_key_and_a_survey_area() {
    let centre = work_xy(GYEONGJU.0, GYEONGJU.1);
    let (mut app, _rect) = map_app(centre, 20.0);
    let rows = |app: &StrataAppState| app.left_layers.entries.iter().filter(|e| e.layer_key == MAP_1919.key).count();

    app.add_history_map_1919_with("  ");
    assert_eq!(app.home_map.draw_guide_message.as_deref(), Some("1919 조선지형도는 국사편찬위원회 역사지리정보DB API 키가 필요합니다. 더보기 → API 키 입력에서 키를 저장하세요. https://hgis.history.go.kr/api/intro.do"));
    assert_eq!(rows(&app), 0);
    app.add_history_map_1919_with("FAKE-KEY");
    assert_eq!(app.home_map.draw_guide_message.as_deref(), Some("조사구역을 먼저 그려 주세요. 조사구역에서 반경 10km 안의 1919 조선지형도를 올립니다."));
    assert_eq!(rows(&app), 0);

    draw_survey_area(&mut app, centre);
    app.add_history_map_1919_with("FAKE-KEY");
    assert_eq!(app.home_map.draw_guide_message.as_deref(), Some("1919 조선지형도를 올렸습니다. 조사구역에서 반경 10km 안에 보입니다."));
    assert_eq!((rows(&app), MAP_1919.title, MAP_1919.radius_m, MAP_1919.kind), (1, "1919 조선지형도 1:5만", 10_000.0, TileKind::History1919));
    assert!(app.reference_tile_layer_shown(&MAP_1919));
    assert_eq!((app.tile_key_for(TileKind::History1919), app.tile_key_for(TileKind::Daedongyeojido), app.tile_key_for(TileKind::Terrain)), ("FAKE-KEY".to_owned(), String::new(), String::new()));
    // The map is framed to the circle: 22 km across the 600 points of the map's shorter side.
    let scale = app.home_map.viewport.meters_per_pixel;
    assert!((30.0..45.0).contains(&scale), "{scale}");
}

/// The 옛 지도 button itself shows the 대동여지도, and hides and shows it again (owner 2026-11-15: 「이것은
/// 위의 ui버튼이 눌려야 나오게하라」). The C++ button only opens its menu (`MainWindowRibbon.cpp:486-500`);
/// here the menu stays on the right click. The button looks pressed while one of the two old maps is shown.
#[test]
fn a_press_on_the_old_map_button_shows_and_hides_the_daedong_map() {
    let centre = work_xy(GYEONGJU.0, GYEONGJU.1);
    let (mut app, _rect) = map_app(centre, 20.0);
    app.handle_ribbon_click("옛 지도");
    assert!(!app.ribbon.old_map_menu_open, "a press is no menu");
    assert_eq!(app.home_map.draw_guide_message.as_deref(), Some(DAEDONGYEOJIDO.needs_area_text));
    assert!(!app.old_map_shown());

    draw_survey_area(&mut app, centre);
    app.handle_ribbon_click("옛 지도");
    assert!(app.reference_tile_layer_shown(&DAEDONGYEOJIDO) && app.old_map_shown());
    app.handle_ribbon_click("옛 지도");
    assert!(!app.reference_tile_layer_shown(&DAEDONGYEOJIDO) && !app.old_map_shown());
    app.handle_ribbon_click("옛 지도");
    assert!(app.reference_tile_layer_shown(&DAEDONGYEOJIDO));
    // The 1919 map alone also shows the button as pressed.
    app.handle_ribbon_click("옛 지도");
    app.add_history_map_1919_with("FAKE-KEY");
    assert!(!app.reference_tile_layer_shown(&DAEDONGYEOJIDO) && app.old_map_shown());
}
