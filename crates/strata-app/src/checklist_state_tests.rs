//! 검수 판정이 C++ 실물과 같다 — docs/senior/checklist-cpp/goldens.md(gogohak1 b036c09 앱으로 연 같은 파일).
//! 견본은 임시 사본만 연다.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use strata_check_export::checklist_engine::ChecklistEngine;

use crate::app_state::StrataAppState;

/// (오류, 주의, 통과, 실패한 규칙 (id, 심각도, 글) 차례대로).
type Outcome = (usize, usize, usize, Vec<(String, String, String)>);

fn outcome_of(src: &Path, extra: &[&str]) -> Outcome {
    let dir = tempfile::tempdir().expect("temp dir");
    let name = src.file_name().expect("name");
    let path = dir.path().join(name);
    std::fs::copy(src, &path).expect("copy");
    for ext in extra {
        std::fs::copy(src.with_extension(ext), path.with_extension(ext)).expect("copy sidecar");
    }
    let mut app = StrataAppState::new_startup();
    assert!(app.open_survey_gpkg(&path), "open {}", path.display());
    let results = ChecklistEngine::with_default_rules().evaluate(&app.checklist_state());
    let failed: Vec<(String, String, String)> =
        results.iter().filter(|r| !r.passed).map(|r| (r.id.clone(), r.severity.clone(), r.message_ko.clone())).collect();
    let errors = ChecklistEngine::failed_count(&results, "error");
    let warns = ChecklistEngine::failed_count(&results, "warn");
    (errors, warns, results.len() - errors - warns, failed)
}

fn sample(rel: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

/// The cpp-made new survey, rebuilt in `dir` from its committed SQL dump (tests/fixtures/cpp_new_survey_5186.sql;
/// the dump's rtree lines need SQLite's default non-defensive mode).
fn cpp_new_survey(dir: &Path) -> std::path::PathBuf {
    let sql = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/cpp_new_survey_5186.sql"))
        .expect("fixture");
    let path = dir.join("cpp-new-survey-5186.gpkg");
    rusqlite::Connection::open(&path).expect("create gpkg").execute_batch(&sql).expect("rebuild gpkg");
    path
}

fn rule(id: &str, severity: &str, message: &str) -> (String, String, String) {
    (id.to_owned(), severity.to_owned(), message.to_owned())
}

fn site() -> (String, String, String) {
    rule("REQ_DRAWING_SITE_LOCATION", "error", "유적위치도가 준비되지 않았습니다.")
}
fn plan() -> (String, String, String) {
    rule("REQ_DRAWING_FEATURE_PLAN", "error", "유구배치도가 준비되지 않았습니다.")
}
fn detail() -> (String, String, String) {
    rule("REQ_DRAWING_FEATURE_DETAIL", "warn", "개별유구실측도가 없습니다.")
}
fn gcp() -> Vec<(String, String, String)> {
    vec![
        rule("GCP_MIN_TWO", "error", "GPS/기준점 2개 미만입니다."),
        rule("GCP_DATUM_META", "error", "측지기준계가 빈 기준점이 있습니다."),
        rule("GCP_ELLIPSOID_META", "error", "타원체가 빈 기준점이 있습니다."),
        rule("GCP_PROJECTION_META", "error", "투영법이 빈 기준점이 있습니다."),
    ]
}

/// demo — 오류 2 · 주의 1 · 통과 24.
#[test]
fn demo_matches_cpp() {
    let (e, w, p, failed) = outcome_of(&sample("samples/demo_survey/demo.gpkg"), &[]);
    assert_eq!(failed, vec![site(), plan(), detail()]);
    assert_eq!((e, w, p), (2, 1, 24));
}

/// bad — 오류 7 · 주의 1 · 통과 19.
#[test]
fn bad_matches_cpp() {
    let (e, w, p, failed) = outcome_of(&sample("samples/bad_survey/bad.gpkg"), &[]);
    let mut want = vec![site(), plan(), detail()];
    want.extend(gcp());
    want.push(rule("FEATURE_LEGEND_FIELDS", "error", "종류·시대가 빈 유구가 있습니다."));
    // 규칙 파일 차례대로 비교한다.
    let order = |v: &mut Vec<(String, String, String)>| v.sort_by_key(|r| rule_index(&r.0));
    let mut got = failed;
    order(&mut got);
    order(&mut want);
    assert_eq!(got, want);
    assert_eq!((e, w, p), (7, 1, 19));
}

/// C++ 이 만든 빈 새 조사 — 오류 7 · 주의 1 · 통과 19. 기준점이 0개여도 측지기준계·타원체·투영법이 오류다(C++ 그대로).
#[test]
fn cpp_new_survey_matches_cpp() {
    let fixture = tempfile::tempdir().expect("temp dir");
    let (e, w, p, failed) = outcome_of(&cpp_new_survey(fixture.path()), &[]);
    let mut want = vec![rule("REQ_DRAWING_SURVEY_AREA", "error", "조사구역도가 없습니다."), site(), plan(), detail()];
    want.extend(gcp());
    let mut got = failed;
    got.sort_by_key(|r| rule_index(&r.0));
    want.sort_by_key(|r| rule_index(&r.0));
    assert_eq!(got, want);
    assert_eq!((e, w, p), (7, 1, 19));
}

fn rule_index(id: &str) -> usize {
    let engine = ChecklistEngine::with_default_rules();
    engine.evaluate(&serde_json::json!({})).iter().position(|r| r.id == id).expect("known rule")
}

fn failed_ids(app: &StrataAppState) -> Vec<String> {
    ChecklistEngine::with_default_rules().evaluate(&app.checklist_state()).into_iter().filter(|r| !r.passed).map(|r| r.id).collect()
}

/// 저장하지 않은 그린 도형도 판정한다(C++ 은 편집 버퍼를 읽는다). 세 꼭짓점 조사구역은 기호로 본다(isAbstractMarker),
/// 네 꼭짓점이면 통과.
#[test]
fn unsaved_survey_area_counts_and_a_triangle_is_a_symbol() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("새조사.gpkg");
    let crs = strata_types::crs::WorkCrs::Epsg5187;
    strata_storage::gpkg::GpkgRepository::create_new_survey(&path, crs).expect("create");
    let mut app = StrataAppState::new_startup();
    app.start_new_survey_project_with_path("새조사", &path, crs);
    app.active_tab = crate::app_state::MainTab::Map;
    app.home_map.viewport.meters_per_pixel = 0.05;
    assert!(failed_ids(&app).contains(&"REQ_DRAWING_SURVEY_AREA".to_owned()));
    app.start_edit_survey_area();
    app.accept_survey_area_setup(); // 이어 그릴 조사구역이 없으면 「조사구역 그리기 설정」 창 → 「그리기 시작」
    for p in [[21_100.0, 451_800.0], [21_160.0, 451_800.0], [21_130.0, 451_850.0]] {
        app.sketch_press(p);
    }
    app.sketch_finish();
    let failed = failed_ids(&app);
    assert!(!failed.contains(&"REQ_DRAWING_SURVEY_AREA".to_owned()), "{failed:?}");
    assert!(failed.contains(&"SURVEY_NO_ABSTRACT_MARKER".to_owned()), "{failed:?}");
    app.undo_map_action();
    app.start_edit_survey_area();
    app.accept_survey_area_setup(); // 이어 그릴 조사구역이 없으면 「조사구역 그리기 설정」 창 → 「그리기 시작」
    for p in [[21_100.0, 451_800.0], [21_160.0, 451_800.0], [21_160.0, 451_850.0], [21_100.0, 451_850.0]] {
        app.sketch_press(p);
    }
    app.sketch_finish();
    assert!(!failed_ids(&app).contains(&"SURVEY_NO_ABSTRACT_MARKER".to_owned()));
}

/// 조사구역(표 5186) 밖에 그린 유구(작업 5187) — 같은 좌표계로 옮겨 견주므로 「조사구역 밖」 주의와 그 도형이 대상으로 잡힌다.
#[test]
fn feature_drawn_outside_the_survey_area_is_flagged_with_a_target() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("demo.gpkg");
    std::fs::copy(sample("samples/demo_survey/demo.gpkg"), &path).expect("copy");
    let mut app = StrataAppState::new_startup();
    app.open_survey_gpkg(&path);
    app.active_tab = crate::app_state::MainTab::Map;
    app.home_map.viewport.meters_per_pixel = 0.05;
    assert!(!failed_ids(&app).contains(&"EXTENT_MATCH".to_owned()));
    let to_work = strata_engine::Transformer::new(5186, 5187).expect("PROJ");
    app.start_edit_feature_poly();
    for p in [[201_000.0, 451_000.0], [201_030.0, 451_000.0], [201_030.0, 451_040.0], [201_000.0, 451_040.0]] {
        app.sketch_press(to_work.transform(p).expect("transform"));
    }
    app.sketch_finish();
    let results = ChecklistEngine::with_default_rules().evaluate(&app.checklist_state());
    let extent = results.iter().find(|r| r.id == "EXTENT_MATCH").expect("rule");
    assert!(!extent.passed);
    assert_eq!(extent.targets.len(), 1);
    assert_eq!(extent.targets[0].layer_id, "feature_poly");
    assert!(extent.targets[0].feature_id < 0, "the unsaved shape is the target");
}

/// 실패한 줄의 단추가 C++ 실물과 같다(goldens.md 괄호 안). 「위치 보기」는 걸린 도형이 있을 때만.
#[test]
fn failed_rows_show_the_cpp_buttons() {
    let buttons = |src: &Path, extra: &[&str]| -> Vec<(String, Vec<&'static str>)> {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join(src.file_name().expect("name"));
        std::fs::copy(src, &path).expect("copy");
        for ext in extra {
            std::fs::copy(src.with_extension(ext), path.with_extension(ext)).expect("copy sidecar");
        }
        let mut app = StrataAppState::new_startup();
        app.open_survey_gpkg(&path);
        ChecklistEngine::with_default_rules()
            .evaluate(&app.checklist_state())
            .iter()
            .filter(|r| !r.passed)
            .map(|r| {
                let (goto, tool) = crate::modal_export_table::row_buttons(r);
                let mut b = Vec::new();
                if goto {
                    b.push("위치 보기");
                }
                b.extend(tool);
                (r.id.clone(), b)
            })
            .collect()
    };
    let layout = |id: &str| (id.to_owned(), vec!["도면 열기"]);
    let demo = buttons(&sample("samples/demo_survey/demo.gpkg"), &[]);
    assert_eq!(demo, vec![layout("REQ_DRAWING_SITE_LOCATION"), layout("REQ_DRAWING_FEATURE_PLAN"), layout("REQ_DRAWING_FEATURE_DETAIL")]);
    let bad = buttons(&sample("samples/bad_survey/bad.gpkg"), &[]);
    let find = |v: &[(String, Vec<&'static str>)], id: &str| v.iter().find(|(i, _)| i == id).map(|(_, b)| b.clone()).expect(id);
    assert_eq!(find(&bad, "GCP_MIN_TWO"), vec!["기준점 찍기"]);
    for id in ["GCP_DATUM_META", "GCP_ELLIPSOID_META", "GCP_PROJECTION_META"] {
        assert_eq!(find(&bad, id), vec!["속성 고치기"], "{id}: no points, so no 위치 보기");
    }
    assert_eq!(find(&bad, "FEATURE_LEGEND_FIELDS"), vec!["위치 보기", "속성 고치기"]);
    let fixture = tempfile::tempdir().expect("temp dir");
    let newcpp = buttons(&cpp_new_survey(fixture.path()), &[]);
    assert_eq!(find(&newcpp, "REQ_DRAWING_SURVEY_AREA"), vec!["조사구역 그리기"]);
}
