//! Owner UI regressions (target/cpp-ref/src/app/MainWindowRibbon.cpp:486-500).
use super::ui_snapshot_support::render;
use crate::app_state::{MainTab, StrataAppState};
use eframe::egui;

#[test]
fn old_map_menu_opens_both_choices_without_selecting_a_map() {
    let mut app = StrataAppState::new_startup();
    app.handle_ribbon_click("옛 지도");
    assert!(app.ribbon.old_map_menu_open, "top old-map button must open choices");
    assert!(app.active_tab == MainTab::Home, "opening choices must keep the current tab");
    assert!(!app.old_map_shown(), "opening choices must not add a map");
    assert!(app.home_map.draw_guide_message.is_none(), "selection has not been requested");
    let rendered = render([420, 180], |ctx| app.render_ribbon_context_menus(ctx));
    let text = rendered.texts();
    assert!(text.iter().any(|line| line.contains("대동여지도")), "first map choice missing");
    assert!(text.iter().any(|line| line.contains("1919 조선지형도 1:5만")), "second map choice missing");
    rendered.save_if_requested("old-map-menu");
}

#[test]
fn inspector_keeps_its_card_without_the_duplicate_basemap_strip() {
    let mut app = StrataAppState::new_startup();
    let rendered = render([280, 720], |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| app.render_ri_body(ui));
    });
    let painted = rendered.texts();
    assert!(painted.iter().any(|text| text.contains("선택한 유구")), "actual inspector card missing: {painted:?}");
    for obsolete in ["위성", "지형", "옛 지도 ▼", "배경 지도"] {
        assert!(!painted.iter().any(|text| text == obsolete), "duplicate basemap control remains: {obsolete}");
    }
    rendered.save_if_requested("map-inspector");
}

#[test]
fn print_button_opens_one_application_tab_without_a_modal() {
    let mut app = StrataAppState::new_startup();
    app.handle_ribbon_click("인쇄");
    app.handle_ribbon_click("인쇄");
    assert_eq!(app.active_tab.label_ko(), "인쇄", "print must be its own app tab");
    assert!(app.modals.active.is_none(), "print still opens a modal");
    assert_eq!(app.open_studio_tabs.iter().filter(|tab| tab.label_ko() == "인쇄").count(), 1);
}

#[test]
fn print_preview_draws_the_current_sheet_instead_of_fabricated_geometry() {
    let mut app = StrataAppState::new_startup();
    app.drawing.sheet.legend_title = "REAL-SHEET-PRINT-301".to_owned();
    let rendered = render([1200, 900], |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| crate::dlg_drawing_print::render_drawing_print(&mut app, ui));
    });
    let texts = rendered.texts();
    assert!(texts.iter().any(|text| text == "REAL-SHEET-PRINT-301"), "preview does not use the actual sheet: {texts:?}");
    assert!(!texts.iter().any(|text| text.contains("ezViewX") || text.contains("6532")), "fabricated printer or scale remains");
    rendered.save_if_requested("print-tab");
}

#[test]
fn print_tab_controls_fit_narrow_and_wide_screens() {
    for [width, height] in [[640, 480], [900, 600], [1920, 1080]] {
        let mut app = StrataAppState::new_startup();
        let rendered = render([width, height], |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| crate::dlg_drawing_print::render_drawing_print(&mut app, ui));
        });
        for (text, rect) in rendered.text_rects() {
            assert!(rect.left() >= -0.5 && rect.right() <= width as f32 + 0.5, "{width}: {text} spills horizontally: {rect:?}");
        }
        rendered.save_if_requested(&format!("print-tab-{width}"));
    }
}
