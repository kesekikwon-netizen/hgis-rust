use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use strata_check_export::checklist_engine::ChecklistEngine;
use strata_check_export::submit_package::export_submission_package;
use strata_geo::trench_grid::{TrenchShapeKind, TrenchStandardLength};
use strata_layout::sheet_composer::{SheetOrientation, SheetPaperSize};
use strata_types::crs::SubmitCrs;

use crate::app_state::{MainTab, StrataAppState};
use crate::dialogs_download_print::render_download_and_print_dialogs;
use crate::ribbon_icons::paint_strata_logo;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveModalDialog {
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
        ui.label(egui::RichText::new(title).size(13.5).color(Color32::from_rgb(30, 30, 28)));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let close_btn = egui::Button::new(egui::RichText::new("×").size(18.0).color(Color32::from_rgb(55, 54, 50)))
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
    let Some(kind) = app.modals.active else { return };
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
                    ui.label(egui::RichText::new(&app.accounts.status_summary_ko).size(12.5).strong());
                    ui.separator();
                    let show_pw = app.accounts.show_passwords;
                    egui::Grid::new("ka_account_grid").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
                        ui.label("VWorld API 키:");
                        ui.add(egui::TextEdit::singleline(&mut app.accounts.vworld_key).desired_width(340.0));
                        ui.end_row();
                        ui.label("역사지리정보DB 키:");
                        ui.add(egui::TextEdit::singleline(&mut app.accounts.history_gis_key).desired_width(340.0));
                        ui.end_row();
                        ui.label("VWorld 지적도 ID / PW:");
                        ui.horizontal(|ui| {
                            ui.add(egui::TextEdit::singleline(&mut app.accounts.cadastral_creds.username).desired_width(140.0));
                            ui.add(egui::TextEdit::singleline(&mut app.accounts.cadastral_creds.password).password(!show_pw).desired_width(188.0));
                        });
                        ui.end_row();
                        ui.label("수치지형도(NGII) ID / PW:");
                        ui.horizontal(|ui| {
                            ui.add(egui::TextEdit::singleline(&mut app.accounts.ngii_creds.username).desired_width(140.0));
                            ui.add(egui::TextEdit::singleline(&mut app.accounts.ngii_creds.password).password(!show_pw).desired_width(188.0));
                        });
                        ui.end_row();
                        ui.label("국가유산청 인트라넷 ID / PW:");
                        ui.horizontal(|ui| {
                            ui.add(egui::TextEdit::singleline(&mut app.accounts.heritage_creds.username).desired_width(140.0));
                            ui.add(egui::TextEdit::singleline(&mut app.accounts.heritage_creds.password).password(!show_pw).desired_width(188.0));
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
                            let save_b = egui::Button::new(egui::RichText::new("저장").color(Color32::WHITE).strong())
                                .fill(Color32::from_rgb(181, 87, 58))
                                .corner_radius(6.0)
                                .min_size(Vec2::new(78.0, 30.0));
                            if ui.add(save_b).clicked() {
                                app.accounts.save_all_to_disk();
                                app.home_map.draw_guide_message = Some("API 키와 사이트 계정(DPAPI 암호화)을 저장했습니다.".to_owned());
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
            egui::Window::new("ka_modal_submit")
                .title_bar(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .default_width(640.0)
                .show(ctx, |ui| {
                    render_strata_modal_header(ui, "국가유산청 검수 · 제출 꾸러미 내보내기 (26개 규칙) - Strata", &mut open);
                    let has_boundary = !app.home_map.real_survey.survey_rings.is_empty();
                    let findings = ChecklistEngine::evaluate_sheet(&app.drawing.sheet, SubmitCrs::Epsg5179, has_boundary);
                    let (err_cnt, warn_cnt, pass_cnt) = ChecklistEngine::count_by_severity(&findings);

                    ui.columns(3, |cols| {
                        for (ci, (lbl, cnt, bg, fg)) in [
                            ("오류 (제출 차단)", err_cnt, Color32::from_rgb(254, 242, 242), Color32::from_rgb(185, 28, 28)),
                            ("주의 (권장 확인)", warn_cnt, Color32::from_rgb(255, 251, 235), Color32::from_rgb(180, 83, 9)),
                            ("통과 (무결성 검증)", pass_cnt, Color32::from_rgb(240, 253, 244), Color32::from_rgb(21, 128, 61)),
                        ]
                        .into_iter()
                        .enumerate()
                        {
                            egui::Frame::NONE
                                .fill(bg)
                                .stroke(Stroke::new(1.0, fg))
                                .corner_radius(6.0)
                                .inner_margin(egui::Margin::symmetric(10, 8))
                                .show(&mut cols[ci], |ui| {
                                    ui.label(egui::RichText::new(lbl).size(11.5).color(fg));
                                    ui.label(egui::RichText::new(format!("{cnt}건")).size(18.0).strong().color(fg));
                                });
                        }
                    });
                    ui.add_space(8.0);
                    egui::ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
                        for f in &findings {
                            ui.horizontal(|ui| {
                                let (badge, col) = if f.passed {
                                    ("[통과]", Color32::from_rgb(21, 128, 61))
                                } else {
                                    ("[주의]", Color32::from_rgb(180, 83, 9))
                                };
                                ui.colored_label(col, egui::RichText::new(badge).strong().size(11.5));
                                ui.label(egui::RichText::new(format!("[{}] {}", f.category_ko, f.message_ko)).size(12.0));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.small_button("위치 보기").clicked() {
                                        if let Some(xy) = f.target_world_xy {
                                            app.home_map.viewport.center_xy = xy;
                                            app.home_map.viewport.meters_per_pixel = 1.8;
                                        }
                                        app.active_tab = MainTab::Map;
                                        app.home_map.draw_guide_message = Some(format!(
                                            "검수 항목 「{}」 해당 피처 위치로 이동했습니다.",
                                            f.rule_id
                                        ));
                                        app.modals.active = None;
                                    }
                                });
                            });
                        }
                    });
                    ui.separator();
                    ui.horizontal(|ui| {
                        let export_btn = egui::Button::new(
                            egui::RichText::new("EPSG:5179 제출 패키지 생성 (SHP + PDF + MANIFEST.sha256)")
                                .color(Color32::WHITE)
                                .strong()
                                .size(13.0),
                        )
                        .fill(Color32::from_rgb(181, 87, 58))
                        .corner_radius(6.0)
                        .min_size(Vec2::new(360.0, 34.0));
                        if ui.add(export_btn).clicked() {
                            let out_dir = app.home_map.real_survey.survey_dir.join("제출꾸러미_5179");
                            if let Ok(m) = export_submission_package(&out_dir, &findings) {
                                app.modals.last_export_status = format!(
                                    "내보내기 완료: {} (파일 {}종, 26개 검수 통과 {pass_cnt}/26, SHA-256 검증됨)",
                                    out_dir.display(),
                                    m.files_sha256.len()
                                );
                            }
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("닫기").clicked() {
                                app.modals.active = None;
                            }
                        });
                    });
                    if !app.modals.last_export_status.is_empty() {
                        ui.add_space(4.0);
                        ui.colored_label(Color32::from_rgb(21, 128, 61), &app.modals.last_export_status);
                    }
                });
        }
        other => render_download_and_print_dialogs(app, ctx, other, &mut open),
    }
    if !open { app.modals.active = None; }
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
