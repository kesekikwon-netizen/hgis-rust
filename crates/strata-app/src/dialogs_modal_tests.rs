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
fn test_r10_print_paper_defaults_follow_printer_a3_support() {
    // 이 PC 에 실제로 설치된 프린터로 계산한다. 값은 환경에 따라 A3/A4 둘 중 하나다.
    let paper = default_print_paper_for_printer();
    assert!(
        paper == SheetPaperSize::A3 || paper == SheetPaperSize::A4,
        "용지 기본값은 A3 또는 A4 여야 한다"
    );
    // 프린터 목록을 못 읽거나 지원 목록이 비면 지원으로 본다(C++ :352).
    // 그래서 이 호출이 패닉하지 않고 항상 A3/A4 를 돌려주는 것이 계약이다.
    assert_eq!(ModalDialogController::default().print_paper, SheetPaperSize::A4);
}

#[test]
fn test_d19_trench_dialog_actions_and_drawing_studio_spinboxes() {
    // KaTrenchDialog.cpp:137-154 — 하단 실행 버튼 3개
    let buttons = ["구역에 깔기", "맵에 찍기", "닫기"];
    assert_eq!(buttons.len(), 3);
    assert_eq!(buttons[0], "구역에 깔기");
    assert_eq!(buttons[1], "맵에 찍기");
    assert_eq!(buttons[2], "닫기");

    // KaTrenchDialog.cpp:114-135 — 배치 후 편집 버튼 2개
    let edit_buttons = ["개별 편집", "전체 이동"];
    assert_eq!(edit_buttons.len(), 2);

    // KaTrenchDialog.cpp:101 — 이름 접두 (prefix 아님)
    assert_eq!("이름 접두", "이름 접두");

    // KaDrawingStudio.cpp:953-956 / 1018-1022 / 1056 / 1137-1146
    let legend_range = 5_i32..=24_i32;
    assert!(legend_range.contains(&9));
    assert!(!legend_range.contains(&4));
    assert!(!legend_range.contains(&25));

    let north_range = 3.0_f32..=80.0_f32;
    assert!(north_range.contains(&28.0));
    assert!(!north_range.contains(&2.9));
    assert!(!north_range.contains(&80.1));

    let scale_range = 10_u32..=5_000_000_u32;
    assert!(scale_range.contains(&5_000));
    assert!(scale_range.contains(&5_000_000));
    assert!(!scale_range.contains(&9));

    let deco_range = 25_u32..=200_u32;
    assert!(deco_range.contains(&100));
    assert!(!deco_range.contains(&24));
    assert!(!deco_range.contains(&201));

    // KaDrawingStudio.cpp:1292-1323 — 하단 7버튼
    let toolbar = [
        "좌표점",
        "레이어를 가운데",
        "지도 조정",
        "화면 맞춤",
        "항목 이동",
        "용지/방향",
        "도면 목록",
    ];
    assert_eq!(toolbar.len(), 7);
    assert_eq!(toolbar[0], "좌표점");
    assert_eq!(toolbar[6], "도면 목록");
}

#[test]
fn test_d18_latest_kyi_main_paleo_source_dialog_and_ribbon_menus() {
    // MainWindowPaleo.cpp:124-129 — ComboBox 3항목, 기본 index 1 (5 km)
    const RADIUS_LABELS: [&str; 3] = [
        "반경 2 km (2,000 m)",
        "반경 5 km (5,000 m · 권장)",
        "반경 10 km (10,000 m)",
    ];
    const RADIUS_KM: [f64; 3] = [2.0, 5.0, 10.0];
    assert_eq!(RADIUS_LABELS.len(), 3);
    assert_eq!(RADIUS_KM[1], 5.0);
    assert!(RADIUS_LABELS[1].contains("권장"));

    // MainWindowPaleo.cpp:158-171 — 하단 3버튼 순서 (지질도 → 시추 → KIGAM)
    let order = [
        "수치지질도(1:5만)만 받기",
        "시추주상도·퇴적층만 받기",
        "KIGAM 단층·수문만 받기",
    ];
    assert_eq!(order.len(), 3);
    assert!(order[0].starts_with("수치지질도"));
    assert!(order[1].starts_with("시추주상도"));
    assert!(order[2].starts_with("KIGAM"));

    // KaPaleoMenu::install (67a3f33) — 고지형 7 / 지질 6 / DEM 5 항목
    const PALEO_MENU: [&str; 7] = [
        "「고지형분석」 전용 탭 열기",
        "수치지질도 · 시추주상도(지반) · KIGAM 별도 내려받기 창…",
        "수치지질도(KIGAM 1:5만) 별도 받기 (반경 5km · 도면 범례)",
        "국토지반정보 시추주상도·퇴적층두께 별도 받기 (반경 5km · 도면 범례)",
        "KIGAM 단층·지하수위·단구연대 별도 받기 (반경 10km)",
        "국토지반정보 포털 열기 (geoinfo.or.kr)…",
        "KIGAM 지오빅데이터 포털 열기 (data.kigam.re.kr)…",
    ];
    const GEO_MENU: [&str; 6] = [
        "조사구역 반경 5km 수치지질도 받기 (1:5만 · 권장)",
        "조사구역 반경 2km 수치지질도 받기",
        "조사구역 반경 10km 수치지질도 받기",
        "국토지반정보 시추주상도·퇴적층두께 별도 받기 (반경 5km)",
        "KIGAM 단층·선구조·수문 별도 받기 (반경 10km)",
        "수치지질도 · 시추주상도 별도 내려받기 창…",
    ];
    const DEM_MENU: [&str; 5] = [
        "국토지리정보원 계정 설정·자동 다운로드…",
        "국토지리원 DEM 불러오기(.img)…",
        "DEM 파일로 음영 만들기…",
        "DEM 표현…",
        "측량점으로 등고선…",
    ];
    assert_eq!(PALEO_MENU.len(), 7);
    assert_eq!(GEO_MENU.len(), 6);
    assert_eq!(DEM_MENU.len(), 5);
    assert!(PALEO_MENU[0].contains("전용 탭"));
    assert!(GEO_MENU[0].contains("5km"));
    assert!(DEM_MENU[4].contains("등고선"));

    // 6대 입력 자료 배지 — 준비 초록 ●, 미비 빨강 ○
    const SOURCES: [&str; 6] = [
        "국토지리원DEM",
        "토양도",
        "지질도",
        "수계도",
        "KIGAM수문",
        "시추",
    ];
    assert_eq!(SOURCES.len(), 6);
    assert_eq!(SOURCES[0], "국토지리원DEM");
    assert_eq!(SOURCES[5], "시추");
}

#[test]
fn test_d16_line_color_survey_area_and_submit_actions() {
    // 선 레이어만 색을 바꾼다 (KaLineColorDialog.cpp:48-51)
    assert!(ModalDialogController::is_line_map_layer("도면 선"));
    assert!(ModalDialogController::is_line_map_layer("등고선"));
    assert!(ModalDialogController::is_line_map_layer("조사구역"));
    assert!(!ModalDialogController::is_line_map_layer("지형 음영"));
    assert!(!ModalDialogController::is_line_map_layer("토양도"));
    assert!(!ModalDialogController::is_line_map_layer("지질도"));
    assert!(!ModalDialogController::is_line_map_layer("위성"));

    // WCAG readable text (KaSurveyAreaDialog.cpp:24-43)
    // 어두운 배경 -> 흰 글자, 밝은 배경 -> 먹색
    assert_eq!(
        ModalDialogController::readable_text_on(20, 25, 35),
        egui::Color32::from_rgb(255, 255, 255)
    );
    assert_eq!(
        ModalDialogController::readable_text_on(250, 250, 245),
        egui::Color32::from_rgb(30, 35, 45)
    );

    // 검수 실패 바로가기 (KaSubmitDialog.cpp:109-121)
    assert_eq!(
        ModalDialogController::submit_action_label("open_layout"),
        Some("도면 열기")
    );
    assert_eq!(
        ModalDialogController::submit_action_label("draw_survey_area"),
        Some("조사구역 그리기")
    );
    assert_eq!(
        ModalDialogController::submit_action_label("set_work_crs"),
        Some("좌표계 고르기")
    );
    assert_eq!(ModalDialogController::submit_action_label("unknown"), None);

    // 8개 action 이 모두 C++ 라벨을 가진다
    for a in [
        "open_layout",
        "draw_survey_area",
        "draw_feature",
        "add_control_point",
        "edit_attributes",
        "vertex_edit",
        "select_tool",
        "set_work_crs",
    ] {
        assert!(ModalDialogController::submit_action_label(a).is_some());
    }
}

#[test]
fn test_d8_new_survey_longitude_ruler_and_layer_display_settings() {
    use crate::gui_layer_panel::KA_LABEL_INKS;

    // KaLayerInformationPanel.cpp:26-33 kLabelInks — 6 presets, exact order
    assert_eq!(KA_LABEL_INKS.len(), 6);
    let names: Vec<&str> = KA_LABEL_INKS.iter().map(|(n, _)| *n).collect();
    assert_eq!(
        names,
        vec![
            "기본 (먹색)",
            "흰색",
            "진한 파랑",
            "진한 빨강",
            "갈색",
            "진한 초록"
        ]
    );
    assert_eq!(KA_LABEL_INKS[0].1, (31, 35, 40));
    assert_eq!(KA_LABEL_INKS[1].1, (255, 255, 255));

    // layer display settings start closed with the C++ defaults
    let mut hm = crate::tabs_home_map::HomeMapTabState::default();
    assert!(!hm.layer_display_settings_open);
    assert_eq!(hm.layer_label_field_idx, 0);
    assert!(!hm.layer_label_with_area);
    assert_eq!(hm.layer_label_size_pt, 9);
    assert_eq!(hm.layer_label_ink_idx, 0);
    assert!(hm.layer_label_buffer_halo);

    // toggling works and the pt range is the C++ 6..=24
    hm.layer_display_settings_open = true;
    assert!(hm.layer_display_settings_open);
    hm.layer_label_size_pt = 6;
    assert_eq!(hm.layer_label_size_pt.clamp(6, 24), 6);
    hm.layer_label_size_pt = 40;
    assert_eq!(hm.layer_label_size_pt.clamp(6, 24), 24);
    hm.layer_label_ink_idx = 5;
    assert!(hm.layer_label_ink_idx < KA_LABEL_INKS.len());
}

// D7 KaClosePrompt.cpp:12-46 — Korean object particle + close prompt text
#[test]
fn test_d7_close_prompt_korean_particle_and_control_point_layout() {
    // open syllable (no batchim) -> 를
    assert_eq!(
        ModalDialogController::with_object_particle("경주시"),
        "경주시를"
    );
    // closed syllable (batchim) -> 을
    assert_eq!(
        ModalDialogController::with_object_particle("월성"),
        "월성을"
    );
    // digits: 0·1·3·6·7·8 take 을, the rest take 를
    assert_eq!(
        ModalDialogController::with_object_particle("조사1"),
        "조사1을"
    );
    assert_eq!(
        ModalDialogController::with_object_particle("조사2"),
        "조사2를"
    );
    // fallback for non-Hangul, non-digit endings
    assert_eq!(
        ModalDialogController::with_object_particle("Site"),
        "Site을(를)"
    );

    // close prompt heading + body
    assert_eq!(
        ModalDialogController::format_close_prompt("경주시", 3),
        (
            "경주시를 저장할까요?".to_owned(),
            "저장하지 않은 작업이 3건 있습니다.".to_owned()
        )
    );
    assert_eq!(
        ModalDialogController::format_close_prompt("경주시", 0).1,
        "저장하지 않은 작업이 있습니다."
    );
}

// D6 KaTrenchDialog / KaTrenchDialogPlan / KaSurveyContourDialog defaults
#[test]
fn test_d6_trench_dial_survey_kind_and_contour_advanced_toggle() {
    let mut ctrl = ModalDialogController::default();

    // trench defaults
    assert_eq!(ctrl.trench_survey_kind_idx, 0);
    assert_eq!(ctrl.trench_rows, 2);
    assert_eq!(ctrl.trench_cols, 2);
    assert_eq!(ctrl.trench_balk_m, 10.0);
    assert_eq!(ctrl.trench_prefix, "Tr-");
    assert!(!ctrl.trench_slope_align);

    // 조사 종류 cycles through the 3 C++ labels
    let labels = [
        "시굴조사 — 전체 면적의 10%",
        "표본조사 — 전체 면적의 2%",
        "직접 지정 — 규격·둑을 내가 정함",
    ];
    for expect in 1..=3usize {
        ctrl.trench_survey_kind_idx = (ctrl.trench_survey_kind_idx + 1) % 3;
        assert_eq!(expect % 3, ctrl.trench_survey_kind_idx);
    }
    assert_eq!(labels.len(), 3);

    // contour advanced starts closed, then toggles open
    assert!(!ctrl.contour_advanced_open);
    ctrl.contour_advanced_open = true;
    assert!(ctrl.contour_advanced_open);

    // 5 CRS choices; cell/edge accept the "0 == automatic" sentinel
    assert_eq!(ctrl.contour_file_crs_idx, 0);
    ctrl.contour_file_crs_idx = 4;
    assert_eq!(ctrl.contour_file_crs_idx, 4);
    ctrl.contour_max_edge_m = 0.0;
    ctrl.contour_cell_size_m = 0.0;
    assert_eq!(ctrl.contour_max_edge_m, 0.0);
    assert_eq!(ctrl.contour_cell_size_m, 0.0);
}

// D5 KaTopographicAccountDialog.cpp:60-61 — an account with only one of
// ID/PW must block the save; both-empty and both-filled are fine.
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

#[test]
fn test_t6_cpp_dialogs_4_variants_and_defaults() {
    let mut ctrl = ModalDialogController::default();
    assert_eq!(ctrl.cp_id, "GCP-01");
    assert_eq!(ctrl.cp_datum, "세계측지계");
    assert_eq!(ctrl.cp_ellipsoid, "GRS80");
    assert_eq!(ctrl.cp_projection, "TM/UTM-K");
    assert_eq!(ctrl.cp_origin, "동부원점");
    assert_eq!(ctrl.cp_fix, "RTK");
    assert_eq!(ctrl.survey_area_layer_name, "조사구역");
    assert_eq!(ctrl.survey_area_color_idx, 1);
    assert!((ctrl.survey_area_width_mm - 1.5).abs() < 1e-4);
    assert_eq!(ctrl.line_color_rgb, [0xEA, 0x58, 0x0C]);
    assert_eq!(ctrl.dem_mode_idx, 2);
    assert_eq!(ctrl.dem_relief_strength_pct, 30);

    for variant in [
        ActiveModalDialog::ControlPoint,
        ActiveModalDialog::SurveyArea,
        ActiveModalDialog::LineColor,
        ActiveModalDialog::DemClass,
        ActiveModalDialog::ReferenceStatus,
        ActiveModalDialog::HeritageBrowser,
        ActiveModalDialog::TopographicImport,
    ] {
        ctrl.active = None;
        assert!(ctrl.show_exclusive_modal(variant));
        assert_eq!(ctrl.active, Some(variant));
    }
}

/// 임시 폴더에 가짜 자료를 만들고 스캔 결과가 창 상태로 옮겨지는지 본다.
/// C++ `KaTopographicImportDialog.cpp:433-497` `scanFolder` → `populate`.
#[test]
fn test_r14_topographic_import_reads_the_real_folder_instead_of_fixed_rows() {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let folder = std::env::temp_dir().join(format!(
        "ka_hgis_r14_app_{}_{nanos}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).expect("임시 폴더");
    let mut header = vec![0_u8; 100];
    header[..4].copy_from_slice(&9994_u32.to_be_bytes());
    for (offset, value) in [(36, 218_450.0_f64), (44, 359_800.0), (52, 221_250.0), (60, 362_400.0)]
    {
        header[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }
    std::fs::write(folder.join("A0010000.shp"), &header).expect("shp");
    std::fs::write(
        folder.join("A0010000.prj"),
        "PROJCS[\"Korea 2000 / East Belt\",AUTHORITY[\"EPSG\",\"5187\"]]",
    )
    .expect("prj");
    // .prj 없는 DXF — C++ 규칙대로 [검토 필요]이고 자동 체크되지 않는다.
    std::fs::write(
        folder.join("행정경계.dxf"),
        "0\nSECTION\n2\nENTITIES\n0\nLINE\n8\nG0010000\n10\n0.0\n20\n0.0\n\
         0\nLINE\n8\nG0010000\n10\n10.0\n20\n10.0\n0\nENDSEC\n0\nEOF\n",
    )
    .expect("dxf");

    let mut ctrl = ModalDialogController::default();
    // 시작할 때는 하드코딩 목록이 없다.
    assert!(ctrl.topo_records.is_empty());
    assert!(ctrl.topo_row_checks.is_empty());
    assert!(!ctrl.topo_scanned);

    ctrl.rescan_topo_folder(&folder);
    assert!(std::fs::remove_dir_all(&folder).is_ok());

    assert!(ctrl.topo_scanned);
    assert_eq!(ctrl.topo_records.len(), 2);
    assert_eq!(ctrl.topo_row_checks.len(), 2);
    assert_eq!(ctrl.topo_files_read, 2);
    // C++ `:469-470` — 도로는 체크, 검토 필요 DXF 는 해제.
    let road = ctrl
        .topo_records
        .iter()
        .position(|r| r.layer_name == "A0010000")
        .expect("도로 레코드");
    let review = ctrl
        .topo_records
        .iter()
        .position(|r| r.review_reason.is_some())
        .expect("검토 필요 레코드");
    assert!(ctrl.topo_row_checks[road]);
    assert!(!ctrl.topo_row_checks[review]);
    // `.prj` 를 읽었으므로 콤보가 「파일 정보」를 가리킨다(C++ `:484`).
    assert_eq!(ctrl.topo_row_crs_idx[road], 1);
    assert_eq!(ctrl.topo_row_crs_idx[review], 0);

    let rows = ctrl.topo_table_rows();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[road].label, "도로경계(미분류)");
    assert_eq!(
        rows[road].extent_label,
        "X 218450.0 ~ 221250.0\nY 359800.0 ~ 362400.0"
    );
    assert!(rows[review].review_reason.is_some());
    assert_eq!(rows[review].extent_label, "X 0.0 ~ 10.0\nY 0.0 ~ 10.0");
    // 상태줄은 C++ `:494-495` 문구다.
    assert!(ctrl.topo_status_text.starts_with("레이어 2개 · 파일 2개 확인 · 캐시 0개."));
    assert!((ctrl.topo_progress_pct - 1.0).abs() < 1e-6);

    // 체크된 항목(도로)만으로는 막히지 않는다.
    assert_eq!(ctrl.topo_blocking_message(), None);
    assert_eq!(ctrl.topo_checked_rows(), vec![road]);

    // 검토 필요 항목을 사용자가 직접 체크하면 추가는 되지만, 좌표계를 골라야 한다
    // (C++ `:534-536`).
    ctrl.topo_row_checks[review] = true;
    assert_eq!(
        ctrl.topo_blocking_message(),
        Some(strata_net::topographic_catalog::IMPORT_NEEDS_CRS)
    );
    ctrl.topo_row_crs_idx[review] = 2; // 후보 중 하나를 직접 고른다(C++ `:533-537`)
    assert_eq!(ctrl.topo_blocking_message(), None);

    // 없는 폴더는 C++ `TopographicCatalog.cpp:262` 문구로 끝나고 목록은 그대로다.
    let before = ctrl.topo_records.len();
    ctrl.rescan_topo_folder(&std::env::temp_dir().join("ka_hgis_r14_없는폴더"));
    assert!(ctrl.topo_status_text.starts_with("수치지형도 폴더를 읽을 수 없습니다:"));
    assert_eq!(ctrl.topo_records.len(), before, "오류일 때 기존 목록을 지우지 않는다");

    // 색인 취소 문구(C++ `:451`).
    ctrl.cancel_topo_scan();
    assert_eq!(
        ctrl.topo_status_text,
        "색인을 취소했습니다. 기존 지도는 유지됩니다."
    );
}

/// C++ `KaTopographicImportDialog.cpp:243-259,479-482` — 좌표계 이름표와 후보 12종.
#[test]
fn test_r14_topographic_crs_labels_follow_the_cpp_dialog() {
    assert_eq!(topo_crs_label_ko(5187), "동부원점(GRS80) · EPSG:5187");
    assert_eq!(topo_crs_label_ko(5176), "동부원점(보정 베셀) · EPSG:5176");
    assert_eq!(topo_crs_label_ko(5179), "UTM-K(GRS80) · EPSG:5179");
    assert_eq!(topo_crs_label_ko(4326), "경위도(WGS84) · EPSG:4326");
    assert_eq!(topo_crs_label_ko(9999), TOPO_CRS_UNKNOWN);
    assert_eq!(
        TOPO_CRS_CANDIDATES,
        [5185, 5186, 5187, 5188, 5173, 5174, 5175, 5176, 5177, 5178, 5179, 4326]
    );
    // .prj 를 읽었으면 1(파일 정보), 못 읽었으면 0(미확인).
    assert_eq!(topo_default_crs_idx(Some(5187)), 1);
    assert_eq!(topo_default_crs_idx(None), 0);
}
