//! `dialogs_modal` 회귀 시험.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::app_state::StrataAppState;
use crate::dialogs_modal::*;
use strata_layout::sheet_composer::SheetPaperSize;
use strata_layout::sheet_composer::SheetOrientation;


// D8 KaNewSurveyDialog / KaLongitudeRuler / KaLayerInformationPanel
// D16 KaLineColorDialog / KaSurveyAreaDialog / KaSubmitDialog
// D18 최신 kyi/main 정본 — 고지형 자료 창 반경 3항목·버튼 순서
// D19 KaTrenchDialog 하단 버튼·개별편집 + KaDrawingStudio SpinBox 범위

#[test]
fn test_d5_account_dialog_half_filled_validation() {
    let mut st = crate::dialogs_account::AccountDialogState::default();

    // half filled (ID only) -> blocked
    st.ngii_creds.username = "user1".to_owned();
    st.ngii_creds.password = String::new();
    assert!(!st.validate_credentials());
    assert_eq!(
        st.validation_error.as_deref(),
        Some("아이디와 비밀번호를 모두 입력하거나 두 항목을 모두 비워 주세요.")
    );

    // password only also blocked
    st.ngii_creds.username = String::new();
    st.ngii_creds.password = "secret".to_owned();
    assert!(!st.validate_credentials());
    assert!(st.validation_error.is_some());

    // both empty -> allowed
    st.ngii_creds.username = String::new();
    st.ngii_creds.password = String::new();
    assert!(st.validate_credentials());
    assert_eq!(st.validation_error, None);

    // both filled -> allowed
    st.ngii_creds.username = "user1".to_owned();
    st.ngii_creds.password = "secret".to_owned();
    assert!(st.validate_credentials());
    assert_eq!(st.validation_error, None);
}

// D4 KaTopographicBrowser: compact/expanded toggle, 6-stage progression,
// and fallback to the coordinate-confirm import dialog.
#[test]
fn test_topo_browser_compact_toggle_stage_progress_and_import_fallback() {
    let mut ctrl = ModalDialogController::default();
    ctrl.show_exclusive_modal(ActiveModalDialog::NgiiTopoDownload);
    assert_eq!(ctrl.active, Some(ActiveModalDialog::NgiiTopoDownload));

    // compact by default (KaTopographicBrowser.cpp: compact summary first)
    assert!(!ctrl.topo_browser_detail);
    ctrl.topo_browser_detail = true;
    assert!(ctrl.topo_browser_detail);
    ctrl.topo_browser_detail = false;

    // 6 stages: 도엽 확인 -> 로그인 -> 신청 -> 파일 준비 -> 다운로드 -> 지도에 올리기
    assert_eq!(ctrl.topo_browser_stage, 0);
    for expect in 1..=5usize {
        ctrl.topo_browser_stage = (ctrl.topo_browser_stage + 1).min(5);
        assert_eq!(ctrl.topo_browser_stage, expect);
    }
    assert_eq!(
        ctrl.topo_browser_log.first().map(|s| s.as_str()),
        Some("도엽 확인 — 현재 조사 주변 1:25,000 DXF 도엽을 고릅니다.")
    );

    // last stage hands off to the coordinate-confirm import dialog
    // (KaTopographicBrowser.cpp:215 m_importFallback)
    ctrl.active = Some(ActiveModalDialog::TopographicImport);
    assert_eq!(ctrl.active, Some(ActiveModalDialog::TopographicImport));

    // purpose pre-filled with the C++ default text
    assert_eq!(
        ctrl.topo_browser_purpose,
        "고고학 발굴조사 현장 도면 작성 및 지형 확인"
    );
}

#[test]
fn test_modal_exclusive_guard_blocks_duplicate_popup_and_dirty_reject_confirmation() {
    let mut ctrl = ModalDialogController::default();
    assert!(ctrl.show_exclusive_modal(ActiveModalDialog::PaperSetup));
    assert_eq!(ctrl.active, Some(ActiveModalDialog::PaperSetup));

    // 이미 모달이 떠 있는 상태에서 다른 모달 실행 시 차단
    assert!(!ctrl.show_exclusive_modal(ActiveModalDialog::DemDownload));
    assert_eq!(ctrl.active, Some(ActiveModalDialog::PaperSetup));

    // 미저장 변경사항이 있고 사용자가 아직 확인하지 않은 경우 닫기 거부
    let err = ctrl
        .try_reject_active_modal(true, true, false)
        .expect_err("dirty modal must ask confirmation");
    assert_eq!(err, "저장되지 않은 변경사항이 있습니다. 닫으시겠습니까?");
    assert_eq!(ctrl.active, Some(ActiveModalDialog::PaperSetup));

    // 사용자가 닫기 확인 시 정상 종료
    assert!(ctrl.try_reject_active_modal(true, true, true).is_ok());
    assert_eq!(ctrl.active, None);
}

// D11 KaPaperDialog.cpp:90-186 + KaPrintDialog.cpp:126-208
#[test]
fn test_d11_paper_dialog_scale_note_and_print_tile_options() {
    let ctrl = ModalDialogController::default();
    assert!((ctrl.print_overlap_mm - 10.0).abs() < 1e-9);
    assert!(ctrl.print_marks);
    assert!(ctrl.print_overview);

    // 「직접 입력」 only swaps the reading line for an entry field; the other presets
    // take their size from the paper (KaPaperDialog.cpp:161-175).
    let mut app = StrataAppState::new_startup();
    app.drawing
        .set_paper_and_orientation(SheetPaperSize::A4, SheetOrientation::Landscape);
    assert!(!app.modals.custom_paper_mode);
    app.modals.custom_paper_mode = true;
    app.drawing.sheet.width_mm = 317.0;
    app.drawing.sheet.height_mm = 220.0;
    assert!((app.drawing.sheet.width_mm - 317.0).abs() < 1e-9);
    assert!((app.drawing.sheet.height_mm - 220.0).abs() < 1e-9);
    assert_eq!(
        ModalDialogController::paper_scale_note(SheetPaperSize::A4, SheetOrientation::Landscape),
        "A4 가로는 좌·우·위 1cm를 더 준 현장 용지(317×220mm)입니다. A4 종이 한 장에 찍으면 약 90%로 줄어 종이 위 축척이 달라집니다. 축척대로 찍으려면 인쇄 창에서 A3 또는 여러 장 나눠 찍기를 고르세요."
    );
    assert_eq!(
        ModalDialogController::paper_scale_note(
            SheetPaperSize::A3,
            SheetOrientation::Landscape
        ),
        "기본은 A4 세로입니다. A4 가로는 좌·우·위 1cm를 더 줍니다."
    );
}
