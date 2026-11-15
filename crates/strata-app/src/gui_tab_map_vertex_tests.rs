//! The handles of the selected shape, with the mouse and the keys through the whole window. Places on the
//! map are read from the app itself (the pointer's ground position), so the shape is built under known
//! screen points.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;

use eframe::egui::{self, pos2, vec2, Event, Modifiers, PointerButton, Pos2, RawInput, Rect};
use strata_canvas2d::map_tools::MapToolKind;
use strata_storage::gpkg_wkt::wkt_to_parts;

use crate::app_state::vertex_edit_geom::vertices;
use crate::app_state::{MainTab, StrataAppState};

struct Bench {
    app: StrataAppState,
    ctx: egui::Context,
    time: f64,
    _dir: tempfile::TempDir,
}

impl Bench {
    fn frame(&mut self, events: Vec<Event>) {
        self.time += 0.3;
        let raw = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1680.0, 960.0))), time: Some(self.time), events, ..Default::default() };
        let app = &mut self.app;
        let _ = self.ctx.run(raw, |ctx| {
            let _ = app.render_workspace_egui(ctx);
        });
    }

    /// The ground under a screen point of the map.
    fn ground(&mut self, at: Pos2) -> [f64; 2] {
        self.frame(vec![Event::PointerMoved(at)]);
        self.app.home_map.cursor_world_xy
    }

    fn button(&mut self, at: Pos2, pressed: bool) {
        self.frame(vec![Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::default() }]);
    }

    fn key(&mut self, key: egui::Key) {
        self.frame(vec![Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::default() }]);
    }

    fn points(&self) -> Vec<[f64; 2]> {
        let t = self.app.survey_tables.iter().find(|t| t.table == "feature_poly").unwrap();
        vertices(&wkt_to_parts(&t.layer.features[0].wkt_geom).unwrap())
    }
}

/// A new survey with one four-cornered shape selected in 「도형선택」; its corners lie under the returned
/// screen points.
fn bench() -> (Bench, [Pos2; 4]) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("새조사.gpkg");
    let crs = strata_types::crs::WorkCrs::Epsg5187;
    strata_storage::gpkg::GpkgRepository::create_new_survey(&path, crs).unwrap();
    let mut app = StrataAppState::new_startup();
    app.start_new_survey_project_with_path("새조사", &path, crs);
    app.active_tab = MainTab::Map;
    let mut bench = Bench { app, ctx: egui::Context::default(), time: 0.0, _dir: dir };
    bench.frame(vec![]);
    let corners = [pos2(700.0, 500.0), pos2(820.0, 500.0), pos2(840.0, 400.0), pos2(720.0, 380.0)];
    let ground: Vec<[f64; 2]> = corners.iter().map(|at| bench.ground(*at)).collect();
    let ring: Vec<String> = ground.iter().chain(ground.first()).map(|p| format!("{} {}", p[0], p[1])).collect();
    bench.app.left_layers.ensure_domain_layer(strata_types::schema::DomainLayerKey::FeaturePoly);
    let pits = bench.app.survey_tables.iter_mut().find(|t| t.table == "feature_poly").unwrap();
    let fid = pits.layer.add_unsaved_feature(format!("POLYGON (({}))", ring.join(", ")), BTreeMap::new());
    bench.app.home_map.selected_shape = Some(("feature_poly".to_owned(), fid));
    bench.app.active_tool = MapToolKind::Select;
    (bench, corners)
}

/// A handle taken with the mouse and let go somewhere else moves its vertex there, as one step, with the C++
/// words; the map does not move under the pointer and the shape stays the selected one.
#[test]
fn a_held_handle_moves_its_vertex() {
    let (mut bench, corners) = bench();
    let (to, steps, view) = (pos2(900.0, 560.0), bench.app.undo_tables.len(), bench.app.home_map.viewport.center_xy);
    let target = bench.ground(to);
    let reach = bench.app.home_map.viewport.meters_per_pixel * 1.5;

    bench.frame(vec![Event::PointerMoved(corners[1])]);
    bench.button(corners[1], true);
    bench.frame(vec![Event::PointerMoved(pos2(860.0, 530.0))]);
    assert!(bench.app.home_map.held_vertex.is_some(), "the handle is held");
    bench.frame(vec![Event::PointerMoved(to)]);
    bench.button(to, false);

    let points = bench.points();
    assert!(points.iter().any(|p| (p[0] - target[0]).hypot(p[1] - target[1]) < reach), "{target:?} in {points:?}");
    assert_eq!(bench.app.undo_tables.len(), steps + 1);
    assert_eq!(bench.app.status_left_message(), "수정점을 옮겼습니다. Ctrl+Z로 되돌릴 수 있습니다.");
    assert_eq!(bench.app.home_map.viewport.center_xy, view, "the map did not pan");
    assert!(bench.app.home_map.selected_shape.is_some() && bench.app.home_map.held_vertex.is_none());
}

/// A click on a handle picks its vertex and changes nothing; Delete then takes out that vertex only, not the
/// shape; Esc lets go of a picked vertex and keeps the tool.
#[test]
fn a_clicked_handle_is_picked_and_the_delete_key_takes_its_vertex_out() {
    let (mut bench, corners) = bench();
    let steps = bench.app.undo_tables.len();
    bench.frame(vec![Event::PointerMoved(corners[2])]);
    bench.button(corners[2], true);
    bench.button(corners[2], false);
    assert_eq!(bench.app.home_map.active_vertex, Some(2));
    assert_eq!((bench.app.undo_tables.len(), bench.points().len()), (steps, 5));
    assert_eq!(bench.app.status_left_message(), "점을 골랐습니다. Delete로 지우고, 끌면 옮깁니다.");

    bench.key(egui::Key::Escape);
    assert_eq!((bench.app.home_map.active_vertex, bench.app.active_tool), (None, MapToolKind::Select));

    bench.frame(vec![Event::PointerMoved(corners[2])]);
    bench.button(corners[2], true);
    bench.button(corners[2], false);
    bench.key(egui::Key::Delete);
    assert_eq!(bench.points().len(), 4, "one corner less");
    assert_eq!(bench.app.survey_tables.iter().find(|t| t.table == "feature_poly").unwrap().layer.features.len(), 1, "the shape is still there");
    assert_eq!(bench.app.status_left_message(), "점을 지웠습니다. Ctrl+Z로 되돌릴 수 있습니다.");
}

/// A right click on a handle offers 「점삭제」 and on a side 「점추가」; elsewhere the menu of the map opens.
#[test]
fn a_right_click_offers_the_vertex_menu_on_a_handle_and_on_a_side() {
    let (mut bench, corners) = bench();
    let side = pos2(760.0, 500.0);
    for (at, vertex) in [(corners[0], true), (side, false)] {
        let place = bench.ground(at);
        assert!(bench.app.open_vertex_menu(at, place));
        assert_eq!(bench.app.home_map.vertex_menu.is_some_and(|menu| format!("{menu:?}").contains("vertex: Some")), vertex);
        bench.app.home_map.vertex_menu = None;
    }
    let far = pos2(1000.0, 700.0);
    let place = bench.ground(far);
    assert!(!bench.app.open_vertex_menu(far, place), "away from the shape the map's own menu opens");
}
