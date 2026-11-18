//! The keys of the main window, in the whole window.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use eframe::egui::{self, vec2, Event, Key, Modifiers, Pos2, RawInput, Rect};

use crate::app_state::{MainTab, StrataAppState};

/// MORE0 M4-39: the arrow keys moved every trench of the survey by 0.1 m (1 m with Shift) whatever tool was
/// on, with no undo. In the C++ they move a trench only in the trench move tool (KaTrenchMoveToolKeys.cpp:
/// 45-72), which this app does not have yet: on the map an arrow key moves no trench and changes nothing.
#[test]
fn an_arrow_key_on_the_map_moves_no_trench() {
    let mut app = StrataAppState::new_startup();
    app.active_tab = MainTab::Map;
    let ring = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 5.0], [0.0, 5.0], [0.0, 0.0]];
    app.home_map.real_survey.trenches.push(crate::real_survey::RealTrenchPoly { name: "Tr-1".to_owned(), ring: ring.clone(), area_m2: 50.0, center: [5.0, 2.5] });
    let ctx = egui::Context::default();
    for (key, modifiers) in [(Key::ArrowRight, Modifiers::default()), (Key::ArrowUp, Modifiers::SHIFT)] {
        let press = Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers };
        let raw = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1680.0, 960.0))), events: vec![press], modifiers, ..Default::default() };
        let _ = ctx.run(raw, |ctx| {
            let _ = app.render_workspace_egui(ctx);
        });
    }
    assert_eq!(app.home_map.real_survey.trenches[0].ring, ring);
    assert!(!app.is_dirty);
}

fn press(
    app: &mut StrataAppState,
    ctx: &egui::Context,
    key: Key,
    modifiers: Modifiers,
    fullscreen: Option<bool>,
) -> egui::FullOutput {
    let press = Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    };
    let mut raw = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1680.0, 960.0))),
        events: vec![press],
        modifiers,
        ..Default::default()
    };
    if let Some(on) = fullscreen {
        raw.viewports
            .get_mut(&egui::ViewportId::ROOT)
            .unwrap()
            .fullscreen = Some(on);
    }
    ctx.run(raw, |ctx| {
        let _ = app.render_workspace_egui(ctx);
    })
}

fn fullscreen_values(out: &egui::FullOutput) -> Vec<bool> {
    out.viewport_output
        .get(&egui::ViewportId::ROOT)
        .map(|viewport| {
            viewport
                .commands
                .iter()
                .filter_map(|command| {
                    if let egui::ViewportCommand::Fullscreen(on) = command {
                        Some(*on)
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

fn maximized_values(out: &egui::FullOutput) -> Vec<bool> {
    out.viewport_output
        .get(&egui::ViewportId::ROOT)
        .map(|viewport| {
            viewport
                .commands
                .iter()
                .filter_map(|command| {
                    if let egui::ViewportCommand::Maximized(on) = command {
                        Some(*on)
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

const MAP_FOCUS_ON: &str =
    "지도 넓게 보기 — Ctrl+F11 키를 다시 누르면 리본과 왼쪽 패널이 돌아옵니다.";
const MAP_FOCUS_OFF: &str = "지도 넓게 보기를 끝냈습니다.";

/// `QKeySequence::Find` focuses the address field and selects its text
/// (`MainWindowRibbon.cpp:240-244`, `KaAppBar.cpp:84-87`).
#[test]
fn ctrl_f_focuses_the_search_field_and_selects_its_text() {
    let mut app = StrataAppState::new_startup();
    app.active_tab = MainTab::Map;
    app.ribbon.app_bar.query = "하회종가길 40".to_owned();
    let ctx = egui::Context::default();
    let ctrl = Modifiers {
        ctrl: true,
        command: true,
        ..Modifiers::default()
    };
    press(&mut app, &ctx, Key::F, ctrl, None);
    let id = crate::ribbon_appbar::search_edit_id();
    assert!(
        ctx.memory(|memory| memory.has_focus(id)),
        "Ctrl+F focuses the search field"
    );
    let state = egui::TextEdit::load_state(&ctx, id).expect("search field state");
    let range = state.cursor.char_range().expect("selectAll");
    let (start, end) = (
        range.primary.index.min(range.secondary.index),
        range.primary.index.max(range.secondary.index),
    );
    assert_eq!((start, end), (0, "하회종가길 40".chars().count()));
    assert_eq!(app.ribbon.app_bar.query, "하회종가길 40");
}

/// A text field keeps Ctrl+C and Ctrl+V, and Shift+Ctrl+F is not Find.
#[test]
fn text_fields_keep_copy_paste_and_shift_ctrl_f_is_not_find() {
    let mut app = StrataAppState::new_startup();
    app.active_tab = MainTab::Map;
    app.ribbon.app_bar.query = "하회".to_owned();
    let ctx = egui::Context::default();
    let ctrl = Modifiers {
        ctrl: true,
        command: true,
        ..Modifiers::default()
    };
    press(&mut app, &ctx, Key::C, ctrl, None);
    press(&mut app, &ctx, Key::V, ctrl, None);
    assert_eq!(app.ribbon.app_bar.query, "하회");
    assert!(app.home_map.rename_layer.is_none());
    let shift_ctrl = Modifiers {
        ctrl: true,
        shift: true,
        command: true,
        ..Modifiers::default()
    };
    press(&mut app, &ctx, Key::F, shift_ctrl, None);
    assert!(!ctx.memory(|memory| memory.has_focus(crate::ribbon_appbar::search_edit_id())));
}

/// F11 toggles full screen (`MainWindow.cpp:270-278`). Map focus is Ctrl+F11
/// (`KaShellFocus.h:27`, messages `KaShellFocus.cpp:133-143`) and that sentence is not shown for F11.
#[test]
fn f11_is_full_screen_and_ctrl_f11_is_map_focus() {
    let mut app = StrataAppState::new_startup();
    app.active_tab = MainTab::Map;
    let ctx = egui::Context::default();
    let entered = press(&mut app, &ctx, Key::F11, Modifiers::default(), None);
    assert!(!app.right_inspector.map_focused, "F11 is not map focus");
    assert_eq!(fullscreen_values(&entered), vec![true]);
    assert!(app.home_map.draw_guide_message.as_deref() != Some(MAP_FOCUS_ON));

    let left = press(&mut app, &ctx, Key::F11, Modifiers::default(), Some(true));
    assert_eq!(fullscreen_values(&left), vec![false]);
    assert_eq!(
        maximized_values(&left),
        vec![true],
        "leaving full screen is showMaximized"
    );
    assert!(!app.right_inspector.map_focused);

    let ctrl = Modifiers {
        ctrl: true,
        command: true,
        ..Modifiers::default()
    };
    let widened = press(&mut app, &ctx, Key::F11, ctrl, Some(false));
    assert!(app.right_inspector.map_focused);
    assert_eq!(
        app.home_map.draw_guide_message.as_deref(),
        Some(MAP_FOCUS_ON)
    );
    assert!(fullscreen_values(&widened).is_empty());
    let restored = press(&mut app, &ctx, Key::F11, ctrl, Some(false));
    assert!(!app.right_inspector.map_focused);
    assert_eq!(
        app.home_map.draw_guide_message.as_deref(),
        Some(MAP_FOCUS_OFF)
    );
    assert!(fullscreen_values(&restored).is_empty());

    app.right_inspector.map_focused = false;
    app.home_map.draw_guide_message = None;
    let shift = press(&mut app, &ctx, Key::F11, Modifiers::SHIFT, None);
    assert!(!app.right_inspector.map_focused);
    assert!(fullscreen_values(&shift).is_empty());
    assert!(app.home_map.draw_guide_message.is_none());
}

/// F2 renames the legend row when the list was clicked last (`MainWindow.cpp:2379-2381`,
/// `renameSelectedLayer` at `:1634-1656`). On the map the key does nothing, the same split as Delete.
#[test]
fn f2_renames_the_legend_row_clicked_last() {
    let mut app = StrataAppState::new_startup();
    app.active_tab = MainTab::Map;
    app.left_layers
        .ensure_domain_layer(strata_types::schema::DomainLayerKey::FeaturePoly);
    let label = strata_types::schema::DomainLayerKey::FeaturePoly.name_ko();
    app.right_inspector.selected_layer_key = Some(label.to_owned());
    let ctx = egui::Context::default();
    press(&mut app, &ctx, Key::F2, Modifiers::default(), None);
    assert!(
        app.home_map.rename_layer.is_none(),
        "the map owns the key until the list is clicked"
    );

    app.home_map.legend_focus = true;
    press(&mut app, &ctx, Key::F2, Modifiers::default(), None);
    let rename = app
        .home_map
        .rename_layer
        .as_ref()
        .expect("F2 opens 이름 바꾸기");
    assert_eq!(
        (rename.key.as_str(), rename.text.as_str()),
        ("feature_poly", label)
    );
}

/// A focused text field keeps F2, the same as Delete (`gui_shell_escape.rs` skips the key when
/// `wants_keyboard_input`). The list still has the row.
#[test]
fn f2_does_not_rename_while_a_text_field_has_focus() {
    let mut app = StrataAppState::new_startup();
    app.active_tab = MainTab::Map;
    app.left_layers.ensure_domain_layer(strata_types::schema::DomainLayerKey::FeaturePoly);
    let label = strata_types::schema::DomainLayerKey::FeaturePoly.name_ko();
    app.right_inspector.selected_layer_key = Some(label.to_owned());
    app.home_map.legend_focus = true;
    let ctx = egui::Context::default();
    let ctrl = Modifiers { ctrl: true, command: true, ..Modifiers::default() };
    press(&mut app, &ctx, Key::F, ctrl, None);
    assert!(ctx.memory(|memory| memory.has_focus(crate::ribbon_appbar::search_edit_id())));
    press(&mut app, &ctx, Key::F2, Modifiers::default(), None);
    assert!(app.home_map.rename_layer.is_none(), "the search field owns the key");
    assert_eq!(app.right_inspector.selected_layer_key.as_deref(), Some(label));
}

/// No selected row: the C++ information text (`MainWindow.cpp:1648-1649`).
#[test]
fn f2_with_no_legend_row_asks_for_a_selection() {
    let mut app = StrataAppState::new_startup();
    app.active_tab = MainTab::Map;
    app.home_map.legend_focus = true;
    let ctx = egui::Context::default();
    press(&mut app, &ctx, Key::F2, Modifiers::default(), None);
    assert!(app.home_map.rename_layer.is_none());
    assert_eq!(
        app.home_map.draw_guide_message.as_deref(),
        Some("이름을 바꿀 레이어 또는 그룹을 선택하세요.")
    );
}
