//! 그린 뒤 「이름·번호」 흐름 — demo 사본(유구 1호 주거지, 2호 수혈). 임시 폴더만.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use crate::app_state::{MainTab, StrataAppState};
use crate::checklist_state::attr;

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
    for p in [[200_080.0, 450_060.0], [200_090.0, 450_060.0], [200_090.0, 450_070.0], [200_080.0, 450_070.0]] {
        app.sketch_press(to_work.transform(p).expect("transform"));
    }
    app.sketch_finish();
}

fn last_poly(app: &StrataAppState) -> strata_storage::edit_buffer::FeatureRecord {
    app.survey_tables.iter().find(|t| t.table == "feature_poly").unwrap().layer.features.last().unwrap().clone()
}

/// 연속 그리기 꺼짐: 창이 열리고(C++ 글), 종류를 고르면 그 종류의 다음 번호, 「저장」이면 편집 버퍼에 쓰고 파일에는 「저장」으로.
#[test]
fn form_opens_after_drawing_and_saves_into_the_shape() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut app, path) = opened_demo(dir.path());
    draw_pit(&mut app);
    assert_eq!(app.status_left_message(), "도형은 넣었습니다. 이름·번호를 적거나 Esc로 건너뛰세요.");
    let form = app.home_map.feature_form.as_mut().expect("form open");
    assert_eq!(form.number, "1호");
    form.name = "주거지".to_owned();
    form.update_number_hint();
    assert_eq!(form.number, "2호");
    form.period = "원삼국".to_owned();
    // 창이 열린 동안 지도는 점을 받지 않는다(C++ 창은 모달).
    app.sketch_press([1.0, 1.0]);
    assert!(app.draw_sketch.points.is_empty());

    app.save_feature_form();
    let f = last_poly(&app);
    assert!(f.fid < 0);
    assert_eq!((attr(&f, "kind"), attr(&f, "period"), attr(&f, "feature_no")), (Some("주거지".into()), Some("원삼국".into()), Some("2호".into())));
    let msg = app.status_left_message().to_owned();
    assert!(msg.starts_with("도형을 넣었습니다 (") && !msg.contains("종류·시대 빈 유구"), "{msg}");
    // 편집 버퍼에만 — 「저장」 전 파일은 그대로(test_feature_card.cpp: featureCount == before).
    let rows = |p: &Path| rusqlite::Connection::open(p).unwrap().query_row("SELECT count(*) FROM feature_poly", [], |r| r.get::<_, i64>(0)).unwrap();
    assert_eq!(rows(&path), 2);

    assert!(app.save_survey_project());
    assert_eq!(rows(&path), 3);
    let conn = rusqlite::Connection::open(&path).expect("open");
    let row: (String, String, String) = conn
        .query_row("SELECT kind, period, feature_no FROM feature_poly ORDER BY fid DESC LIMIT 1", [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .expect("row");
    assert_eq!(row, ("주거지".into(), "원삼국".into(), "2호".into()));
}

/// ported from tests/test_feature_card.cpp:form_applyToWritesEditBufferOnly — 「주거 지」는 조사 맞춤법 「주거지」로, 시대 원삼국,
/// 번호 3호, updated_at 이 찍히고, 편집 버퍼에만 쓴다(파일 없음 = 아무것도 커밋하지 않는다).
#[test]
fn apply_writes_the_edit_buffer_only() {
    use strata_storage::edit_buffer::FeatureRecord;
    let mut t = crate::feature_form::tests::table(
        &["kind", "period", "feature_no", "note", "updated_at"],
        &[
            &[("kind", "주거지"), ("period", "청동기"), ("feature_no", "1호")],
            &[("kind", "주거지"), ("period", "청동기"), ("feature_no", "2호")],
            &[("kind", "수혈"), ("period", "청동기"), ("feature_no", "1호")],
        ],
    );
    t.layer.features.push(FeatureRecord { fid: -1, wkt_geom: String::new(), attributes: Default::default() });
    let mut app = StrataAppState::new_startup();
    let mut form = crate::feature_form_state::FeatureForm::new(&t, -1).expect("offered");
    form.name = "주거 지".to_owned();
    form.update_number_hint();
    form.period = "원삼국".to_owned();
    app.survey_tables.push(t);
    app.home_map.feature_form = Some(form);
    app.save_feature_form();
    let written = app.survey_tables[0].layer.features.last().unwrap().clone();
    assert_eq!(attr(&written, "kind").as_deref(), Some("주거지"));
    assert_eq!(attr(&written, "period").as_deref(), Some("원삼국"));
    assert_eq!(attr(&written, "feature_no").as_deref(), Some("3호"));
    assert!(attr(&written, "updated_at").is_some_and(|s| !s.is_empty()));
    assert!(app.is_dirty);
    assert_eq!(app.survey_tables[0].baseline.len(), 3, "nothing committed");
}

/// 「건너뛰기」(Esc): 그린 도형은 빈 값으로 남고, 상태 글에 종류·시대 빈 유구 수.
#[test]
fn skip_keeps_the_shape_with_empty_values() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut app, _) = opened_demo(dir.path());
    draw_pit(&mut app);
    app.skip_feature_form();
    assert!(app.home_map.feature_form.is_none());
    assert_eq!(attr(&last_poly(&app), "kind"), None);
    assert!(app.status_left_message().contains(" · 종류·시대 빈 유구 1개(제출 전 「검수·제출」에서 확인)"));
}

/// 연속 그리기 켜짐: 창 없이 이어 그리고, 상태 글 끝에 「이름·번호는 나중에 속성에서」(MainWindowEditing.cpp:909-910).
#[test]
fn continuous_drawing_skips_the_window() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut app, _) = opened_demo(dir.path());
    app.ribbon.continuous_draw = true;
    draw_pit(&mut app);
    assert!(app.home_map.feature_form.is_none());
    assert!(app.status_left_message().contains("「저장」(Ctrl+S)으로 파일에 씁니다 · 이름·번호는 나중에 속성에서"));
}

/// 도형과 이름·번호는 되돌리기 한 단계(KaUndoGroup) — Ctrl+Z 한 번에 둘 다 없어진다.
#[test]
fn shape_and_values_undo_as_one_step() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut app, _) = opened_demo(dir.path());
    draw_pit(&mut app);
    app.home_map.feature_form.as_mut().unwrap().name = "수혈".to_owned();
    app.save_feature_form();
    let before = app.survey_tables.iter().find(|t| t.table == "feature_poly").unwrap().layer.features.len();
    app.undo_map_action();
    let after = app.survey_tables.iter().find(|t| t.table == "feature_poly").unwrap().layer.features.len();
    assert_eq!(before - after, 1);
    assert_eq!(app.status_left_message(), "이전 상태로 되돌렸습니다.");
}

/// 옛 조사 파일은 첫 「저장」에서 기록 항목(uid·created_at·updated_at)을 얻고(migrateGenerationCopy), C++ 은 저장한 사본을
/// 다시 붙이므로(remountCopiedSurveyLayers) 그다음 그린 도형에는 그 값이 찍힌다.
#[test]
fn shapes_drawn_after_the_first_save_get_record_stamps() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut app, path) = opened_demo(dir.path());
    draw_pit(&mut app);
    app.skip_feature_form();
    assert!(app.save_survey_project());
    draw_pit(&mut app);
    app.home_map.feature_form.as_mut().expect("form").name = "수혈".to_owned();
    app.save_feature_form();
    assert!(app.save_survey_project());
    let conn = rusqlite::Connection::open(&path).expect("open");
    let stamps: (Option<String>, Option<String>, Option<String>) = conn
        .query_row("SELECT uid, created_at, updated_at FROM feature_poly ORDER BY fid DESC LIMIT 1", [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .expect("stamps");
    assert!(stamps.0.as_ref().is_some_and(|s| s.len() == 36), "uid {stamps:?}");
    assert!(stamps.1.is_some_and(|s| !s.is_empty()) && stamps.2.is_some_and(|s| !s.is_empty()), "times");
}

/// 빈 종류·번호로 「저장」하면 파일에는 NULL(C++ 과 같은 파일 — hgis-2e 결정). 빈 글 '' 이 아니다.
#[test]
fn empty_name_and_number_are_saved_as_null() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut app, path) = opened_demo(dir.path());
    draw_pit(&mut app);
    let form = app.home_map.feature_form.as_mut().expect("form");
    form.number = String::new();
    form.number_typed = true;
    app.save_feature_form();
    assert!(app.save_survey_project());
    let conn = rusqlite::Connection::open(&path).expect("open");
    let nulls: (bool, bool) = conn
        .query_row("SELECT kind IS NULL, feature_no IS NULL FROM feature_poly ORDER BY fid DESC LIMIT 1", [], |r| Ok((r.get(0)?, r.get(1)?)))
        .expect("row");
    assert_eq!(nulls, (true, true));
}
