//! Popup/canvas Escape ownership, target/cpp-ref/src/app/KaCaptureMapTool.cpp:379-391 and MainWindowContextMenus.cpp:176-448.
use crate::app_state::{MainTab, StrataAppState};
use eframe::egui::{self, Event, Key, PointerButton, Pos2, RawInput, Rect, Vec2};

pub(super) struct Harness {
    pub(super) app: StrataAppState,
    pub(super) ctx: egui::Context,
    time: f64,
    _directory: tempfile::TempDir,
    textures: crate::gui_tab_drawing::sheet_raster::Textures,
    pub(super) size: [u32; 2],
}
impl Harness {
    pub(super) fn drawing() -> Self {
        let directory = tempfile::tempdir().expect("temporary survey");
        let file = directory.path().join("demo.gpkg");
        std::fs::copy(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg"), &file).expect("tracked demo copy");
        let mut app = StrataAppState::new_startup();
        assert!(app.open_survey_gpkg(&file));
        assert_eq!(app.work_crs.auth_id(), "EPSG:5187");
        app.active_tab = MainTab::Map;
        for entry in app.left_layers.entries.clone() {
            if entry.group != strata_types::schema::LegendGroup::SurveyData {
                app.home_map.real_survey.layer_visible.insert(entry.label_ko, false);
            }
        }
        app.handle_ribbon_click("그리기");
        app.start_edit_feature_poly();
        let ctx = egui::Context::default();
        crate::gui_theme_fonts::configure_korean_fonts_and_theme(&ctx, &app.theme);
        let mut h = Self { app, ctx, time: 0.0, _directory: directory, textures: Default::default(), size: [1680, 960] };
        for _ in 0..3 {
            h.frame(vec![]);
        }
        // Client pixels, as in gui_tab_map_capture_tests.rs:85; existing projection produces work-CRS EPSG:5187 vertices.
        for at in [Pos2::new(700.0, 450.0), Pos2::new(820.0, 450.0), Pos2::new(820.0, 570.0)] {
            h.press(at, PointerButton::Primary);
        }
        assert_eq!(h.app.draw_sketch.points.len(), 3, "actual canvas capture setup");
        h
    }
    pub(super) fn frame(&mut self, events: Vec<Event>) -> egui::FullOutput {
        self.frame_with_modifiers(events, egui::Modifiers::default())
    }
    pub(super) fn frame_with_modifiers(&mut self, events: Vec<Event>, modifiers: egui::Modifiers) -> egui::FullOutput {
        self.frame_input(RawInput { events, modifiers, ..Default::default() })
    }
    pub(super) fn frame_with_close_request(&mut self, events: Vec<Event>, modifiers: egui::Modifiers) -> egui::FullOutput {
        // target/cpp-ref/src/app/MainWindow.cpp:333-352: closeEvent is a request, not a direct prompt-state assignment.
        let mut input = RawInput { events, modifiers, ..Default::default() };
        input.viewports.entry(egui::ViewportId::ROOT).or_default().events.push(egui::ViewportEvent::Close);
        self.frame_input(input)
    }
    fn frame_input(&mut self, mut input: RawInput) -> egui::FullOutput {
        self.time += 0.05;
        input.time = Some(self.time);
        input.screen_rect = Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(self.size[0] as f32, self.size[1] as f32)));
        let output = self.ctx.run(input, |ctx| {
            self.app.render_workspace_egui(ctx);
        });
        self.textures.apply(&output.textures_delta);
        output
    }
    pub(super) fn press(&mut self, at: Pos2, button: PointerButton) {
        self.press_output(at, button);
        self.frame(vec![]);
    }
    pub(super) fn press_output(&mut self, at: Pos2, button: PointerButton) -> egui::FullOutput {
        assert!(self.ctx.screen_rect().contains(at), "click outside app");
        self.time += 0.5;
        self.frame(vec![Event::PointerMoved(at)]);
        self.frame(vec![Event::PointerButton { pos: at, button, pressed: true, modifiers: Default::default() }]);
        self.frame(vec![Event::PointerButton { pos: at, button, pressed: false, modifiers: Default::default() }])
    }
    pub(super) fn point_at(&mut self, title: &str) -> Pos2 {
        let output = self.frame(vec![]);
        output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.text() == title => {
                    let rect = t.galley.rect.translate(t.pos.to_vec2());
                    (self.ctx.screen_rect().contains_rect(rect) && s.clip_rect.contains_rect(rect)).then_some(rect.center())
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("actual visible action missing: {title}"))
    }
    pub(super) fn escape(&mut self) {
        for pressed in [true, false] {
            self.frame(vec![Event::Key { key: Key::Escape, physical_key: None, pressed, repeat: false, modifiers: Default::default() }]);
        }
    }
    pub(super) fn save(&mut self, name: &str) {
        let output = self.frame(vec![]);
        crate::test_mods::ui_snapshot_support::save_frame_if_requested(&self.ctx, &output, &self.textures, self.size, name);
    }
    pub(super) fn close_popup_without_losing_capture(&mut self, name: &str) {
        assert_eq!(self.app.draw_sketch.points.len(), 3, "opening the popup discarded capture");
        let points = self.app.draw_sketch.points.clone();
        let tool = self.app.active_tool;
        self.save(&format!("{name}-open"));
        self.escape();
        self.save(&format!("{name}-closed"));
        assert_eq!(self.app.draw_sketch.points, points, "popup Escape discarded pending EPSG:5187 vertices");
        assert_eq!(self.app.active_tool, tool, "popup Escape changed the canvas tool");
    }
}
