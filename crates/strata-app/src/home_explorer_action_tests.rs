//! Explorer actions against actual files, target/cpp-ref/src/app/KaFileBrowserPanel.cpp:142-151, MainWindow.cpp:902-914.
use super::*;
use crate::app_state::{MainTab, StrataAppState};
use eframe::egui::{self, Event, Pos2, RawInput, Rect, Vec2};

fn settle(state: &mut ExplorerState) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while state.busy() && std::time::Instant::now() < deadline {
        state.poll();
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    assert!(!state.busy());
}
#[test]
fn changing_sort_keeps_the_active_search_and_selected_file() {
    let temp = tempfile::tempdir().expect("temporary content");
    let file = temp.path().join("plan.png");
    std::fs::write(&file, "real").expect("file");
    let mut state = ExplorerState::default();
    state.navigate(Location::Folder(temp.path().into()));
    settle(&mut state);
    state.set_filter("plan".to_owned());
    state.selected = Some(file.clone());
    state.set_sort(Sort::Modified, true);
    settle(&mut state);
    assert_eq!(state.filter, "plan", "sorting discarded the active search");
    assert_eq!(state.selected, Some(file), "sorting discarded the selected file");
}
#[test]
fn an_unsupported_file_reports_its_actual_error_in_the_explorer_footer() {
    let temp = tempfile::tempdir().expect("temporary file");
    let file = temp.path().join("readme.txt");
    std::fs::write(&file, "real").expect("file");
    let mut app = StrataAppState::new_startup();
    app.explorer.navigate(Location::Folder(temp.path().into()));
    settle(&mut app.explorer);
    app.explorer.selected = Some(file);
    app.activate_explorer_selection();
    assert_eq!(app.active_tab, MainTab::Home);
    let rendered = crate::test_mods::ui_snapshot_support::render([900, 600], |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| app.render_home_tab_ui(ui));
    });
    assert!(rendered.texts().iter().any(|s| s.contains("선택한 파일을 지도 레이어로 열지 못했습니다")), "the file open error is hidden on the home tab");
}
#[test]
fn choosing_a_recent_survey_preserves_the_unsaved_work_confirmation() {
    let mut app = StrataAppState::new_startup();
    app.is_dirty = true;
    app.explorer.selected = Some("recorded-choice.gpkg".into());
    app.activate_explorer_selection();
    assert!(app.save_before.is_some());
    assert!(app.survey_path.is_none());
    assert!(app.is_dirty);
}

#[test]
fn actual_row_selection_and_open_button_load_the_real_picture_without_changing_it() {
    let temp = tempfile::tempdir().expect("temporary file");
    let file = temp.path().join("plan.png");
    image::save_buffer(&file, &[20; 32 * 16 * 3], 32, 16, image::ColorType::Rgb8).expect("actual image");
    let original = std::fs::read(&file).expect("original bytes");
    let mut app = StrataAppState::new_startup();
    app.explorer.navigate(Location::Folder(temp.path().into()));
    settle(&mut app.explorer);
    let ctx = egui::Context::default();
    crate::gui_theme_fonts::configure_korean_fonts_and_theme(&ctx, &Default::default());
    let mut time = 0.0;
    let mut frame = |app: &mut StrataAppState, events: Vec<Event>| {
        time += 0.05;
        ctx.run(RawInput { time: Some(time), screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(900.0, 600.0))), events, ..Default::default() }, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| app.render_home_tab_ui(ui));
        })
    };
    let position = |out: &egui::FullOutput, title: &str| {
        out.shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(t) if t.galley.text() == title => Some(t.pos + Vec2::splat(5.0)),
                _ => None,
            })
            .max_by(|a, b| a.y.total_cmp(&b.y))
            .expect("actual row/button glyph")
    };
    for _ in 0..3 {
        frame(&mut app, vec![]);
    }
    let at = position(&frame(&mut app, vec![]), "plan.png");
    let pointer = |pos, pressed| Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() };
    frame(&mut app, vec![Event::PointerMoved(at), pointer(at, true)]);
    frame(&mut app, vec![pointer(at, false)]);
    assert_eq!(app.explorer.selected, Some(file.clone()));
    assert!(app.pictures.layers.is_empty(), "single click opened a file");
    let button = position(&frame(&mut app, vec![]), "열기");
    frame(&mut app, vec![Event::PointerMoved(button), pointer(button, true)]);
    frame(&mut app, vec![pointer(button, false)]);
    assert_eq!(app.active_tab, MainTab::Map);
    assert_eq!(app.pictures.layers.len(), 1);
    assert_eq!(app.pictures.layers[0].file, file);
    assert_eq!(std::fs::read(file).expect("unchanged source"), original);
}
