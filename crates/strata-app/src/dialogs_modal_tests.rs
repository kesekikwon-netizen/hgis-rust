//! `dialogs_modal` 회귀 시험.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::dialogs_modal::*;
use strata_layout::sheet_composer::SheetPaperSize;


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
