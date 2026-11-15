//! 리본 겹침 토글 시험 — 원래 `mod tests_overlay_toggles` 를 그대로 옮겼다.

use crate::app_state::StrataAppState;
use crate::dialogs_modal::ActiveModalDialog;

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

/// R8 — 「DEM」은 `MainWindowOverlay.cpp:303-348`. 없으면 받고(C++ 은 :344 에서 Copernicus 를 받는
/// 창을 연다; 여기서는 국토정보플랫폼 공개DEM 을 숨은 주문으로 받는다, 소유자 2026-11-15), 있으면
/// :337-340 두 문구로 켜고 끈다.
#[test]
fn test_r8_dem_orders_when_absent_and_toggles_when_present() {
    let mut app = StrataAppState::new_startup();
    app.handle_ribbon_click("DEM");
    assert_eq!(app.modals.active, None, "no window: the order runs hidden");
    assert_eq!(app.home_map.draw_guide_message.as_deref(), Some("먼저 새 조사를 만들거나 기존 조사를 여세요. 조사 폴더 안의 DEM 폴더에 저장합니다."));
    assert!(!app.topographic_download_busy() && app.download_jobs.is_empty());

    // A check mark that an earlier survey left behind is no DEM: the press still goes for the order.
    // Only the legend row of the DEM makes the button a switch.
    app.home_map
        .real_survey
        .layer_visible
        .insert("DEM".to_owned(), true);
    app.handle_ribbon_click("DEM");
    assert!(app.home_map.real_survey.is_layer_visible("DEM"), "nothing was there to hide");
    assert_ne!(app.status_bar_hint, "DEM 지형을 숨겼습니다.");

    let spec = crate::ref_vector::data::LoadSpec { file: std::path::PathBuf::new(), table: String::new(), work_epsg: 5187, look: crate::ref_vector::data::Look::Heights { ramp: crate::ref_vector::data::HeightRamp::National }, label: None };
    app.show_reference_vector_layer(crate::topo_download::dem::DEM_KEY, "DEM", spec);
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

/// 지질 and 고지형 are plain buttons in the C++ (`MainWindowRibbon.cpp:462-470`, `:501-507`): a right click
/// opens nothing. The menus this app had offered downloads that were not there (survey MENU0, 2026-11-15:
/// rows 8-BX2, 8-BX3, 9-BX1, 9-BX2). 토양 keeps its two rows (`:447-452`).
#[test]
fn geology_and_paleo_have_no_menu() {
    let mut app = StrataAppState::new_startup();
    let chip = eframe::egui::Rect::from_min_size(eframe::egui::Pos2::ZERO, eframe::egui::vec2(60.0, 80.0));
    for label in ["지질", "고지형"] {
        app.open_ribbon_context_menu(label, chip);
        assert!(!app.ribbon.geology_menu_open && !app.ribbon.paleo_menu_open, "{label}");
    }
    app.open_ribbon_context_menu("토양", chip);
    assert!(app.ribbon.soil_menu_open);
}

/// The checkable buttons 토양, 지질 and 수계 look pressed while their map is shown
/// (`MainWindowOverlay.cpp:669-673` syncThematicButtons).
#[test]
fn a_reference_map_button_looks_pressed_while_its_map_is_shown() {
    use crate::ref_vector::map::{GEOLOGY, RIVER, SOIL};
    let mut app = StrataAppState::new_startup();
    for kind in [&GEOLOGY, &SOIL, &RIVER] {
        assert!(!app.reference_map_shown(kind), "{}", kind.title);
        let spec = crate::ref_vector::data::LoadSpec { file: std::path::PathBuf::new(), table: kind.table.to_owned(), work_epsg: 5187, look: kind.look.clone(), label: None };
        app.show_reference_vector_layer(kind.key, kind.title, spec);
        assert!(app.reference_map_shown(kind), "{}", kind.title);
        app.press_reference_map(kind);
        assert!(!app.reference_map_shown(kind), "hidden by its button: {}", kind.title);
    }
}
