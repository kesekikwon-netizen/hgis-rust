//! 오른쪽 카드의 대상·적던 글·창 뒤 막기·없어진 도형 — demo 임시 사본.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use crate::app_state::{MainTab, StrataAppState};
use crate::checklist_state::attr;

fn opened_demo(dir: &Path) -> StrataAppState {
    let path = dir.join("demo.gpkg");
    std::fs::copy(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg"), &path).expect("copy");
    let mut app = StrataAppState::new_startup();
    app.open_survey_gpkg(&path);
    app.active_tab = MainTab::Map;
    app.home_map.viewport.meters_per_pixel = 0.05;
    app
}

fn note_of(app: &StrataAppState, fid: i64) -> Option<String> {
    let t = app.survey_tables.iter().find(|t| t.table == "feature_poly").unwrap();
    attr(t.layer.features.iter().find(|f| f.fid == fid).unwrap(), "note")
}

/// 하나를 고르면 카드가 따라가고, 고른 것이 없어지면 비운다(showSelection). 그린 도형의 카드는 남는다.
#[test]
fn card_follows_the_selection_but_keeps_a_drawn_shape() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut app = opened_demo(dir.path());
    app.home_map.selected_shape = Some(("feature_poly".to_owned(), 1));
    app.sync_card_with_selection();
    let v = app.card_view_now().expect("card");
    assert_eq!((v.title, v.header.as_str()), ("선택한 유구", "1호 주거지"));
    app.home_map.selected_shape = None;
    app.sync_card_with_selection();
    assert!(app.card_view_now().is_none());
    app.set_card_target("feature_poly", 2, false); // 그린 도형(setFeature)
    app.sync_card_with_selection();
    assert_eq!(app.home_map.card.as_ref().map(|c| c.fid), Some(2), "a drawn shape's card stays");
}

/// 적던 메모는 다른 도형으로 바뀌기 전에 그 도형에 쓴다(commitTyping) — 새 도형에 붙지 않는다.
#[test]
fn typed_memo_stays_on_its_own_shape() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut app = opened_demo(dir.path());
    app.set_card_target("feature_poly", 1, true);
    app.card_draft("note", "북벽 교란".to_owned());
    app.set_card_target("feature_poly", 2, true);
    assert_eq!(note_of(&app, 1).as_deref(), Some("북벽 교란"));
    assert_ne!(note_of(&app, 2).as_deref(), Some("북벽 교란"));
    assert!(app.is_dirty);
    assert_eq!(app.card_view_now().unwrap().state.text(), "저장된 값입니다");
}

/// 「도형 속성」을 열기 전에 적던 메모를 쓴다 — 창의 비고 칸이 그 글을 보인다(저장 때 지워지지 않게).
#[test]
fn record_window_sees_the_typed_memo() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut app = opened_demo(dir.path());
    app.set_card_target("feature_poly", 1, true);
    app.card_draft("note", "바닥 다짐".to_owned());
    app.open_feature_record("feature_poly", 1);
    let form = app.home_map.feature_record.as_ref().expect("open");
    assert_eq!(form.rows.iter().find(|r| r.field == "note").unwrap().text, "바닥 다짐");
}

/// 창(모달)이 열린 동안 카드는 고치지 않는다 — 창의 저장이 덮어쓰지 않게.
#[test]
fn card_edits_wait_while_a_window_is_open() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut app = opened_demo(dir.path());
    app.set_card_target("feature_poly", 1, true);
    app.open_feature_record("feature_poly", 2);
    assert!(!app.card_set_value("note", "창 뒤"));
    assert_ne!(note_of(&app, 1).as_deref(), Some("창 뒤"));
}

/// 카드 고침은 「기록 고치기」 한 단계(Ctrl+Z 하나), 상태줄 글은 없다. 같은 값이면 단계 없음.
#[test]
fn a_card_edit_is_one_undo_step_without_status_text() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut app = opened_demo(dir.path());
    let status = app.status_left_message().to_owned();
    app.set_card_target("feature_poly", 1, true);
    assert!(app.card_set_value("kind", "수 혈"));
    assert!(!app.card_set_value("kind", "수혈"), "same value: no step");
    assert_eq!(app.status_left_message(), status);
    app.undo_map_action();
    let t = app.survey_tables.iter().find(|t| t.table == "feature_poly").unwrap();
    assert_eq!(attr(t.layer.features.iter().find(|f| f.fid == 1).unwrap(), "kind").as_deref(), Some("주거지"));
    assert!(app.card_view_now().is_some(), "the saved shape stays on the card");
}

/// 그린 도형을 되돌리면, 또 저장으로 번호가 바뀌면 카드는 비운다 — 다른 도형을 보이지 않는다.
#[test]
fn card_clears_when_its_shape_is_gone() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut app = opened_demo(dir.path());
    let to_work = strata_engine::Transformer::new(5186, 5187).expect("PROJ");
    let draw = |app: &mut StrataAppState| {
        app.start_edit_feature_poly();
        for p in [[200_080.0, 450_060.0], [200_090.0, 450_060.0], [200_090.0, 450_070.0]] {
            app.sketch_press(to_work.transform(p).expect("transform"));
        }
        app.sketch_finish();
        app.skip_feature_form();
    };
    draw(&mut app);
    let drawn = app.home_map.card.clone().expect("drawn shape on the card");
    assert!(drawn.fid < 0 && !drawn.follows_selection);
    app.undo_map_action();
    assert!(app.home_map.card.is_none());
    draw(&mut app);
    assert!(app.save_survey_project());
    assert!(app.home_map.card.is_none(), "the unsaved id is gone after 저장");
}
