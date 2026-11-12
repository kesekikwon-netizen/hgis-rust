//! 실제 우클릭 경로(화면 없는 egui) — 지도 칸에서 오른쪽 단추를 눌렀다 떼면 메뉴가 열리고, 우클릭 자리는
//! 상태줄의 마우스 지도 좌표와 같고, 다음 프레임에도 열려 있고, 흐린 줄을 눌러도 닫히지 않고(QMenu 처럼),
//! 바깥을 누르면 닫힌다. 위성 타일은 끄고 돌린다(시험이 타일을 내려받지 않게).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use eframe::egui::{self, Event, Modifiers, PointerButton, Pos2, RawInput, Rect};

use crate::app_state::{MainTab, StrataAppState};

fn frame(app: &mut StrataAppState, ctx: &egui::Context, events: Vec<Event>) {
    let raw = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1680.0, 960.0))),
        events,
        ..Default::default()
    };
    let _ = ctx.run(raw, |ctx| {
        let _ = app.render_workspace_egui(ctx);
    });
}

fn button(pos: Pos2, button: PointerButton, pressed: bool) -> Event {
    Event::PointerButton { pos, button, pressed, modifiers: Modifiers::default() }
}

fn click(app: &mut StrataAppState, ctx: &egui::Context, at: Pos2, which: PointerButton) {
    frame(app, ctx, vec![Event::PointerMoved(at)]);
    frame(app, ctx, vec![button(at, which, true)]);
    frame(app, ctx, vec![button(at, which, false)]);
}

#[test]
fn a_real_right_click_opens_and_closes_like_qmenu() {
    let mut app = StrataAppState::new_startup();
    app.active_tab = MainTab::Map;
    app.home_map.real_survey.layer_visible.insert("위성지도 (VWorld)".to_owned(), false);
    let ctx = egui::Context::default();
    let at = Pos2::new(900.0, 500.0);
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![Event::PointerMoved(at)]);
    let hover = app.home_map.cursor_world_xy;

    click(&mut app, &ctx, at, PointerButton::Secondary);
    assert!(app.map_context_menu_open, "right-click on the map opens the menu");
    assert_eq!(app.map_context_menu_world, hover, "the click point is the status-bar coordinate");

    frame(&mut app, &ctx, vec![]);
    assert!(app.map_context_menu_open, "stays open on the next frame");

    // 조사가 없으면 첫 줄은 흐린 「이전 지도 범위」 — 눌러도 닫히지 않는다.
    click(&mut app, &ctx, Pos2::new(at.x + 40.0, at.y + 16.0), PointerButton::Primary);
    assert!(app.map_context_menu_open, "a grey row keeps the menu open");

    click(&mut app, &ctx, Pos2::new(at.x - 300.0, at.y + 200.0), PointerButton::Primary);
    assert!(!app.map_context_menu_open, "a click outside closes the menu");
}
