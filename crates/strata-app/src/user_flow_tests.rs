//! 사용자 흐름 시험 — "새 조사 → 그리기 → 저장 → 다시 열기" 가 실제로 이어지는지.
//!
//! ## 왜 이 시험이 필요한가
//!
//! 2026-11-10 사용자 지적: "현재 아무것도 안 된다". 개별 함수 시험은 많았지만
//! **사용자가 실제로 밟는 순서**를 끝까지 따라가 본 적이 없었다.
//! 이 시험은 리본 버튼만 눌러서 한 조사를 만들고, 도형을 그리고, 저장하고,
//! 다시 열어서 데이터가 살아 있는지 확인한다.
//!
//! ## 네이티브 창을 띄우지 않는다
//!
//! `새 조사` 는 대화상자를 연다(그건 `modals.active` 상태일 뿐 창이 아니다).
//! `열기`·`저장` 은 실제 파일 창을 띄우므로 **환경변수로 경로를 주입**한다
//! (`KA_HGIS_OPEN_PATH`, `KA_HGIS_SAVE_PATH`). 가드로 감싸 새지 않게 한다.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::app_state::{MainTab, StrataAppState};
use crate::test_env_guard::{EnvGuard, TempDirGuard};

/// 새 조사 모달을 채우고 확정하면 조사가 만들어진다.
#[test]
fn new_survey_flow_creates_a_project() {
    let dir = TempDirGuard::new("ka_flow_new");
    let mut a = StrataAppState::new_startup();

    // 1) 「새 조사」 → 모달이 열린다.
    a.handle_ribbon_click("새 조사");
    assert!(
        matches!(
            a.modals.active,
            Some(crate::dialogs_modal::ActiveModalDialog::NewSurvey)
        ),
        "「새 조사」 가 대화상자를 열어야 한다"
    );

    // 2) 이름·폴더를 채우고 확정한다(사용자가 하는 일과 같다).
    a.modals.new_survey_name = "흐름검증".to_owned();
    a.modals.new_survey_dir = dir.path.to_string_lossy().into_owned();
    a.modals.new_survey_crs = strata_types::crs::WorkCrs::Epsg5187;
    a.snap_apply_action_for_test("@new_survey_go");

    // 3) 조사가 생기고 파일이 실제로 만들어졌다.
    let p = a.survey_path.clone().expect("조사 경로가 잡혀야 한다");
    assert!(p.exists(), "GPKG 파일이 실제로 생겨야 한다: {}", p.display());
    assert_eq!(a.active_tab, MainTab::Map, "조사가 열리면 지도 탭");
}

/// 그리기 → 저장 → 다시 열기에서 도형이 살아남는다.
#[test]
fn draw_save_reopen_keeps_the_polygon() {
    let dir = TempDirGuard::new("ka_flow_save");
    let gpkg = dir.path.join("보존검증.gpkg");

    // 1) 새 조사를 만들고 사각형을 그린다.
    let mut a = StrataAppState::new_startup();
    a.modals.new_survey_name = "보존검증".to_owned();
    a.modals.new_survey_dir = dir.path.to_string_lossy().into_owned();
    a.modals.new_survey_crs = strata_types::crs::WorkCrs::Epsg5187;
    a.snap_apply_action_for_test("@new_survey_go");

    a.last_canvas_rect = Some(eframe::egui::Rect::from_min_size(
        eframe::egui::Pos2::ZERO,
        eframe::egui::Vec2::new(600.0, 400.0),
    ));
    a.snap_apply_action_for_test("@draw_polygon_tool");
    a.snap_apply_action_for_test("@draw:100x100");
    a.snap_apply_action_for_test("@draw:300x100");
    a.snap_apply_action_for_test("@draw:300x300");
    a.snap_apply_action_for_test("@draw:100x300");
    a.snap_apply_action_for_test("@finish_draw");

    let drawn = a.home_map.real_survey.survey_rings.len();
    assert!(drawn > 0, "도형이 그려져야 한다");

    // 2) 저장한다(환경변수로 경로 주입 — 실제 파일 창을 띄우지 않는다).
    let saved_ok = {
        let _env = EnvGuard::set("KA_HGIS_SAVE_PATH", &gpkg);
        a.save_survey_project_as()
    };
    assert!(saved_ok, "저장이 성공해야 한다");
    assert!(gpkg.exists(), "파일이 실제로 생겨야 한다");

    // 3) 새 앱에서 그 파일을 연다.
    let mut b = StrataAppState::new_startup();
    {
        let _env = EnvGuard::set("KA_HGIS_OPEN_PATH", &gpkg);
        b.handle_ribbon_click("열기");
    }
    assert_eq!(
        b.survey_path.as_deref(),
        Some(gpkg.as_path()),
        "연 파일이 조사 경로로 잡혀야 한다"
    );

    // 4) 도형이 되살아났는지 — 저장했다가 다시 열었으니 개수가 같아야 한다.
    let reopened = b.home_map.real_survey.survey_rings.len();
    assert_eq!(
        reopened, drawn,
        "다시 열면 그린 도형 {drawn}개가 그대로 있어야 한다(실제 {reopened}개)"
    );
}

/// 리본 버튼만으로 "조사 → 데이터 받기 창 → 도구 켜기" 가 이어진다.
#[test]
fn ribbon_only_workflow_reaches_each_stage() {
    let dir = TempDirGuard::new("ka_flow_ribbon");
    let mut a = StrataAppState::new_startup();

    // 조사 만들기
    a.modals.new_survey_name = "리본흐름".to_owned();
    a.modals.new_survey_dir = dir.path.to_string_lossy().into_owned();
    a.modals.new_survey_crs = strata_types::crs::WorkCrs::Epsg5187;
    a.snap_apply_action_for_test("@new_survey_go");
    assert!(a.survey_path.is_some(), "1단계: 조사가 만들어져야");

    // 자료 받기 — 창이 열린다
    a.handle_ribbon_click("지적");
    assert!(a.modals.active.is_some(), "2단계: 지적 받기 창이 열려야");
    a.snap_apply_action_for_test("@close_modal");

    // 도구 켜기
    a.handle_ribbon_click("그리기");
    assert_eq!(
        a.active_tool,
        strata_canvas2d::map_tools::MapToolKind::DrawPolygon,
        "3단계: 그리기 도구가 켜져야"
    );

    // 저장
    let p = dir.path.join("리본흐름.gpkg");
    let ok = {
        let _env = EnvGuard::set("KA_HGIS_SAVE_PATH", &p);
        a.save_survey_project_as()
    };
    assert!(ok && p.exists(), "4단계: 저장이 되어야");
}
