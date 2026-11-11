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

/// R8 — 「지형」: C++ `MainWindowOverlay.cpp:243-271`.
/// 처음 누르면 :267 문구(올림), 다음부터는 :231/:236 문구(껐다 켰다).
#[test]
fn test_r8_terrain_toggle_uses_cpp_first_and_repeat_texts() {
    let mut app = StrataAppState::new_startup();
    app.handle_ribbon_click("지형");
    assert!(app.home_map.real_survey.is_layer_visible("지형맵"));
    assert_eq!(app.status_bar_hint, "지형맵을 올렸습니다. 다시 누르면 숨깁니다.");

    app.handle_ribbon_click("지형");
    assert!(!app.home_map.real_survey.is_layer_visible("지형맵"));
    assert_eq!(app.status_bar_hint, "지형맵를 껐습니다.");

    app.handle_ribbon_click("지형");
    assert!(app.home_map.real_survey.is_layer_visible("지형맵"));
    assert_eq!(app.status_bar_hint, "지형맵를 다시 켰습니다.");
}

/// 「토양」·「수계」 받기는 아직 Rust 판에 없다(「지질」은 옮겼다 — `reference_geology`). 예전에는 아무것도 받지 않고
/// 「추가했습니다」라고 했다. 이제는 레이어를 올린 척하지 않고 없다고 알린다.
/// (받기를 옮기면 이 시험을 C++ `MainWindowOverlay.cpp:357,409,479` 의 토글 시험으로 되돌린다.)
#[test]
fn test_r8_soil_and_river_say_they_are_not_built_yet_and_add_nothing() {
    for (button, title) in [
        ("토양", "토양도(흙토람)"),
        ("수계", "수계도(하천망)"),
    ] {
        let mut app = StrataAppState::new_startup();
        app.handle_ribbon_click(button);
        assert!(
            !app.home_map.real_survey.layer_visible.contains_key(title),
            "{button}: 받지 않은 레이어를 올린 것처럼 기록하면 안 된다"
        );
        assert!(
            app.status_bar_hint.contains("아직 Rust 판에 옮기지 못했습니다")
                && !app.status_bar_hint.contains("추가했습니다"),
            "{button}: {}",
            app.status_bar_hint
        );
    }
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
