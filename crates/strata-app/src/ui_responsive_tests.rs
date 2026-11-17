//! Full-shell regressions for owner monitor request, target/cpp-ref/src/app/MainWindowRibbon.cpp:248-254.
use super::ui_snapshot_support::render;
use crate::app_state::{MainTab, StrataAppState};

#[test]
fn full_home_keeps_actual_folder_rows_visible_within_each_monitor() {
    let temp = tempfile::tempdir().expect("temporary folder");
    std::fs::write(temp.path().join("monitor-plan.png"), b"actual metadata fixture").expect("file");
    for [width, height] in [[480, 320], [640, 360], [640, 480], [800, 600], [1024, 720], [1280, 720], [1920, 1080]] {
        let mut app = StrataAppState::new_startup();
        app.explorer.navigate(crate::gui_tab_home::explorer::Location::Folder(temp.path().into()));
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while app.explorer.busy() && std::time::Instant::now() < deadline {
            app.explorer.poll();
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        let rendered = render([width, height], |ctx| {
            crate::gui_theme_fonts::apply_theme_tokens(ctx, &app.theme);
            app.render_workspace_egui(ctx);
        });
        rendered.save_if_requested(&format!("full-home-{width}-{height}"));
        let (_, bounds, clip) = rendered.text_clips().into_iter().find(|(text, _, _)| text == "monitor-plan.png").expect("first real file is missing below app chrome");
        assert!(clip.contains_rect(bounds) && bounds.bottom() <= height as f32, "{width}x{height}: file row is outside visible Home: {bounds:?} by{clip:?}");
        for (text, rect) in rendered.text_rects() {
            assert!(rect.left() >= -0.5 && rect.right() <= width as f32 + 0.5, "{width}x{height}: app glyph {text} spills horizontally: {rect:?}");
        }
    }
}

#[test]
fn full_home_at_native_double_dpi_still_keeps_the_actual_rows_visible() {
    let temp = tempfile::tempdir().expect("temporary directory");
    std::fs::write(temp.path().join("double-dpi-file.png"), b"actual metadata").expect("file");
    let mut app = StrataAppState::new_startup();
    app.explorer.navigate(crate::gui_tab_home::explorer::Location::Folder(temp.path().into()));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while app.explorer.busy() && std::time::Instant::now() < deadline {
        app.explorer.poll();
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    let rendered = super::ui_snapshot_support::render_at_dpi([640, 360], 2.0, |ctx| {
        crate::gui_theme_fonts::apply_theme_tokens(ctx, &app.theme);
        app.render_workspace_egui(ctx);
    });
    rendered.save_if_requested("full-home-1280-720-200percent");
    let (_, r, c) = rendered.text_clips().into_iter().find(|(text, _, _)| text == "double-dpi-file.png").expect("actual file");
    assert!(c.contains_rect(r) && r.bottom() <= 360.0, "native200percent hid the actual file: {r:?} by{c:?}");
}

#[test]
fn all_open_studio_tabs_and_long_survey_name_do_not_overlap() {
    let temp = tempfile::tempdir().expect("temporary title");
    let survey = temp.path().join("a-long-survey-title-that-must-not-cover-studio-tabs.gpkg");
    std::fs::copy(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg"), &survey).expect("actual survey copy");
    let mut app = StrataAppState::new_startup();
    app.survey_path = Some(survey);
    app.open_studio_tabs = vec![MainTab::Drawing, MainTab::Print, MainTab::Paleo, MainTab::Terrain3d];
    let rendered = render([640, 90], |ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| app.paint_ribbon_tab_bar(ctx, ui));
    });
    rendered.save_if_requested("all-studio-tabs-640");
    let glyphs = rendered.text_rects();
    for (i, (a, ar)) in glyphs.iter().enumerate() {
        assert!(ar.left() >= -0.5 && ar.right() <= 640.5, "tab/title {a} is outside640: {ar:?}");
        for (b, br) in &glyphs[i + 1..] {
            assert!(!ar.shrink(1.0).intersects(br.shrink(1.0)), "tab/title {a} overlaps {b}");
        }
    }
}
