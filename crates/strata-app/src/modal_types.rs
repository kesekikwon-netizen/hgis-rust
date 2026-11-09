//! `dialogs_modal` 의 타입과 상태 기본값.
//!
//! 3,431줄 한 파일이던 `dialogs_modal.rs` 를 나눌 때 여기로 옮겼다.
//! `ActiveModalDialog` 는 facade 에 남겨 두었다(모든 곳에서 쓴다).

use strata_geo::trench_grid::{TrenchShapeKind, TrenchStandardLength};
use strata_layout::sheet_composer::{SheetOrientation, SheetPaperSize};
use strata_layout::tile_print::HGIS_PRINT_OVERLAP_DEFAULT_MM;
use strata_types::crs::WorkCrs;

use crate::dialogs_modal::ActiveModalDialog;
use eframe::egui::{self};

#[derive(Debug, Clone)]
pub struct ModalDialogController {
    pub active: Option<ActiveModalDialog>,
    /// KaCadImportNotice.cpp — 끌어 놓은 CAD 도면 경로
    pub cad_import_path: String,
    pub new_survey_name: String,
    pub new_survey_dir: String,
    pub new_survey_lon_deg: f64,
    pub new_survey_crs: WorkCrs,
    pub new_survey_crs_manual: bool,
    pub new_survey_error: Option<String>,
    pub contour_file_path: String,
    pub contour_interval_m: f64,
    pub contour_swap_axes: bool,
    pub contour_exclude_suspicious: bool,
    pub contour_use_breaklines: bool,
    pub contour_advanced_open: bool,
    pub contour_max_edge_m: f64,
    pub contour_cell_size_m: f64,
    pub vworld_key_input: String,
    pub heritage_id_input: String,
    pub ngii_id_input: String,
    pub trench_length: TrenchStandardLength,
    pub trench_shape: TrenchShapeKind,
    pub trench_grid_snap: bool,
    pub trench_auto_place: bool,
    pub trench_azimuth_deg: f64,
    pub trench_coverage_pct: f32,
    // D6 KaTrenchDialog.cpp / KaTrenchDialogPlan.cpp
    /// 0 = 시굴조사 10%, 1 = 표본조사 2%, 2 = 직접 지정 (KaTrenchDialog.cpp:27-155)
    pub trench_survey_kind_idx: usize,
    /// "지형 사면 방향으로 넣기(등고선 직교)"
    pub trench_slope_align: bool,
    /// 둑(간격) m
    pub trench_balk_m: f64,
    pub trench_rows: usize,
    pub trench_cols: usize,
    pub trench_prefix: String,
    // D6 KaSurveyContourDialog.cpp — 파일 좌표계 선택 (5개 라벨)
    pub contour_file_crs_idx: usize,
    pub last_export_status: String,
    pub custom_paper_mode: bool,
    pub print_single_sheet: bool,
    pub print_paper: SheetPaperSize,
    pub print_dpi: u32,
    // D11 KaPrintDialog.cpp:126-166 — 나눠 찍기 겹침 폭·자름 표시·순서 안내
    pub print_overlap_mm: f64,
    pub print_marks: bool,
    pub print_overview: bool,
    // T6 (1) KaControlPointDialog 상태
    pub cp_id: String,
    pub cp_x: String,
    pub cp_y: String,
    pub cp_z: String,
    pub cp_swap_axes: bool,
    pub cp_datum: String,
    pub cp_ellipsoid: String,
    pub cp_projection: String,
    pub cp_origin: String,
    pub cp_accuracy: String,
    pub cp_pdop: String,
    pub cp_fix: String,
    pub control_points_world: Vec<(String, [f64; 2], Option<f64>)>,
    // T6 (2) KaSurveyAreaDialog 상태
    pub survey_area_layer_name: String,
    pub survey_area_continue_idx: usize,
    pub survey_area_color_idx: usize,
    pub survey_area_width_mm: f32,
    // T6 (3) KaLineColorDialog 상태
    pub line_color_idx: usize,
    pub line_color_rgb: [u8; 3],
    pub line_width_mm: f32,
    pub applied_line_color_rgb: Option<[u8; 3]>,
    // T6 (4) KaDemClassDialog 상태
    pub dem_mode_idx: usize,
    pub dem_preset_idx: usize,
    pub dem_relief_enabled: bool,
    pub dem_exaggeration: f32,
    pub dem_relief_strength_pct: u32,
    pub dem_status_text: String,
    pub dem_applied_mode_idx: usize,
    pub dem_applied_preset_idx: usize,
    pub dem_applied_relief_enabled: bool,
    pub dem_applied_exaggeration: f32,
    pub dem_applied_relief_strength_pct: u32,
    // T8 KaHeritageBrowser ("주변유적 받기") 상태
    pub heritage_sido: String,
    pub heritage_city: String,
    pub heritage_scope_idx: usize,
    pub heritage_dataset_checks: [bool; 7],
    pub heritage_reuse_recent: bool,
    pub heritage_stage_idx: usize,
    pub heritage_progress_pct: f32,
    pub heritage_status_detail: String,
    pub heritage_show_details: bool,
    pub heritage_request_log: Vec<String>,
    pub heritage_selected_row: Option<usize>,
    // T9 KaTopographicImportDialog ("수치지형도 — 좌표계 확인과 지도 추가") 상태
    /// C++ `MainWindowTopographic.cpp:131-148` `showTopographicFiles(folder)` 의 그 폴더.
    /// 비어 있으면 아직 고르지 않았다.
    pub topo_folder: String,
    /// 폴더를 실제로 스캔했는가(`TopographicCatalog::scan` 을 한 번 돌렸는가).
    pub topo_scanned: bool,
    /// C++ `KaTopographicImportDialog::m_records` — 스캔 결과 그대로.
    pub topo_records: Vec<strata_net::topographic_catalog::Record>,
    pub topo_row_checks: Vec<bool>,
    pub topo_row_crs_idx: Vec<usize>,
    pub topo_row_already_loaded: Vec<bool>,
    pub topo_selected_row: usize,
    pub topo_maps_enabled: bool,
    pub topo_progress_pct: f32,
    pub topo_status_text: String,
    pub topo_imported_count: usize,
    /// C++ `ScanResult.filesRead` / `cacheHits` / `warnings` — 상태줄 문구용.
    pub topo_files_read: usize,
    pub topo_cache_hits: usize,
    pub topo_warnings: Vec<String>,
    pub topo_scan_error: Option<String>,
    // D4 KaTopographicBrowser ("수치지형도 내려받기") 상태
    /// false = compact summary panel, true = expanded official details
    pub topo_browser_detail: bool,
    /// 0..=5 -> "도엽 확인" .. "지도에 올리기"
    pub topo_browser_stage: usize,
    pub topo_browser_login_id: String,
    pub topo_browser_login_pw: String,
    pub topo_browser_birth_date: String,
    pub topo_browser_purpose: String,
    pub topo_browser_auto_stopped: bool,
    pub topo_browser_log: Vec<String>,
}

impl Default for ModalDialogController {
    fn default() -> Self {
        Self {
            active: None,
            cad_import_path: String::new(),
            new_survey_name: "병산동".to_owned(),
            new_survey_dir: r"C:\Users\권을\Documents".to_owned(),
            new_survey_lon_deg: 129.21,
            new_survey_crs: WorkCrs::Epsg5187,
            new_survey_crs_manual: false,
            new_survey_error: None,
            contour_file_path: "실측점_표고_248점.csv".to_owned(),
            contour_interval_m: 0.5,
            contour_swap_axes: false,
            contour_exclude_suspicious: true,
            contour_use_breaklines: true,
            contour_advanced_open: false,
            contour_max_edge_m: 50.0,
            contour_cell_size_m: 0.5,
            vworld_key_input: String::new(),
            heritage_id_input: String::new(),
            ngii_id_input: String::new(),
            trench_length: TrenchStandardLength::M20,
            trench_shape: TrenchShapeKind::Straight,
            trench_grid_snap: false,
            trench_auto_place: true,
            trench_azimuth_deg: 0.0,
            trench_coverage_pct: 10.0,
            trench_survey_kind_idx: 0,
            trench_slope_align: false,
            trench_balk_m: 10.0,
            trench_rows: 2,
            trench_cols: 2,
            trench_prefix: "Tr-".to_owned(),
            contour_file_crs_idx: 0,
            last_export_status: String::new(),
            custom_paper_mode: false,
            print_single_sheet: true,
            // KaPrintDialog.cpp:124 — 기본은 A4 다. A3 는 프린터가 A3 를 지원할 때만
            // 인쇄 창을 열 때 고른다 (`refresh_print_paper_for_printer`).
            print_paper: SheetPaperSize::A4,
            print_dpi: 300,
            print_overlap_mm: HGIS_PRINT_OVERLAP_DEFAULT_MM,
            print_marks: true,
            print_overview: true,
            cp_id: "GCP-01".to_owned(),
            cp_x: "219748.608".to_owned(),
            cp_y: "361019.219".to_owned(),
            cp_z: "48.50".to_owned(),
            cp_swap_axes: false,
            cp_datum: "세계측지계".to_owned(),
            cp_ellipsoid: "GRS80".to_owned(),
            cp_projection: "TM/UTM-K".to_owned(),
            cp_origin: "동부원점".to_owned(),
            cp_accuracy: "1.0".to_owned(),
            cp_pdop: "1.5".to_owned(),
            cp_fix: "RTK".to_owned(),
            control_points_world: Vec::new(),
            survey_area_layer_name: "조사구역".to_owned(),
            survey_area_continue_idx: 0,
            survey_area_color_idx: 1,
            survey_area_width_mm: 1.5,
            line_color_idx: 1,
            line_color_rgb: [0xEA, 0x58, 0x0C],
            line_width_mm: 1.5,
            applied_line_color_rgb: None,
            dem_mode_idx: 2,
            dem_preset_idx: 0,
            dem_relief_enabled: true,
            dem_exaggeration: 1.0,
            dem_relief_strength_pct: 30,
            dem_status_text: "지금 화면의 높이차는 약 42 m뿐이라 이 색 표현으로는 거의 한 색으로 보입니다. 「현재 화면에 색 맞추기」를 누르면 화면 안의 높이로 색을 나눕니다.".to_owned(),
            dem_applied_mode_idx: 1,
            dem_applied_preset_idx: 0,
            dem_applied_relief_enabled: true,
            dem_applied_exaggeration: 1.0,
            dem_applied_relief_strength_pct: 30,
            heritage_sido: "경상북도".to_owned(),
            heritage_city: "경주시".to_owned(),
            heritage_scope_idx: 2,
            heritage_dataset_checks: [true; 7],
            heritage_reuse_recent: false,
            heritage_stage_idx: 10,
            heritage_progress_pct: 1.0,
            heritage_status_detail: "경상북도 경주시 — 검색결과 4,569건. 내려받기 완료 (서약서 자동 동의 및 영수증 주변유적/receipts 기록됨).".to_owned(),
            heritage_show_details: false,
            heritage_request_log: vec![
                strata_net::heritage_intranet::HeritageIntranetFlow::redact_request_line(
                    "POST",
                    strata_net::heritage_intranet::HeritageIntranetFlow::LOGIN_URL,
                ),
                strata_net::heritage_intranet::HeritageIntranetFlow::redact_request_line(
                    "GET",
                    "https://intranet.gis-heritage.go.kr/intranet/codedeta/selectCodeDetaList.do?sido=47&sigungu=47130",
                ),
                strata_net::heritage_intranet::HeritageIntranetFlow::redact_request_line(
                    "POST",
                    "https://intranet.gis-heritage.go.kr/intranet/codedeta/downloadFilesAll.do",
                ),
            ],
            heritage_selected_row: Some(0),
            topo_folder: String::new(),
            topo_scanned: false,
            topo_records: Vec::new(),
            topo_row_checks: Vec::new(),
            topo_row_crs_idx: Vec::new(),
            topo_row_already_loaded: Vec::new(),
            topo_selected_row: 0,
            topo_maps_enabled: true,
            topo_progress_pct: 1.0,
            topo_status_text: String::new(),
            topo_imported_count: 0,
            topo_files_read: 0,
            topo_cache_hits: 0,
            topo_warnings: Vec::new(),
            topo_scan_error: None,
            topo_browser_detail: false,
            topo_browser_stage: 0,
            topo_browser_login_id: String::new(),
            topo_browser_login_pw: String::new(),
            topo_browser_birth_date: String::new(),
            topo_browser_purpose: "고고학 발굴조사 현장 도면 작성 및 지형 확인".to_owned(),
            topo_browser_auto_stopped: false,
            topo_browser_log: vec![
                "도엽 확인 — 현재 조사 주변 1:25,000 DXF 도엽을 고릅니다.".to_owned(),
            ],
        }
    }
}

/// C++ `KaTopographicImportDialog::recordKey` — 같은 자료를 알아보는 열쇠.
/// C++ 은 원본·레이어·서명·CAD 레이어·도형 갈래를 SHA-256 으로 묶는다(`:401-404`).
#[must_use]
pub fn topo_record_key(record: &strata_net::topographic_catalog::Record) -> String {
    format!(
        "{}|{}|{}",
        record.source, record.layer_name, record.cad_layer
    )
}

/// C++ `KaTopographicImportDialog.cpp:476` 콤보 첫 줄.
pub const TOPO_CRS_UNKNOWN: &str = "좌표계 미확인 — 선택 필요";

/// C++ `KaTopographicImportDialog.cpp:479` 후보 목록(이 순서 그대로).
pub const TOPO_CRS_CANDIDATES: [u32; 12] = [
    5185, 5186, 5187, 5188, 5173, 5174, 5175, 5176, 5177, 5178, 5179, 4326,
];

/// C++ `KaTopographicImportDialog.cpp:243-259` `crsLabel` 이 돌려주는 이름.
#[must_use]
pub const fn topo_crs_label_ko(epsg: u32) -> &'static str {
    match epsg {
        5185 => "서부원점(GRS80) · EPSG:5185",
        5186 => "중부원점(GRS80) · EPSG:5186",
        5187 => "동부원점(GRS80) · EPSG:5187",
        5188 => "동해원점(GRS80) · EPSG:5188",
        5173 => "서부원점(보정 베셀) · EPSG:5173",
        5174 => "중부원점(보정 베셀) · EPSG:5174",
        5175 => "제주원점(보정 베셀) · EPSG:5175",
        5176 => "동부원점(보정 베셀) · EPSG:5176",
        5177 => "동해원점(보정 베셀) · EPSG:5177",
        5178 => "UTM-K(베셀) · EPSG:5178",
        5179 => "UTM-K(GRS80) · EPSG:5179",
        4326 => "경위도(WGS84) · EPSG:4326",
        _ => TOPO_CRS_UNKNOWN,
    }
}

/// 표 한 줄에 그릴 값 — 창을 그리는 동안 `self` 를 빌리지 않게 미리 뽑아 둔다.
#[derive(Debug, Clone, PartialEq)]
pub struct TopoTableRow {
    /// `KaTopographicImportDialog.cpp:466` — `[검토 필요]` 를 뺀 이름.
    pub label: String,
    pub file_name: String,
    /// 있으면 `[검토 필요]` 로 표시하고 자동 체크하지 않는다(`:465-470`).
    pub review_reason: Option<String>,
    /// `:488-491` 범위 문구.
    pub extent_label: String,
    /// `:476-486` 콤보 항목. 0 번은 늘 「좌표계 미확인」, 1 번은 `.prj` 파일 정보(있을 때).
    pub choices: Vec<String>,
}

impl TopoTableRow {
    #[must_use]
    pub fn choice_label(&self, index: usize) -> String {
        self.choices
            .get(index)
            .cloned()
            .unwrap_or_else(|| TOPO_CRS_UNKNOWN.to_owned())
    }

    /// `.prj` 에서 읽은 좌표계를 콤보 자리로 바꾼다. 없으면 0(미확인).
    #[must_use]
    pub fn default_choice_index(&self, record: &strata_net::topographic_catalog::Record) -> usize {
        if record.crs_epsg().is_some() && self.choices.len() > 1 {
            1
        } else {
            0
        }
    }

    /// 고른 좌표계가 작업 좌표계 계열(GRS80 원점대)인가.
    /// C++ `previewCurrent(:498-522)` 는 실제로 좌표변환해 겹쳐 보지만, 이 창에는 지도 캔버스가
    /// 없어 같은 판단만 보인다. 베셀·경위도는 원점대가 달라 그대로 겹치지 않는다.
    #[must_use]
    pub fn choice_is_work_crs_family(&self, index: usize) -> bool {
        self.choices
            .get(index)
            .and_then(|choice| choice.rsplit_once("EPSG:"))
            .and_then(|(_, code)| code.parse::<u32>().ok())
            .is_some_and(|epsg| matches!(epsg, 5186 | 5187 | 5188 | 5185 | 5179))
    }
}

/// `.prj` 에서 읽은 EPSG 를 콤보 자리로. 못 읽었으면 0(미확인), 읽었으면 1(파일 정보).
#[must_use]
pub fn topo_default_crs_idx(epsg: Option<u32>) -> usize {
    if epsg.is_some() {
        1
    } else {
        0
    }
}

/// `KaPrintDialog.cpp:124` — 용지 기본값은 `printerSupports(A3) && !printers.isEmpty()`
/// 일 때만 A3, 아니면 A4 다. `:348-354` 대로 프린터 이름이 없거나 지원 목록이 비면 지원으로 본다.
#[must_use]
pub fn default_print_paper_for_printer() -> SheetPaperSize {
    if windows_a3_printer_supports() {
        SheetPaperSize::A3
    } else {
        SheetPaperSize::A4
    }
}

/// 설치된 프린터 중 하나라도 A3 를 지원하는지 본다. `KaPrintDialog.cpp:83-91` 처럼
/// 프린터 목록을 훑고, 목록을 못 읽으면 `:352` 대로 지원으로 친다.
#[cfg(windows)]
fn windows_a3_printer_supports() -> bool {
    use std::os::windows::process::CommandExt;

    // PowerShell 로 프린터 이름과 용지만 읽는다. 인쇄는 하지 않는다.
    let script = "[Console]::OutputEncoding=[Text.Encoding]::UTF8; \
         Get-Printer | ForEach-Object { $_.Name + '|' + (($_.PrinterPaperNames) -join ',') }";
    let mut cmd = std::process::Command::new("powershell");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", script]);
    // CREATE_NO_WINDOW — 콘솔 창이 깜빡이지 않게 한다.
    cmd.creation_flags(0x0800_0000);
    let Ok(out) = cmd.output() else {
        return true; // 목록을 못 읽으면 지원으로 본다 (`:352` sizes.isEmpty() 와 같은 뜻)
    };
    if !out.status.success() {
        return true;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let rows: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    if rows.is_empty() {
        return false; // 설치된 프린터 없음 (`:85-87`)
    }
    rows.iter().any(|row| {
        row.split_once('|')
            .is_some_and(|(_, papers)| papers.to_ascii_uppercase().contains("A3"))
    })
}

#[cfg(not(windows))]
fn windows_a3_printer_supports() -> bool {
    // 다른 OS 에는 이 확인 경로가 없다. C++ `:352` 처럼 지원으로 친다.
    true
}

impl ModalDialogController {
    /// C++ `KaLineColorDialog.cpp:48-51` — 면·래스터·음영 레이어는 선 색을 바꾸지 않는다.
    #[must_use]
    pub fn is_line_map_layer(layer_name: &str) -> bool {
        const NON_LINE: [&str; 7] = [
            "지형 음영",
            "지표모델(DSM) 30m · Coperni...",
            "토양도",
            "지질도",
            "고지형",
            "위성",
            "항공",
        ];
        !NON_LINE.iter().any(|n| layer_name.contains(n))
    }

    /// C++ `KaSurveyAreaDialog.cpp:24-43` `readableTextOn` — WCAG 상대 휘도로
    /// 배경 위에서 읽히는 글자색을 고른다(명암비 4.5 이상이면 흰색).
    #[must_use]
    pub fn readable_text_on(r: u8, g: u8, b: u8) -> egui::Color32 {
        let lin = |c: u8| -> f64 {
            let s = f64::from(c) / 255.0;
            if s <= 0.040_45 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        };
        let lum = 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
        let contrast_white = 1.05 / (lum + 0.05);
        if contrast_white >= 4.5 {
            egui::Color32::from_rgb(255, 255, 255)
        } else {
            egui::Color32::from_rgb(30, 35, 45)
        }
    }

    /// C++ `KaSubmitDialog.cpp:109-121` — 검수 실패 항목의 바로가기 버튼 이름.
    #[must_use]
    pub fn submit_action_label(action: &str) -> Option<&'static str> {
        match action {
            "open_layout" => Some("도면 열기"),
            "draw_survey_area" => Some("조사구역 그리기"),
            "draw_feature" => Some("유구 그리기"),
            "add_control_point" => Some("기준점 찍기"),
            "edit_attributes" => Some("속성 고치기"),
            "vertex_edit" => Some("꼭짓점 편집"),
            "select_tool" => Some("도형 선택"),
            "set_work_crs" => Some("좌표계 고르기"),
            _ => None,
        }
    }

    /// C++ KaClosePrompt.cpp:12-22 `withObjectParticle` — attach 을/를 to a
    /// Korean name using the Hangul batchim (final consonant) rule.
    pub fn with_object_particle(name: &str) -> String {
        match name.chars().last() {
            Some(c) if ('\u{AC00}'..='\u{D7A3}').contains(&c) => {
                let batchim = !((c as u32) - 0xAC00).is_multiple_of(28);
                format!("{name}{}", if batchim { "을" } else { "를" })
            }
            Some(c) if c.is_ascii_digit() => {
                // 0·1·3·6·7·8 read with a final consonant
                let batchim = "013678".contains(c);
                format!("{name}{}", if batchim { "을" } else { "를" })
            }
            _ => format!("{name}을(를)"),
        }
    }

    /// C++ KaClosePrompt.cpp:30-46 — heading and body of the unsaved-work prompt.
    pub fn format_close_prompt(survey_name: &str, unsaved_features: usize) -> (String, String) {
        let heading = format!("{} 저장할까요?", Self::with_object_particle(survey_name));
        let body = if unsaved_features > 0 {
            format!("저장하지 않은 작업이 {unsaved_features}건 있습니다.")
        } else {
            "저장하지 않은 작업이 있습니다.".to_owned()
        };
        (heading, body)
    }

    /// C++ `KaPaperDialog.cpp:149-153` `paperScaleNote` — A4 가로만 현장 용지를 쓴다는 안내
    #[must_use]
    pub fn paper_scale_note(paper: SheetPaperSize, orientation: SheetOrientation) -> &'static str {
        // The field sheet is larger than A4 on purpose. Say so where it is chosen:
        // printed on one A4 sheet it shrinks and the written 1:N no longer holds on paper.
        if paper == SheetPaperSize::A4 && orientation == SheetOrientation::Landscape {
            "A4 가로는 좌·우·위 1cm를 더 준 현장 용지(317×220mm)입니다. A4 종이 한 장에 찍으면 약 90%로 줄어 종이 위 축척이 달라집니다. 축척대로 찍으려면 인쇄 창에서 A3 또는 여러 장 나눠 찍기를 고르세요."
        } else {
            "기본은 A4 세로입니다. A4 가로는 좌·우·위 1cm를 더 줍니다."
        }
    }

    /// C++ `KaPaperDialog.cpp:39` `plain(mm)` — `297`, not `297.0`
    #[must_use]
    pub fn plain_mm(mm: f64) -> String {
        format!("{mm:.6}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_owned()
    }

    /// C++ `KaModalDialogController::showExclusiveModal` — 이미 활성화된 모달이 존재할 때 중복 모달 팝업을 차단하는 단일 가드
    pub fn show_exclusive_modal(&mut self, dialog: ActiveModalDialog) -> bool {
        if self.active.is_some() {
            return false;
        }
        self.active = Some(dialog);
        true
    }

    /// C++ `KaTopographicImportDialog.cpp:433-448` `scanFolder` — 폴더를 실제로 훑는다.
    /// 안내문은 `:437`, 결과는 `:449-497` `populate` 를 따른다.
    pub fn rescan_topo_folder(&mut self, folder: &std::path::Path) {
        self.topo_folder = folder.to_string_lossy().into_owned();
        self.topo_status_text =
            "받은 자료를 확인하고 있습니다… 지도 작업을 계속할 수 있습니다.".to_owned();
        self.topo_progress_pct = 0.0;
        let result = strata_net::topographic_catalog::scan_folder(folder);
        self.topo_scanned = true;
        // C++ `:450-453` — 취소·오류면 기존 목록을 건드리지 않고 문구만 바꾼다.
        if let Some(error) = result.error {
            self.topo_scan_error = Some(error.clone());
            self.topo_status_text = error;
            self.topo_progress_pct = 0.0;
            return;
        }
        self.topo_scan_error = None;
        self.topo_files_read = result.files_read;
        self.topo_cache_hits = result.cache_hits;
        self.topo_warnings = result.warnings.clone();
        // C++ `:454-459` — 이전에 사용자가 만진 체크·좌표계 선택을 같은 항목에 옮긴다.
        let previous: Vec<(String, bool, usize)> = self
            .topo_records
            .iter()
            .enumerate()
            .filter_map(|(index, record)| {
                let checked = *self.topo_row_checks.get(index)?;
                let crs_idx = *self.topo_row_crs_idx.get(index)?;
                Some((topo_record_key(record), checked, crs_idx))
            })
            .collect();
        self.topo_row_checks = result
            .records
            .iter()
            .map(|record| {
                let key = topo_record_key(record);
                previous
                    .iter()
                    .find(|(known, _, _)| *known == key)
                    .map_or_else(|| record.initially_selected(), |(_, checked, _)| *checked)
            })
            .collect();
        self.topo_row_crs_idx = result
            .records
            .iter()
            .map(|record| {
                let key = topo_record_key(record);
                previous
                    .iter()
                    .find(|(known, _, _)| *known == key)
                    .map_or_else(|| topo_default_crs_idx(record.crs_epsg()), |(_, _, index)| *index)
            })
            .collect();
        self.topo_row_already_loaded = vec![false; result.records.len()];
        self.topo_selected_row = 0;
        // C++ `:494-495` 상태줄 — 레코드를 옮기기 전에 만든다.
        let summary = result.summary_ko();
        self.topo_records = result.records;
        self.topo_progress_pct = 1.0;
        self.topo_status_text = summary;
    }

    /// 표에 그릴 줄들 — `KaTopographicImportDialog.cpp:462-491` `populate` 의 표 부분.
    #[must_use]
    pub fn topo_table_rows(&self) -> Vec<TopoTableRow> {
        self.topo_records
            .iter()
            .map(|record| {
                // C++ `:475-482` — 0 번은 「좌표계 미확인」, 1 번은 .prj 파일 정보,
                // 이어서 후보 열두 개.
                let mut choices = vec![TOPO_CRS_UNKNOWN.to_owned()];
                if let Some(epsg) = record.crs_epsg() {
                    choices.push(format!("파일 정보: {}", topo_crs_label_ko(epsg)));
                }
                for epsg in TOPO_CRS_CANDIDATES {
                    choices.push(topo_crs_label_ko(epsg).to_owned());
                }
                TopoTableRow {
                    label: record.label_ko().to_owned(),
                    file_name: std::path::Path::new(&record.source)
                        .file_name()
                        .map_or_else(
                            || record.source.clone(),
                            |name| name.to_string_lossy().into_owned(),
                        ),
                    review_reason: record.review_reason.clone(),
                    extent_label: record.extent_label_ko(),
                    choices,
                }
            })
            .collect()
    }

    /// C++ `:433-434` 색인 취소 — 목록을 비우고 문구만 남긴다.
    pub fn cancel_topo_scan(&mut self) {
        self.topo_status_text = "색인을 취소했습니다. 기존 지도는 유지됩니다.".to_owned();
        self.topo_progress_pct = 0.0;
    }

    /// 체크된 항목 가운데 올릴 수 없는 것이 있으면 C++ 문구(`:529-537`).
    /// C++ 은 범위를 먼저 보고, 이어서 **콤보에서 고른** 좌표계가 유효한지 본다.
    /// `.prj` 를 못 읽어 0(「좌표계 미확인」)으로 남은 항목은 사용자가 골라야 한다.
    #[must_use]
    pub fn topo_blocking_message(&self) -> Option<&'static str> {
        if self.topo_records.iter().enumerate().any(|(index, record)| {
            self.topo_row_checks.get(index).copied().unwrap_or(false) && !record.has_extent()
        }) {
            return Some(strata_net::topographic_catalog::IMPORT_NEEDS_EXTENT);
        }
        if self.topo_records.iter().enumerate().any(|(index, _)| {
            self.topo_row_checks.get(index).copied().unwrap_or(false)
                && self.topo_row_crs_idx.get(index).copied().unwrap_or(0) == 0
        }) {
            return Some(strata_net::topographic_catalog::IMPORT_NEEDS_CRS);
        }
        None
    }

    /// 체크된 항목의 행 번호들.
    #[must_use]
    pub fn topo_checked_rows(&self) -> Vec<usize> {
        self.topo_row_checks
            .iter()
            .enumerate()
            .filter(|(_, checked)| **checked)
            .map(|(index, _)| index)
            .collect()
    }

    /// C++ `KaModalDialogController` ESC 취소 및 미저장 입력값 존재 시 닫기 확인(`"저장되지 않은 변경사항이 있습니다. 닫으시겠습니까?"`)
    pub fn try_reject_active_modal(
        &mut self,
        confirm_on_dirty: bool,
        is_form_dirty: bool,
        user_confirmed_discard: bool,
    ) -> Result<(), &'static str> {
        if confirm_on_dirty && is_form_dirty && !user_confirmed_discard {
            return Err("저장되지 않은 변경사항이 있습니다. 닫으시겠습니까?");
        }
        self.active = None;
        Ok(())
    }
}
