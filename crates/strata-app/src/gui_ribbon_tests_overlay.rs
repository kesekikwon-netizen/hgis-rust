//! 리본 겹침 토글 시험 — 원래 `mod tests_overlay_toggles` 를 그대로 옮겼다.

use crate::app_state::StrataAppState;
use crate::dialogs_modal::ActiveModalDialog;

/// R8 — C++ `MainWindowOverlay.cpp:225-240` `toggleExistingOverlay`.
/// 없는 레이어면 `false` 이고 아무 문구도 내지 않는다.
#[test]
fn test_r8_toggle_existing_overlay_returns_false_when_absent() {
    let mut app = StrataAppState::new_startup();
    assert!(!app.toggle_existing_overlay("지질도(KIGAM 1:5만)"));
    assert_eq!(app.status_bar_hint, "");
}

/// R8 — 「수계」는 `MainWindowOverlay.cpp:479` 가 `수계도(하천망)` 이름으로 토글한다.
/// (「토양」은 실제로 받는다: `ref_map_tests_soil.rs`.)
#[test]
fn test_r8_river_toggles_with_cpp_layer_name() {
    let (button, title) = ("수계", "수계도(하천망)");
    let mut app = StrataAppState::new_startup();
    // 없는 상태: 받아 오는 길(여기서는 안내 문구)로 간다.
    app.handle_ribbon_click(button);
    assert!(app.home_map.real_survey.is_layer_visible(title), "{button}");
    assert!(app.status_bar_hint.contains("추가했습니다"), "{button}: {}", app.status_bar_hint);

    // 있는 상태: 끄고 켜며 C++ toggleExistingOverlay 문구를 쓴다.
    app.handle_ribbon_click(button);
    assert!(!app.home_map.real_survey.is_layer_visible(title), "{button}");
    assert_eq!(app.status_bar_hint, format!("{title}를 껐습니다."));

    app.handle_ribbon_click(button);
    assert!(app.home_map.real_survey.is_layer_visible(title), "{button}");
    assert_eq!(app.status_bar_hint, format!("{title}를 다시 켰습니다."));
}

/// R8 — 「지질」 fetches a real map (ref_map.rs, order NET-GEO1): a press that cannot fetch adds no layer
/// and says what to do first (`MainWindowDownloads.cpp:93-103`), instead of calling a map "added" that was
/// never fetched. The hide-and-show of `MainWindowOverlay.cpp:409` is tested on a fetched map in
/// ref_map_tests.rs; the relief under it (`:422-423`) returns with order NET-GEO1c.
#[test]
fn test_r8_geology_press_without_a_survey_adds_nothing() {
    let mut app = StrataAppState::new_startup();
    app.handle_ribbon_click("지질");
    assert!(!app.home_map.real_survey.layer_visible.contains_key("지질도(KIGAM 1:5만)"));
    assert!(app.left_layers.entries.iter().all(|e| e.label_ko != "지질도(KIGAM 1:5만)"));
    assert_eq!(
        app.home_map.draw_guide_message.as_deref(),
        Some("먼저 「새 조사」를 만들거나 조사를 여세요. 받은 자료는 조사 폴더에 저장해 다음에도 인터넷 없이 열 수 있게 합니다.")
    );
    assert!(app.download_jobs.is_empty());
}

/// R8 — 「DEM」은 `MainWindowOverlay.cpp:303-348`. 없으면 받는 창(:344),
/// 있으면 :337-340 두 문구로 켜고 끈다.
#[test]
fn test_r8_dem_opens_download_when_absent_and_toggles_when_present() {
    let mut app = StrataAppState::new_startup();
    app.handle_ribbon_click("DEM");
    assert_eq!(app.modals.active, Some(ActiveModalDialog::DemDownload));

    app.modals.active = None;
    app.home_map
        .real_survey
        .layer_visible
        .insert("DEM".to_owned(), true);
    app.handle_ribbon_click("DEM");
    assert!(!app.home_map.real_survey.is_layer_visible("DEM"));
    assert_eq!(app.status_bar_hint, "DEM 지형을 숨겼습니다.");
    // :330-332 — 지질도가 꺼져 있으면 지형 음영도 함께 내린다.
    assert!(!app.home_map.real_survey.is_layer_visible("지형 음영"));

    app.handle_ribbon_click("DEM");
    assert!(app.home_map.real_survey.is_layer_visible("DEM"));
    assert_eq!(
        app.status_bar_hint,
        "DEM을 켰습니다. 「DEM 표현」에서 색과 음영을 조절하세요."
    );
}

/// R8 — 「등고선」은 토글이 아니다(`MainWindowRibbon.cpp:429-436` 에 setCheckable 없음).
/// `MainWindowSurveyContour.cpp:104` 도 도메인 레이어를 만들지 않는다 — 빈 단면선
/// 레이어를 자동으로 넣으면 「빈 도메인 레이어 자동 추가 금지」 규칙에 어긋난다.
#[test]
fn test_r8_contour_opens_dialog_without_creating_a_domain_layer() {
    let mut app = StrataAppState::new_startup();
    app.handle_ribbon_click("등고선");
    assert_eq!(app.modals.active, Some(ActiveModalDialog::SurveyContour));
    assert!(
        app.left_layers.entries.is_empty(),
        "등고선은 빈 도메인 레이어를 만들지 않는다: {:?}",
        app.left_layers.entries.len()
    );
}
