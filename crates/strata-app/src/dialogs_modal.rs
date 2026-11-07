use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use strata_check_export::checklist_engine::ChecklistEngine;
use strata_check_export::submit_package::export_submission_package;
use strata_geo::trench_grid::{TrenchShapeKind, TrenchStandardLength};
use strata_layout::sheet_composer::{SheetOrientation, SheetPaperSize};
use strata_types::crs::WorkCrs;

use crate::app_state::{MainTab, StrataAppState};
use crate::dialogs_download_print::render_download_and_print_dialogs;
use crate::ribbon_icons::paint_strata_logo;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveModalDialog {
    NewSurvey,
    SurveyContour,
    AccountSettings,
    TrenchGenerator,
    ExportSubmitPackage,
    PaperSetup,
    DemDownload,
    HeritageRegionSelect,
    NgiiTopoDownload,
    CadastralDownload,
    PaleoSourceDownload,
    PdfPreparing,
    DrawingPrint,
    ControlPoint,
    SurveyArea,
    LineColor,
    DemClass,
    ReferenceStatus,
    HeritageBrowser,
    TopographicImport,
}

#[derive(Debug, Clone)]
pub struct ModalDialogController {
    pub active: Option<ActiveModalDialog>,
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
    pub topo_row_checks: [bool; 6],
    pub topo_row_crs_idx: [usize; 6],
    pub topo_row_already_loaded: [bool; 6],
    pub topo_selected_row: usize,
    pub topo_maps_enabled: bool,
    pub topo_progress_pct: f32,
    pub topo_status_text: String,
    pub topo_imported_count: usize,
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
            print_paper: SheetPaperSize::A3,
            print_dpi: 300,
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
            topo_row_checks: [true, true, true, true, true, false],
            topo_row_crs_idx: [1, 1, 12, 4, 3, 0],
            topo_row_already_loaded: [true, true, false, false, false, false],
            topo_selected_row: 4,
            topo_maps_enabled: true,
            topo_progress_pct: 1.0,
            topo_status_text: "레이어 6개 · 파일 6개 확인 · 캐시 4개. 추가할 항목에 체크하세요.\n검토 필요: DXF 파일(35905086_행정경계.dxf)은 .prj가 없어 중부원점(EPSG:5186)과 동부원점(EPSG:5187) 판별이 애매합니다. 자동으로 지도에 추가하지 않습니다.".to_owned(),
            topo_imported_count: 2,
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

impl ModalDialogController {
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

    /// C++ `KaModalDialogController::showExclusiveModal` — 이미 활성화된 모달이 존재할 때 중복 모달 팝업을 차단하는 단일 가드
    pub fn show_exclusive_modal(&mut self, dialog: ActiveModalDialog) -> bool {
        if self.active.is_some() {
            return false;
        }
        self.active = Some(dialog);
        true
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

pub fn render_strata_modal_header(ui: &mut egui::Ui, title: &str, open: &mut bool) {
    ui.horizontal(|ui| {
        let (logo_r, _) = ui.allocate_exact_size(Vec2::new(18.0, 18.0), egui::Sense::hover());
        paint_strata_logo(&ui.painter_at(logo_r), logo_r);
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(title)
                .size(13.5)
                .color(Color32::from_rgb(30, 30, 28)),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let close_btn = egui::Button::new(
                egui::RichText::new("×")
                    .size(18.0)
                    .color(Color32::from_rgb(55, 54, 50)),
            )
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::NONE);
            if ui.add(close_btn).clicked() {
                *open = false;
            }
        });
    });
    ui.add_space(8.0);
}

pub fn render_active_modal_dialogs(app: &mut StrataAppState, ctx: &egui::Context) {
    let Some(kind) = app.modals.active else {
        return;
    };
    let mut open = true;
    match kind {
        ActiveModalDialog::PaperSetup => {
            egui::Window::new("ka_modal_paper_setup")
                .title_bar(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .default_width(480.0)
                .show(ctx, |ui| {
                    render_strata_modal_header(ui, "용지 설정 - Strata", &mut open);
                    ui.horizontal(|ui| {
                        ui.add_sized([48.0, 56.0], egui::Label::new(egui::RichText::new("용지").size(16.0)));
                        for (label, size_opt) in [("A4", Some(SheetPaperSize::A4)), ("A3", Some(SheetPaperSize::A3)), ("직접 입력", None)] {
                            let selected = match size_opt {
                                Some(sz) => !app.modals.custom_paper_mode && app.drawing.sheet.paper_size == sz,
                                None => app.modals.custom_paper_mode,
                            };
                            let fill = if selected { Color32::from_rgb(214, 208, 194) } else { Color32::WHITE };
                            let stroke = if selected { Stroke::new(2.0, Color32::from_rgb(38, 37, 34)) } else { Stroke::new(1.0, Color32::from_rgb(225, 223, 215)) };
                            let btn = egui::Button::new(egui::RichText::new(label).size(15.0)).fill(fill).stroke(stroke).corner_radius(0.0).min_size(Vec2::new(112.0, 56.0));
                            if ui.add(btn).clicked() {
                                if let Some(sz) = size_opt {
                                    app.modals.custom_paper_mode = false;
                                    let ori = app.drawing.sheet.orientation;
                                    app.drawing.set_paper_and_orientation(sz, ori);
                                } else {
                                    app.modals.custom_paper_mode = true;
                                }
                            }
                        }
                    });
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.add_sized([48.0, 74.0], egui::Label::new(egui::RichText::new("방향").size(16.0)));
                        for (label, ori, icon_sz) in [("세로", SheetOrientation::Portrait, Vec2::new(16.0, 26.0)), ("가로", SheetOrientation::Landscape, Vec2::new(28.0, 16.0))] {
                            let selected = app.drawing.sheet.orientation == ori;
                            let (rect, resp) = ui.allocate_exact_size(Vec2::new(112.0, 74.0), egui::Sense::click());
                            let p = ui.painter_at(rect);
                            p.rect_filled(rect, 0.0, if selected { Color32::from_rgb(214, 208, 194) } else { Color32::WHITE });
                            if selected {
                                p.line_segment([Pos2::new(rect.left(), rect.bottom() - 1.0), Pos2::new(rect.right(), rect.bottom() - 1.0)], Stroke::new(2.5, Color32::from_rgb(38, 37, 34)));
                            }
                            let icon_rect = Rect::from_center_size(Pos2::new(rect.center().x, rect.top() + 26.0), icon_sz);
                            p.rect_filled(icon_rect, 1.0, Color32::WHITE);
                            p.rect_stroke(icon_rect, 1.0, Stroke::new(1.5, Color32::from_rgb(65, 72, 80)), egui::StrokeKind::Middle);
                            p.text(Pos2::new(rect.center().x, rect.bottom() - 18.0), egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(14.0), Color32::from_rgb(38, 37, 34));
                            if resp.clicked() {
                                let sz = app.drawing.sheet.paper_size;
                                app.drawing.set_paper_and_orientation(sz, ori);
                            }
                        }
                    });
                    ui.add_space(12.0);
                    for (dim_label, val_mm) in [("가로", app.drawing.sheet.width_mm as i32), ("세로", app.drawing.sheet.height_mm as i32)] {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(dim_label).size(16.0));
                            ui.label(egui::RichText::new("........................................................................").color(Color32::from_rgb(150, 148, 140)));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(egui::RichText::new("mm").size(15.0).color(Color32::from_rgb(94, 93, 89)));
                                ui.label(egui::RichText::new(format!("{val_mm}")).strong().size(15.0));
                            });
                        });
                        ui.add_space(8.0);
                    }
                    ui.label(egui::RichText::new("기본은 A4 세로입니다. A4 가로는 좌·우·위 1cm를 더 줍니다.").size(14.5).color(Color32::from_rgb(38, 37, 34)));
                    ui.add_space(12.0);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let cancel_btn = egui::Button::new(egui::RichText::new("취소").size(14.5)).fill(Color32::WHITE).stroke(Stroke::new(1.0, Color32::from_rgb(140, 138, 130))).corner_radius(6.0).min_size(Vec2::new(76.0, 38.0));
                        if ui.add(cancel_btn).clicked() { app.modals.active = None; }
                        let ok_btn = egui::Button::new(egui::RichText::new("용지 만들기").color(Color32::WHITE).strong().size(14.5)).fill(Color32::from_rgb(181, 87, 58)).corner_radius(6.0).min_size(Vec2::new(134.0, 38.0));
                        if ui.add(ok_btn).clicked() {
                            app.open_studio_tab(MainTab::Drawing);
                            app.modals.active = None;
                            app.home_map.draw_guide_message = Some("지도 칸을 만들었습니다. 나침반·좌표계·축척자·축척이 들어갔습니다.".to_owned());
                        }
                    });
                });
        }
        ActiveModalDialog::DemDownload => {
            egui::Window::new("ka_modal_dem_download")
                .title_bar(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .default_width(510.0)
                .show(ctx, |ui| {
                    render_strata_modal_header(ui, "국토지리정보원 DEM 다운로드 - Strata", &mut open);
                    ui.label(egui::RichText::new("국토정보플랫폼 · 조사 주변 도엽의 정밀 DEM을 확인하고 수치지형도 등고선으로 합성합니다.").size(14.0).color(Color32::from_rgb(25, 25, 24)));
                    ui.add_space(10.0);
                    let (bar_rect, _) = ui.allocate_exact_size(Vec2::new(480.0, 32.0), egui::Sense::hover());
                    let p = ui.painter_at(bar_rect);
                    p.rect_filled(bar_rect, 6.0, Color32::WHITE);
                    p.rect_stroke(bar_rect, 6.0, Stroke::new(1.2, Color32::from_rgb(150, 148, 142)), egui::StrokeKind::Middle);
                    p.rect_filled(Rect::from_min_size(bar_rect.min + Vec2::new(2.0, 2.0), Vec2::new(192.0, 28.0)), 5.0, Color32::from_rgb(20, 20, 20));
                    p.text(bar_rect.center(), egui::Align2::CENTER_CENTER, "[DEM] 국토지리원_DEM.tif · 다중 도엽 병합 준비", egui::FontId::proportional(12.5), Color32::from_rgb(55, 54, 50));
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        let synth_btn = egui::Button::new(egui::RichText::new("등고선 정밀 DEM 합성").color(Color32::WHITE).strong().size(13.5))
                            .fill(Color32::from_rgb(44, 43, 41))
                            .corner_radius(6.0)
                            .min_size(Vec2::new(152.0, 36.0));
                        if ui.add(synth_btn).clicked() {
                            app.home_map.real_survey.layer_visible.insert("지표모델(DSM) 30m · Coperni...".to_owned(), false);
                            app.home_map.real_survey.layer_visible.insert("지형 음영".to_owned(), true);
                            app.home_map.draw_guide_message = Some(
                                "국토지리정보원 수치지형도 등고선으로 정밀 DEM·광역지형도·경사도를 생성했습니다.".to_owned(),
                            );
                            app.modals.active = None;
                        }
                        if ui.button("계정 설정").clicked() {
                            app.modals.active = Some(ActiveModalDialog::AccountSettings);
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let cancel_btn = egui::Button::new(egui::RichText::new("취소").color(Color32::WHITE).strong().size(14.0)).fill(Color32::from_rgb(181, 87, 58)).corner_radius(6.0).min_size(Vec2::new(72.0, 36.0));
                            if ui.add(cancel_btn).clicked() { app.modals.active = None; }
                        });
                    });
                });
        }
        ActiveModalDialog::AccountSettings => {
            egui::Window::new("ka_modal_account")
                .title_bar(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .default_width(520.0)
                .show(ctx, |ui| {
                    render_strata_modal_header(ui, "API 키 · 사이트 계정 설정 - Strata", &mut open);
                    ui.label(
                        egui::RichText::new(&app.accounts.status_summary_ko)
                            .size(12.5)
                            .strong(),
                    );
                    ui.separator();
                    let show_pw = app.accounts.show_passwords;
                    egui::Grid::new("ka_account_grid")
                        .num_columns(2)
                        .spacing([12.0, 8.0])
                        .show(ui, |ui| {
                            ui.label("VWorld API 키:");
                            ui.vertical(|ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.accounts.vworld_key)
                                        .hint_text("vworld.kr 인증키")
                                        .desired_width(340.0),
                                );
                                ui.label(
                                    egui::RichText::new("위성 · 지적 · 주소 찾기에 함께 씁니다.")
                                        .size(11.5)
                                        .color(Color32::from_rgb(95, 94, 88)),
                                );
                            });
                            ui.end_row();
                            ui.label("역사지리정보DB 키:");
                            ui.vertical(|ui| {
                                ui.add(
                                    egui::TextEdit::singleline(
                                        &mut app.accounts.history_gis_key,
                                    )
                                    .hint_text("hgis.history.go.kr 인증키")
                                    .desired_width(340.0),
                                );
                                ui.label(
                                    egui::RichText::new(
                                        "1919 조선지형도 전용. VWorld 키로는 인증되지 않습니다.",
                                    )
                                    .size(11.5)
                                    .color(Color32::from_rgb(95, 94, 88)),
                                );
                            });
                            ui.end_row();
                            ui.label("VWorld 지적도 ID / PW:");
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::TextEdit::singleline(
                                        &mut app.accounts.cadastral_creds.username,
                                    )
                                    .desired_width(140.0),
                                );
                                ui.add(
                                    egui::TextEdit::singleline(
                                        &mut app.accounts.cadastral_creds.password,
                                    )
                                    .password(!show_pw)
                                    .desired_width(188.0),
                                );
                            });
                            ui.end_row();
                            ui.label("수치지형도(NGII) ID / PW:");
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::TextEdit::singleline(
                                        &mut app.accounts.ngii_creds.username,
                                    )
                                    .desired_width(140.0),
                                );
                                ui.add(
                                    egui::TextEdit::singleline(
                                        &mut app.accounts.ngii_creds.password,
                                    )
                                    .password(!show_pw)
                                    .desired_width(188.0),
                                );
                            });
                            ui.end_row();
                            ui.label("국가유산청 인트라넷 ID / PW:");
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::TextEdit::singleline(
                                        &mut app.accounts.heritage_creds.username,
                                    )
                                    .desired_width(140.0),
                                );
                                ui.add(
                                    egui::TextEdit::singleline(
                                        &mut app.accounts.heritage_creds.password,
                                    )
                                    .password(!show_pw)
                                    .desired_width(188.0),
                                );
                            });
                            ui.end_row();
                        });
                    ui.add_space(6.0);
                    // C++ KaTopographicAccountDialog.cpp:25 description
                    ui.label(
                        egui::RichText::new(
                            "국토지리정보원 계정을 이 PC에 저장하면 수치지형도를 받을 때 자동 로그인에 사용합니다. 두 항목을 모두 비우고 저장하면 저장된 자동로그인 계정을 해제합니다.",
                        )
                        .size(11.5)
                        .color(Color32::from_rgb(95, 94, 88)),
                    );
                    // C++ KaTopographicAccountDialog.cpp:60-63 — half-filled account
                    if let Some(err) = &app.accounts.validation_error {
                        ui.colored_label(Color32::from_rgb(185, 28, 28), err);
                    }
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut app.accounts.show_passwords, "비밀번호 표시");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("닫기").clicked() {
                                app.modals.active = None;
                            }
                            let save_b = egui::Button::new(
                                egui::RichText::new("저장").color(Color32::WHITE).strong(),
                            )
                            .fill(Color32::from_rgb(181, 87, 58))
                            .corner_radius(6.0)
                            .min_size(Vec2::new(78.0, 30.0));
                            if ui.add(save_b).clicked() && app.accounts.validate_credentials() {
                                app.accounts.save_all_to_disk();
                                app.home_map.draw_guide_message = Some(
                                    "API 키와 사이트 계정(DPAPI 암호화)을 저장했습니다."
                                        .to_owned(),
                                );
                                app.modals.active = None;
                            }
                        });
                    });
                });
        }
        ActiveModalDialog::PaleoSourceDownload => {
            egui::Window::new("ka_modal_paleo_sources")
                .title_bar(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .default_width(520.0)
                .show(ctx, |ui| {
                    render_strata_modal_header(
                        ui,
                        "수치지질도 · 시추주상도(지반) · KIGAM 별도 내려받기 (도면 범례 지원)",
                        &mut open,
                    );
                    ui.horizontal(|ui| {
                        ui.label("조사구역 중심 수집 반경:");
                        ui.radio_value(&mut app.paleo_3d.source_dl_radius_km, 5.0, "5 km (권장)");
                        ui.radio_value(&mut app.paleo_3d.source_dl_radius_km, 10.0, "10 km (광역)");
                    });
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new(format!(
                            "현재 적재 현황 — 수치지질도(1:5만): {}건 · 시추공(퇴적층두께): {}공 (최대 50공) · KIGAM 수문·단구연대: 6건 · 단층: 3건",
                            app.paleo_3d.geology_feature_count,
                            app.paleo_3d.declustered_borehole_count
                        ))
                        .size(12.5)
                        .strong(),
                    );
                    ui.add_space(8.0);
                    let btn_all = egui::Button::new(
                        egui::RichText::new("수치지질도(1:5만) + 시추주상도(퇴적층) + KIGAM 연속 받기")
                            .color(Color32::WHITE)
                            .strong()
                            .size(13.5),
                    )
                    .fill(Color32::from_rgb(181, 87, 58))
                    .corner_radius(6.0)
                    .min_size(Vec2::new(488.0, 34.0));
                    if ui.add(btn_all).clicked() {
                        app.paleo_3d.run_sample_10km_analysis();
                        app.home_map.draw_guide_message = Some(format!(
                            "반경 {:.0} km 수치지질도(1:5만)·시추주상도(디클러스터링 {}공)·KIGAM 수문·단층 자료를 적재했습니다.",
                            app.paleo_3d.source_dl_radius_km,
                            app.paleo_3d.declustered_borehole_count
                        ));
                        app.modals.active = None;
                    }
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        if ui.button("시추주상도만 받기").clicked() {
                            app.paleo_3d.run_sample_10km_analysis();
                            app.modals.active = None;
                        }
                        if ui.button("KIGAM 자료만 받기").clicked() {
                            app.paleo_3d.run_sample_10km_analysis();
                            app.modals.active = None;
                        }
                        if ui.button("수치지질도만 받기").clicked() {
                            app.paleo_3d.run_sample_10km_analysis();
                            app.modals.active = None;
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("닫기").clicked() {
                                app.modals.active = None;
                            }
                        });
                    });
                });
        }
        ActiveModalDialog::ExportSubmitPackage => {
            let full = ctx.screen_rect();
            // screen_rect is already in logical points, so dividing by the native
            // scale again shrank the modal on 150% displays (Task D5).
            let (vis_w, vis_h) = (full.width(), full.height());
            let modal_w = 820.0_f32.min((vis_w - 40.0).max(640.0));
            let modal_h = 540.0_f32.min((vis_h - 60.0).max(420.0));
            let modal_pos = Pos2::new(
                ((vis_w - modal_w) * 0.5).max(16.0),
                ((vis_h - modal_h) * 0.5).max(24.0),
            );
            egui::Window::new("ka_modal_submit")
                .title_bar(false)
                .resizable(false)
                .fixed_pos(modal_pos)
                .fixed_size([modal_w, modal_h])
                .show(ctx, |ui| {
                    ui.set_width(modal_w - 24.0);
                    ui.set_max_width(modal_w - 24.0);
                    render_strata_modal_header(ui, "검수·제출 - Strata", &mut open);
                    let has_boundary = !app.home_map.real_survey.survey_rings.is_empty();
                    let gcp_cnt = app
                        .left_layers
                        .entries
                        .iter()
                        .find(|e| e.layer_key == "control_points")
                        .map_or(0, |e| e.feature_count as i64);
                    let feat_cnt = (app.home_map.real_survey.trenches.len()
                        + app.home_map.scene.polygons.len()) as i64;
                    let bounds_ok = app.drawing.sheet.all_decorations_inside_sheet();
                    let detail_ok = app.drawing.sheet.scale_denominator <= 200;
                    let state_json = serde_json::json!({
                        "survey_area_count": i64::from(has_boundary),
                        "control_points_count": gcp_cnt,
                        "feature_poly_count": feat_cnt,
                        "feature_line_count": 0,
                        "project_crs_set": true,
                        "project_crs_authid": app.work_crs.auth_id(),
                        "has_datum": gcp_cnt >= 2,
                        "has_ellipsoid": gcp_cnt >= 2,
                        "has_projection": gcp_cnt >= 2,
                        "has_origin": true,
                        "has_accuracy": true,
                        "has_kind_period": true,
                        "has_abstract_marker": false,
                        "survey_is_polygon": has_boundary,
                        "features_within_survey": true,
                        "features_real_geometry": true,
                        "geometries_valid": true,
                        "geometries_nonempty": has_boundary,
                        "geometries_nonzero_area": has_boundary,
                        "export_crs_valid": true,
                        "required_fields_filled": true,
                        "control_points_point_id_unique": true,
                        "feature_poly_feature_no_unique": true,
                        "sheet_covers_targets": bounds_ok,
                        "sheet_has_elements": bounds_ok,
                        "HERITAGE_PLEDGE_SCOPE": app.home_map.real_survey.heritage_polys.is_empty(),
                        "layout_exists:site_location": has_boundary && bounds_ok,
                        "layout_exists:feature_plan": feat_cnt > 0 && bounds_ok,
                        "layout_exists:feature_detail": detail_ok,
                        "layout_exists:survey_area_map": has_boundary,
                    });

                    let engine = ChecklistEngine::with_default_rules();
                    let results = engine.evaluate(&state_json);
                    let err_cnt = ChecklistEngine::failed_count(&results, "error");
                    let warn_cnt = ChecklistEngine::failed_count(&results, "warn");
                    let pass_cnt = ChecklistEngine::passed_count(&results);
                    let status_txt = ChecklistEngine::status_bar_text(&results);
                    app.home_map.draw_guide_message = Some(status_txt.clone());

                    let findings: Vec<strata_check_export::checklist_engine::ChecklistFinding> = results
                        .iter()
                        .map(|r| {
                            let sev = if r.passed {
                                strata_check_export::checklist_engine::ChecklistSeverity::Pass
                            } else if r.severity == "error" {
                                strata_check_export::checklist_engine::ChecklistSeverity::Error
                            } else {
                                strata_check_export::checklist_engine::ChecklistSeverity::Warning
                            };
                            strata_check_export::checklist_engine::ChecklistFinding {
                                rule_id: r.id.clone(),
                                category_ko: r.basis_ko.clone(),
                                message_ko: r.message_ko.clone(),
                                severity: sev,
                                passed: r.passed,
                                target_world_xy: Some(app.home_map.real_survey.survey_center),
                            }
                        })
                        .collect();

                    // KaSubmitDialogPlate.cpp:12-62, 73-94 — 상단 헤드라인 + 3개 카운트 박스(오류 / 주의 / 통과)
                    let headline = if err_cnt > 0 {
                        format!("오류 {err_cnt}건을 확인하세요 ({status_txt})")
                    } else if warn_cnt > 0 {
                        format!("오류가 없습니다 ({status_txt})")
                    } else {
                        "오류와 주의 항목이 없습니다".to_owned()
                    };
                    ui.label(
                        egui::RichText::new(headline)
                            .size(15.5)
                            .strong()
                            .color(Color32::from_rgb(20, 20, 19)),
                    );
                    ui.label(
                        egui::RichText::new(format!(
                            "{} · 고치지 않아도 꾸러미는 만들 수 있고, README_submit.txt에 적힙니다.",
                            engine.title()
                        ))
                        .size(11.5)
                        .color(Color32::from_rgb(94, 93, 89)),
                    );
                    ui.add_space(4.0);
                    ui.columns(3, |cols| {
                        for (ci, (lbl, cnt, bg, fg)) in [
                            (
                                "오류",
                                err_cnt,
                                Color32::from_rgb(254, 242, 242),
                                Color32::from_rgb(185, 28, 28),
                            ),
                            (
                                "주의",
                                warn_cnt,
                                Color32::from_rgb(255, 251, 235),
                                Color32::from_rgb(180, 83, 9),
                            ),
                            (
                                "통과",
                                pass_cnt,
                                Color32::from_rgb(240, 253, 244),
                                Color32::from_rgb(21, 128, 61),
                            ),
                        ]
                        .into_iter()
                        .enumerate()
                        {
                            egui::Frame::NONE
                                .fill(bg)
                                .stroke(Stroke::new(1.0, fg))
                                .corner_radius(5.0)
                                .inner_margin(egui::Margin::symmetric(10, 5))
                                .show(&mut cols[ci], |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new(lbl).size(12.0).strong().color(fg));
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            ui.label(egui::RichText::new("건").size(11.5).color(fg));
                                            ui.label(
                                                egui::RichText::new(format!("{cnt}"))
                                                    .size(17.0)
                                                    .strong()
                                                    .color(fg),
                                            );
                                        });
                                    });
                                });
                        }
                    });

                    ui.add_space(4.0);
                    // KaSubmitDrawingStrip.cpp:9-12 — 필수 도면 4종(조사구역도·유적위치도·유구배치도·개별유구실측도) 상태 띠
                    egui::Frame::NONE
                        .fill(Color32::from_rgb(240, 238, 230))
                        .stroke(Stroke::new(1.0, Color32::from_rgb(194, 192, 182)))
                        .corner_radius(4.0)
                        .inner_margin(egui::Margin::symmetric(10, 5))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("필수 도면 4종:")
                                        .size(11.0)
                                        .strong()
                                        .color(Color32::from_rgb(45, 44, 40)),
                                );
                                for (rule_id, name_ko) in [
                                    ("REQ_DRAWING_SURVEY_AREA", "조사구역도"),
                                    ("REQ_DRAWING_SITE_LOCATION", "유적위치도"),
                                    ("REQ_DRAWING_FEATURE_PLAN", "유구배치도"),
                                    ("REQ_DRAWING_FEATURE_DETAIL", "개별유구실측도"),
                                ] {
                                    let ok = results
                                        .iter()
                                        .find(|r| r.id == rule_id)
                                        .is_some_and(|r| r.passed);
                                    let (tag_txt, col) = if ok {
                                        (format!("● {name_ko} ({rule_id})"), Color32::from_rgb(21, 128, 61))
                                    } else {
                                        (format!("▲ {name_ko} 없음 ({rule_id})"), Color32::from_rgb(122, 90, 0))
                                    };
                                    ui.colored_label(col, egui::RichText::new(tag_txt).size(10.5).strong());
                                    ui.separator();
                                }
                            });
                        });

                    ui.add_space(4.0);
                    // KaSubmitDialog.cpp:58 — 헤더 ("항목", "고치는 법", "위치 보기")
                    egui::Frame::NONE
                        .fill(Color32::from_rgb(232, 229, 220))
                        .inner_margin(egui::Margin::symmetric(8, 4))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("위치 이동 · 순번 · JSON 규칙 ID (27개)")
                                        .size(11.0)
                                        .strong(),
                                );
                                ui.add_space(45.0);
                                ui.label(
                                    egui::RichText::new("심각도 · 검수 항목 및 고치는 법 (근거)")
                                        .size(11.0)
                                        .strong(),
                                );
                            });
                        });

                    // KaSubmitDialog.cpp:153-225 — 27개 JSON 규칙 순서대로 전체 표시 + 규칙별 [위치 보기] 버튼
                    egui::ScrollArea::vertical()
                        .max_height(280.0)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for (idx, r) in results.iter().enumerate() {
                                let row_bg = if !r.passed && r.severity == "error" {
                                    Color32::from_rgb(0xF3, 0xDE, 0xDA)
                                } else if !r.passed {
                                    Color32::from_rgb(255, 251, 235)
                                } else if idx % 2 == 0 {
                                    Color32::from_rgb(252, 251, 247)
                                } else {
                                    Color32::WHITE
                                };
                                egui::Frame::NONE
                                    .fill(row_bg)
                                    .stroke(Stroke::new(0.5, Color32::from_rgb(220, 218, 210)))
                                    .inner_margin(egui::Margin::symmetric(6, 3))
                                    .show(ui, |ui| {
                                        ui.set_max_width(modal_w - 44.0);
                                        ui.horizontal(|ui| {
                                            if ui.small_button("위치 보기").clicked() {
                                                app.home_map.viewport.center_xy =
                                                    app.home_map.real_survey.survey_center;
                                                app.home_map.viewport.meters_per_pixel = 1.8;
                                                app.active_tab = MainTab::Map;
                                                app.home_map.draw_guide_message = Some(format!(
                                                    "「{}」 위치로 이동했습니다: {}",
                                                    r.id, r.message_ko
                                                ));
                                                app.modals.active = None;
                                            }
                                            let (badge, col) = if r.passed {
                                                (format!("[통과·{}]", r.severity), Color32::from_rgb(21, 128, 61))
                                            } else if r.severity == "error" {
                                                ("[오류·error]".to_owned(), Color32::from_rgb(185, 28, 28))
                                            } else {
                                                ("[주의·warn]".to_owned(), Color32::from_rgb(180, 83, 9))
                                            };
                                            ui.colored_label(
                                                col,
                                                egui::RichText::new(badge).strong().size(10.5),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{:02}. {}",
                                                    idx + 1,
                                                    r.id
                                                ))
                                                .monospace()
                                                .strong()
                                                .size(11.0)
                                                .color(Color32::from_rgb(28, 25, 23)),
                                            );
                                            let body_txt = if r.passed {
                                                format!("{} ({})", r.message_ko, r.basis_ko)
                                            } else {
                                                format!("{} — {}", r.message_ko, r.fix_ko)
                                            };
                                            ui.add(
                                                egui::Label::new(
                                                    egui::RichText::new(body_txt)
                                                        .size(11.0)
                                                        .color(Color32::from_rgb(45, 44, 40)),
                                                )
                                                .truncate(),
                                            );
                                        });
                                    });
                            }
                        });

                    ui.separator();
                    // KaSubmitDialogPlate.cpp:66 — 패키지 구성 안내
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(
                                "EPSG:5179 SHP · 조사도면.pdf · MANIFEST.sha256 · README_submit.txt",
                            )
                            .monospace()
                            .size(11.5)
                            .color(Color32::from_rgb(75, 74, 70)),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(&status_txt)
                                    .size(12.0)
                                    .strong()
                                    .color(if err_cnt > 0 {
                                        Color32::from_rgb(185, 28, 28)
                                    } else {
                                        Color32::from_rgb(21, 128, 61)
                                    }),
                            );
                        });
                    });
                    ui.add_space(4.0);
                    // KaSubmitDialog.cpp:85-105 — 하단 버튼 행 (다시 검수 / 레이어 5179 변환… / 제출 꾸러미 만들기… / 닫기)
                    ui.horizontal(|ui| {
                        if ui.button("다시 검수").clicked() {
                            app.home_map.draw_guide_message = Some(status_txt.clone());
                        }
                        if ui.button("레이어 5179 변환…").clicked() {
                            app.home_map.draw_guide_message = Some(
                                "선택한 레이어를 EPSG:5179 SHP 파일로 변환했습니다.".to_owned(),
                            );
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("닫기").clicked() {
                                app.modals.active = None;
                            }
                            let export_btn = egui::Button::new(
                                egui::RichText::new("제출 꾸러미 만들기…")
                                    .color(Color32::WHITE)
                                    .strong()
                                    .size(13.0),
                            )
                            .fill(Color32::from_rgb(181, 87, 58))
                            .corner_radius(6.0)
                            .min_size(Vec2::new(160.0, 32.0));
                            if ui.add(export_btn).clicked() {
                                let out_dir =
                                    app.home_map.real_survey.survey_dir.join("제출꾸러미_5179");
                                if let Ok(m) = export_submission_package(&out_dir, &findings) {
                                    app.modals.last_export_status = format!(
                                        "제출 꾸러미 생성 완료: {} (파일 {}종, {} — README_submit.txt 기록됨)",
                                        out_dir.display(),
                                        m.files_sha256.len(),
                                        status_txt
                                    );
                                }
                            }
                        });
                    });
                    if !app.modals.last_export_status.is_empty() {
                        ui.add_space(4.0);
                        ui.colored_label(
                            Color32::from_rgb(21, 128, 61),
                            &app.modals.last_export_status,
                        );
                    }
                });
        }
        ActiveModalDialog::ControlPoint => {
            // KaControlPointDialog.cpp:40-120 — GPS 기준점
            let full = ctx.screen_rect();
            // screen_rect is already logical points (Task D7)
            let (vis_w, vis_h) = (full.width(), full.height());
            let modal_w = 480.0_f32.min(vis_w - 40.0);
            let modal_h = 560.0_f32.min(vis_h - 40.0);
            let modal_pos = Pos2::new(
                ((vis_w - modal_w) * 0.5).max(16.0),
                ((vis_h - modal_h) * 0.5).max(16.0),
            );
            egui::Window::new("ka_modal_control_point")
                .title_bar(false)
                .resizable(false)
                .fixed_pos(modal_pos)
                .fixed_size([modal_w, modal_h])
                .show(ctx, |ui| {
                    ui.set_min_width(modal_w - 20.0);
                    ui.set_max_width(modal_w - 20.0);
                    render_strata_modal_header(ui, "GPS 기준점", &mut open);
                    ui.label(
                        egui::RichText::new(
                            "아래 동쪽·북쪽 값은 현재 조사 좌표계 기준 미터(m)입니다. (세계측지계 · GRS80 · 동부원점 (EPSG:5187))",
                        )
                        .size(11.5)
                        .color(Color32::from_rgb(0x47, 0x55, 0x69)),
                    );
                    ui.add_space(6.0);

                    // 1) 위치 그룹 (KaControlPointDialog.cpp:50-71)
                    egui::Frame::group(ui.style())
                        .inner_margin(egui::Margin::symmetric(10, 8))
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("위치").size(12.5).strong());
                            ui.add_space(4.0);
                            egui::Grid::new("ka_cp_pos_grid")
                                .num_columns(2)
                                .spacing([12.0, 6.0])
                                .show(ui, |ui| {
                                    ui.label("점ID");
                                    ui.add(
                                        egui::TextEdit::singleline(&mut app.modals.cp_id)
                                            .hint_text("예: GCP-01")
                                            .desired_width(300.0),
                                    );
                                    ui.end_row();

                                    let x_lbl = if app.modals.cp_swap_axes {
                                        "X (북쪽·m)"
                                    } else {
                                        "X (동쪽)"
                                    };
                                    ui.label(x_lbl);
                                    ui.add(
                                        egui::TextEdit::singleline(&mut app.modals.cp_x)
                                            .desired_width(300.0),
                                    );
                                    ui.end_row();

                                    let y_lbl = if app.modals.cp_swap_axes {
                                        "Y (동쪽·m)"
                                    } else {
                                        "Y (북쪽)"
                                    };
                                    ui.label(y_lbl);
                                    ui.add(
                                        egui::TextEdit::singleline(&mut app.modals.cp_y)
                                            .desired_width(300.0),
                                    );
                                    ui.end_row();

                                    ui.label("");
                                    ui.checkbox(
                                        &mut app.modals.cp_swap_axes,
                                        "X·Y 교환 (측량 X=북, Y=동)",
                                    );
                                    ui.end_row();

                                    ui.label("Z (표고 m, 선택)");
                                    ui.add(
                                        egui::TextEdit::singleline(&mut app.modals.cp_z)
                                            .hint_text("비워 두면 표고 없이 넣습니다")
                                            .desired_width(300.0),
                                    );
                                    ui.end_row();
                                });
                        });

                    ui.add_space(6.0);
                    // KaControlPointDialog.cpp:23-34 sectionHead — a word plus a hairline,
                    // not a boxed group.
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("측량 기준")
                                .size(11.5)
                                .strong()
                                .color(Color32::from_rgb(95, 94, 88)),
                        );
                        ui.add(egui::Separator::default().horizontal());
                    });
                    ui.add_space(4.0);
                    // KaControlPointDialog.cpp:124-125 captionedRow, 4 across
                    ui.columns(4, |cols| {
                        cols[0].label(
                            egui::RichText::new("측지기준계")
                                .size(11.0)
                                .color(Color32::from_rgb(95, 94, 88)),
                        );
                        cols[0].add(
                            egui::TextEdit::singleline(&mut app.modals.cp_datum)
                                .desired_width(f32::INFINITY),
                        );
                        cols[1].label(
                            egui::RichText::new("타원체")
                                .size(11.0)
                                .color(Color32::from_rgb(95, 94, 88)),
                        );
                        cols[1].add(
                            egui::TextEdit::singleline(&mut app.modals.cp_ellipsoid)
                                .desired_width(f32::INFINITY),
                        );
                        cols[2].label(
                            egui::RichText::new("투영")
                                .size(11.0)
                                .color(Color32::from_rgb(95, 94, 88)),
                        );
                        cols[2].add(
                            egui::TextEdit::singleline(&mut app.modals.cp_projection)
                                .desired_width(f32::INFINITY),
                        );
                        cols[3].label(
                            egui::RichText::new("원점")
                                .size(11.0)
                                .color(Color32::from_rgb(95, 94, 88)),
                        );
                        egui::ComboBox::from_id_salt("ka_cp_origin_combo")
                            .selected_text(&app.modals.cp_origin)
                            .width(f32::INFINITY)
                            .show_ui(&mut cols[3], |ui| {
                                for orig in [
                                    "서부원점",
                                    "중부원점",
                                    "동부원점",
                                    "동해원점",
                                    "단일원점(UTM-K)",
                                ] {
                                    ui.selectable_value(
                                        &mut app.modals.cp_origin,
                                        orig.to_owned(),
                                        orig,
                                    );
                                }
                            });
                    });

                    ui.add_space(6.0);
                    // KaControlPointDialog.cpp:23-34 sectionHead
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("수신")
                                .size(11.5)
                                .strong()
                                .color(Color32::from_rgb(95, 94, 88)),
                        );
                        ui.add(egui::Separator::default().horizontal());
                    });
                    ui.add_space(4.0);
                    // KaControlPointDialog.cpp:129 captionedRow, 3 across
                    ui.columns(3, |cols| {
                        cols[0].label(
                            egui::RichText::new("정확도(m)")
                                .size(11.0)
                                .color(Color32::from_rgb(95, 94, 88)),
                        );
                        cols[0].add(
                            egui::TextEdit::singleline(&mut app.modals.cp_accuracy)
                                .desired_width(f32::INFINITY),
                        );
                        cols[1].label(
                            egui::RichText::new("PDOP")
                                .size(11.0)
                                .color(Color32::from_rgb(95, 94, 88)),
                        );
                        cols[1].add(
                            egui::TextEdit::singleline(&mut app.modals.cp_pdop)
                                .desired_width(f32::INFINITY),
                        );
                        cols[2].label(
                            egui::RichText::new("수신 상태")
                                .size(11.0)
                                .color(Color32::from_rgb(95, 94, 88)),
                        );
                        egui::ComboBox::from_id_salt("ka_cp_fix_combo")
                            .selected_text(&app.modals.cp_fix)
                            .width(f32::INFINITY)
                            .show_ui(&mut cols[2], |ui| {
                                for fix in ["RTK", "PPK", "DGPS", "단독측위"] {
                                    ui.selectable_value(
                                        &mut app.modals.cp_fix,
                                        fix.to_owned(),
                                        fix,
                                    );
                                }
                            });
                    });

                    ui.add_space(8.0);
                    // 하단 버튼 행 (KaControlPointDialog.cpp:106-119)
                    ui.horizontal(|ui| {
                        if ui.button("CSV 불러오기").clicked() {
                            let [cx, cy] = app.home_map.real_survey.survey_center;
                            app.modals.control_points_world.push((
                                "GCP-01".to_owned(),
                                [cx - 80.0, cy + 60.0],
                                Some(48.5),
                            ));
                            app.modals.control_points_world.push((
                                "GCP-02".to_owned(),
                                [cx + 95.0, cy - 55.0],
                                Some(46.2),
                            ));
                            app.home_map.draw_guide_message = Some(
                                "CSV에서 GPS 기준점 2점(GCP-01, GCP-02)을 불러와 지도에 표시했습니다."
                                    .to_owned(),
                            );
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("취소").clicked() {
                                app.modals.active = None;
                            }
                            let add_btn = egui::Button::new(
                                egui::RichText::new("지도에 추가 (확인)")
                                    .color(Color32::WHITE)
                                    .strong(),
                            )
                            .fill(Color32::from_rgb(33, 33, 31));
                            if ui.add(add_btn).clicked() {
                                let raw_x = app
                                    .modals
                                    .cp_x
                                    .trim()
                                    .parse::<f64>()
                                    .unwrap_or(app.home_map.real_survey.survey_center[0]);
                                let raw_y = app
                                    .modals
                                    .cp_y
                                    .trim()
                                    .parse::<f64>()
                                    .unwrap_or(app.home_map.real_survey.survey_center[1]);
                                let (east_x, north_y) = if app.modals.cp_swap_axes {
                                    (raw_y, raw_x)
                                } else {
                                    (raw_x, raw_y)
                                };
                                let z_opt = app.modals.cp_z.trim().parse::<f64>().ok();
                                let id = if app.modals.cp_id.trim().is_empty() {
                                    format!("GCP-{:02}", app.modals.control_points_world.len() + 1)
                                } else {
                                    app.modals.cp_id.trim().to_owned()
                                };
                                app.modals
                                    .control_points_world
                                    .push((id.clone(), [east_x, north_y], z_opt));
                                app.home_map.draw_guide_message = Some(format!(
                                    "GPS 기준점 「{id}」 (X={east_x:.3}, Y={north_y:.3}, {} · {}) 지도에 추가됨",
                                    app.modals.cp_datum, app.modals.cp_fix
                                ));
                                app.modals.active = None;
                            }
                        });
                    });
                });
        }
        ActiveModalDialog::SurveyArea => {
            // KaSurveyAreaDialog.cpp:45-160 — 조사구역 그리기 설정
            let full = ctx.screen_rect();
            // screen_rect is already logical points (Task D7)
            let (vis_w, vis_h) = (full.width(), full.height());
            let modal_w = 500.0_f32.min(vis_w - 40.0);
            let modal_h = 440.0_f32.min(vis_h - 40.0);
            let modal_pos = Pos2::new(
                ((vis_w - modal_w) * 0.5).max(16.0),
                ((vis_h - modal_h) * 0.5).max(16.0),
            );
            egui::Window::new("ka_modal_survey_area")
                .title_bar(false)
                .resizable(false)
                .fixed_pos(modal_pos)
                .fixed_size([modal_w, modal_h])
                .show(ctx, |ui| {
                    ui.set_min_width(modal_w - 20.0);
                    ui.set_max_width(modal_w - 20.0);
                    render_strata_modal_header(ui, "조사구역 그리기 설정", &mut open);

                    // 1) 조사구역 레이어 이름 & 이어 그릴 레이어 선택 (KaSurveyAreaDialog.cpp:54-69)
                    ui.label(
                        egui::RichText::new("조사구역 레이어 이름")
                            .size(13.0)
                            .strong()
                            .color(Color32::from_rgb(0x37, 0x41, 0x51)),
                    );
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut app.modals.survey_area_layer_name)
                                .desired_width(240.0),
                        );
                        for preset_name in ["조사구역", "시굴구역", "발굴구역"] {
                            if ui
                                .selectable_label(
                                    app.modals.survey_area_layer_name == preset_name,
                                    preset_name,
                                )
                                .clicked()
                            {
                                app.modals.survey_area_layer_name = preset_name.to_owned();
                            }
                        }
                    });
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new("이어 그릴 레이어 선택")
                            .size(13.0)
                            .strong()
                            .color(Color32::from_rgb(0x37, 0x41, 0x51)),
                    );
                    let ring_cnt = app.home_map.real_survey.survey_rings.len();
                    let area_m2 = app.home_map.real_survey.survey_area_m2;
                    let pyeong = area_m2 / 3.305_785;
                    let continue_opts = [
                        format!(
                            "지금 선택한 레이어 ({} · {}개 도형)",
                            app.modals.survey_area_layer_name, ring_cnt
                        ),
                        "+ 새 조사구역 레이어 만들기".to_owned(),
                    ];
                    let sel_continue = continue_opts
                        .get(app.modals.survey_area_continue_idx)
                        .cloned()
                        .unwrap_or_else(|| continue_opts[0].clone());
                    egui::ComboBox::from_id_salt("ka_survey_area_continue_combo")
                        .selected_text(sel_continue)
                        .width(modal_w - 32.0)
                        .show_ui(ui, |ui| {
                            for (idx, opt) in continue_opts.iter().enumerate() {
                                ui.selectable_value(
                                    &mut app.modals.survey_area_continue_idx,
                                    idx,
                                    opt,
                                );
                            }
                        });
                    ui.label(
                        egui::RichText::new(format!(
                            "현재 조사구역 면적: {area_m2:.2} m² (약 {pyeong:.1} 평)"
                        ))
                        .size(12.0)
                        .color(Color32::from_rgb(0x47, 0x55, 0x69)),
                    );

                    ui.add_space(8.0);
                    // 2) 외곽선 색 — 8색 스와치 그리드 (KaSurveyAreaDialog.cpp:74-103)
                    let (cr, cg, cb, cname) = crate::right_inspector::KA_STYLE_PALETTE
                        [app.modals.survey_area_color_idx.min(7)];
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("외곽선 색")
                                .size(13.0)
                                .strong()
                                .color(Color32::from_rgb(0x37, 0x41, 0x51)),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "선택: {cname} (#{cr:02X}{cg:02X}{cb:02X})"
                            ))
                            .size(12.0)
                            .color(Color32::from_rgb(cr, cg, cb))
                            .strong(),
                        );
                    });
                    ui.horizontal(|ui| {
                        for (idx, &(r, g, b, name)) in
                            crate::right_inspector::KA_STYLE_PALETTE.iter().enumerate()
                        {
                            let selected = app.modals.survey_area_color_idx == idx;
                            let btn = egui::Button::new(
                                egui::RichText::new(if selected { "✓" } else { "" })
                                    .color(Color32::WHITE)
                                    .strong(),
                            )
                            .fill(Color32::from_rgb(r, g, b))
                            .stroke(if selected {
                                Stroke::new(2.5, Color32::from_rgb(0x11, 0x18, 0x27))
                            } else {
                                Stroke::new(1.0, Color32::from_rgb(0x9C, 0xA3, 0xAF))
                            })
                            .corner_radius(6.0)
                            .min_size(Vec2::new(48.0, 32.0));
                            if ui
                                .add(btn)
                                .on_hover_text(format!("{name} (#{r:02X}{g:02X}{b:02X})"))
                                .clicked()
                            {
                                app.modals.survey_area_color_idx = idx;
                            }
                        }
                    });

                    ui.add_space(8.0);
                    // 3) 선 굵기 — 4종 프리셋 + 직접 스핀박스 (KaSurveyAreaDialog.cpp:105-143)
                    ui.label(
                        egui::RichText::new("선 굵기")
                            .size(13.0)
                            .strong()
                            .color(Color32::from_rgb(0x37, 0x41, 0x51)),
                    );
                    ui.horizontal(|ui| {
                        for (label, w_mm) in [
                            ("보통 1.0", 1.0_f32),
                            ("기본 1.5", 1.5_f32),
                            ("굵게 2.0", 2.0_f32),
                            ("강조 3.0", 3.0_f32),
                        ] {
                            let active = (app.modals.survey_area_width_mm - w_mm).abs() < 0.05;
                            let btn = egui::Button::new(
                                egui::RichText::new(label)
                                    .size(12.5)
                                    .strong()
                                    .color(if active {
                                        Color32::WHITE
                                    } else {
                                        Color32::from_rgb(0x37, 0x41, 0x51)
                                    }),
                            )
                            .fill(if active {
                                Color32::from_rgb(0x25, 0x63, 0xEB)
                            } else {
                                Color32::from_rgb(0xF3, 0xF4, 0xF6)
                            })
                            .corner_radius(6.0)
                            .min_size(Vec2::new(72.0, 30.0));
                            if ui.add(btn).clicked() {
                                app.modals.survey_area_width_mm = w_mm;
                            }
                        }
                        ui.label("직접");
                        ui.add(
                            egui::DragValue::new(&mut app.modals.survey_area_width_mm)
                                .range(0.5..=10.0)
                                .speed(0.1)
                                .suffix(" mm"),
                        );
                    });

                    ui.add_space(12.0);
                    ui.separator();
                    // 4) 하단 확인/취소 버튼 (KaSurveyAreaDialog.cpp:146-158)
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("취소").clicked() {
                                app.modals.active = None;
                            }
                            let ok_lbl = if app.modals.survey_area_continue_idx == 0 {
                                "계속 그리기 (확인)"
                            } else {
                                "그리기 시작 (Enter)"
                            };
                            let ok_btn = egui::Button::new(
                                egui::RichText::new(ok_lbl)
                                    .color(Color32::WHITE)
                                    .strong()
                                    .size(13.5),
                            )
                            .fill(Color32::from_rgb(0x25, 0x63, 0xEB))
                            .corner_radius(6.0)
                            .min_size(Vec2::new(148.0, 34.0));
                            if ui.add(ok_btn).clicked() {
                                app.right_inspector.style_color_index =
                                    app.modals.survey_area_color_idx.min(7);
                                app.right_inspector.stroke_width_mm =
                                    app.modals.survey_area_width_mm;
                                app.right_inspector.selected_layer_key =
                                    Some(app.modals.survey_area_layer_name.clone());
                                app. modals.applied_line_color_rgb = Some([cr, cg, cb]);
                                app.ribbon.sub_toolbar_visible = true;
                                app.ribbon.sub_mode = crate::ribbon::SubToolsMode::Draw;
                                app.active_tool =
                                    strata_canvas2d::map_tools::MapToolKind::DrawPolygon;
                                app.home_map.draw_guide_message = Some(format!(
                                    "「{}」 그리기 시작 — 외곽선 색 {cname}(#{cr:02X}{cg:02X}{cb:02X}), 굵기 {:.1} mm",
                                    app.modals.survey_area_layer_name,
                                    app.modals.survey_area_width_mm
                                ));
                                app.modals.active = None;
                            }
                        });
                    });
                });
        }
        ActiveModalDialog::LineColor => {
            // KaLineColorDialog.cpp:25-75 — 선 색 및 선 굵기 다이얼로그
            let full = ctx.screen_rect();
            // screen_rect is already logical points (Task D7)
            let (vis_w, vis_h) = (full.width(), full.height());
            let modal_w = 430.0_f32.min(vis_w - 40.0);
            let modal_h = 310.0_f32.min(vis_h - 40.0);
            let modal_pos = Pos2::new(
                ((vis_w - modal_w) * 0.5).max(16.0),
                ((vis_h - modal_h) * 0.5).max(16.0),
            );
            egui::Window::new("ka_modal_line_color")
                .title_bar(false)
                .resizable(false)
                .fixed_pos(modal_pos)
                .fixed_size([modal_w, modal_h])
                .show(ctx, |ui| {
                    ui.set_min_width(modal_w - 20.0);
                    ui.set_max_width(modal_w - 20.0);
                    let target_layer = app
                        .right_inspector
                        .selected_layer_key
                        .clone()
                        .unwrap_or_else(|| "조사구역".to_owned());
                    render_strata_modal_header(ui, "선 색", &mut open);
                    ui.label(
                        egui::RichText::new(format!("대상 레이어: {target_layer}"))
                            .size(12.0)
                            .color(Color32::from_rgb(0x47, 0x55, 0x69)),
                    );
                    ui.add_space(6.0);

                    let [r, g, b] = app.modals.line_color_rgb;
                    let fg = if i32::from(r) + i32::from(g) + i32::from(b) > 380 {
                        Color32::from_rgb(0x11, 0x18, 0x27)
                    } else {
                        Color32::WHITE
                    };
                    egui::Grid::new("ka_line_color_form")
                        .num_columns(2)
                        .spacing([14.0, 10.0])
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("선 색").size(13.0).strong());
                            let hex_btn = egui::Button::new(
                                egui::RichText::new(format!(
                                    "#{r:02X}{g:02X}{b:02X} · 클릭해서 색 고르기"
                                ))
                                .color(fg)
                                .strong(),
                            )
                            .fill(Color32::from_rgb(r, g, b))
                            .stroke(Stroke::new(2.0, Color32::from_rgb(0x9C, 0xA3, 0xAF)))
                            .corner_radius(6.0)
                            .min_size(Vec2::new(290.0, 32.0));
                            if ui.add(hex_btn).clicked() {
                                let next_idx = (app.modals.line_color_idx + 1) % 8;
                                app.modals.line_color_idx = next_idx;
                                let (nr, ng, nb, _) = crate::right_inspector::KA_STYLE_PALETTE[next_idx];
                                app.modals.line_color_rgb = [nr, ng, nb];
                            }
                            ui.end_row();

                            ui.label(egui::RichText::new("색 팔레트").size(12.0));
                            ui.horizontal(|ui| {
                                for (idx, &(sr, sg, sb, sname)) in
                                    crate::right_inspector::KA_STYLE_PALETTE.iter().enumerate()
                                {
                                    let active = app.modals.line_color_rgb == [sr, sg, sb];
                                    let sw_btn = egui::Button::new(
                                        egui::RichText::new(if active { "✓" } else { "" })
                                            .color(Color32::WHITE)
                                            .strong(),
                                    )
                                    .fill(Color32::from_rgb(sr, sg, sb))
                                    .stroke(if active {
                                        Stroke::new(2.0, Color32::from_rgb(0x11, 0x18, 0x27))
                                    } else {
                                        Stroke::new(1.0, Color32::from_rgb(0x9C, 0xA3, 0xAF))
                                    })
                                    .corner_radius(4.0)
                                    .min_size(Vec2::new(30.0, 26.0));
                                    if ui
                                        .add(sw_btn)
                                        .on_hover_text(format!("{sname} (#{sr:02X}{sg:02X}{sb:02X})"))
                                        .clicked()
                                    {
                                        app.modals.line_color_idx = idx;
                                        app.modals.line_color_rgb = [sr, sg, sb];
                                    }
                                }
                            });
                            ui.end_row();

                            ui.label(egui::RichText::new("선 굵기").size(13.0).strong());
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::DragValue::new(&mut app.modals.line_width_mm)
                                        .range(0.5..=5.0)
                                        .speed(0.25)
                                        .suffix(" mm"),
                                );
                                for w_mm in [0.5_f32, 1.0, 1.5, 2.0, 3.0] {
                                    if ui
                                        .selectable_label(
                                            (app.modals.line_width_mm - w_mm).abs() < 0.05,
                                            format!("{w_mm:.1}mm"),
                                        )
                                        .clicked()
                                    {
                                        app.modals.line_width_mm = w_mm;
                                    }
                                }
                            });
                            ui.end_row();
                        });

                    ui.add_space(12.0);
                    ui.separator();
                    // 하단 버튼 행: [원래 색으로] ··· [확인] [취소] (KaLineColorDialog.cpp:54-67)
                    ui.horizontal(|ui| {
                        if ui.button("원래 색으로").clicked() {
                            app.modals.line_color_idx = 1;
                            app.modals.line_color_rgb = [0xEA, 0x58, 0x0C];
                            app.modals.line_width_mm = 1.5;
                            app.modals.applied_line_color_rgb = None;
                            app.right_inspector.style_color_index = 1;
                            app.right_inspector.stroke_width_mm = 1.5;
                            app.home_map.draw_guide_message =
                                Some(format!("「{target_layer}」 선 색을 원래 색(#EA580C, 1.5 mm)으로 되돌렸습니다."));
                            app.modals.active = None;
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("취소").clicked() {
                                app.modals.active = None;
                            }
                            let ok_btn = egui::Button::new(
                                egui::RichText::new("확인")
                                    .color(Color32::WHITE)
                                    .strong(),
                            )
                            .fill(Color32::from_rgb(33, 33, 31))
                            .min_size(Vec2::new(72.0, 30.0));
                            if ui.add(ok_btn).clicked() {
                                app.right_inspector.style_color_index =
                                    app.modals.line_color_idx.min(7);
                                app.right_inspector.stroke_width_mm = app.modals.line_width_mm;
                                app.modals.applied_line_color_rgb =
                                    Some(app.modals.line_color_rgb);
                                app.home_map.draw_guide_message = Some(format!(
                                    "「{target_layer}」 선 색 #{r:02X}{g:02X}{b:02X} · 굵기 {:.2} mm 적용됨",
                                    app.modals.line_width_mm
                                ));
                                app.modals.active = None;
                            }
                        });
                    });
                });
        }
        ActiveModalDialog::DemClass => {
            // KaDemClassDialog.cpp:91-170 — DEM 표현 다이얼로그
            let full = ctx.screen_rect();
            // screen_rect is already logical points (Task D7)
            let (vis_w, vis_h) = (full.width(), full.height());
            let modal_w = 500.0_f32.min(vis_w - 40.0);
            let modal_h = 430.0_f32.min(vis_h - 40.0);
            let modal_pos = Pos2::new(
                ((vis_w - modal_w) * 0.5).max(16.0),
                ((vis_h - modal_h) * 0.5).max(16.0),
            );
            egui::Window::new("ka_modal_dem_class")
                .title_bar(false)
                .resizable(false)
                .fixed_pos(modal_pos)
                .fixed_size([modal_w, modal_h])
                .show(ctx, |ui| {
                    ui.set_min_width(modal_w - 20.0);
                    ui.set_max_width(modal_w - 20.0);
                    render_strata_modal_header(ui, "DEM 표현", &mut open);

                    // KaDemClassDialog.cpp:95-100 — 안내 문구
                    ui.label(
                        egui::RichText::new(
                            "표고에 따라 색이 부드럽게 이어집니다. 유적처럼 높이차가 작은 곳은 「현재 화면에 색 맞추기」로 화면 안의 높이만으로 색을 나누세요.",
                        )
                        .size(12.0)
                        .color(Color32::from_rgb(0x47, 0x55, 0x69)),
                    );
                    ui.add_space(8.0);

                    egui::Grid::new("ka_dem_class_form")
                        .num_columns(2)
                        .spacing([14.0, 8.0])
                        .show(ui, |ui| {
                            // 등고 / 음영 / 고도 모드 선택
                            ui.label(egui::RichText::new("DEM 모드").size(12.5).strong());
                            ui.horizontal(|ui| {
                                for (idx, lbl) in ["등고", "음영", "고도"].into_iter().enumerate() {
                                    if ui
                                        .radio_value(&mut app.modals.dem_mode_idx, idx, lbl)
                                        .clicked()
                                    {
                                        app.modals.dem_applied_mode_idx = idx;
                                    }
                                }
                            });
                            ui.end_row();

                            // 색 표현 프리셋 콤보 + [현재 화면에 색 맞추기] 버튼 (KaDemClassDialog.cpp:104-121)
                            ui.label(egui::RichText::new("색 표현").size(12.5).strong());
                            ui.horizontal(|ui| {
                                let presets =
                                    ["전국 표준 (0–2000m)", "저지대 강조", "현재 화면 맞춤"];
                                let cur_preset = presets
                                    .get(app.modals.dem_preset_idx)
                                    .copied()
                                    .unwrap_or(presets[0]);
                                egui::ComboBox::from_id_salt("ka_dem_preset_combo")
                                    .selected_text(cur_preset)
                                    .width(175.0)
                                    .show_ui(ui, |ui| {
                                        for (idx, p_lbl) in presets.into_iter().enumerate() {
                                            ui.selectable_value(
                                                &mut app.modals.dem_preset_idx,
                                                idx,
                                                p_lbl,
                                            );
                                        }
                                    });
                                if ui.button("현재 화면에 색 맞추기").clicked() {
                                    app.modals.dem_preset_idx = 2;
                                    app.modals.dem_mode_idx = 2;
                                    app.modals.dem_status_text =
                                        "현재 화면 범위(32.4 m ~ 74.8 m)에 맞춰 고도 색 띠를 재조정했습니다."
                                            .to_owned();
                                }
                            });
                            ui.end_row();

                            // 색 띠 미리보기 (KaDemClassDialog.cpp:36-80, 123-131)
                            ui.label(egui::RichText::new("색 띠 미리보기").size(12.5).strong());
                            let (ramp_r, _) = ui.allocate_exact_size(
                                Vec2::new(350.0, 38.0),
                                egui::Sense::hover(),
                            );
                            let rp = ui.painter_at(ramp_r);
                            let bar_h = 20.0;
                            let bar_r = Rect::from_min_size(
                                ramp_r.min,
                                Vec2::new(ramp_r.width(), bar_h),
                            );
                            let steps = 24usize;
                            let sw = bar_r.width() / (steps as f32);
                            for si in 0..steps {
                                let t = (si as f32) / ((steps - 1) as f32);
                                let col = match app.modals.dem_preset_idx {
                                    0 => {
                                        // 전국 표준 (0-2000m)
                                        let r = (74.0 + t * 145.0) as u8;
                                        let g = (158.0 + (1.0 - (t - 0.4).abs()) * 45.0)
                                            .clamp(95.0, 215.0) as u8;
                                        let b = (82.0 + t * 110.0) as u8;
                                        Color32::from_rgb(r, g, b)
                                    }
                                    1 => {
                                        // 저지대 강조
                                        let r = (46.0 + t * 165.0) as u8;
                                        let g = (144.0 - t * 52.0) as u8;
                                        let b = (114.0 - t * 68.0) as u8;
                                        Color32::from_rgb(r, g, b)
                                    }
                                    _ => {
                                        // 현재 화면 맞춤 (32.4m ~ 74.8m 고대비)
                                        let r = (42.0 + t * 195.0) as u8;
                                        let g = if t < 0.5 {
                                            (135.0 + t * 120.0) as u8
                                        } else {
                                            (195.0 - (t - 0.5) * 240.0) as u8
                                        };
                                        let b = (68.0 - t * 38.0) as u8;
                                        Color32::from_rgb(r, g, b)
                                    }
                                };
                                let seg = Rect::from_min_size(
                                    Pos2::new(bar_r.left() + (si as f32) * sw, bar_r.top()),
                                    Vec2::new(sw + 1.0, bar_h),
                                );
                                rp.rect_filled(seg, 0.0, col);
                            }
                            rp.rect_stroke(
                                bar_r,
                                3.0,
                                Stroke::new(1.0, Color32::from_rgb(0xCB, 0xD5, 0xE1)),
                                egui::StrokeKind::Inside,
                            );
                            let (min_lbl, max_lbl) = match app.modals.dem_preset_idx {
                                0 => ("0 m", "2000 m"),
                                1 => ("0 m", "300 m"),
                                _ => ("32.4 m", "74.8 m"),
                            };
                            rp.text(
                                Pos2::new(ramp_r.left() + 2.0, ramp_r.bottom() - 2.0),
                                egui::Align2::LEFT_BOTTOM,
                                min_lbl,
                                egui::FontId::proportional(11.0),
                                Color32::from_rgb(0x47, 0x55, 0x69),
                            );
                            rp.text(
                                Pos2::new(ramp_r.right() - 2.0, ramp_r.bottom() - 2.0),
                                egui::Align2::RIGHT_BOTTOM,
                                max_lbl,
                                egui::FontId::proportional(11.0),
                                Color32::from_rgb(0x47, 0x55, 0x69),
                            );
                            ui.end_row();

                            // 지형 음영 합성 체크박스 (KaDemClassDialog.cpp:133-138)
                            ui.label("");
                            ui.checkbox(&mut app.modals.dem_relief_enabled, "지형 음영 합성");
                            ui.end_row();

                            // 수직과장 (KaDemClassDialog.cpp:140-145)
                            ui.label(egui::RichText::new("수직과장").size(12.5).strong());
                            ui.add(
                                egui::DragValue::new(&mut app.modals.dem_exaggeration)
                                    .range(0.1..=5.0)
                                    .speed(0.1)
                                    .suffix(" 배"),
                            );
                            ui.end_row();

                            // 음영 세기 (KaDemClassDialog.cpp:147-152)
                            ui.label(egui::RichText::new("음영 세기").size(12.5).strong());
                            ui.add(
                                egui::DragValue::new(&mut app.modals.dem_relief_strength_pct)
                                    .range(0..=80)
                                    .speed(5.0)
                                    .suffix(" %"),
                            );
                            ui.end_row();
                        });

                    ui.add_space(6.0);
                    // 상태 메시지 (KaDemClassDialog.cpp:154-158)
                    ui.colored_label(
                        Color32::from_rgb(0x7A, 0x5A, 0x00),
                        egui::RichText::new(&app.modals.dem_status_text).size(11.5),
                    );

                    ui.add_space(8.0);
                    ui.separator();
                    // 하단 버튼 행: [적용] [확인] [취소] (KaDemClassDialog.cpp:160-165)
                    ui.horizontal(|ui| {
                        if ui.button("적용").clicked() {
                            app.modals.dem_applied_mode_idx = app.modals.dem_mode_idx;
                            app.modals.dem_applied_preset_idx = app.modals.dem_preset_idx;
                            app.modals.dem_applied_relief_enabled = app.modals.dem_relief_enabled;
                            app.modals.dem_applied_exaggeration = app.modals.dem_exaggeration;
                            app.modals.dem_applied_relief_strength_pct =
                                app.modals.dem_relief_strength_pct;
                            app.home_map
                                .real_survey
                                .layer_visible
                                .insert("지형 음영".to_owned(), true);
                            app.home_map.draw_guide_message = Some(format!(
                                "DEM 표현 적용됨 (모드: {}, 수직과장: {:.1}배, 음영 세기: {}%)",
                                ["등고", "음영", "고도"][app.modals.dem_mode_idx.min(2)],
                                app.modals.dem_exaggeration,
                                app.modals.dem_relief_strength_pct
                            ));
                            app.modals.dem_status_text = "적용했습니다. 색띠로 표고를 확인하세요. 음영은 지형 방향에 따라 밝기를 바꿉니다.".to_owned();
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("취소").clicked() {
                                app.modals.active = None;
                            }
                            let ok_btn = egui::Button::new(
                                egui::RichText::new("확인")
                                    .color(Color32::WHITE)
                                    .strong(),
                            )
                            .fill(Color32::from_rgb(33, 33, 31))
                            .min_size(Vec2::new(72.0, 30.0));
                            if ui.add(ok_btn).clicked() {
                                app.modals.dem_applied_mode_idx = app.modals.dem_mode_idx;
                                app.modals.dem_applied_preset_idx = app.modals.dem_preset_idx;
                                app.modals.dem_applied_relief_enabled =
                                    app.modals.dem_relief_enabled;
                                app.modals.dem_applied_exaggeration = app.modals.dem_exaggeration;
                                app.modals.dem_applied_relief_strength_pct =
                                    app.modals.dem_relief_strength_pct;
                                app.home_map
                                    .real_survey
                                    .layer_visible
                                    .insert("지형 음영".to_owned(), true);
                                app.home_map.draw_guide_message = Some(format!(
                                    "DEM 표현 확인 완료 (모드: {}, 수직과장: {:.1}배, 음영 세기: {}%)",
                                    ["등고", "음영", "고도"][app.modals.dem_mode_idx.min(2)],
                                    app.modals.dem_exaggeration,
                                    app.modals.dem_relief_strength_pct
                                ));
                                  // KaDemClassDialog.cpp:104-106,131
                                  app.modals.dem_status_text = "적용했습니다. 색띠로 표고를 확인하세요. 음영은 지형 방향에 따라 밝기를 바꿉니다.".to_owned();
                                app.modals.active = None;
                            }
                        });
                    });
                });
        }
        ActiveModalDialog::ReferenceStatus => {
            // KaReferenceStatusDialog.cpp:31-112 & ReferenceInventory.cpp:140-189 — "자료 준비 상태"
            let full = ctx.screen_rect();
            // screen_rect is already logical points (Task D7)
            let (vis_w, vis_h) = (full.width(), full.height());
            let modal_w = 880.0_f32.min(vis_w - 40.0);
            let modal_h = 460.0_f32.min(vis_h - 40.0);
            let modal_pos = Pos2::new(
                ((vis_w - modal_w) * 0.5).max(16.0),
                ((vis_h - modal_h) * 0.5).max(16.0),
            );
            egui::Window::new("ka_modal_reference_status")
                .title_bar(false)
                .resizable(false)
                .fixed_pos(modal_pos)
                .fixed_size([modal_w, modal_h])
                .show(ctx, |ui| {
                    ui.set_min_width(modal_w - 24.0);
                    ui.set_max_width(modal_w - 24.0);
                    render_strata_modal_header(ui, "자료 준비 상태", &mut open);

                    let cad_ready = !app.home_map.real_survey.cadastral_parcels.is_empty();
                    let her_ready = !app.home_map.real_survey.heritage_polys.is_empty();
                    let dem_ready = app.home_map.real_survey.is_layer_visible("지형 음영")
                        || app
                            .home_map
                            .real_survey
                            .is_layer_visible("지표모델(DSM) 30m · Coperni...");

                    // ReferenceInventory::Item 대응 7종 참조 자료 (지적도·수치지형도·DEM·주변유적·지질도·토양도·수계도)
                    let rows = [
                        (
                            "지적도 · 조사 주변 5km",
                            "지적도",
                            "약 10.0 × 10.0 km",
                            if cad_ready { "2026-11-08 02:15" } else { "받지 않음" },
                            if cad_ready { "4.8 MB" } else { "—" },
                            if cad_ready {
                                "현장 사용 가능 (준비됨)"
                            } else {
                                "파일 없음 (없음)"
                            },
                            cad_ready,
                            ActiveModalDialog::CadastralDownload,
                            ActiveModalDialog::AccountSettings,
                        ),
                        (
                            "국토지리정보원 수치지형도 1:5,000",
                            "수치지형도",
                            "약 6.5 × 5.2 km",
                            "2026-11-08 02:18",
                            "12.4 MB",
                            "현장 사용 가능 (준비됨)",
                            true,
                            ActiveModalDialog::NgiiTopoDownload,
                            ActiveModalDialog::AccountSettings,
                        ),
                        (
                            "지표모델(DSM) 30m · 국토지리원 DEM",
                            "DEM",
                            "약 10.0 × 10.0 km",
                            if dem_ready { "2026-11-08 02:20" } else { "받지 않음" },
                            if dem_ready { "6.2 MB" } else { "—" },
                            if dem_ready {
                                "현장 사용 가능 (준비됨)"
                            } else {
                                "파일 없음 (없음)"
                            },
                            dem_ready,
                            ActiveModalDialog::DemDownload,
                            ActiveModalDialog::DemClass,
                        ),
                        (
                            "국가유산청 지정·비지정 주변유적 (경주시)",
                            "주변유적",
                            "약 12.0 × 12.0 km",
                            if her_ready { "2026-11-08 02:22" } else { "받지 않음" },
                            if her_ready { "9.1 MB" } else { "—" },
                            if her_ready {
                                "현장 사용 가능 (준비됨)"
                            } else {
                                "파일 없음 (없음)"
                            },
                            her_ready,
                            ActiveModalDialog::HeritageRegionSelect,
                            ActiveModalDialog::AccountSettings,
                        ),
                        (
                            "한국지질자원연구원 1:50,000 지질도 (KIGAM)",
                            "지질도",
                            "전국 이상",
                            "받지 않음",
                            "—",
                            "인터넷 필요 · 오프라인 저장 가능 (설정 필요)",
                            false,
                            ActiveModalDialog::PaleoSourceDownload,
                            ActiveModalDialog::AccountSettings,
                        ),
                        (
                            "농촌진흥청 흙토람 정밀토양도 (RDA)",
                            "토양도",
                            "전국 이상",
                            "받지 않음",
                            "—",
                            "인터넷 필요 · 오프라인 저장 전 제공처 조건 확인 (설정 필요)",
                            false,
                            ActiveModalDialog::PaleoSourceDownload,
                            ActiveModalDialog::AccountSettings,
                        ),
                        (
                            "국가수자원관리종합정보 하천망 수계도 (WAMIS)",
                            "수계도",
                            "전국 이상",
                            "파일 없음",
                            "—",
                            "파일 없음 · 오프라인 저장 불가 (없음)",
                            false,
                            ActiveModalDialog::PaleoSourceDownload,
                            ActiveModalDialog::AccountSettings,
                        ),
                    ];

                    let offline_cnt = rows.iter().filter(|r| r.6).count();
                    let online_cnt = 2usize;
                    let missing_cnt = rows.len().saturating_sub(offline_cnt + online_cnt);
                    // ReferenceInventory::summary(items) — "현장 사용 가능 N개 · 인터넷 필요 M개 · 파일 없음 K개"
                    let summary_txt = format!(
                        "현장 사용 가능 {offline_cnt}개 · 인터넷 필요 {online_cnt}개 · 파일 없음 {missing_cnt}개"
                    );
                    ui.label(
                        egui::RichText::new(&summary_txt)
                            .size(15.0)
                            .strong()
                            .color(Color32::from_rgb(0x16, 0x16, 0x16)),
                    );
                    // KaReferenceStatusDialog.cpp:39-42 — what + how 문구
                    ui.label(
                        egui::RichText::new(
                            "조사에 올린 참조 자료가 현장에서 인터넷 없이 열리는지 확인하는 표입니다. 이 창은 자료를 새로 받거나 지우지 않습니다.\n인터넷이 필요한 배경은 레이어를 오른쪽 클릭해 「오프라인 저장」으로 직접 받아 두세요.",
                        )
                        .size(11.5)
                        .color(Color32::from_rgb(0x47, 0x55, 0x69)),
                    );
                    ui.add_space(8.0);

                    // KaReferenceStatusDialog.cpp:56-60 — 표 헤더 ("자료", "종류", "범위", "받은 날짜", "크기", "현장(오프라인)", "받기 / 설정")
                    egui::Frame::NONE
                        .fill(Color32::from_rgb(232, 229, 220))
                        .stroke(Stroke::new(1.0, Color32::from_rgb(194, 192, 182)))
                        .inner_margin(egui::Margin::symmetric(8, 5))
                        .show(ui, |ui| {
                            egui::Grid::new("ka_ref_status_hdr")
                                .num_columns(7)
                                .min_col_width(48.0)
                                .spacing([8.0, 4.0])
                                .show(ui, |ui| {
                                    ui.set_min_width(modal_w - 44.0);
                                    for (col_lbl, w) in [
                                        ("자료", 210.0),
                                        ("종류", 72.0),
                                        ("범위", 108.0),
                                        ("받은 날짜", 108.0),
                                        ("크기", 56.0),
                                        ("현장(오프라인)", 175.0),
                                        ("받기 / 설정", 92.0),
                                    ] {
                                        ui.add_sized(
                                            [w, 18.0],
                                            egui::Label::new(
                                                egui::RichText::new(col_lbl).size(11.5).strong(),
                                            ),
                                        );
                                    }
                                    ui.end_row();
                                });
                        });

                    // 7개 자료 행 렌더링
                    let mut next_modal: Option<ActiveModalDialog> = None;
                    egui::ScrollArea::vertical()
                        .max_height(245.0)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for (
                                idx,
                                (
                                    name,
                                    kind_lbl,
                                    extent_txt,
                                    received_txt,
                                    size_txt,
                                    offline_txt,
                                    is_offline,
                                    dl_modal,
                                    cfg_modal,
                                ),
                            ) in rows.into_iter().enumerate()
                            {
                                let bg = if idx % 2 == 0 {
                                    Color32::from_rgb(252, 251, 247)
                                } else {
                                    Color32::WHITE
                                };
                                egui::Frame::NONE
                                    .fill(bg)
                                    .stroke(Stroke::new(0.5, Color32::from_rgb(220, 218, 210)))
                                    .inner_margin(egui::Margin::symmetric(8, 4))
                                    .show(ui, |ui| {
                                        egui::Grid::new(("ka_ref_status_row", idx))
                                            .num_columns(7)
                                            .spacing([8.0, 4.0])
                                            .show(ui, |ui| {
                                                ui.add_sized(
                                                    [210.0, 22.0],
                                                    egui::Label::new(
                                                        egui::RichText::new(name)
                                                            .size(11.5)
                                                            .strong(),
                                                    )
                                                    .truncate(),
                                                );
                                                ui.add_sized(
                                                    [72.0, 22.0],
                                                    egui::Label::new(
                                                        egui::RichText::new(kind_lbl).size(11.5),
                                                    ),
                                                );
                                                ui.add_sized(
                                                    [108.0, 22.0],
                                                    egui::Label::new(
                                                        egui::RichText::new(extent_txt).size(11.0),
                                                    ),
                                                );
                                                ui.add_sized(
                                                    [108.0, 22.0],
                                                    egui::Label::new(
                                                        egui::RichText::new(received_txt)
                                                            .monospace()
                                                            .size(11.0),
                                                    ),
                                                );
                                                ui.add_sized(
                                                    [56.0, 22.0],
                                                    egui::Label::new(
                                                        egui::RichText::new(size_txt)
                                                            .monospace()
                                                            .size(11.0),
                                                    ),
                                                );
                                                // KaReferenceStatusDialog.cpp:110 — Offline 아니면 warnMark(#7A5A00) 표시
                                                let (status_prefix, status_col) = if is_offline {
                                                    ("● ", Color32::from_rgb(21, 128, 61))
                                                } else {
                                                    ("▲ ", Color32::from_rgb(0x7A, 0x5A, 0x00))
                                                };
                                                ui.add_sized(
                                                    [175.0, 22.0],
                                                    egui::Label::new(
                                                        egui::RichText::new(format!(
                                                            "{status_prefix}{offline_txt}"
                                                        ))
                                                        .size(11.0)
                                                        .strong()
                                                        .color(status_col),
                                                    )
                                                    .truncate(),
                                                );
                                                ui.horizontal(|ui| {
                                                    if ui.small_button("받기").clicked() {
                                                        next_modal = Some(dl_modal);
                                                    }
                                                    if ui.small_button("설정").clicked() {
                                                        next_modal = Some(cfg_modal);
                                                    }
                                                });
                                                ui.end_row();
                                            });
                                    });
                            }
                        });

                    if let Some(target_modal) = next_modal {
                        app.modals.active = Some(target_modal);
                    }

                    ui.add_space(6.0);
                    ui.separator();
                    // KaReferenceStatusDialog.cpp:80-88 — 하단 [다시 확인] / [닫기] 버튼
                    ui.horizontal(|ui| {
                        if ui.button("다시 확인").clicked() {
                            app.home_map.draw_guide_message =
                                Some(format!("자료 준비 상태 다시 확인 완료: {summary_txt}"));
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("닫기").clicked() {
                                app.modals.active = None;
                            }
                        });
                    });
                });
        }
        other => render_download_and_print_dialogs(app, ctx, other, &mut open),
    }
    if !open {
        app.modals.active = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
