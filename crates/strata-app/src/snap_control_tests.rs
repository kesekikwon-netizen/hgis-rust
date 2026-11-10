//! 검증 제어 통로 시험 — `--snap` 의 `@명령` 이 실제로 상태를 바꾸는지 확인한다.
//!
//! ## 왜 화면 캡처가 아니라 상태인가
//!
//! 스크린샷 비교는 프레임 타이밍·창 크기·테마에 흔들린다. 반면 "명령을 넣으면
//! 상태가 이렇게 바뀐다"는 결정적이다. 화면은 이 상태를 그릴 뿐이므로,
//! 상태가 맞으면 화면도 맞는다(ENGINE_DESIGN 원칙 3 — 창 없이 검증).
//!
//! 이 시험이 있으면 `--snap` 스크립트를 짤 때 "명령 이름이 틀렸다"를
//! 앱을 20초 띄우지 않고 알 수 있다.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::app_state::{MainTab, StrataAppState};

fn app() -> StrataAppState {
    StrataAppState::new_startup()
}

/// 탭 전환 명령이 실제로 탭을 바꾼다.
#[test]
fn tab_commands_switch_tabs() {
    let mut a = app();
    a.snap_apply_action_for_test("@tab_map");
    assert_eq!(a.active_tab, MainTab::Map, "@tab_map");

    a.snap_apply_action_for_test("@tab_home");
    assert_eq!(a.active_tab, MainTab::Home, "@tab_home");
}

/// 도구 전환 명령이 활성 도구를 바꾼다.
#[test]
fn tool_commands_switch_tools() {
    use strata_canvas2d::map_tools::MapToolKind;
    let mut a = app();
    a.snap_apply_action_for_test("@draw_polygon_tool");
    assert_eq!(a.active_tool, MapToolKind::DrawPolygon);
    a.snap_apply_action_for_test("@select_tool");
    assert_eq!(a.active_tool, MapToolKind::Select);
    a.snap_apply_action_for_test("@pan_tool");
    assert_eq!(a.active_tool, MapToolKind::Pan);
}

/// `@draw` 는 캔버스 사각형이 있어야 동작한다(없으면 조용히 무시).
#[test]
fn draw_requires_canvas_rect() {
    let mut a = app();
    assert!(a.last_canvas_rect.is_none(), "처음엔 캔버스 사각형이 없다");
    a.snap_apply_action_for_test("@draw:100x200");
    assert!(
        a.home_map.scene.active_rubberband.is_empty(),
        "캔버스 사각형이 없으면 점을 찍지 않는다"
    );
}

/// 캔버스 사각형이 있으면 `@draw:x,y` 가 지도 좌표로 바뀌어 점이 쌓인다.
#[test]
fn draw_appends_points_when_canvas_known() {
    let mut a = app();
    a.last_canvas_rect = Some(eframe::egui::Rect::from_min_size(
        eframe::egui::Pos2::new(300.0, 100.0),
        eframe::egui::Vec2::new(600.0, 400.0),
    ));
    a.snap_apply_action_for_test("@draw:600x300");
    a.snap_apply_action_for_test("@draw:900x300");
    a.snap_apply_action_for_test("@draw:900x600");

    assert_eq!(a.home_map.scene.active_rubberband.len(), 3, "점 3개");
    // 캔버스 좌상단(300,100) → 중심에서 왼쪽 위.
    let first = a.home_map.scene.active_rubberband[0];
    assert!(first[0].is_finite() && first[1].is_finite());
    // 화면에서 오른쪽으로 간 점은 지도 x 도 커야 한다.
    let second = a.home_map.scene.active_rubberband[1];
    assert!(second[0] > first[0], "오른쪽 점의 x 가 더 커야 한다");
    // 화면에서 아래로 간 점은 지도 y 가 작아야 한다(화면 y 는 뒤집힘).
    let third = a.home_map.scene.active_rubberband[2];
    assert!(third[1] < second[1], "아래 점의 y 가 더 작아야 한다");
}

/// `x,y` 뿐 아니라 `x;y` 도 받는다(단계 구분자가 쉼표라서).
#[test]
fn draw_accepts_semicolon_separator() {
    let mut a = app();
    a.last_canvas_rect = Some(eframe::egui::Rect::from_min_size(
        eframe::egui::Pos2::ZERO,
        eframe::egui::Vec2::new(600.0, 400.0),
    ));
    a.snap_apply_action_for_test("@draw:300;200");
    assert_eq!(a.home_map.scene.active_rubberband.len(), 1);
}

/// `@clear_draw` 가 그리는 중인 점을 지운다.
#[test]
fn clear_draw_empties_ring() {
    let mut a = app();
    a.last_canvas_rect = Some(eframe::egui::Rect::from_min_size(
        eframe::egui::Pos2::ZERO,
        eframe::egui::Vec2::new(600.0, 400.0),
    ));
    a.snap_apply_action_for_test("@draw:100x100");
    a.snap_apply_action_for_test("@draw:200x100");
    assert_eq!(a.home_map.scene.active_rubberband.len(), 2);
    a.snap_apply_action_for_test("@clear_draw");
    assert!(a.home_map.scene.active_rubberband.is_empty());
}

/// `@finish_draw` 는 점 3개 이상일 때만 저장한다(2개는 무시).
#[test]
fn finish_draw_saves_only_with_enough_points() {
    let mut a = app();
    a.last_canvas_rect = Some(eframe::egui::Rect::from_min_size(
        eframe::egui::Pos2::ZERO,
        eframe::egui::Vec2::new(600.0, 400.0),
    ));
    // 시작 상태에 이미 조사구역이 있을 수 있으므로 **개수 차이**로 판정한다.
    let before = a.home_map.real_survey.survey_rings.len();

    a.snap_apply_action_for_test("@draw:100x100");
    a.snap_apply_action_for_test("@draw:300x100");
    a.snap_apply_action_for_test("@finish_draw");
    assert_eq!(
        a.home_map.real_survey.survey_rings.len(),
        before,
        "점 2개로는 저장하지 않는다"
    );

    a.snap_apply_action_for_test("@draw:300x300");
    a.snap_apply_action_for_test("@finish_draw");
    assert_eq!(
        a.home_map.real_survey.survey_rings.len(),
        before + 1,
        "점 3개면 조사구역 1개가 저장된다"
    );
    assert!(
        a.home_map.scene.active_rubberband.is_empty(),
        "저장 후 링이 비워진다"
    );
}

/// 스냅 켜고 끄기.
#[test]
fn snap_toggle_works() {
    let mut a = app();
    a.snap_apply_action_for_test("@snap_off");
    assert!(!a.home_map.cad.snap_enabled);
    a.snap_apply_action_for_test("@snap_on");
    assert!(a.home_map.cad.snap_enabled);
}

/// 모르는 명령은 리본 라벨로 넘어가고, 그것도 아니면 아무 일도 안 한다(패닉 없음).
#[test]
fn unknown_command_does_not_panic() {
    let mut a = app();
    a.snap_apply_action_for_test("@존재하지않는명령");
    a.snap_apply_action_for_test("리본에도 없는 라벨");
    // 상태가 망가지지 않았는지 — 시작 탭 그대로.
    assert_eq!(a.active_tab, MainTab::Home);
}

/// 그리기 → 마침 → 꼭짓점 편집 → 이동 → 삽입 → 삭제가 한 줄로 이어진다.
///
/// 이게 앱을 스크립트로 조종해 편집 기능을 검증하는 통로다.
#[test]
fn vertex_edit_commands_work_end_to_end() {
    let mut a = app();
    a.last_canvas_rect = Some(eframe::egui::Rect::from_min_size(
        eframe::egui::Pos2::ZERO,
        eframe::egui::Vec2::new(600.0, 400.0),
    ));
    // 사각형 하나 그려 저장한다.
    a.snap_apply_action_for_test("@draw:100x100");
    a.snap_apply_action_for_test("@draw:300x100");
    a.snap_apply_action_for_test("@draw:300x300");
    a.snap_apply_action_for_test("@draw:100x300");
    a.snap_apply_action_for_test("@finish_draw");
    let n = a.home_map.real_survey.survey_rings.len();
    assert!(n > 0, "조사구역이 저장돼야 한다");

    // 마지막에 저장한 링을 편집 대상으로 잡는다.
    let idx = n - 1;
    a.home_map.selected_feature_idx = Some(idx);
    a.snap_apply_action_for_test("@vertex_edit");
    assert!(a.home_map.vertex_edit.is_some(), "편집 세션이 열려야");
    assert_eq!(a.home_map.vertex_edit.as_ref().unwrap().feature_index, idx);

    let before = a.home_map.real_survey.survey_rings[idx].len();

    // 점 하나를 지도 좌표로 옮긴다.
    let target = a.home_map.real_survey.survey_rings[idx][0];
    let moved = [target[0] + 5.0, target[1] + 5.0];
    a.snap_apply_action_for_test(&format!(
        "@move_vertex:0;{};{}",
        moved[0], moved[1]
    ));
    let got = a.home_map.real_survey.survey_rings[idx][0];
    assert!((got[0] - moved[0]).abs() < 1e-6, "이동한 x");
    assert!((got[1] - moved[1]).abs() < 1e-6, "이동한 y");

    // 선분 하나에 점을 넣는다.
    a.snap_apply_action_for_test(&format!("@insert_vertex:0;{};{}", moved[0], moved[1]));
    assert_eq!(
        a.home_map.real_survey.survey_rings[idx].len(),
        before + 1,
        "삽입으로 점이 하나 늘어야"
    );

    // 하나 지운다.
    a.snap_apply_action_for_test("@delete_vertex:0");
    assert_eq!(
        a.home_map.real_survey.survey_rings[idx].len(),
        before,
        "삭제로 원래대로"
    );

    // 편집을 끝낸다.
    a.snap_apply_action_for_test("@vertex_edit_off");
    assert!(a.home_map.vertex_edit.is_none());
}

/// 편집 세션이 없으면 정점 명령은 조용히 무시된다(패닉 없음).
#[test]
fn vertex_commands_need_session() {
    let mut a = app();
    assert!(a.home_map.vertex_edit.is_none());
    a.snap_apply_action_for_test("@move_vertex:0;1;2");
    a.snap_apply_action_for_test("@insert_vertex:0;1;2");
    a.snap_apply_action_for_test("@delete_vertex:0");
    a.snap_apply_action_for_test("@dump_vertices");
    assert!(a.home_map.vertex_edit.is_none());
}

/// **Ctrl+Z 가 실제로 되돌린다** — 예전에는 안내 문구만 띄우고 아무 것도 안 했다.
#[test]
fn undo_restores_vertex_position() {
    let mut a = app();
    a.last_canvas_rect = Some(eframe::egui::Rect::from_min_size(
        eframe::egui::Pos2::ZERO,
        eframe::egui::Vec2::new(600.0, 400.0),
    ));
    a.snap_apply_action_for_test("@draw:100x100");
    a.snap_apply_action_for_test("@draw:300x100");
    a.snap_apply_action_for_test("@draw:300x300");
    a.snap_apply_action_for_test("@finish_draw");
    let idx = a.home_map.real_survey.survey_rings.len() - 1;
    a.home_map.selected_feature_idx = Some(idx);
    a.snap_apply_action_for_test("@vertex_edit");

    let original = a.home_map.real_survey.survey_rings[idx][0];
    let moved = [original[0] + 25.0, original[1] + 25.0];
    a.snap_apply_action_for_test(&format!("@move_vertex:0;{};{}", moved[0], moved[1]));

    let after_move = a.home_map.real_survey.survey_rings[idx][0];
    assert!((after_move[0] - moved[0]).abs() < 1e-6, "옮겨졌다");

    // 되돌린다.
    a.snap_apply_action_for_test("@undo");
    let restored = a.home_map.real_survey.survey_rings[idx][0];
    assert!(
        (restored[0] - original[0]).abs() < 1e-6 && (restored[1] - original[1]).abs() < 1e-6,
        "Ctrl+Z 로 원래 자리로 돌아와야 한다: {restored:?} vs {original:?}"
    );

    // 다시 실행하면 옮긴 자리로 간다.
    a.snap_apply_action_for_test("@redo");
    let redone = a.home_map.real_survey.survey_rings[idx][0];
    assert!(
        (redone[0] - moved[0]).abs() < 1e-6,
        "Ctrl+Y 로 다시 옮긴 자리로"
    );
}

/// 되돌릴 것이 없으면 `false` 를 돌려주고 상태를 안 바꾼다.
#[test]
fn undo_without_history_is_noop() {
    let mut a = app();
    assert!(!a.snap_undo_ring(), "처음엔 되돌릴 것이 없다");
    assert!(!a.snap_redo_ring(), "처음엔 다시 실행할 것도 없다");
}
