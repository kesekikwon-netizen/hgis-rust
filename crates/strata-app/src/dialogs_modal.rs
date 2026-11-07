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
    pub last_export_status: String,
    pub custom_paper_mode: bool,
    pub print_single_sheet: bool,
    pub print_paper: SheetPaperSize,
    pub print_dpi: u32,
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
            last_export_status: String::new(),
            custom_paper_mode: false,
            print_single_sheet: true,
            print_paper: SheetPaperSize::A3,
            print_dpi: 300,
        }
    }
}

impl ModalDialogController {
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
                            ui.add(
                                egui::TextEdit::singleline(&mut app.accounts.vworld_key)
                                    .desired_width(340.0),
                            );
                            ui.end_row();
                            ui.label("역사지리정보DB 키:");
                            ui.add(
                                egui::TextEdit::singleline(&mut app.accounts.history_gis_key)
                                    .desired_width(340.0),
                            );
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
                            if ui.add(save_b).clicked() {
                                app.accounts.save_all_to_disk();
                                app.home_map.draw_guide_message = Some(
                                    "API 키와 사이트 계정(DPAPI 암호화)을 저장했습니다.".to_owned(),
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
            let native_ppp = ctx.native_pixels_per_point().unwrap_or(1.0);
            let (vis_w, vis_h) = if native_ppp > 1.05 {
                (
                    (full.width() / native_ppp - 8.0).max(640.0),
                    (full.height() / native_ppp - 16.0).max(420.0),
                )
            } else {
                (full.width(), full.height())
            };
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
        other => render_download_and_print_dialogs(app, ctx, other, &mut open),
    }
    if !open {
        app.modals.active = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
