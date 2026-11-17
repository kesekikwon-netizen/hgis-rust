//! QMenu reachability, target/cpp-ref/src/app/MainWindowContextMenus.cpp:176-448.
use super::{show_menu, MenuPick};
use crate::layer_menu_model::{layer_menu_entries, MenuEntry, MenuRow};
use eframe::egui::{self, Event, Pos2, RawInput, Rect, Vec2};

struct Harness {
    ctx: egui::Context,
    entries: Vec<MenuEntry>,
    path: Vec<usize>,
    position: Pos2,
    size: [u32; 2],
    time: f64,
    rects: Vec<Rect>,
    selected: Option<MenuPick>,
    textures: crate::gui_tab_drawing::sheet_raster::Textures,
}
impl Harness {
    fn new(entries: Vec<MenuEntry>, size: [u32; 2], position: Pos2) -> Self {
        let ctx = egui::Context::default();
        crate::gui_theme_fonts::configure_korean_fonts_and_theme(&ctx, &Default::default());
        let mut h = Self { ctx, entries, path: vec![], position, size, time: 0.0, rects: vec![], selected: None, textures: Default::default() };
        h.settle();
        h
    }
    fn frame(&mut self, events: Vec<Event>) -> egui::FullOutput {
        self.time += 0.05;
        let input = RawInput {
            time: Some(self.time),
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(self.size[0] as f32, self.size[1] as f32))),
            events,
            ..Default::default()
        };
        let output = self.ctx.run(input, |ctx| {
            let (pick, rects) = show_menu(ctx, egui::Id::new("context_menu_bounds_test"), &self.entries, self.position, &mut self.path, 0);
            self.rects = rects;
            if pick.is_some() {
                self.selected = pick;
            }
        });
        self.textures.apply(&output.textures_delta);
        output
    }
    fn settle(&mut self) {
        for _ in 0..3 {
            self.frame(vec![]);
        }
    }
    fn visible_text(&self, output: &egui::FullOutput, title: &str) -> Option<Rect> {
        output.shapes.iter().find_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.text() == title => {
                let rect = t.galley.rect.translate(t.pos.to_vec2());
                (self.ctx.screen_rect().contains_rect(rect) && s.clip_rect.contains_rect(rect)).then_some(rect)
            }
            _ => None,
        })
    }
    fn point_at(&mut self, title: &str) -> Pos2 {
        let output = self.frame(vec![]);
        self.visible_text(&output, title).unwrap_or_else(|| panic!("action not visible inside app: {title}")).center()
    }
    fn hover(&mut self, title: &str) {
        let at = self.point_at(title);
        self.frame(vec![Event::PointerMoved(at)]);
        self.settle();
    }
    fn click(&mut self, title: &str) {
        let at = self.point_at(title);
        self.frame(vec![Event::PointerMoved(at)]);
        for pressed in [true, false] {
            self.frame(vec![Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() }]);
        }
    }
    fn save(&mut self, name: &str) {
        let output = self.frame(vec![]);
        crate::test_mods::ui_snapshot_support::save_frame_if_requested(&self.ctx, &output, &self.textures, self.size, name);
    }
    fn assert_panels_fit(&self) {
        assert!(self.rects.iter().all(|r| self.ctx.screen_rect().contains_rect(*r)), "menu panel outside actual screen: {:?}", self.rects);
    }
}

fn actual_feature_menu() -> Vec<MenuEntry> {
    let dir = tempfile::tempdir().expect("temporary survey");
    let file = dir.path().join("demo.gpkg");
    std::fs::copy(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg"), &file).expect("tracked fixture copy");
    let mut app = crate::app_state::StrataAppState::new_startup();
    assert!(app.open_survey_gpkg(&file));
    layer_menu_entries(&app.layer_menu_input("feature_poly").expect("actual feature layer"))
}

#[test]
fn context_menu_bounds_scrolls_to_the_real_last_action_on_a_small_screen() {
    let mut h = Harness::new(actual_feature_menu(), [480, 320], Pos2::new(450.0, 290.0));
    h.save("context-menu-small-first");
    h.assert_panels_fit();
    h.click("레이어 불러오기 (SHP·DXF·GPKG)");
    assert_eq!(h.selected, None, "disabled import action activated");
    let anchor = h.point_at("이름 바꾸기 (두 번 클릭)");
    let mut reached = false;
    for _ in 0..30 {
        let output =
            h.frame(vec![Event::PointerMoved(anchor), Event::MouseWheel { unit: egui::MouseWheelUnit::Point, delta: Vec2::new(0.0, -160.0), modifiers: Default::default() }]);
        if h.visible_text(&output, "레이어 삭제").is_some() {
            reached = true;
            break;
        }
    }
    h.save("context-menu-small-last");
    assert!(reached, "last actual layer action cannot be reached by scrolling");
    assert_eq!(h.selected, None, "scrolling invoked an action");
    h.click("레이어 삭제");
    assert_eq!(h.selected, Some(MenuPick { id: "layer.remove", value: String::new() }));
}

#[test]
fn context_menu_bounds_child_opens_to_the_available_side_and_returns_exact_value() {
    let mut h = Harness::new(actual_feature_menu(), [640, 900], Pos2::new(620.0, 650.0));
    h.hover("투명도");
    h.save("context-menu-nested-edge");
    h.assert_panels_fit();
    assert_eq!(h.rects.len(), 2);
    // The renderer shares a 2-point border; the submenu must not cover parent actions when the left side fits.
    assert!(h.rects[1].right() <= h.rects[0].left() + 2.0, "child covered parent actions despite space on the left: {:?}", h.rects);
    assert_eq!(h.selected, None, "hovering invoked an action");
    h.click("80% 투명하게");
    assert_eq!(h.selected, Some(MenuPick { id: "layer.opacityValue", value: "80".to_owned() }));
}

#[test]
fn context_menu_bounds_diagonal_pointer_keeps_the_actual_child_open() {
    let mut h = Harness::new(actual_feature_menu(), [640, 900], Pos2::new(620.0, 650.0));
    h.hover("투명도");
    let from = h.ctx.input(|i| i.pointer.hover_pos()).expect("actual parent hover");
    let to = h.point_at("80% 투명하게");
    for step in 1..=10 {
        let at = from + (to - from) * (step as f32 / 10.0);
        h.frame(vec![Event::PointerMoved(at)]);
        assert_eq!(h.rects.len(), 2, "child vanished while moving towards it at {at:?}; path={:?}", h.path);
    }
    assert_eq!(h.selected, None, "pointer movement invoked an action");
    h.click("80% 투명하게");
    assert_eq!(h.selected, Some(MenuPick { id: "layer.opacityValue", value: "80".to_owned() }));
}

#[test]
fn context_menu_bounds_long_field_stays_selectable_without_losing_its_value() {
    // Explicit synthetic schema fixture. No user records or files are read by this case.
    let field = "field_name_with_many_characters_".repeat(16);
    let row = MenuRow { id: "layer.labelFieldValue", text: field.clone(), enabled: true, tooltip: String::new(), checked: Some(true), value: field.clone(), submenu: None };
    let mut h = Harness::new(vec![MenuEntry::Row(row)], [480, 320], Pos2::new(450.0, 290.0));
    h.save("context-menu-long-field");
    h.assert_panels_fit();
    h.hover(&field);
    let mut discovered = false;
    for _ in 0..20 {
        let output = h.frame(vec![]);
        discovered = output.shapes.iter().any(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.text() == field && !t.galley.elided => {
                let rect = t.galley.rect.translate(t.pos.to_vec2());
                h.ctx.screen_rect().contains_rect(rect) && s.clip_rect.contains_rect(rect)
            }
            _ => false,
        });
        if discovered {
            break;
        }
    }
    h.save("context-menu-long-field-tooltip");
    assert!(discovered, "the full truncated field cannot be discovered inside the app");
    h.click(&field);
    assert_eq!(h.selected, Some(MenuPick { id: "layer.labelFieldValue", value: field }));
}
