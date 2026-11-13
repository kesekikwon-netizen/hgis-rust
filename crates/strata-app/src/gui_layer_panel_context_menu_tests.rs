//! 레이어 목록 우클릭 메뉴 — 닫기·창 뒤 막기·그 레이어에 하는 동작. demo 임시 사본.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use eframe::egui::{self, Event, Key, Modifiers, Pos2, RawInput, Rect};

use crate::app_state::{MainTab, StrataAppState};
use crate::menu_popup::MenuPick;

fn opened_demo(dir: &Path) -> StrataAppState {
    let path = dir.join("demo.gpkg");
    std::fs::copy(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg"), &path).expect("copy");
    let mut app = StrataAppState::new_startup();
    app.open_survey_gpkg(&path);
    app.active_tab = MainTab::Map;
    app
}

fn frame(app: &mut StrataAppState, ctx: &egui::Context, events: Vec<Event>) {
    let raw = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1680.0, 960.0))), events, ..Default::default() };
    let _ = ctx.run(raw, |ctx| {
        let _ = app.render_workspace_egui(ctx);
    });
}

fn open_menu(app: &mut StrataAppState, key: &str) {
    app.layer_context_menu_key = Some(key.to_owned());
    app.layer_context_menu_pos = [180.0, 260.0];
    app.layer_context_menu_open = true;
}

/// Esc 는 메뉴를 닫는다. 창(모달)이 열려 있으면 메뉴는 열려 있지 않는다.
#[test]
fn escape_closes_and_a_modal_window_keeps_it_shut() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut app = opened_demo(dir.path());
    let ctx = egui::Context::default();
    open_menu(&mut app, "feature_poly");
    frame(&mut app, &ctx, vec![]);
    assert!(app.layer_context_menu_open);
    for pressed in [true, false] {
        frame(&mut app, &ctx, vec![Event::Key { key: Key::Escape, physical_key: None, pressed, repeat: false, modifiers: Modifiers::default() }]);
    }
    assert!(!app.layer_context_menu_open);
    app.open_feature_record("feature_poly", 1);
    open_menu(&mut app, "feature_poly");
    frame(&mut app, &ctx, vec![]);
    assert!(!app.layer_context_menu_open);
}

/// 동작은 고른 레이어에(먼저 현재 레이어로): 라벨 숨기기, 그리기·편집 시작.
#[test]
fn actions_apply_to_the_clicked_layer() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut app = opened_demo(dir.path());
    let label = app.left_layers.entries.iter().find(|e| e.layer_key == "feature_poly").unwrap().label_ko.clone();
    assert!(app.home_map.real_survey.is_label_visible(&label));
    app.run_layer_menu_action(Some("feature_poly"), &MenuPick { id: "layer.labelsVisible", value: String::new() });
    assert!(!app.home_map.real_survey.is_label_visible(&label));
    assert_eq!(app.right_inspector.selected_layer_key.as_deref(), Some(label.as_str()));
    app.run_layer_menu_action(Some("feature_poly"), &MenuPick { id: "layer.draw", value: String::new() });
    assert_eq!(app.edit_layer_key.as_deref(), Some("feature_poly"));
}

/// 두 번 클릭 = 그 자리에서 이름 고치기(m_layerTree->edit): 적고 Enter 면 바뀌고, Esc 면 그대로.
#[test]
fn inline_rename_on_the_row() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut app = opened_demo(dir.path());
    let ctx = egui::Context::default();
    let key = |pressed, k| Event::Key { key: k, physical_key: None, pressed, repeat: false, modifiers: Modifiers::default() };
    let name = |app: &StrataAppState| app.left_layers.entries.iter().find(|e| e.layer_key == "feature_poly").unwrap().label_ko.clone();
    let old = name(&app);
    app.start_inline_rename("feature_poly");
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![key(true, Key::A), key(false, Key::A)]);
    frame(&mut app, &ctx, vec![Event::Text(" 북".to_owned())]);
    frame(&mut app, &ctx, vec![key(true, Key::Enter)]);
    frame(&mut app, &ctx, vec![key(false, Key::Enter)]);
    assert!(app.home_map.inline_rename.is_none());
    assert_eq!(name(&app), format!("{old} 북"));
    app.start_inline_rename("feature_poly");
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![Event::Text("x".to_owned())]);
    frame(&mut app, &ctx, vec![key(true, Key::Escape)]);
    assert!(app.home_map.inline_rename.is_none());
    assert_eq!(name(&app), format!("{old} 북"), "Esc keeps the name");
}
