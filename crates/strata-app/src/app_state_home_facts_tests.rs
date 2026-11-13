//! 홈의 조사 사실 — 임시 ini 를 붙여서만(사용자 ini 는 읽지도 쓰지도 않는다). demo 임시 사본.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use crate::app_state::{MainTab, StrataAppState};

fn demo_copy(dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::copy(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg"), &path).expect("copy");
    path
}

/// 홈에 오면 열리지 않은 최근 조사를 세어 기록한다(probeSurveyFacts) — 좌표계는 저장만 남긴다(「—」).
#[test]
fn home_visit_counts_an_unopened_recent_survey() {
    let dir = tempfile::tempdir().expect("temp dir");
    let survey = demo_copy(dir.path(), "a.gpkg");
    let mut app = StrataAppState::new_startup();
    app.attach_user_settings(Some(dir.path().join("ka-hgis.ini")));
    app.remember_survey(&survey, "a");
    app.active_tab = MainTab::Home;
    // 드라이브 종류는 뒤 실행에서 읽는다 — 다 읽을 때까지 매 프레임처럼 부른다.
    for _ in 0..200 {
        app.home_facts_on_visit(None);
        if app.home_map.facts_probed {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    let f = app.facts_of(&app.recent_surveys[0].path.clone());
    assert!(f.has_counts(), "{f:?}");
    assert!(f.areas >= 1);
    assert_eq!(strata_storage::survey_facts::crs_text(&f.work_crs), "—");
    assert_eq!(strata_storage::survey_facts::check_text(&f), "검수 전");
}

/// 저장하면 그 조사의 구역·유구 수와 작업 좌표계를 기록한다(recordSurveyFacts).
#[test]
fn saving_records_counts_and_the_work_crs() {
    let dir = tempfile::tempdir().expect("temp dir");
    let survey = demo_copy(dir.path(), "b.gpkg");
    let mut app = StrataAppState::new_startup();
    app.attach_user_settings(Some(dir.path().join("ka-hgis.ini")));
    app.open_survey_gpkg(&survey);
    assert!(app.save_survey_project());
    let polys = app.survey_tables.iter().filter(|t| t.table == "feature_poly" || t.table == "feature_line").map(|t| t.layer.features.len()).sum::<usize>();
    let f = app.facts_of(&app.recent_surveys[0].path.clone());
    assert_eq!(f.features, i32::try_from(polys).unwrap());
    assert_eq!(f.work_crs, "EPSG:5187");
    assert_eq!(strata_storage::survey_facts::crs_text(&f.work_crs), "5187 동부원점");
}

/// ini 를 붙이지 않으면 사실을 모른다(읽지도 쓰지도 않는다).
#[test]
fn without_an_ini_nothing_is_known() {
    let dir = tempfile::tempdir().expect("temp dir");
    let survey = demo_copy(dir.path(), "c.gpkg");
    let mut app = StrataAppState::new_startup();
    app.open_survey_gpkg(&survey);
    assert!(app.save_survey_project());
    app.active_tab = MainTab::Home;
    app.home_facts_on_visit(None);
    assert!(!app.facts_of(&survey.to_string_lossy()).known);
}
