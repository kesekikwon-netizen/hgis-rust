//! The selection box of 「도형선택」 (KaFeatureSelectTool.cpp:209-225, 263-270, 417-484), in the whole window with
//! real pointer events: places on the map are read from the app itself, so the shapes are built under known
//! screen points.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;

use eframe::egui::{self, pos2, vec2, Event, Modifiers, PointerButton, Pos2, RawInput, Rect};
use strata_canvas2d::map_tools::MapToolKind;

use crate::app_state::{MainTab, StrataAppState};

struct Bench {
    app: StrataAppState,
    ctx: egui::Context,
    time: f64,
    _dir: tempfile::TempDir,
}

impl Bench {
    fn frame(&mut self, events: Vec<Event>, shift: bool) {
        self.time += 0.3;
        let modifiers = if shift { Modifiers::SHIFT } else { Modifiers::default() };
        let raw = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1680.0, 960.0))), time: Some(self.time), events, modifiers, ..Default::default() };
        let app = &mut self.app;
        let _ = self.ctx.run(raw, |ctx| {
            let _ = app.render_workspace_egui(ctx);
        });
    }

    fn ground(&mut self, at: Pos2) -> [f64; 2] {
        self.frame(vec![Event::PointerMoved(at)], false);
        self.app.home_map.cursor_world_xy
    }

    /// A square whose corners lie under the two screen points.
    fn square(&mut self, a: Pos2, b: Pos2) -> i64 {
        let ([x0, y0], [x1, y1]) = (self.ground(a), self.ground(b));
        let wkt = format!("POLYGON (({x0} {y0}, {x1} {y0}, {x1} {y1}, {x0} {y1}, {x0} {y0}))");
        self.app.survey_tables.iter_mut().find(|t| t.table == "feature_poly").unwrap().layer.add_unsaved_feature(wkt, BTreeMap::new())
    }

    fn drag(&mut self, from: Pos2, to: Pos2, button: PointerButton, shift: bool) {
        let press = |at: Pos2, pressed: bool| Event::PointerButton { pos: at, button, pressed, modifiers: if shift { Modifiers::SHIFT } else { Modifiers::default() } };
        self.frame(vec![Event::PointerMoved(from)], shift);
        self.frame(vec![press(from, true)], shift);
        self.frame(vec![Event::PointerMoved(from + (to - from) * 0.5)], shift);
        self.frame(vec![Event::PointerMoved(to)], shift);
        self.frame(vec![press(to, false)], shift);
    }
}

fn bench() -> Bench {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("새조사.gpkg");
    let crs = strata_types::crs::WorkCrs::Epsg5187;
    strata_storage::gpkg::GpkgRepository::create_new_survey(&path, crs).unwrap();
    let mut app = StrataAppState::new_startup();
    app.start_new_survey_project_with_path("새조사", &path, crs);
    app.active_tab = MainTab::Map;
    app.left_layers.ensure_domain_layer(strata_types::schema::DomainLayerKey::FeaturePoly);
    let mut bench = Bench { app, ctx: egui::Context::default(), time: 0.0, _dir: dir };
    bench.frame(vec![], false);
    bench.app.active_tool = MapToolKind::Select;
    bench
}

/// A drag on bare map draws the box and selects every shape it meets (the one inside and the one across its
/// edge, not the one apart), with 「도형 2개 선택됨」; the map does not move. Shift adds the next box's shapes.
#[test]
fn a_box_selects_what_it_meets_and_shift_adds() {
    let mut bench = bench();
    let inside = bench.square(pos2(700.0, 500.0), pos2(740.0, 460.0));
    let across = bench.square(pos2(780.0, 520.0), pos2(860.0, 440.0));
    let apart = bench.square(pos2(1000.0, 500.0), pos2(1040.0, 460.0));
    let view = bench.app.home_map.viewport.center_xy;
    bench.drag(pos2(680.0, 560.0), pos2(800.0, 400.0), PointerButton::Primary, false);
    let fp = |fid: i64| ("feature_poly".to_owned(), fid);
    let mut chosen = bench.app.all_selected();
    chosen.sort();
    let mut wanted = vec![fp(inside), fp(across)];
    wanted.sort();
    assert_eq!(chosen, wanted);
    assert_eq!(bench.app.status_left_message(), "도형 2개 선택됨");
    assert_eq!(bench.app.home_map.viewport.center_xy, view, "the map did not move");
    bench.drag(pos2(980.0, 540.0), pos2(1060.0, 420.0), PointerButton::Primary, true);
    assert_eq!(bench.app.all_selected().len(), 3, "Shift adds {apart}");
    assert_eq!(bench.app.status_left_message(), "도형 3개 선택됨");
}

/// The middle button still moves the map in 「도형선택」.
#[test]
fn the_middle_button_moves_the_map() {
    let mut bench = bench();
    let view = bench.app.home_map.viewport.center_xy;
    bench.drag(pos2(700.0, 500.0), pos2(800.0, 450.0), PointerButton::Middle, false);
    assert_ne!(bench.app.home_map.viewport.center_xy, view);
    assert!(bench.app.all_selected().is_empty());
}
