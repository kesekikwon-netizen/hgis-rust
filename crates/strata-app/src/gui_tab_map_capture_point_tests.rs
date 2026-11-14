//! Headless egui point capture input, draw-row button, legacy-CAD isolation, and feature-pick tests
//! (`target/cpp-ref/src/app/MainWindowRibbon.cpp:905-910`,
//! `target/cpp-ref/src/app/MainWindowEditing.cpp:782-920,1168-1174`,
//! `target/cpp-ref/src/app/KaCaptureMapTool.cpp:260-314,379-467`).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use eframe::egui::{self, Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, Shape};
use strata_canvas2d::map_tools::MapToolKind;
use strata_storage::gpkg_wkt::wkt_to_parts;
use strata_storage::wkb_codec::WkbGeometry;

use crate::app_state::{MainTab, StrataAppState};

struct PointHarness {
    app: StrataAppState,
    ctx: egui::Context,
    time: f64,
    _dir: tempfile::TempDir,
}

impl PointHarness {
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
        let mut h = Self { app, ctx: egui::Context::default(), time: 0.0, _dir: dir };
        h.frame(vec![]);
        h
    }

    fn frame(&mut self, events: Vec<Event>) -> egui::FullOutput {
        self.time += 0.05;
        let raw = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1680.0, 960.0))),
            time: Some(self.time),
            events,
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

    fn artifact_count(&self) -> usize {
        self.app.survey_tables.iter().find(|t| t.table == "artifact_point").unwrap().layer.features.len()
    }
}

#[test]
fn clicking_draw_row_artifact_button_and_map_inserts_point_opens_form_and_skips_legacy_scene() {
    let mut h = PointHarness::new();
    let artifact_btn = h.draw_row_button_pos(1189.0).expect("artifact button in draw row");
    h.press(artifact_btn, PointerButton::Primary);

    assert_eq!(h.app.edit_layer_key.as_deref(), Some("artifact_point"));
    assert_eq!(h.app.active_tool, MapToolKind::PlacePoint);
    assert_eq!(h.app.status_left_message(), "유물 — 좌클릭으로 점 찍기");
    assert!(h.draw_row_button_pos(1264.0).is_none(), "sketch finish/undo/cancel stay hidden for points");

    let spot = Pos2::new(760.0, 480.0);
    h.press(spot, PointerButton::Primary);

    assert_eq!(h.artifact_count(), 2, "demo had 1 artifact; click inserts a 2nd into survey_tables");
    assert!(h.app.home_map.feature_form.is_some(), "name/number form opens after placing point");
    assert!(h.app.home_map.scene.active_rubberband.is_empty(), "legacy scene rubberband untouched");
    assert!(h.app.home_map.scene.polygons.is_empty(), "legacy scene polygons untouched");
    assert!(h.app.home_map.scene.heritage_badges_world.is_empty(), "legacy transient markers untouched");
    assert!(h.draw_row_button_pos(1264.0).is_none(), "sketch buttons never appear after point click");

    // Accept the pre-filled form ("2") with Enter.
    h.key(Key::Enter);
    assert!(h.app.home_map.feature_form.is_none());
    let last = h.app.survey_tables.iter().find(|t| t.table == "artifact_point").unwrap().layer.features.last().unwrap();
    assert_eq!(last.attributes.get("artifact_no").map(String::as_str), Some("2"));
    let parts = wkt_to_parts(&last.wkt_geom).expect("wkt");
    assert!(matches!(parts.as_slice(), [WkbGeometry::Point(_)]));
    let placed_fid = last.fid;

    // Switching to Select (key A) and clicking the same canvas spot picks the newly placed artifact point.
    h.key(Key::A);
    assert_eq!(h.app.active_tool, MapToolKind::Select);
    h.press(spot, PointerButton::Primary);
    assert_eq!(h.app.home_map.selected_shape, Some(("artifact_point".to_owned(), placed_fid)));
}

#[test]
fn point_capture_right_click_enter_escape_and_double_click_match_cpp_canvas_rules() {
    let mut h = PointHarness::new();
    h.app.ribbon.continuous_draw = true;
    let artifact_btn = h.draw_row_button_pos(1189.0).expect("artifact button");
    h.press(artifact_btn, PointerButton::Primary);
    let before = h.artifact_count();

    // Right-click on empty point capture neither adds a point nor opens the map context menu.
    h.press(Pos2::new(780.0, 500.0), PointerButton::Secondary);
    assert_eq!(h.artifact_count(), before);
    assert!(!h.app.map_context_menu_open);
    assert_eq!(h.app.status_left_message(), "유물 — 좌클릭으로 점 찍기");

    // Enter and Escape on empty point capture stay silent and keep PlacePoint + draw row open.
    h.key(Key::Enter);
    h.key(Key::Escape);
    assert_eq!(h.artifact_count(), before);
    assert_eq!(h.app.active_tool, MapToolKind::PlacePoint);
    assert!(h.app.draw_row_open());
    assert_eq!(h.app.status_left_message(), "유물 — 좌클릭으로 점 찍기");

    // A fast double-click in continuous-draw mode adds two points (two presses), not three from double-click.
    let spot = Pos2::new(740.0, 460.0);
    h.press(spot, PointerButton::Primary);
    h.quick_press(spot, PointerButton::Primary);
    assert_eq!(h.artifact_count(), before + 2, "double-click handler must not fire a 3rd finish in Point mode");

    // Dragging on Primary while placing a point (opening the modal form) does not pan the viewport.
    h.app.ribbon.continuous_draw = false;
    let center = h.app.home_map.viewport.center_xy;
    let (a, b) = (Pos2::new(700.0, 450.0), Pos2::new(780.0, 520.0));
    h.frame(vec![Event::PointerMoved(a)]);
    h.frame(vec![Event::PointerButton { pos: a, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::default() }]);
    h.frame(vec![Event::PointerMoved(Pos2::new(740.0, 485.0))]);
    h.frame(vec![Event::PointerMoved(b)]);
    h.frame(vec![Event::PointerButton { pos: b, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::default() }]);
    assert_eq!(h.app.home_map.viewport.center_xy, center, "point press-drag must not pan");
    assert_eq!(h.artifact_count(), before + 3);
}
