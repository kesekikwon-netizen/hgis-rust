//! 레이어 삭제·Ctrl+Z — 목록·지도에서만, 저장 안 한 편집은 되돌리기 단계 안에, 그동안 저장하지 않음. demo 임시 사본.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use eframe::egui;
use strata_types::schema::LegendGroup;

use crate::app_state::{MainTab, StrataAppState};

fn opened_demo(dir: &Path) -> (StrataAppState, std::path::PathBuf) {
    let path = dir.join("demo.gpkg");
    std::fs::copy(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg"), &path).expect("copy");
    let mut app = StrataAppState::new_startup();
    app.open_survey_gpkg(&path);
    app.active_tab = MainTab::Map;
    app.home_map.viewport.meters_per_pixel = 0.05;
    (app, path)
}

fn draw_pit(app: &mut StrataAppState) {
    let to_work = strata_engine::Transformer::new(5186, 5187).expect("PROJ");
    app.start_edit_feature_poly();
    for p in [[200_080.0, 450_060.0], [200_090.0, 450_060.0], [200_090.0, 450_070.0]] {
        app.sketch_press(to_work.transform(p).expect("transform"));
    }
    app.sketch_finish();
    app.skip_feature_form();
}

fn rows(path: &Path) -> i64 {
    rusqlite::Connection::open(path).unwrap().query_row("SELECT count(*) FROM feature_poly", [], |r| r.get(0)).unwrap()
}

/// 빼면 목록·조사 표에서 없어지고(편집 레이어·카드도), C++ 상태 글. Ctrl+Z 면 같은 자리에 저장 안 한 도형까지 돌아온다.
#[test]
fn remove_and_undo_keep_the_unsaved_shape() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut app, _) = opened_demo(dir.path());
    draw_pit(&mut app);
    let index = app.left_layers.entries.iter().position(|e| e.layer_key == "feature_poly").unwrap();
    let count = app.survey_tables.iter().find(|t| t.table == "feature_poly").unwrap().layer.features.len();
    assert!(app.remove_layers(&["feature_poly".to_owned()]));
    assert!(!app.left_layers.entries.iter().any(|e| e.layer_key == "feature_poly"));
    assert!(!app.survey_tables.iter().any(|t| t.table == "feature_poly"));
    assert_eq!(app.edit_layer_key, None);
    assert!(app.home_map.card.is_none());
    assert_eq!(app.status_left_message(), "레이어를 목록에서 제거했습니다. Ctrl+Z로 복원할 수 있습니다. 원본 파일은 그대로입니다.");
    app.undo_map_action();
    assert_eq!(app.left_layers.entries.iter().position(|e| e.layer_key == "feature_poly"), Some(index));
    assert_eq!(app.survey_tables.iter().find(|t| t.table == "feature_poly").unwrap().layer.features.len(), count);
}

/// 빼 둔 동안 「저장」은 그 레이어를 쓰지 않고 다시 읽어도 돌아오지 않는다. 되돌린 뒤 저장하면 그 도형이 써진다.
#[test]
fn a_removed_layer_is_not_saved_until_restored() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut app, path) = opened_demo(dir.path());
    let before = rows(&path);
    draw_pit(&mut app);
    app.remove_layers(&["feature_poly".to_owned()]);
    assert!(app.save_survey_project());
    assert_eq!(rows(&path), before, "the removed layer's unsaved shape is not written");
    assert!(!app.survey_tables.iter().any(|t| t.table == "feature_poly"), "re-read does not bring it back");
    app.undo_map_action();
    assert!(app.save_survey_project());
    assert_eq!(rows(&path), before + 1);
}

/// 묶음 머리 「레이어 삭제」는 그 묶음의 레이어 모두. 빼 둔 레이어는 검수에 세지 않는다.
#[test]
fn group_removal_and_checklist() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut app, _) = opened_demo(dir.path());
    let polys = app.checklist_state()["feature_poly_count"].as_i64().unwrap_or(0);
    assert!(polys > 0);
    app.remove_layers(&["feature_poly".to_owned()]);
    assert_eq!(app.checklist_state()["feature_poly_count"].as_i64(), Some(0));
    app.undo_map_action();
    assert!(app.remove_group_layers(strata_types::schema::LegendGroup::SurveyData));
    assert!(!app.left_layers.entries.iter().any(|e| e.group == strata_types::schema::LegendGroup::SurveyData));
    assert!(!app.remove_layers(&[]));
    assert_eq!(app.status_left_message(), "제거할 레이어를 먼저 클릭하세요.");
}

fn pits(app: &StrataAppState) -> usize {
    app.survey_tables.iter().find(|t| t.table == "feature_poly").map_or(0, |t| t.layer.features.len())
}

fn select_row(app: &mut StrataAppState, key: &str) {
    app.right_inspector.selected_layer_key = app.left_layers.entries.iter().find(|e| e.layer_key == key).map(|e| e.label_ko.clone());
}

fn has_row(app: &StrataAppState, key: &str) -> bool {
    app.left_layers.entries.iter().any(|e| e.layer_key == key)
}

/// A frame of the whole window in which `key` goes down.
fn press_key(app: &mut StrataAppState, ctx: &egui::Context, key: egui::Key) {
    let event = egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::default() };
    let raw = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1680.0, 960.0))),
        events: vec![event],
        ..Default::default()
    };
    let _ = ctx.run(raw, |ctx| {
        let _ = app.render_workspace_egui(ctx);
    });
}

/// Delete on the map (`MainWindow.cpp:256-268`, `MainWindowUndo.cpp:505-511`, `:538-575`): the selected shape
/// of a survey layer is deleted as one step, with the C++ words. Ctrl+Z brings it back, Ctrl+Y deletes it
/// again, and 저장 takes it out of the file.
#[test]
fn delete_removes_the_selected_shape_and_ctrl_z_brings_it_back() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, path) = opened_demo(dir.path());
    let (saved, before) = (rows(&path), pits(&app));
    let fid = app.survey_tables.iter().find(|t| t.table == "feature_poly").unwrap().layer.features[0].fid;
    app.home_map.selected_shape = Some(("feature_poly".to_owned(), fid));
    select_row(&mut app, "feature_poly");

    app.delete_selected();
    assert_eq!(pits(&app), before - 1);
    assert!(app.home_map.selected_shape.is_none() && app.is_dirty);
    assert!(has_row(&app, "feature_poly"), "the layer of the shape stays");
    assert_eq!(app.status_left_message(), "도형 1개를 지웠습니다. Ctrl+Z로 복원할 수 있습니다.");

    app.undo_map_action();
    assert_eq!(pits(&app), before);
    app.redo_map_action();
    assert_eq!(pits(&app), before - 1);
    assert!(app.save_survey_project());
    assert_eq!(rows(&path), saved - 1);
}

/// In the legend (a row was clicked last) Delete takes the selected row off the list, as 「레이어 삭제」 does
/// (`MainWindow.cpp:264-265`), whatever the layer is; Ctrl+Z puts it back. Owner 2026-11-15: 「도형선택
/// 레이어선택 어떤것이든 delete키 삭제가능하게 컨트롤 z 전단계」.
#[test]
fn delete_in_the_legend_takes_the_selected_row_off() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, _) = opened_demo(dir.path());
    select_row(&mut app, "feature_poly");
    app.home_map.legend_focus = true;
    app.delete_selected();
    assert!(!has_row(&app, "feature_poly"));
    assert_eq!(app.status_left_message(), "레이어를 목록에서 제거했습니다. Ctrl+Z로 복원할 수 있습니다. 원본 파일은 그대로입니다.");
    app.undo_map_action();
    assert!(has_row(&app, "feature_poly"));
}

/// On the map Delete never takes a survey layer off the list: with no shape selected nothing happens to the
/// selected survey row (`MainWindowUndo.cpp:512-536`). A selected reference map does go (`:516-523`).
#[test]
fn delete_on_the_map_keeps_a_survey_row_and_takes_a_reference_map_off() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, _) = opened_demo(dir.path());
    let mut reference = app.left_layers.entries[0].clone();
    (reference.layer_key, reference.label_ko, reference.group) = ("ref_test".to_owned(), "시험 참조 지도".to_owned(), LegendGroup::ReferenceMap);
    app.left_layers.entries.push(reference);
    let rows = app.left_layers.entries.len();

    select_row(&mut app, "feature_poly");
    app.delete_selected();
    assert_eq!(app.left_layers.entries.len(), rows, "a survey row stays");
    assert!(!app.is_dirty);

    select_row(&mut app, "ref_test");
    app.delete_selected();
    assert!(!has_row(&app, "ref_test"));
    app.undo_map_action();
    assert!(has_row(&app, "ref_test"));
}

/// The key itself, through the window: Delete deletes the selected shape, Backspace does not
/// (`QKeySequence::Delete`), and on the sheet the key is not the map's (`MainWindow.cpp:1197-1203`).
#[test]
fn the_delete_key_reaches_the_selected_shape_only_on_the_map() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, _) = opened_demo(dir.path());
    let ctx = egui::Context::default();
    let before = pits(&app);
    let fid = app.survey_tables.iter().find(|t| t.table == "feature_poly").unwrap().layer.features[0].fid;
    let shape = Some(("feature_poly".to_owned(), fid));

    app.home_map.selected_shape = shape.clone();
    press_key(&mut app, &ctx, egui::Key::Backspace);
    assert_eq!(pits(&app), before, "Backspace is not Delete");

    app.active_tab = MainTab::Drawing;
    press_key(&mut app, &ctx, egui::Key::Delete);
    assert_eq!(pits(&app), before, "on the sheet the key leaves the map alone");

    app.active_tab = MainTab::Map;
    app.home_map.selected_shape = shape;
    press_key(&mut app, &ctx, egui::Key::Delete);
    assert_eq!(pits(&app), before - 1);
}

/// Whose the key is (Qt: which widget has the keyboard focus): a click on a legend row makes Delete the
/// legend's, a press on the map gives it back to the map.
#[test]
fn a_clicked_row_takes_the_delete_key_and_a_press_on_the_map_takes_it_back() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, _) = opened_demo(dir.path());
    let ctx = egui::Context::default();
    let mut time = 0.0;
    let mut press = |app: &mut StrataAppState, at: egui::Pos2| {
        time += 0.5;
        let button = |pressed| egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::default() };
        for event in [egui::Event::PointerMoved(at), button(true), button(false)] {
            time += 0.05;
            let raw = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1680.0, 960.0))),
                time: Some(time),
                events: vec![event],
                ..Default::default()
            };
            let _ = ctx.run(raw, |ctx| {
                let _ = app.render_workspace_egui(ctx);
            });
        }
    };
    press(&mut app, egui::pos2(900.0, 500.0));
    app.right_inspector.selected_layer_key = None;
    assert!(!app.home_map.legend_focus);

    // Down the legend until a row answers: the rows are where the panel lays them out.
    let row = (0..100u8).map(|step| 120.0 + 8.0 * f32::from(step)).find(|y| {
        press(&mut app, egui::pos2(120.0, *y));
        app.right_inspector.selected_layer_key.is_some()
    });
    assert!(row.is_some(), "no row of the legend answered a click");
    assert!(app.home_map.legend_focus);

    press(&mut app, egui::pos2(900.0, 500.0));
    assert!(!app.home_map.legend_focus);
}
