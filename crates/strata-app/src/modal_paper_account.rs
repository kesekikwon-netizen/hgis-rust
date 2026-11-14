//! `dialogs_modal` 에서 옮긴 대화상자 묶음: PaperSetup, AccountSettings.
//!
//! 분할 전에는 `render_active_modal_dialogs` 의 match arm 이었다.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use strata_check_export::checklist_engine::ChecklistEngine;
use strata_layout::sheet_composer::{DrawingSheet, SheetOrientation, SheetPaperSize};
use strata_types::crs::SubmitCrs;

use crate::app_state::{MainTab, StrataAppState};
use crate::dialogs_modal::render_strata_modal_header;
use crate::modal_types::ModalDialogController;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PaperChoice { A4, A3, Custom }

#[derive(Debug, Clone, Copy, PartialEq)]
struct PaperDialogDraft { choice: PaperChoice, orientation: SheetOrientation, width_mm: f64, height_mm: f64, last_pass: u64 }

impl PaperDialogDraft {
    // target/cpp-ref/src/app/KaPaperDialog.cpp:64-66, 80-81, 99-100, 179
    const fn new_default(pass: u64) -> Self {
        Self { choice: PaperChoice::A4, orientation: SheetOrientation::Portrait, width_mm: DrawingSheet::A4_PORTRAIT_WIDTH_MM, height_mm: DrawingSheet::A4_PORTRAIT_HEIGHT_MM, last_pass: pass }
    }

    // target/cpp-ref/src/app/KaPaperDialog.cpp:161-171
    fn apply_preset(&mut self) {
        let sz = match self.choice { PaperChoice::A4 => Some(SheetPaperSize::A4), PaperChoice::A3 => Some(SheetPaperSize::A3), PaperChoice::Custom => None };
        if let Some(paper) = sz { (self.width_mm, self.height_mm) = DrawingSheet::preset_dimensions_mm(paper, self.orientation); }
    }

    // target/cpp-ref/src/app/KaPaperDialog.cpp:149-153 — only A4 landscape shows the field-sheet warning.
    fn scale_note(&self) -> &'static str {
        let ori = if self.choice == PaperChoice::A4 && self.orientation == SheetOrientation::Landscape { SheetOrientation::Landscape } else { SheetOrientation::Portrait };
        ModalDialogController::paper_scale_note(SheetPaperSize::A4, ori)
    }
}

pub(crate) fn render_paper_setup(app: &mut StrataAppState, ctx: &egui::Context, open: &mut bool) {
    let draft_id = egui::Id::new("ka_paper_dialog_draft");
    let pass = ctx.cumulative_pass_nr();
    let mut draft = ctx.data(|d| d.get_temp::<PaperDialogDraft>(draft_id)).filter(|d| pass <= d.last_pass.saturating_add(1)).unwrap_or_else(|| PaperDialogDraft::new_default(pass));
    draft.last_pass = pass;
    let mut clear_draft = false;

    egui::Window::new("ka_modal_paper_setup").title_bar(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).default_width(480.0).show(ctx, |ui| {
            // target/cpp-ref/src/app/KaPaperDialog.cpp:45 — setWindowTitle("용지 설정")
            render_strata_modal_header(ui, "용지 설정", open);
            ui.horizontal(|ui| {
                ui.add_sized([48.0, 56.0], egui::Label::new(egui::RichText::new("용지").size(16.0)));
                for (label, choice) in [("A4", PaperChoice::A4), ("A3", PaperChoice::A3), ("직접 입력", PaperChoice::Custom)] {
                    let selected = draft.choice == choice;
                    let fill = if selected { Color32::from_rgb(214, 208, 194) } else { Color32::WHITE };
                    let stroke = if selected { Stroke::new(2.0, Color32::from_rgb(38, 37, 34)) } else { Stroke::new(1.0, Color32::from_rgb(225, 223, 215)) };
                    let btn = egui::Button::new(egui::RichText::new(label).size(15.0)).fill(fill).stroke(stroke).corner_radius(0.0).min_size(Vec2::new(112.0, 56.0));
                    if ui.add(btn).clicked() {
                        draft.choice = choice;
                        draft.apply_preset();
                    }
                }
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.add_sized([48.0, 74.0], egui::Label::new(egui::RichText::new("방향").size(16.0)));
                for (label, ori, icon_sz) in [("세로", SheetOrientation::Portrait, Vec2::new(16.0, 26.0)), ("가로", SheetOrientation::Landscape, Vec2::new(28.0, 16.0))] {
                    let selected = draft.orientation == ori;
                    let (rect, resp) = ui.allocate_exact_size(Vec2::new(112.0, 74.0), egui::Sense::click());
                    resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, ui.is_enabled(), selected, label));
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
                        draft.orientation = ori;
                        draft.apply_preset();
                    }
                }
            });
            ui.add_space(12.0);
            // target/cpp-ref/src/app/KaPaperDialog.cpp:44, 90-134, 154-175 — paperLook shows a reading line
            // unless 「직접 입력」 is active; stock look keeps its disabled spin fields.
            let custom = draft.choice == PaperChoice::Custom;
            for (dim_label, is_width) in [("가로", true), ("세로", false)] {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(dim_label).size(16.0));
                    if !app.paper_look || custom {
                        let target = if is_width { &mut draft.width_mm } else { &mut draft.height_mm };
                        let _ = ui.add_enabled(custom, egui::DragValue::new(target).range(50.0..=2000.0).speed(1.0).fixed_decimals(1).suffix(" mm"));
                    } else {
                        let val_mm = if is_width { draft.width_mm } else { draft.height_mm };
                        ui.label(egui::RichText::new("........................................................................").color(Color32::from_rgb(150, 148, 140)));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(egui::RichText::new("mm").size(15.0).color(Color32::from_rgb(94, 93, 89)));
                            ui.label(egui::RichText::new(ModalDialogController::plain_mm(val_mm)).monospace().strong().size(15.0).color(Color32::from_rgb(28, 25, 23)));
                        });
                    }
                });
                ui.add_space(8.0);
            }
            ui.label(egui::RichText::new(draft.scale_note()).size(14.5).color(Color32::from_rgb(38, 37, 34)));
            ui.add_space(12.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let cancel_btn = egui::Button::new(egui::RichText::new("취소").size(14.5)).fill(Color32::WHITE).stroke(Stroke::new(1.0, Color32::from_rgb(140, 138, 130))).corner_radius(6.0).min_size(Vec2::new(76.0, 38.0));
                if ui.add(cancel_btn).clicked() {
                    app.modals.active = None;
                    clear_draft = true;
                }
                let ok_btn = egui::Button::new(egui::RichText::new("용지 만들기").color(Color32::WHITE).strong().size(14.5)).fill(Color32::from_rgb(181, 87, 58)).corner_radius(6.0).min_size(Vec2::new(134.0, 38.0));
                if ui.add(ok_btn).clicked() {
                    // target/cpp-ref/src/app/MainWindowExport.cpp:216-255 + KaDrawingStudio.cpp:447-454, 555-559
                    let was_open = app.open_studio_tabs.contains(&MainTab::Drawing);
                    app.modals.custom_paper_mode = custom;
                    match draft.choice {
                        PaperChoice::A4 => app.drawing.set_paper_and_orientation(SheetPaperSize::A4, draft.orientation),
                        PaperChoice::A3 => app.drawing.set_paper_and_orientation(SheetPaperSize::A3, draft.orientation),
                        PaperChoice::Custom => {
                            app.drawing.sheet.reset_paper_mm(draft.width_mm, draft.height_mm);
                            app.drawing.last_findings = ChecklistEngine::evaluate_sheet(&app.drawing.sheet, SubmitCrs::Epsg5179, true);
                        }
                    }
                    app.open_studio_tab(MainTab::Drawing);
                    app.hide_sub_tools();
                    app.home_map.draw_guide_message = Some(if was_open {
                        let (w, h) = (app.drawing.sheet.width_mm.round() as i64, app.drawing.sheet.height_mm.round() as i64);
                        format!("용지 전환 완료: {w}×{h} mm — 기존 지도 범위와 배치가 유지되었습니다.")
                    } else {
                        app.drawing.sheet_center = None;
                        StrataAppState::DRAWING_STUDIO_WELCOME.to_owned()
                    });
                    app.modals.active = None;
                    clear_draft = true;
                }
            });
        });
    if clear_draft || !*open {
        ctx.data_mut(|d| d.remove::<PaperDialogDraft>(draft_id));
    } else {
        ctx.data_mut(|d| d.insert_temp(draft_id, draft));
    }
}

pub(crate) fn render_account_settings(app: &mut StrataAppState, ctx: &egui::Context, open: &mut bool) {
    egui::Window::new("ka_modal_account")
        .title_bar(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .default_width(520.0)
        .show(ctx, |ui| {
            render_strata_modal_header(ui, "API 키 · 사이트 계정 설정 - Strata", open);
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
                        match app.accounts.save_all_to_disk() {
                            Ok(()) => {
                                app.home_map.draw_guide_message = Some(
                                    "API 키와 사이트 계정(DPAPI 암호화)을 저장했습니다."
                                        .to_owned(),
                                );
                                app.modals.active = None;
                            }
                            // 암호화하지 못하면 창을 닫지 않고 그 까닭을 보인다.
                            Err(message) => app.accounts.validation_error = Some(message),
                        }
                    }
                });
            });
        });
}

#[cfg(test)]
#[path = "modal_paper_account_tests.rs"]
mod tests;
