//! Popup/canvas Escape ownership, target/cpp-ref/src/app/KaCaptureMapTool.cpp:379-391 and MainWindowContextMenus.cpp:176-448.
use crate::app_state::{MainTab, StrataAppState};
use eframe::egui::{self, Event, Key, PointerButton, Pos2, RawInput, Rect, Vec2};

struct Harness {
    app: StrataAppState,
    ctx: egui::Context,
    time: f64,
    _directory: tempfile::TempDir,
    textures: crate::gui_tab_drawing::sheet_raster::Textures,
    size: [u32; 2],
}
impl Harness {
    fn drawing() -> Self {
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
    fn frame(&mut self, events: Vec<Event>) -> egui::FullOutput {
        self.time += 0.05;
        let output = self.ctx.run(
            RawInput {
                time: Some(self.time),
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(self.size[0] as f32, self.size[1] as f32))),
                events,
                ..Default::default()
            },
            |ctx| {
                self.app.render_workspace_egui(ctx);
            },
        );
        self.textures.apply(&output.textures_delta);
        output
    }
    fn press(&mut self, at: Pos2, button: PointerButton) {
        assert!(self.ctx.screen_rect().contains(at), "click outside app");
        self.time += 0.5;
        self.frame(vec![Event::PointerMoved(at)]);
        for pressed in [true, false] {
            self.frame(vec![Event::PointerButton { pos: at, button, pressed, modifiers: Default::default() }]);
        }
        self.frame(vec![]);
    }
    fn point_at(&mut self, title: &str) -> Pos2 {
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
    fn escape(&mut self) {
        for pressed in [true, false] {
            self.frame(vec![Event::Key { key: Key::Escape, physical_key: None, pressed, repeat: false, modifiers: Default::default() }]);
        }
    }
    fn save(&mut self, name: &str) {
        let output = self.frame(vec![]);
        crate::test_mods::ui_snapshot_support::save_frame_if_requested(&self.ctx, &output, &self.textures, self.size, name);
    }
    fn close_popup_without_losing_capture(&mut self, name: &str) {
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

#[test]
fn popup_escape_closes_actual_compact_layer_menu_and_preserves_unfinished_polygon() {
    let mut h = Harness::drawing();
    h.size = [1024, 768];
    h.frame(vec![]);
    let at = h.point_at("레이어");
    h.press(at, PointerButton::Primary);
    h.point_at("파일함");
    h.close_popup_without_losing_capture("popup-escape-native-layer");
    let output = h.frame(vec![]);
    assert!(!output.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text() == "파일함")), "native menu did not close");
    h.escape();
    assert!(h.app.draw_sketch.points.is_empty(), "canvas Escape no longer cancels after native menu closed");
}

#[test]
fn popup_escape_closes_reopened_compact_ribbon_menu_without_discarding_capture() {
    let mut h = Harness::drawing();
    h.size = [1024, 768];
    h.frame(vec![]);
    for _ in 0..2 {
        let at = h.point_at("기록");
        h.press(at, PointerButton::Primary);
        h.point_at("그리기");
        h.close_popup_without_losing_capture("popup-escape-native-ribbon");
        let output = h.frame(vec![]);
        assert!(!output.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text() == "그리기")), "native ribbon menu did not close");
    }
    h.escape();
    assert!(h.app.draw_sketch.points.is_empty(), "canvas Escape blocked after reopening a native menu");
}

#[test]
fn popup_escape_on_native_menu_click_release_keeps_capture() {
    let mut h = Harness::drawing();
    h.size = [1024, 768];
    h.frame(vec![]);
    let at = h.point_at("레이어");
    let points = h.app.draw_sketch.points.clone();
    h.frame(vec![Event::PointerMoved(at)]);
    h.frame(vec![Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: true, modifiers: Default::default() }]);
    h.frame(vec![
        Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: false, modifiers: Default::default() },
        Event::Key { key: Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: Default::default() },
    ]);
    assert_eq!(h.app.draw_sketch.points, points, "batched native menu click/Escape discarded capture");
}

#[test]
fn popup_escape_stale_stock_popup_id_does_not_block_canvas_cancel() {
    let mut h = Harness::drawing();
    h.ctx.memory_mut(|memory| memory.open_popup(egui::Id::new("orphan_combo_after_closed_dialog")));
    assert!(h.ctx.memory(|memory| memory.any_popup_open()));
    h.escape();
    assert!(h.app.draw_sketch.points.is_empty(), "orphan stock popup id blocked real canvas cancellation");
}

#[test]
fn popup_escape_with_enter_in_one_input_batch_does_not_commit_the_cancelled_polygon() {
    let mut h = Harness::drawing();
    let before = h.app.survey_tables.iter().find(|table| table.table == "feature_poly").expect("actual polygon table").layer.features.len();
    h.frame([Key::Escape, Key::Enter].into_iter().map(|key| Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: Default::default() }).collect());
    let after = h.app.survey_tables.iter().find(|table| table.table == "feature_poly").expect("actual polygon table").layer.features.len();
    assert_eq!(after, before, "Escape/Enter committed the polygon before cancellation");
    assert!(h.app.draw_sketch.points.is_empty());
}

#[test]
fn popup_escape_closes_compact_layer_set_submenu_and_its_parent() {
    let mut h = Harness::drawing();
    h.size = [1024, 768];
    h.app.sketch_cancel();
    h.app.active_tool = strata_canvas2d::map_tools::MapToolKind::Pan;
    h.app.open_studio_tab(MainTab::Drawing);
    assert_eq!(h.app.active_tab, MainTab::Drawing);
    for _ in 0..3 {
        h.frame(vec![]);
    }
    let at = h.point_at("레이어");
    h.press(at, PointerButton::Primary);
    let at = h.point_at("세트 ▾");
    h.press(at, PointerButton::Primary);
    h.point_at("지금 켜진 레이어를 세트로 저장…");
    h.save("popup-escape-native-layer-set-child-open");
    h.escape();
    h.save("popup-escape-native-layer-set-child-closed");
    assert_eq!(h.app.active_tab, MainTab::Drawing);
    let output = h.frame(vec![]);
    assert!(
        !output.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Text(t) if ["파일함", "지금 켜진 레이어를 세트로 저장…"].contains(&t.galley.text()))),
        "native menu hierarchy stayed open"
    );
}

#[test]
fn popup_escape_closes_actual_dem_menu_and_preserves_unfinished_polygon() {
    let mut h = Harness::drawing();
    let at = h.point_at("DEM");
    h.press(at, PointerButton::Secondary);
    assert!(h.app.ribbon.dem_menu_open, "actual DEM chip did not open its menu");
    h.close_popup_without_losing_capture("popup-escape-dem");
    assert!(!h.app.ribbon.dem_menu_open);
    h.escape();
    assert!(h.app.draw_sketch.points.is_empty(), "canvas Escape no longer cancels capture after the menu closed");
}

#[test]
fn popup_escape_closes_actual_layer_menu_and_preserves_unfinished_polygon() {
    let mut h = Harness::drawing();
    let name = h.app.left_layers.entries.iter().find(|row| row.layer_key == "feature_poly").expect("actual feature layer").label_ko.clone();
    let at = h.point_at(&name);
    h.press(at, PointerButton::Secondary);
    assert!(h.app.layer_context_menu_open, "actual layer right click did not open its menu");
    h.close_popup_without_losing_capture("popup-escape-layer");
    assert!(!h.app.layer_context_menu_open);
    h.escape();
    assert!(h.app.draw_sketch.points.is_empty(), "canvas Escape no longer cancels capture after the menu closed");
}

#[test]
fn popup_escape_closes_actual_region_address_form_and_preserves_unfinished_polygon() {
    let mut h = Harness::drawing();
    let at = h.point_at("지역");
    h.press(at, PointerButton::Primary);
    assert!(h.app.ribbon.app_bar.region_popup_open);
    let at = h.point_at("서울");
    h.press(at, PointerButton::Primary);
    assert!(h.app.ribbon.app_bar.address_popup_open, "actual province chip did not open the address form");
    h.close_popup_without_losing_capture("popup-escape-address");
    assert!(!h.app.ribbon.app_bar.address_popup_open);
    h.escape();
    assert!(h.app.draw_sketch.points.is_empty(), "canvas Escape no longer cancels capture after the address form closed");
}
