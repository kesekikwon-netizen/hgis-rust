//! 「열기」 시험 — C++ 앱이 `samples/demo_survey/demo.gpkg` 사본을 열었을 때와 같은 범례·제목·
//! 탭·상태인지(`MainWindowSession.cpp:242-420`, `LayerOps.cpp:1831-1913`). 원본 demo.gpkg 는
//! 임시 폴더 사본만 연다.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use eframe::egui::Color32;
use strata_types::crs::WorkCrs;

use crate::app_state::{MainTab, StrataAppState};
use crate::gui_layer_panel::gui_layer_panel_row_spec::{layer_rows, SwatchKind};

fn demo_copy(dir: &Path, name: &str) -> PathBuf {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg");
    let dst = dir.join(name);
    std::fs::copy(src, &dst).expect("copy demo.gpkg to a temp folder");
    dst
}

/// 범례 「조사 데이터」 묶음의 (이름, 도형 수, 글자 칸) — 위에서 아래로.
fn survey_rows(app: &StrataAppState) -> Vec<(String, usize, Option<&'static str>)> {
    let c = Color32::BLACK;
    let rows = layer_rows(&app.left_layers, c, c, c, c);
    let mut out = Vec::new();
    let mut in_survey = false;
    for r in rows {
        if !r.section.is_empty() {
            in_survey = r.section == "조사 데이터";
            continue;
        }
        if in_survey {
            out.push((r.name.clone(), r.count, r.right_label));
        }
    }
    out
}

fn sha(path: &Path) -> String {
    strata_storage::session_save::sha256_fingerprint(path).expect("hash")
}

/// C++ 실물(demo.gpkg 사본 열기): 「demo - Strata」, 지도 탭, 저장됨, 범례 7행(표 역순·한국어 이름·
/// 도형 수·글자 칸), 조사 폴더에 「지형도」, 파일 바이트 그대로, 작업 좌표계는 바꾸지 않는다.
#[test]
fn open_demo_copy_matches_the_cpp_app() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = demo_copy(dir.path(), "demo.gpkg");
    let before = sha(&path);
    let mut app = StrataAppState::new_startup();
    assert_eq!(app.work_crs, WorkCrs::Epsg5187);

    app.open_survey_gpkg(&path);

    assert_eq!(app.window_title(), "demo - Strata");
    assert_eq!(app.active_tab, MainTab::Map);
    assert!(!app.is_dirty);
    assert_eq!(app.survey_path.as_deref(), Some(path.as_path()));
    assert!(app.home_map.recent_files.contains(&path.to_string_lossy().into_owned()));
    assert!(dir.path().join("지형도").is_dir(), "C++ makes <survey dir>\\지형도 on open");
    // C++ 조사 데이터만 여는 길은 m_workCrs 를 바꾸지 않는다(MainWindowSession.cpp:762-773).
    assert_eq!(app.work_crs, WorkCrs::Epsg5187);
    assert_eq!(
        survey_rows(&app),
        vec![
            ("시굴격자".to_owned(), 1, Some("이름·면적")),
            ("유물".to_owned(), 1, Some("이름")),
            ("기준점".to_owned(), 2, Some("이름")),
            ("단면선".to_owned(), 1, Some("이름")),
            ("유구 (선)".to_owned(), 1, Some("이름")),
            ("유구 (면)".to_owned(), 2, Some("면적")),
            ("조사구역".to_owned(), 1, Some("면적")),
        ]
    );
    let tables: Vec<usize> = app.survey_tables.iter().map(|t| t.layer.features.len()).collect();
    assert_eq!(tables, vec![1, 2, 1, 1, 2, 1, 1], "the opened features stay in memory");
    assert_eq!(sha(&path), before, "opening must not write the survey file");
}

/// 범례 견본 색은 C++ `LayerStyleDefaults::forKey`(`LayerStyleDefaults.cpp:42-86`) 그대로.
#[test]
fn opened_survey_swatches_use_cpp_default_style_colors() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = demo_copy(dir.path(), "demo.gpkg");
    let mut app = StrataAppState::new_startup();
    app.open_survey_gpkg(&path);
    let c = Color32::BLACK;
    let swatch = |name: &str| {
        layer_rows(&app.left_layers, c, c, c, c)
            .into_iter()
            .find(|r| r.section.is_empty() && r.name == name)
            .map(|r| r.swatch)
    };
    let rgba = Color32::from_rgba_unmultiplied;
    let rgb = Color32::from_rgb;
    assert_eq!(swatch("조사구역"), Some(SwatchKind::DomainFill { fill: rgba(180, 83, 9, 70), stroke: rgb(146, 64, 14) }));
    assert_eq!(swatch("유구 (면)"), Some(SwatchKind::DomainFill { fill: rgba(22, 163, 74, 90), stroke: rgb(17, 94, 44) }));
    assert_eq!(swatch("시굴격자"), Some(SwatchKind::DomainFill { fill: rgba(220, 38, 38, 18), stroke: rgb(220, 38, 38) }));
    assert_eq!(swatch("유구 (선)"), Some(SwatchKind::DomainLine { stroke: rgb(202, 138, 4), casing: false }));
    assert_eq!(swatch("단면선"), Some(SwatchKind::DomainLine { stroke: rgb(190, 24, 93), casing: true }));
    assert_eq!(swatch("기준점"), Some(SwatchKind::DomainMarker { fill: rgb(234, 179, 8), stroke: rgb(161, 98, 7) }));
    assert_eq!(swatch("유물"), Some(SwatchKind::DomainMarker { fill: rgb(185, 28, 28), stroke: rgb(127, 29, 29) }));
}

/// 열 수 없는 파일은 C++ 문구로 알리고 지금 조사는 그대로 둔다(`SurveyStorage.cpp:371-389`).
#[test]
fn unreadable_or_damaged_file_keeps_the_current_survey() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = demo_copy(dir.path(), "demo.gpkg");
    let mut app = StrataAppState::new_startup();
    app.open_survey_gpkg(&path);
    let rows_before = survey_rows(&app);

    let text = dir.path().join("memo.gpkg");
    std::fs::write(&text, b"not a geopackage").expect("write");
    app.open_survey_gpkg(&text);
    assert_eq!(
        app.home_map.draw_guide_message.as_deref(),
        Some("조사 열기 실패: 읽을 수 있는 GeoPackage 조사 파일이 아닙니다.")
    );

    let damaged = demo_copy(dir.path(), "damaged.gpkg");
    let (root, page): (i64, i64) = {
        let c = rusqlite::Connection::open(&damaged).expect("open copy");
        let root = c.query_row("SELECT rootpage FROM sqlite_master WHERE name = 'trial_trench'", [], |r| r.get(0)).expect("root");
        (root, c.query_row("PRAGMA page_size", [], |r| r.get(0)).expect("page"))
    };
    let mut bytes = std::fs::read(&damaged).expect("read");
    bytes[usize::try_from((root - 1) * page).expect("offset")] = 0;
    std::fs::write(&damaged, bytes).expect("write");
    app.open_survey_gpkg(&damaged);
    assert_eq!(
        app.home_map.draw_guide_message.as_deref(),
        Some("조사 열기 실패: 조사 파일의 무결성을 확인하지 못했습니다. 현재 작업은 유지됩니다.")
    );

    assert_eq!(app.survey_path.as_deref(), Some(path.as_path()));
    assert_eq!(app.window_title(), "demo - Strata");
    assert_eq!(survey_rows(&app), rows_before);
}

/// `--snap` 의 「@open=<경로>」는 그 파일을 「열기」와 같이 연다(임시 폴더 자료만).
#[test]
fn snap_open_action_opens_the_given_file() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = demo_copy(dir.path(), "demo.gpkg");
    let mut app = StrataAppState::new_startup();
    app.snap_apply_action(&format!("@open={}", path.display()));
    assert_eq!(app.survey_path.as_deref(), Some(path.as_path()));
    assert_eq!(app.window_title(), "demo - Strata");
}

/// 도메인이 아닌 면 표(user_poly_*)는 모두 「면」 이름으로 올라온다(`LayerOps.cpp:1757`).
/// 이름이 같아도 행마다 제 도형 수를 보여야 한다.
#[test]
fn same_titled_user_polygon_tables_keep_their_own_counts() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = demo_copy(dir.path(), "demo.gpkg");
    {
        let c = rusqlite::Connection::open(&path).expect("open copy");
        c.execute_batch(
            "CREATE TABLE user_poly_1 (fid INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL, geom POLYGON, name TEXT);
             CREATE TABLE user_poly_2 (fid INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL, geom POLYGON, name TEXT);
             INSERT INTO gpkg_contents (table_name, data_type, identifier, srs_id) VALUES ('user_poly_1', 'features', 'user_poly_1', 5186);
             INSERT INTO gpkg_contents (table_name, data_type, identifier, srs_id) VALUES ('user_poly_2', 'features', 'user_poly_2', 5186);
             INSERT INTO user_poly_1 (name) VALUES ('a');
             INSERT INTO user_poly_2 (name) VALUES ('b');
             INSERT INTO user_poly_2 (name) VALUES ('c');",
        )
        .expect("add user polygon tables");
    }
    let mut app = StrataAppState::new_startup();
    app.open_survey_gpkg(&path);
    let rows = survey_rows(&app);
    assert_eq!(rows[0], ("면".to_owned(), 2, Some("면적")), "user_poly_2 is on top");
    assert_eq!(rows[1], ("면".to_owned(), 1, Some("면적")));
}
