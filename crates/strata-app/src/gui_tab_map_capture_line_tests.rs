//! Headless egui line capture input and open-polyline rubberband painting tests
//! (`target/cpp-ref/src/app/KaCaptureMapTool.cpp:192-226,260-314,379-467`).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use eframe::egui::{self, Color32, Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, Shape};
use strata_canvas2d::map_tools::MapToolKind;
use strata_storage::gpkg_wkt::wkt_to_parts;
use strata_storage::wkb_codec::WkbGeometry;

use crate::app_state::{MainTab, StrataAppState};
// Canceled-capture status text (target/cpp-ref/src/app/MainWindowEditing.cpp:994-998).
const TOO_FEW_TEXT: &str = "아직 넣지 않았습니다 — 면은 점 3개, 선은 2개 이상 필요합니다. 우클릭·Enter로 완료.";

struct LineHarness {
    app: StrataAppState,
    ctx: egui::Context,
    time: f64,
    _dir: tempfile::TempDir,
}

impl LineHarness {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("demo.gpkg");
        std::fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg"),
            &path,
        )
        .expect("copy");
        let mut app = StrataAppState::new_startup();
        app.open_survey_gpkg(&path);
        app.active_tab = MainTab::Map;
        for e in app.left_layers.entries.clone() {
            if !matches!(e.group, strata_types::schema::LegendGroup::SurveyData) {
                app.home_map.real_survey.layer_visible.insert(e.label_ko, false);
            }
        }
        app.handle_ribbon_click("그리기");
        app.start_edit_feature_line();
        let mut h = Self { app, ctx: egui::Context::default(), time: 0.0, _dir: dir };
        h.frame(vec![]);
        h
    }

    fn frame(&mut self, events: Vec<Event>) -> egui::FullOutput {
        self.frame_with(events, Modifiers::default())
    }

    fn frame_with(&mut self, events: Vec<Event>, modifiers: Modifiers) -> egui::FullOutput {
        self.time += 0.05;
        let raw = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1680.0, 960.0))),
            time: Some(self.time),
            events,
            modifiers,
            ..Default::default()
        };
        let app = &mut self.app;
        self.ctx.run(raw, |ctx| {
            let _ = app.render_workspace_egui(ctx);
        })
    }

    fn press(&mut self, at: Pos2, which: PointerButton) {
        self.time += 0.5;
        self.quick_press(at, which);
    }

    fn quick_press(&mut self, at: Pos2, which: PointerButton) {
        self.frame(vec![Event::PointerMoved(at)]);
        self.frame(vec![Event::PointerButton { pos: at, button: which, pressed: true, modifiers: Modifiers::default() }]);
        self.frame(vec![Event::PointerButton { pos: at, button: which, pressed: false, modifiers: Modifiers::default() }]);
    }

    fn key(&mut self, key: Key) {
        self.frame(vec![Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::default() }]);
        self.frame(vec![Event::Key { key, physical_key: None, pressed: false, repeat: false, modifiers: Modifiers::default() }]);
    }

    fn ctrl_key(&mut self, key: Key) {
        let ctrl = Modifiers::CTRL;
        self.frame_with(vec![Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: ctrl }], ctrl);
        self.frame_with(vec![Event::Key { key, physical_key: None, pressed: false, repeat: false, modifiers: ctrl }], ctrl);
    }

    fn two_points(&mut self) {
        for at in [Pos2::new(700.0, 450.0), Pos2::new(840.0, 450.0)] {
            self.press(at, PointerButton::Primary);
        }
        assert_eq!(self.app.draw_sketch.points.len(), 2);
    }

    fn last_line_len(&self) -> (usize, usize) {
        let t = self.app.survey_tables.iter().find(|t| t.table == "feature_line").unwrap();
        let parts = wkt_to_parts(&t.layer.features.last().unwrap().wkt_geom).expect("wkt");
        let WkbGeometry::LineString(pts) = &parts[0] else { panic!("linestring") };
        (t.layer.features.len(), pts.len())
    }

    fn draw_row_button_pos(&mut self, actual_x: f32) -> Option<Pos2> {
        let out = self.frame(vec![]);
        let want_x = actual_x / 1.5;
        let want_w = 55.0 / 1.5;
        out.shapes.iter().find_map(|c| match &c.shape {
            Shape::Rect(r) if (r.rect.width() - want_w).abs() < 0.5 && (r.rect.min.x - want_x).abs() < 2.0 => {
                Some(r.rect.center())
            }
            _ => None,
        })
    }
}

#[test]
fn click_click_enter_finishes_line_and_skips_legacy_cad() {
    let mut h = LineHarness::new();
    h.two_points();
    assert!(h.app.home_map.scene.active_rubberband.is_empty(), "legacy CAD received nothing");
    h.key(Key::Enter);
    assert_eq!(h.last_line_len(), (2, 2));
    assert!(h.app.is_dirty);
}

#[test]
fn click_click_right_click_finishes_line_without_menu() {
    let mut h = LineHarness::new();
    h.two_points();
    h.press(Pos2::new(780.0, 500.0), PointerButton::Secondary);
    assert_eq!(h.last_line_len(), (2, 2));
    assert!(!h.app.map_context_menu_open);
}

#[test]
fn double_click_finishes_line_without_a_duplicate_vertex() {
    let mut h = LineHarness::new();
    h.press(Pos2::new(700.0, 450.0), PointerButton::Primary);
    h.press(Pos2::new(840.0, 450.0), PointerButton::Primary);
    h.quick_press(Pos2::new(840.0, 450.0), PointerButton::Primary);
    assert_eq!(h.last_line_len(), (2, 2));
}

#[test]
fn one_point_finish_shows_too_few_text_and_keeps_vertex() {
    let mut h = LineHarness::new();
    h.press(Pos2::new(700.0, 450.0), PointerButton::Primary);
    h.key(Key::Enter);
    assert_eq!(h.app.status_left_message(), TOO_FEW_TEXT);
    assert_eq!(h.app.draw_sketch.points.len(), 1);
    assert_eq!(h.last_line_len().0, 1);
}

#[test]
fn undo_vertex_keys_and_escape_work_for_line_sketch() {
    let mut h = LineHarness::new();
    for x in [680.0, 740.0, 800.0, 860.0] {
        h.press(Pos2::new(x, 450.0), PointerButton::Primary);
    }
    assert_eq!(h.app.draw_sketch.points.len(), 4);
    h.key(Key::Backspace);
    assert_eq!(h.app.draw_sketch.points.len(), 3);
    assert_eq!(h.app.status_left_message(), "꼭짓점 하나를 되돌렸습니다.");
    h.key(Key::Delete);
    assert_eq!(h.app.draw_sketch.points.len(), 2);
    assert_eq!(h.app.status_left_message(), "꼭짓점 하나를 되돌렸습니다.");
    h.ctrl_key(Key::Z);
    assert_eq!(h.app.draw_sketch.points.len(), 1);
    assert_eq!(h.app.status_left_message(), "꼭짓점 하나를 되돌렸습니다.");
    h.key(Key::Escape);
    assert!(h.app.draw_sketch.points.is_empty());
    assert_eq!(h.app.active_tool, MapToolKind::DrawLine);
    assert!(h.app.draw_row_open());
    assert_eq!(h.app.status_left_message(), "그리던 도형을 지웠습니다.");
}

#[test]
fn draw_row_buttons_work_and_line_sketch_paints_open_polyline_without_fill() {
    let mut h = LineHarness::new();
    assert!(h.draw_row_button_pos(1264.0).is_none(), "sketch buttons hidden while empty");
    h.two_points();
    h.press(Pos2::new(900.0, 480.0), PointerButton::Primary);
    assert_eq!(h.app.draw_sketch.points.len(), 3);

    // Paint sketch directly into a layer painter to inspect shapes: open polyline + cursor, no fill.
    let rect = Rect::from_min_size(Pos2::new(300.0, 200.0), egui::vec2(800.0, 500.0));
    let cursor = Pos2::new(920.0, 520.0);
    let out = h.ctx.run(RawInput::default(), |ctx| {
        let p = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("test_sketch")));
        h.app.paint_polygon_sketch(&p, rect, Some(cursor));
    });
    let shapes: Vec<&Shape> = out.shapes.iter().map(|c| &c.shape).collect();
    assert!(!shapes.iter().any(|s| matches!(s, Shape::Mesh(_))), "line sketch must have no fill mesh");
    let paths: Vec<_> = shapes
        .iter()
        .filter_map(|s| match s {
            Shape::Path(ps) => Some(ps),
            _ => None,
        })
        .collect();
    assert_eq!(paths.len(), 2, "secondary white + primary blue open strokes");
    for ps in paths {
        assert!(!ps.closed, "line sketch must not close back to the first vertex");
        assert_eq!(ps.fill, Color32::TRANSPARENT);
        assert_eq!(ps.points.len(), 4, "three sketch points plus the segment to the cursor");
        assert_eq!(ps.points[3], cursor);
    }

    // On-screen undo, cancel, and finish buttons in the draw row
    // (target/cpp-ref/src/app/KaDrawSketchTools.cpp:96-114,144-169).
    let undo_pos = h.draw_row_button_pos(1324.0).expect("undo button visible");
    h.press(undo_pos, PointerButton::Primary);
    assert_eq!(h.app.draw_sketch.points.len(), 2);
    assert_eq!(h.app.status_left_message(), "꼭짓점 하나를 되돌렸습니다.");

    let cancel_pos = h.draw_row_button_pos(1384.0).expect("cancel button visible");
    h.press(cancel_pos, PointerButton::Primary);
    assert!(h.app.draw_sketch.points.is_empty());
    assert_eq!(h.app.status_left_message(), "그리던 도형을 지웠습니다.");

    h.two_points();
    let finish_pos = h.draw_row_button_pos(1264.0).expect("finish button visible");
    h.press(finish_pos, PointerButton::Primary);
    assert_eq!(h.last_line_len(), (2, 2));
}
