//! UI 버튼이 실제로 상태를 바꾸는지 검사하는 시험.
//!
//! 사용자 지적(2026-11-09): "UI를 누르면 되는 게 하나도 없는데".
//!
//! 리본 그룹 단추는 `gui_ribbon_top` → `handle_ribbon_click(label)` 로 들어온다.
//! 탭 바(홈·지도·도면…)는 `gui_ribbon_tabbar` 가 직접 `active_tab` 을 바꾼다.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use strata_types::schema::DomainLayerKey;

use crate::app_state::{MainTab, StrataAppState};

/// 파일 대화상자를 여는 단추는 시험에서 누르면 안 된다(창이 떠서 멈춘다).
///
/// C++ 도 같은 동작이다: 조사 파일이 없으면 「저장」이 `saveProjectAs()` 로
/// 파일 대화상자를 연다(`MainWindowSession.cpp:1409-1418`).
const OPENS_FILE_DIALOG: [&str; 4] = ["열기", "저장", "다른 이름", "폴더"];

/// 리본 단추 중 **파일 대화상자를 여는 것 말고는** 눌렀을 때 뭔가 바뀌어야 한다.
///
/// 확인하는 상태 필드(하나라도 바뀌면 "동작함"):
/// 탭 · 모달 · 안내문 · 레이어 수 · 서브툴바 · 사용 도구 · 상태줄 힌트 · 레이어 표시.
#[test]
fn test_ribbon_buttons_change_state() {
    use crate::gui_ribbon_bar::UX_RIBBON_GROUPS;

    let mut silent: Vec<&str> = Vec::new();
    for (_group, items) in UX_RIBBON_GROUPS {
        for label in items {
            if OPENS_FILE_DIALOG.contains(label) {
                continue; // 파일 창은 시험에서 열 수 없다
            }
            let mut app = StrataAppState::new_startup();
            app.active_tab = MainTab::Map;

            let before = (
                app.active_tab,
                app.modals.active,
                app.home_map.draw_guide_message.clone(),
                app.left_layers.entries.len(),
                app.ribbon.sub_toolbar_visible,
                app.active_tool,
                app.status_bar_hint.clone(),
                app.home_map.real_survey.layer_visible.len(),
            );

            app.handle_ribbon_click(label);

            let after = (
                app.active_tab,
                app.modals.active,
                app.home_map.draw_guide_message.clone(),
                app.left_layers.entries.len(),
                app.ribbon.sub_toolbar_visible,
                app.active_tool,
                app.status_bar_hint.clone(),
                app.home_map.real_survey.layer_visible.len(),
            );
            if before == after {
                silent.push(label);
            }
        }
    }

    assert!(
        silent.is_empty(),
        "아무 것도 하지 않는 리본 단추: {silent:?}\n\
         눌렀을 때 탭·모달·안내문·레이어·도구·상태줄 중 하나는 바뀌어야 한다."
    );
}

/// 「새 조사」는 새 조사 모달을 연다.
#[test]
fn test_ribbon_new_survey_opens_modal() {
    let mut app = StrataAppState::new_startup();
    app.handle_ribbon_click("새 조사");
    assert!(
        app.modals.active.is_some(),
        "리본 「새 조사」를 눌렀는데 모달이 열리지 않았다"
    );
}

/// 「저장」은 조사 파일이 있으면 실제로 파일을 쓴다(대화상자 없이).
#[test]
fn test_ribbon_save_writes_existing_survey_file() {
    let mut app = StrataAppState::new_startup();
    app.start_new_survey_project("저장시험");
    let path = app.survey_path.clone().expect("새 조사에 조사 파일이 없다");
    assert!(path.exists(), "새 조사 파일이 실제로 만들어지지 않았다");

    app.is_dirty = true;
    app.handle_ribbon_click("저장");
    assert!(!app.is_dirty, "저장했는데 아직 변경됨 상태다");
    assert!(path.exists(), "저장 후 파일이 사라졌다");
}

/// 탭 바가 하는 일 — `active_tab` 대입으로 화면이 실제로 바뀐다.
#[test]
fn test_tab_switch_changes_active_tab() {
    let mut app = StrataAppState::new_startup();
    assert_eq!(app.active_tab, MainTab::Home, "시작은 홈이어야 한다");

    app.active_tab = MainTab::Map;
    assert_eq!(app.active_tab, MainTab::Map);

    app.open_studio_tab(MainTab::Drawing);
    assert!(
        app.open_studio_tabs.contains(&MainTab::Drawing),
        "도면 탭이 열리지 않았다"
    );
    app.open_studio_tab(MainTab::Paleo);
    assert!(
        app.open_studio_tabs.contains(&MainTab::Paleo),
        "고지형 탭이 열리지 않았다"
    );
}

/// 레이어 패널의 체크박스 토글이 실제로 표시 상태를 바꾼다.
#[test]
fn test_layer_visibility_toggle_changes_state() {
    let mut app = StrataAppState::new_startup();
    app.left_layers
        .ensure_domain_layer(DomainLayerKey::SurveyArea);
    let name = app.left_layers.entries[0].label_ko.clone();
    let before = app.home_map.real_survey.layer_check_state(&name);

    app.home_map.real_survey.toggle_layer_visible(&name);
    let after = app.home_map.real_survey.layer_check_state(&name);

    assert_ne!(
        before, after,
        "레이어 표시 토글을 눌렀는데 상태가 그대로다 ({name})"
    );
}

/// 새 조사 후 위성·지적 체크박스가 실제로 조작 가능해야 한다.
#[test]
fn test_basemap_layer_visibility_is_toggleable() {
    let mut app = StrataAppState::new_startup();
    app.start_new_survey_project("토글확인");

    for key in ["vworld_satellite", "vworld_cadastral_wms"] {
        let entry = app
            .left_layers
            .entries
            .iter()
            .find(|e| e.layer_key == key)
            .unwrap_or_else(|| panic!("{key} 레이어가 없다"));
        let name = entry.label_ko.clone();
        let before = app.home_map.real_survey.layer_check_state(&name);
        app.home_map.real_survey.toggle_layer_visible(&name);
        let after = app.home_map.real_survey.layer_check_state(&name);
        assert_ne!(before, after, "{key} 표시 토글이 먹히지 않는다");
    }
}

/// Ctrl+N(임시 폴더 새 조사)은 부를 때마다 새 폴더에 만든다. 같은 이름 파일이 있어도 그
/// 파일로 계속하지 않는다(C++ `SurveyProjectFactory.cpp:44-49` 거절을 지킨다).
#[test]
fn temp_new_survey_never_continues_on_an_existing_file() {
    let mut app = StrataAppState::new_startup();
    app.start_new_survey_project("임시중복");
    let first = app.survey_path.clone().expect("first survey path");
    let first_bytes = std::fs::read(&first).expect("first survey file");
    app.start_new_survey_project("임시중복");
    let second = app.survey_path.clone().expect("second survey path");

    assert_ne!(first, second, "the second new survey must not reuse the first file");
    assert!(second.starts_with(std::env::temp_dir()));
    assert_eq!(std::fs::read(&first).expect("first survey file"), first_bytes);
    for p in [&first, &second] {
        if let Some(d) = p.parent().filter(|d| *d != std::env::temp_dir()) {
            let _ = std::fs::remove_dir_all(d);
        }
    }
}

/// `--snap` 의 「@new_survey_go」는 언제나 `%TEMP%\strata-snap-<pid>` 에 만들고(문서 폴더
/// 금지), 같은 이름이 있어 거절되면 오류를 창에 남기고 멈춘다.
#[test]
fn snap_new_survey_uses_temp_snap_folder_and_stops_on_refusal() {
    use crate::dialogs_modal::ActiveModalDialog;
    let elsewhere = tempfile::tempdir().expect("temp dir");
    let snap_dir = std::env::temp_dir().join(format!("strata-snap-{}", std::process::id()));
    let mut app = StrataAppState::new_startup();
    app.modals.new_survey_name = "스냅조사".to_owned();
    app.modals.new_survey_dir = elsewhere.path().to_string_lossy().into_owned();
    app.modals.active = Some(ActiveModalDialog::NewSurvey);

    app.snap_apply_action("@new_survey_go");
    let made = snap_dir.join("스냅조사.gpkg");
    assert_eq!(app.survey_path.as_deref(), Some(made.as_path()));
    assert!(!elsewhere.path().join("스냅조사.gpkg").exists());

    app.modals.active = Some(ActiveModalDialog::NewSurvey);
    app.snap_apply_action("@new_survey_go");
    assert_eq!(app.modals.active, Some(ActiveModalDialog::NewSurvey), "refusal keeps the dialog");
    assert!(app.modals.new_survey_error.is_some(), "refusal must be shown");
    let _ = std::fs::remove_dir_all(&snap_dir);
}

/// 새 조사 창은 C++ 처럼 만들기 오류 문구(`SurveyProjectFactory.cpp` errorOut)를 앞말 없이 보인다.
#[test]
fn new_survey_dialog_shows_storage_errors_verbatim() {
    let msg = "새 조사 파일을 준비할 수 없습니다. 저장 폴더의 쓰기 권한과 남은 공간을 확인해 주세요.";
    let err = strata_types::error::StrataError::Storage(msg.to_owned());
    assert_eq!(crate::dlg_new_survey::new_survey_error_text(&err), msg);
}
