//! Reference tile layers, second half: opacity, renaming, survey change, the tile address, the circle.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use eframe::egui::{self, vec2, Color32, ColorImage, Pos2, RawInput, Rect};

use super::ref_tile_paint::{edge_mesh, tile_cover, ScreenCircle, TileCover, EDGE_MESH_STEPS};
use super::tests::{add_old_map_and_look_at, draw_survey_area, map_app, mesh_colours, paint_at_rest, work_xy, GYEONGJU};
use super::{ref_tile_layer, DAEDONGYEOJIDO};
use crate::basemap_cadastral_jobs::TexKey;
use crate::basemap_tiles::{tile_url, TileKind};

/// The row's 투명도 of the legend menu fades the picture (applyLayerOpacity).
#[test]
fn test_f1_row_opacity_tints_the_painted_tiles() {
    let centre = work_xy(GYEONGJU.0, GYEONGJU.1);
    let (mut app, rect) = map_app(centre, 30.0);
    draw_survey_area(&mut app, centre);
    add_old_map_and_look_at(&mut app, centre, 30.0);
    assert!(app.layer_set_transparency("ref_daedongyeojido", 40));
    let (asked, shapes) = paint_at_rest(&mut app, rect, Some(ColorImage::new([1, 1], Color32::RED)));
    assert!(!asked.is_empty());
    let colours = mesh_colours(&shapes);
    assert!(!colours.is_empty(), "the answered tiles are painted");
    let want = Color32::WHITE.gamma_multiply(0.6);
    assert!(colours.iter().all(|c| *c == want), "{:?} != {want:?}", colours.first());
}

/// A renamed row keeps its map: the row is found by its key, its check mark by its present name.
#[test]
fn test_f1_renamed_row_is_still_painted_and_unchecked_by_its_new_name() {
    let centre = work_xy(GYEONGJU.0, GYEONGJU.1);
    let (mut app, rect) = map_app(centre, 30.0);
    draw_survey_area(&mut app, centre);
    add_old_map_and_look_at(&mut app, centre, 30.0);
    app.left_layers.entries.iter_mut().find(|e| e.layer_key == "ref_daedongyeojido").unwrap().label_ko = "옛 지도 1861".to_owned();
    let (asked, _) = paint_at_rest(&mut app, rect, None);
    assert!(!asked.is_empty());
    app.home_map.real_survey.toggle_layer_visible("옛 지도 1861");
    let (asked, _) = paint_at_rest(&mut app, rect, None);
    assert!(asked.is_empty(), "{asked:?}");
}

#[test]
fn test_f1_another_survey_does_not_inherit_the_reference_rows() {
    let centre = work_xy(GYEONGJU.0, GYEONGJU.1);
    let (mut app, _rect) = map_app(centre, 30.0);
    draw_survey_area(&mut app, centre);
    assert!(app.add_reference_tile_layer(&DAEDONGYEOJIDO));
    app.start_new_survey_project("다른 조사");
    assert!(app.left_layers.entries.iter().all(|e| ref_tile_layer(&e.layer_key).is_none()));
    assert!(app.left_layers.entries.iter().any(|e| e.layer_key == "vworld_satellite"), "the background rows stay");
}

/// BasemapOps.cpp:1547-1563 — the NGII old-map WMS: version 1.1.1 (1.3.0 answered empty), PNG, transparent,
/// layer korea_oldmap_ddymap_kyu, no key. The square is the XYZ tile's own EPSG:3857 square.
#[test]
fn test_f1_old_map_tile_address_is_the_wms_getmap_of_the_tile_square() {
    let url = tile_url(TileKind::Daedongyeojido, "FAKE-TEST-KEY", 1, 1, 0);
    assert_eq!(
        url,
        "https://map.ngii.go.kr/spcemapserver/korea_old_map/ows?SERVICE=WMS&VERSION=1.1.1&REQUEST=GetMap\
         &LAYERS=korea_oldmap_ddymap_kyu&STYLES=&SRS=EPSG:3857\
         &BBOX=0.000,0.000,20037508.343,20037508.343&WIDTH=256&HEIGHT=256&FORMAT=image/png&TRANSPARENT=TRUE"
    );
    assert!(!url.contains("FAKE-TEST-KEY"), "this service takes no key");
    // Zoom 12, the tile over 경주: 4096 tiles across, each 9783.940 m wide.
    let url = tile_url(TileKind::Daedongyeojido, "", 12, 3518, 1610);
    assert!(url.contains("&BBOX=14382391.242,4275581.614,14392175.182,4285365.554&"), "{url}");
    let cache = crate::basemap_fetch::tile_cache_path(TileKind::Daedongyeojido, 12, 3518, 1610);
    assert!(cache.ends_with("daedongyeojido/12/3518/1610.png") || cache.ends_with(r"daedongyeojido\12\3518\1610.png"), "{}", cache.display());
}

#[test]
fn test_f1_tile_cover_tells_inside_edge_and_outside() {
    let circle = ScreenCircle { center: Pos2::new(0.0, 0.0), radius: 100.0 };
    let tile = |x: f32, y: f32| Rect::from_min_size(Pos2::new(x, y), vec2(20.0, 20.0));
    // A 20 px tile (0.625 px cells) is clear from 0.88 px (a cell diagonal) inside the rim outwards and
    // fades over the 1.25 px (two cells) before that: wholly inside means every corner 2.13 px inside.
    assert_eq!(tile_cover(tile(-10.0, -10.0), &circle), TileCover::Inside);
    assert_eq!(tile_cover(tile(40.0, 40.0), &circle), TileCover::Inside, "far corner at 84.9");
    assert_eq!(tile_cover(tile(50.36, 50.36), &circle), TileCover::Edge, "far corner at 99.5: inside the fade");
    assert_eq!(tile_cover(tile(49.3, 49.3), &circle), TileCover::Edge, "far corner at 98.0: 2.0 px inside is not enough");
    assert_eq!(tile_cover(tile(48.5, 48.5), &circle), TileCover::Inside, "far corner at 96.9");
    assert_eq!(tile_cover(tile(60.0, 60.0), &circle), TileCover::Edge, "near corner 84.9, far corner 113.1");
    assert_eq!(tile_cover(tile(90.0, -10.0), &circle), TileCover::Edge);
    assert_eq!(tile_cover(tile(101.0, -10.0), &circle), TileCover::Outside);
    assert_eq!(tile_cover(tile(75.0, 75.0), &circle), TileCover::Outside, "near corner at 106.1");
}

/// A tile on the rim is painted as a finer mesh: full strength well inside, a ramp, and nothing at all from one
/// cell diagonal inside the rim outwards — so no cell that is painted reaches beyond the circle. The map ends
/// on the circle, not on tile borders.
#[test]
fn test_f1_edge_mesh_fades_out_before_the_circle_and_paints_nothing_beyond_it() {
    let n = EDGE_MESH_STEPS;
    // A 160 px tile: 5 px cells (diagonal 7.07 px), a 10 px fade. Rim at x = 80 on the middle row.
    let pts: Vec<Pos2> = (0..=n).flat_map(|j| (0..=n).map(move |i| Pos2::new(i as f32 * 5.0, j as f32 * 5.0))).collect();
    let tint = Color32::WHITE.gamma_multiply(0.5);
    let circle = ScreenCircle { center: Pos2::new(-10_000.0, 80.0), radius: 10_080.0 };
    let mesh = edge_mesh(egui::TextureId::default(), &pts, &circle, tint);
    assert_eq!(mesh.vertices.len(), (n + 1) * (n + 1));
    // Along the middle row (y = 80): full at x = 60 (20 px inside), part at x = 65 and x = 70, clear from x = 75.
    let mid = |i: usize| mesh.vertices[(n / 2) * (n + 1) + i].color;
    assert_eq!(mid(12), tint);
    assert!(mid(13).a() < tint.a() && mid(13).a() > mid(14).a() && mid(14).a() > 0, "{:?} {:?}", mid(13), mid(14));
    assert_eq!(mid(15), Color32::TRANSPARENT);
    assert_eq!(mid(16), Color32::TRANSPARENT);
    // The texture is spread over the whole tile, whatever part is painted.
    assert_eq!(mesh.vertices[0].uv, Pos2::new(0.0, 0.0));
    assert_eq!(mesh.vertices[(n + 1) * (n + 1) - 1].uv, Pos2::new(1.0, 1.0));

    // Wherever the rim falls between the grid lines, no painted cell has a corner beyond it.
    for offset in [0.0_f32, 0.5, 1.0, 2.0, 2.5, 4.0, 4.9] {
        let circle = ScreenCircle { center: Pos2::new(-10_000.0, 80.0), radius: 10_080.0 + offset };
        let mesh = edge_mesh(egui::TextureId::default(), &pts, &circle, tint);
        assert!(!mesh.indices.is_empty());
        let reach = mesh.indices.iter().map(|&i| (mesh.vertices[i as usize].pos - circle.center).length()).fold(0.0_f32, f32::max);
        assert!(reach <= circle.radius, "offset {offset}: a painted corner is {} px beyond the rim", reach - circle.radius);
        // And the fade is not pulled in further than it has to be: something is painted within 13 px of the rim.
        assert!(reach >= circle.radius - 13.0, "offset {offset}: {reach}");
    }
}

/// The legend menu treats the row as a picture (no vertex, area or label rows of a vector layer).
#[test]
fn test_f1_reference_tile_row_is_a_picture_in_the_legend_menu() {
    let centre = work_xy(GYEONGJU.0, GYEONGJU.1);
    let (mut app, _rect) = map_app(centre, 30.0);
    draw_survey_area(&mut app, centre);
    assert!(app.add_reference_tile_layer(&DAEDONGYEOJIDO));
    let input = app.layer_menu_input("ref_daedongyeojido").unwrap();
    assert!(!input.vector);
    assert_eq!(input.geom, None);
    assert_eq!(input.kind, crate::layer_menu_model::LayerMenuKind::Reference);
}

/// Background tiles and reference tiles share one texture store. When it is trimmed for one kind, the tiles
/// of the other kinds stay: otherwise the old map would blink each time the background store fills up.
#[test]
fn test_f1_trimming_one_kind_keeps_the_textures_of_the_other_kinds() {
    let ctx = egui::Context::default();
    let mut cache = crate::basemap_paint::TileTextureCache::default();
    let _ = ctx.run(RawInput::default(), |ctx| {
        for (kind, x) in [(TileKind::Satellite, 1), (TileKind::Satellite, 2), (TileKind::Daedongyeojido, 1), (TileKind::Daedongyeojido, 2)] {
            let tex = ctx.load_texture(format!("{kind:?}{x}"), ColorImage::new([1, 1], Color32::RED), egui::TextureOptions::LINEAR);
            cache.insert((kind, 10, x, 0), tex);
        }
    });
    let used: std::collections::HashSet<TexKey> = [(TileKind::Satellite, 10, 2, 0)].into_iter().collect();
    cache.retain_kinds(&[TileKind::Satellite, TileKind::Base], &used);
    assert!(cache.get(&(TileKind::Satellite, 10, 1, 0)).is_none());
    assert!(cache.get(&(TileKind::Satellite, 10, 2, 0)).is_some());
    assert!(cache.get(&(TileKind::Daedongyeojido, 10, 1, 0)).is_some());
    assert!(cache.get(&(TileKind::Daedongyeojido, 10, 2, 0)).is_some());
}

/// The 투명도 rail on the map (KaLayerOpacityRail.cpp:18-177) and the 투명도 row of the legend menu act on the
/// one opacity of the selected picture row: the rail fades the old map, and shows what the menu set.
#[test]
fn test_f1_opacity_rail_and_legend_menu_share_the_opacity_of_a_picture_row() {
    let centre = work_xy(GYEONGJU.0, GYEONGJU.1);
    let (mut app, rect) = map_app(centre, 30.0);
    draw_survey_area(&mut app, centre);
    add_old_map_and_look_at(&mut app, centre, 30.0);
    let opacity = |app: &crate::app_state::StrataAppState, key: &str| app.left_layers.entries.iter().find(|e| e.layer_key == key).unwrap().opacity;

    app.apply_rail_opacity("대동여지도", 0.3);
    assert!((opacity(&app, "ref_daedongyeojido") - 0.3).abs() < f32::EPSILON);
    let (_, shapes) = paint_at_rest(&mut app, rect, Some(ColorImage::new([1, 1], Color32::RED)));
    let want = Color32::WHITE.gamma_multiply(0.3);
    assert!(!mesh_colours(&shapes).is_empty() && mesh_colours(&shapes).iter().all(|c| *c == want));

    assert!(app.layer_set_transparency("ref_daedongyeojido", 40));
    assert!((app.rail_opacity("대동여지도") - 0.6).abs() < f32::EPSILON, "the rail shows what the menu set");

    // The background row is faded by its own opacity too (gui_tab_map_basemap.rs basemap_opacity).
    app.apply_rail_opacity("위성지도 (VWorld)", 0.25);
    assert!((opacity(&app, "vworld_satellite") - 0.25).abs() < f32::EPSILON);
    assert!((opacity(&app, "ref_daedongyeojido") - 0.6).abs() < f32::EPSILON, "only the selected row");
}

/// A map of tiles added to the legend is unsaved work like a received map, so that 저장 keeps it for the
/// next open (survey_map_list.rs).
#[test]
fn test_f1_an_added_tile_map_leaves_the_survey_unsaved() {
    let centre = work_xy(GYEONGJU.0, GYEONGJU.1);
    let (mut app, _rect) = map_app(centre, 30.0);
    draw_survey_area(&mut app, centre);
    app.is_dirty = false;
    assert!(app.add_reference_tile_layer(&DAEDONGYEOJIDO));
    assert!(app.is_dirty);
}
