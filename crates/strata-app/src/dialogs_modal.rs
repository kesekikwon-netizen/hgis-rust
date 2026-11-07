use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use strata_check_export::checklist_engine::ChecklistEngine;
use strata_check_export::submit_package::export_submission_package;
use strata_geo::trench_grid::TrenchStandardLength;
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
    pub trench_coverage_pct: f32,
    pub last_export_status: String,
    pub custom_paper_mode: bool,
}

impl Default for ModalDialogController {
    fn default() -> Self {
        Self {
            active: None,
            vworld_key_input: String::new(),
            heritage_id_input: String::new(),
            ngii_id_input: String::new(),
            trench_length: TrenchStandardLength::M20,
            trench_coverage_pct: 10.0,
            last_export_status: String::new(),
            custom_paper_mode: false,
        }
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
                            app.active_tab = MainTab::Drawing;
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
                .default_width(470.0)
                .show(ctx, |ui| {
                    render_strata_modal_header(ui, "DEM 내려받기 - Strata", &mut open);
                    ui.label(egui::RichText::new("DEM 자료를 내려받는 중입니다. 지도 작업을 계속할 수 있습니다.").size(15.5).color(Color32::from_rgb(25, 25, 24)));
                    ui.add_space(12.0);
                    let (bar_rect, _) = ui.allocate_exact_size(Vec2::new(450.0, 34.0), egui::Sense::hover());
                    let p = ui.painter_at(bar_rect);
                    p.rect_filled(bar_rect, 6.0, Color32::WHITE);
                    p.rect_stroke(bar_rect, 6.0, Stroke::new(1.2, Color32::from_rgb(150, 148, 142)), egui::StrokeKind::Middle);
                    p.rect_filled(Rect::from_min_size(bar_rect.min + Vec2::new(2.0, 2.0), Vec2::new(128.0, 30.0)), 5.0, Color32::from_rgb(20, 20, 20));
                    p.rect_filled(Rect::from_min_size(Pos2::new(bar_rect.right() - 92.0, bar_rect.top() + 2.0), Vec2::new(90.0, 30.0)), 5.0, Color32::from_rgb(20, 20, 20));
                    ui.add_space(10.0);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let cancel_btn = egui::Button::new(egui::RichText::new("취소").color(Color32::WHITE).strong().size(14.5)).fill(Color32::from_rgb(181, 87, 58)).corner_radius(6.0).min_size(Vec2::new(72.0, 38.0));
                        if ui.add(cancel_btn).clicked() { app.modals.active = None; }
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
        ActiveModalDialog::ExportSubmitPackage => {
            egui::Window::new("ka_modal_submit").title_bar(false).resizable(false).show(ctx, |ui| {
                render_strata_modal_header(ui, "국가유산청 제출 꾸러미 내보내기 - Strata", &mut open);
                let findings = ChecklistEngine::evaluate_sheet(&app.drawing.sheet, SubmitCrs::Epsg5179, true);
                for f in &findings {
                    let badge = if f.passed { "[통과]" } else { "[안내]" };
                    ui.label(format!("{badge} {} — {}", f.rule_id, f.message_ko));
                }
                ui.separator();
                if ui.button("EPSG:5179 제출 패키지 생성").clicked() {
                    let out_dir = app.home_map.real_survey.survey_dir.join("제출꾸러미_5179");
                    if let Ok(m) = export_submission_package(&out_dir, &findings) {
                        app.modals.last_export_status = format!("내보내기 완료: {} (파일 {}종, SHA-256 검증됨)", out_dir.display(), m.files_sha256.len());
                    }
                }
                if !app.modals.last_export_status.is_empty() { ui.label(&app.modals.last_export_status); }
            });
        }
        other => render_download_and_print_dialogs(app, ctx, other, &mut open),
    }
    if !open { app.modals.active = None; }
}
