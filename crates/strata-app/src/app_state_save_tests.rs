//! 「저장」 시험 — 리본 저장·Ctrl+S 가 C++ persistSurveyWork(`MainWindowSession.cpp:874-1000`)처럼
//! 조사 파일에 실제로 쓰고, 실패하면 편집·「*」·파일을 그대로 둔다. demo.gpkg 는 임시 사본만.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use eframe::egui;
use strata_storage::edit_buffer::FeatureRecord;

use crate::app_state::StrataAppState;

fn opened_demo(dir: &Path) -> (StrataAppState, PathBuf) {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg");
    let path = dir.join("demo.gpkg");
    std::fs::copy(src, &path).expect("copy demo.gpkg");
    let mut app = StrataAppState::new_startup();
    app.open_survey_gpkg(&path);
    assert_eq!(app.survey_path.as_deref(), Some(path.as_path()));
    (app, path)
}

fn add_poly(app: &mut StrataAppState, wkt: &str) {
    let mut attributes = BTreeMap::new();
    attributes.insert("kind".to_owned(), "수혈".to_owned());
    attributes.insert("period".to_owned(), "삼국".to_owned());
    let poly = app.survey_tables.iter_mut().find(|t| t.table == "feature_poly").expect("feature_poly");
    poly.layer.features.push(FeatureRecord { fid: -1, wkt_geom: wkt.to_owned(), attributes });
    app.is_dirty = true;
}

fn poly_count(path: &Path) -> i64 {
    rusqlite::Connection::open(path)
        .expect("open")
        .query_row("SELECT count(*) FROM feature_poly", [], |r| r.get(0))
        .expect("count")
}

const NEW_POLY: &str = "POLYGON ((200100 600100,200130 600100,200130 600140,200100 600100))";

/// 리본 「저장」: 파일에 새 도형을 쓰고, 「*」를 지우고, C++ 상태줄 문구를 띄우고, 메모리를 저장된
/// 상태(진짜 fid)로 다시 맞춘다.
#[test]
fn ribbon_save_writes_the_added_feature_and_clears_the_star() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut app, path) = opened_demo(dir.path());
    add_poly(&mut app, NEW_POLY);

    app.handle_ribbon_click("저장");

    assert!(!app.is_dirty);
    assert_eq!(app.home_map.draw_guide_message.as_deref(), Some("조사 데이터와 작업 구성을 저장했습니다: demo.gpkg"));
    assert_eq!(poly_count(&path), 3);
    let fids: Vec<i64> = app.survey_tables.iter().find(|t| t.table == "feature_poly").unwrap().layer.features.iter().map(|f| f.fid).collect();
    assert_eq!(fids, vec![1, 2, 3]);
    assert_eq!(app.window_title(), "demo - Strata");
}

/// Ctrl+S 도 같은 저장을 한다(예전에는 쓰지 않고 「저장했습니다」만 띄웠다).
#[test]
fn ctrl_s_runs_the_same_save() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut app, path) = opened_demo(dir.path());
    add_poly(&mut app, NEW_POLY);
    let ctx = egui::Context::default();
    let raw = egui::RawInput {
        events: vec![egui::Event::Key {
            key: egui::Key::S,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::CTRL,
        }],
        modifiers: egui::Modifiers::CTRL,
        ..Default::default()
    };
    let _ = ctx.run(raw, |ctx| app.handle_cpp_keyboard_shortcuts(ctx));
    assert_eq!(poly_count(&path), 3);
    assert!(!app.is_dirty);
}

/// 저장에 실패하면 원본 파일·편집·「*」를 그대로 두고 C++ 처럼 알린다.
#[test]
fn a_failed_save_keeps_the_file_the_edits_and_the_star() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut app, path) = opened_demo(dir.path());
    let before = std::fs::read(&path).expect("read");
    add_poly(&mut app, "POLYGON Z ((0 0 1,1 0 1,1 1 1,0 0 1))");

    app.handle_ribbon_click("저장");

    assert!(app.is_dirty);
    assert_eq!(std::fs::read(&path).expect("read"), before);
    assert_eq!(
        app.home_map.draw_guide_message.as_deref(),
        Some("저장 실패 — 유구 (면)의 편집을 다음 세대 파일에 쓰지 못했습니다. 미저장 편집은 그대로 있습니다. 다시 저장하세요.")
    );
    let poly = app.survey_tables.iter().find(|t| t.table == "feature_poly").unwrap();
    assert_eq!(poly.layer.features.len(), 3, "the unsaved edit is kept");
}

/// 열어 둔 조사 파일이 없어지면(USB 뽑힘·옮김) 「저장」은 그 자리에 아무 파일도 만들지 않고
/// C++ 문구(`SurveyStorage.cpp:875`)로 실패한다. 편집과 「*」는 그대로다(되살리기는 지시 2E).
#[test]
fn saving_a_vanished_survey_file_creates_nothing_and_keeps_edits() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut app, path) = opened_demo(dir.path());
    add_poly(&mut app, NEW_POLY);
    std::fs::remove_file(&path).expect("remove the opened file");

    assert!(!app.save_survey_project());

    assert!(!path.exists(), "nothing may be created at the missing path");
    let left: Vec<String> = std::fs::read_dir(dir.path()).expect("list").map(|e| e.expect("entry").file_name().to_string_lossy().into_owned()).collect();
    assert!(left.iter().all(|n| !n.to_lowercase().contains(".gpkg")), "no survey file may appear: {left:?}");
    assert!(app.is_dirty);
    assert_eq!(
        app.home_map.draw_guide_message.as_deref(),
        Some("저장 실패 — 원래 조사 파일이 없어 새 조사 파일을 만들지 못했습니다. 미저장 편집은 그대로 있습니다. 다시 저장하세요.")
    );
    let poly = app.survey_tables.iter().find(|t| t.table == "feature_poly").unwrap();
    assert!(poly.layer.features.iter().any(|f| f.fid < 0), "the unsaved edit stays in memory");
}

/// 조사카드·레이어 카드의 「저장」은 조사 파일에 쓰지 않으므로 「*」를 지우지 않는다.
#[test]
fn card_save_buttons_do_not_clear_the_unsaved_star() {
    let mut app = StrataAppState::new_startup();
    app.is_dirty = true;
    app.on_survey_card_save_clicked();
    assert!(app.is_dirty, "survey card 저장 must not clear '*'");
    app.on_layer_card_save_clicked();
    assert!(app.is_dirty, "layer card 저장 must not clear '*'");
}
