//! Full app actions, target/cpp-ref/src/app/KaDrawingStudio.cpp:929-932 and MainWindowRibbon.cpp:486-500.
use super::ui_snapshot_support::render;
use crate::app_state::{MainTab, StrataAppState};
use eframe::egui::{self, Event, Pos2, RawInput, Rect, Vec2};

#[test]
fn small_map_keeps_a_usable_canvas_and_access_to_layers_and_properties() {
    for [width, height] in [[480, 320], [640, 480], [900, 600]] {
        let mut app = StrataAppState::new_startup();
        app.active_tab = MainTab::Map;
        let rendered = render([width, height], |ctx| {
            crate::gui_theme_fonts::apply_theme_tokens(ctx, &app.theme);
            app.render_workspace_egui(ctx);
        });
        rendered.save_if_requested(&format!("full-map-{width}-{height}"));
        assert!(
            app.home_map.viewport.screen_size_px[0] >= 200.0 && app.home_map.viewport.screen_size_px[1] >= 120.0,
            "{width}x{height}: side panels consumed the map: {:?}",
            app.home_map.viewport.screen_size_px
        );
        for text in ["레이어", "선택한 유구"] {
            assert!(rendered.text_clips().iter().any(|(t, r, c)| t == text && c.contains_rect(*r) && r.right() <= width as f32), "{width}: missing reachable {text}");
        }
    }
}

#[test]
fn narrow_map_status_message_does_not_overlap_scale_and_crs_controls() {
    let mut app = StrataAppState::new_startup();
    app.active_tab = MainTab::Map;
    app.show_status_message("도면을 옮기지 못했습니다 — 맞출 도면이 없습니다");
    let rendered = render([480, 320], |ctx| {
        crate::gui_theme_fonts::apply_theme_tokens(ctx, &app.theme);
        app.render_workspace_egui(ctx);
    });
    let glyphs = rendered.text_rects().into_iter().filter(|(_, r)| r.center().y >= 320.0 - crate::gui_shell::UX_STATUS_BAR_HEIGHT).collect::<Vec<_>>();
    for (i, (a, ar)) in glyphs.iter().enumerate() {
        for (b, br) in &glyphs[i + 1..] {
            assert!(!ar.shrink(1.0).intersects(br.shrink(1.0)), "status message {a} {ar:?} overlaps control {b} {br:?}");
        }
    }
}

#[test]
fn drawing_export_and_print_remain_visible_without_scrolling_the_properties() {
    for [width, height] in [[640, 480], [900, 600], [1920, 1080]] {
        let mut app = StrataAppState::new_startup();
        app.active_tab = MainTab::Drawing;
        app.open_studio_tabs.push(MainTab::Drawing);
        let rendered = render([width, height], |ctx| {
            crate::gui_theme_fonts::apply_theme_tokens(ctx, &app.theme);
            app.render_workspace_egui(ctx);
        });
        rendered.save_if_requested(&format!("full-drawing-{width}-{height}"));
        for label in ["PDF 내보내기", "인쇄"] {
            assert!(
                rendered.text_clips().iter().any(|(text, r, c)| text == label && c.contains_rect(*r) && r.bottom() <= height as f32 && r.right() <= width as f32),
                "{width}x{height}: drawing action {label} is outside the workspace"
            );
        }
    }
}

#[test]
fn full_print_tab_keeps_its_real_sheet_preview_and_actions_on_screen() {
    for [width, height] in [[640, 480], [900, 600], [1920, 1080]] {
        let mut app = StrataAppState::new_startup();
        app.active_tab = MainTab::Print;
        app.open_studio_tabs.push(MainTab::Print);
        app.drawing.sheet.legend_title = "ACTUAL-SHEET-UI6".to_owned();
        let rendered = render([width, height], |ctx| {
            crate::gui_theme_fonts::apply_theme_tokens(ctx, &app.theme);
            app.render_workspace_egui(ctx);
        });
        rendered.save_if_requested(&format!("full-print-{width}-{height}"));
        assert!(
            rendered.text_clips().iter().any(|(text, r, c)| text == "ACTUAL-SHEET-UI6" && r.intersects(*c) && r.bottom() <= height as f32),
            "{width}x{height}: actual sheet fell below print controls"
        );
        for (text, r, c) in rendered.text_clips() {
            if r.intersects(c) {
                assert!(r.right() <= width as f32 + 0.5 && r.left() >= -0.5, "{width}: print control {text} overflows horizontally: {r:?}");
            }
        }
    }
}

#[test]
fn compact_ribbon_routes_the_old_map_button_to_both_real_choices() {
    let mut app = StrataAppState::new_startup();
    let ctx = egui::Context::default();
    crate::gui_theme_fonts::configure_korean_fonts_and_theme(&ctx, &app.theme);
    let mut time = 0.0;
    let mut frame = |app: &mut StrataAppState, events: Vec<Event>| {
        time += 0.05;
        ctx.run(RawInput { time: Some(time), screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(640.0, 480.0))), events, ..Default::default() }, |ctx| {
            app.render_workspace_egui(ctx);
        })
    };
    let point = |out: &egui::FullOutput, title: &str| {
        out.shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.text() == title => Some(t.pos + Vec2::splat(4.0)),
                _ => None,
            })
            .unwrap_or_else(|| panic!("actual compact menu is missing {title}"))
    };
    for _ in 0..3 {
        frame(&mut app, vec![]);
    }
    for title in ["배경 지도", "옛 지도"] {
        let at = point(&frame(&mut app, vec![]), title);
        let press = |pressed| Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() };
        frame(&mut app, vec![Event::PointerMoved(at), press(true)]);
        frame(&mut app, vec![press(false)]);
    }
    let output = frame(&mut app, vec![]);
    for title in ["▤  대동여지도", "▤  1919 조선지형도 1:5만"] {
        point(&output, title);
    }
    assert!(app.ribbon.old_map_menu_open);
    assert!(!app.old_map_shown());
}
