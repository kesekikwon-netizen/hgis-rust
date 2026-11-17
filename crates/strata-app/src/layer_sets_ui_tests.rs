//! Actual set-menu input, target/cpp-ref/src/app/KaDrawingLayerSets.cpp:63-95.
use crate::app_state::StrataAppState;
use eframe::egui::{self, Event, Pos2, RawInput, Rect, Vec2};

struct Harness {
    app: StrataAppState,
    ctx: egui::Context,
    time: f64,
    size: [u32; 2],
    textures: crate::gui_tab_drawing::sheet_raster::Textures,
}
impl Harness {
    fn new() -> Self {
        Self::sized([640, 480])
    }
    fn sized(size: [u32; 2]) -> Self {
        let mut app = StrataAppState::new_startup();
        app.left_layers.add_satellite_basemap_layer();
        app.left_layers.add_cadastral_wms_layer();
        app.left_layers.entries[1].visible = false;
        app.home_map.real_survey.layer_visible.insert(app.left_layers.entries[1].label_ko.clone(), false);
        let ctx = egui::Context::default();
        crate::gui_theme_fonts::configure_korean_fonts_and_theme(&ctx, &app.theme);
        let mut h = Self { app, ctx, time: 0.0, size, textures: Default::default() };
        for _ in 0..3 {
            h.frame(vec![]);
        }
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
                self.app.handle_cpp_keyboard_shortcuts(ctx);
                egui::CentralPanel::default().show(ctx, |ui| {
                    self.app.render_cpp_left_layer_panel(ui, true);
                });
                self.app.render_layer_set_question(ctx);
            },
        );
        self.textures.apply(&output.textures_delta);
        output
    }
    fn save(&mut self, name: &str) {
        let output = self.frame(vec![]);
        crate::test_mods::ui_snapshot_support::save_frame_if_requested(&self.ctx, &output, &self.textures, self.size, name);
    }
    fn click(&mut self, title: &str) {
        let output = self.frame(vec![]);
        let screen = self.ctx.screen_rect();
        let at = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.text() == title || (title == "세트" && t.galley.text().starts_with("세트 ")) => {
                    let at = t.pos + Vec2::splat(4.0);
                    (screen.contains(at) && s.clip_rect.contains(at)).then_some(at)
                }
                _ => None,
            })
            .unwrap_or_else(|| {
                let texts = output
                    .shapes
                    .iter()
                    .filter_map(|s| match &s.shape {
                        egui::Shape::Text(t) => Some(t.galley.text()),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                panic!("actual action missing: {title}; question={} popup={} texts={texts:?}", self.app.layer_set_question_open(), self.ctx.memory(|m| m.any_popup_open()))
            });
        self.click_at(at);
    }
    fn click_at(&mut self, at: Pos2) {
        assert!(self.ctx.screen_rect().contains(at), "click outside the app");
        let press = |pressed| Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() };
        self.frame(vec![Event::PointerMoved(at), press(true)]);
        self.frame(vec![press(false)]);
        for _ in 0..2 {
            self.frame(vec![]);
        }
    }
    fn key(&mut self, key: egui::Key) {
        self.frame(vec![Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: Default::default() }]);
        self.frame(vec![Event::Key { key, physical_key: None, pressed: false, repeat: false, modifiers: Default::default() }]);
    }
}

#[test]
fn layer_sets_capture_the_actual_checkbox_state_not_a_stale_row_copy() {
    let mut h = Harness::new();
    let name = h.app.left_layers.entries[0].label_ko.clone();
    let output = h.frame(vec![]);
    let at = output
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            // gui_layer_panel_rows.rs:109-126: label x=69 and checkbox x=27 in the same row.
            egui::Shape::Text(t) if t.galley.text() == name => Some(t.pos + Vec2::new(-42.0, 4.0)),
            _ => None,
        })
        .expect("actual row checkbox");
    h.click_at(at);
    assert!(!h.app.home_map.real_survey.is_layer_visible(&name), "actual checkbox did not turn off");
    h.click("세트");
    h.click("지금 켜진 레이어를 세트로 저장…");
    h.click("확인");
    h.click_at(at);
    assert!(h.app.home_map.real_survey.is_layer_visible(&name));
    h.click("세트");
    h.click("적용: 세트 1");
    assert!(!h.app.home_map.real_survey.is_layer_visible(&name), "saved set ignored the actual checkbox");
}

#[test]
fn layer_sets_actual_menu_saves_applies_and_confirms_removal_without_changing_layers() {
    let mut h = Harness::new();
    h.click("세트");
    h.save("layer-set-empty-menu");
    h.click("지금 켜진 레이어를 세트로 저장…");
    h.save("layer-set-name-question");
    h.click("확인");
    assert_eq!(h.app.layer_set_names(), ["세트 1"]);
    assert!(h.app.left_layers.entries[0].visible && !h.app.left_layers.entries[1].visible);
    h.app.left_layers.entries[0].visible = false;
    h.app.left_layers.entries[1].visible = true;
    h.click("세트");
    h.click("적용: 세트 1");
    assert!(h.app.left_layers.entries[0].visible && !h.app.left_layers.entries[1].visible);
    let before = h.app.left_layers.entries.clone();
    h.click("세트");
    h.click("세트 지우기");
    h.save("layer-set-delete-list");
    h.click("세트 1");
    h.save("layer-set-delete-question");
    h.click("아니요");
    assert_eq!(h.app.layer_set_names(), ["세트 1"]);
    h.click("세트");
    h.click("세트 지우기");
    h.click("세트 1");
    h.click("예");
    assert!(h.app.layer_set_names().is_empty());
    assert_eq!(h.app.left_layers.entries, before);
}

#[test]
fn layer_sets_many_saved_choices_can_scroll_to_an_actual_pick_on_a_small_screen() {
    let mut h = Harness::sized([480, 320]);
    for i in 0..100 {
        assert!(h.app.store_layer_set(&format!("set-{i:03}")));
    }
    h.app.left_layers.entries[0].visible = false;
    h.click("세트");
    let initial = h.frame(vec![]);
    let screen = h.ctx.screen_rect();
    let anchor = initial
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.text() == "적용: set-000" => {
                let rect = t.galley.rect.translate(t.pos.to_vec2());
                (screen.contains_rect(rect) && s.clip_rect.contains_rect(rect)).then_some(t.pos + Vec2::splat(4.0))
            }
            _ => None,
        })
        .expect("visible first choice");
    let mut reached = false;
    for _ in 0..30 {
        let output =
            h.frame(vec![Event::PointerMoved(anchor), Event::MouseWheel { unit: egui::MouseWheelUnit::Point, delta: Vec2::new(0.0, -240.0), modifiers: Default::default() }]);
        reached = output.shapes.iter().any(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.text() == "적용: set-099" => {
                let rect = t.galley.rect.translate(t.pos.to_vec2());
                s.clip_rect.contains_rect(rect) && screen.contains_rect(rect)
            }
            _ => false,
        });
        if reached {
            break;
        }
    }
    h.save("layer-set-many-small-screen");
    assert!(reached, "last saved set cannot be reached by scrolling the actual menu");
    assert!(!h.app.left_layers.entries[0].visible, "scrolling applied a set");
    h.click("적용: set-099");
    assert!(h.app.left_layers.entries[0].visible);
}

#[test]
fn layer_sets_escape_cancels_name_and_enter_defaults_to_no_on_duplicate() {
    let mut h = Harness::new();
    h.click("세트");
    h.click("지금 켜진 레이어를 세트로 저장…");
    h.key(egui::Key::F9);
    assert!(!h.app.right_inspector.left_panel_collapsed, "a name question let the main-window shortcut through");
    h.key(egui::Key::Escape);
    assert!(h.app.layer_set_names().is_empty());
    assert!(h.app.store_layer_set("세트 1"));
    h.app.left_layers.entries[0].visible = false;
    h.click("세트");
    h.click("지금 켜진 레이어를 세트로 저장…");
    let modifiers = egui::Modifiers { ctrl: true, command: true, ..Default::default() };
    h.frame(vec![Event::Key { key: egui::Key::A, physical_key: None, pressed: true, repeat: false, modifiers }, Event::Text("세트 1".to_owned())]);
    h.click("확인");
    h.key(egui::Key::Enter);
    assert!(h.app.apply_layer_set("세트 1"));
    assert!(h.app.left_layers.entries[0].visible, "default No replaced the saved set");
}

#[test]
fn layer_sets_button_opens_named_actions_without_fabricated_success() {
    let mut app = StrataAppState::new_startup();
    let ctx = egui::Context::default();
    crate::gui_theme_fonts::configure_korean_fonts_and_theme(&ctx, &app.theme);
    let mut time = 0.0;
    let mut frame = |app: &mut StrataAppState, events| {
        time += 0.05;
        ctx.run(RawInput { time: Some(time), screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(640.0, 480.0))), events, ..Default::default() }, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                app.paint_layer_card_header(ui, true);
            });
        })
    };
    for _ in 0..3 {
        frame(&mut app, vec![]);
    }
    let output = frame(&mut app, vec![]);
    let at = output
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.text().starts_with("세트") => Some(t.pos + Vec2::splat(4.0)),
            _ => None,
        })
        .expect("set button");
    let press = |pressed| Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() };
    frame(&mut app, vec![Event::PointerMoved(at), press(true)]);
    frame(&mut app, vec![press(false)]);
    let output = frame(&mut app, vec![]);
    assert_eq!(app.home_map.draw_guide_message, None, "opening a menu did not apply a layer set");
    let texts = output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Text(t) => Some(t.galley.text()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(texts.contains(&"지금 켜진 레이어를 세트로 저장…"), "actual C++ save action missing: {texts:?}");
    assert!(texts.contains(&"저장한 세트가 없습니다"));
}
