//! `dialogs_modal` 에서 옮긴 대화상자 묶음: PaperSetup, AccountSettings.
//!
//! 분할 전에는 `render_active_modal_dialogs` 의 match arm 이었다.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use strata_layout::sheet_composer::{SheetOrientation, SheetPaperSize};
use crate::app_state::{MainTab, StrataAppState};
use crate::dialogs_modal::render_strata_modal_header;
use crate::modal_types::ModalDialogController;


pub(crate) fn render_paper_setup(app: &mut StrataAppState, ctx: &egui::Context, mut mut open: &mut bool) {
    egui::Window::new("ka_modal_paper_setup")
        .title_bar(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .default_width(480.0)
        .show(ctx, |ui| {
            render_strata_modal_header(ui, "용지 설정 - Strata", open);
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
            // KaPaperDialog.cpp:90-134 — 새 모양: the size that follows from the paper is a
            // reading line on a dotted leader; the entry field stands in the same place and
            // shows only under 「직접 입력」.
            for (dim_label, is_width) in [("가로", true), ("세로", false)] {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(dim_label).size(16.0));
                    if app.modals.custom_paper_mode {
                        let spin = if is_width {
                            egui::DragValue::new(&mut app.drawing.sheet.width_mm)
                        } else {
                            egui::DragValue::new(&mut app.drawing.sheet.height_mm)
                        };
                        let _ = ui.add(
                            spin.range(50.0..=2000.0)
                                .speed(1.0)
                                .fixed_decimals(1)
                                .suffix(" mm"),
                        );
                    } else {
                        ui.label(egui::RichText::new("........................................................................").color(Color32::from_rgb(150, 148, 140)));
                        let val_mm = if is_width {
                            app.drawing.sheet.width_mm
                        } else {
                            app.drawing.sheet.height_mm
                        };
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(egui::RichText::new("mm").size(15.0).color(Color32::from_rgb(94, 93, 89)));
                            ui.label(egui::RichText::new(ModalDialogController::plain_mm(val_mm)).strong().size(15.0));
                        });
                    }
                });
                ui.add_space(8.0);
            }
            ui.label(egui::RichText::new(ModalDialogController::paper_scale_note(
                app.drawing.sheet.paper_size,
                app.drawing.sheet.orientation,
            )).size(14.5).color(Color32::from_rgb(38, 37, 34)));
            ui.add_space(12.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let cancel_btn = egui::Button::new(egui::RichText::new("취소").size(14.5)).fill(Color32::WHITE).stroke(Stroke::new(1.0, Color32::from_rgb(140, 138, 130))).corner_radius(6.0).min_size(Vec2::new(76.0, 38.0));
                if ui.add(cancel_btn).clicked() { app.modals.active = None; }
                let ok_btn = egui::Button::new(egui::RichText::new("용지 만들기").color(Color32::WHITE).strong().size(14.5)).fill(Color32::from_rgb(181, 87, 58)).corner_radius(6.0).min_size(Vec2::new(134.0, 38.0));
                if ui.add(ok_btn).clicked() {
                    app.open_studio_tab(MainTab::Drawing);
                    app.modals.active = None;
                    app.home_map.draw_guide_message =
                        Some(StrataAppState::DRAWING_STUDIO_WELCOME.to_owned());
                }
            });
        });
}

pub(crate) fn render_account_settings(app: &mut StrataAppState, ctx: &egui::Context, mut mut open: &mut bool) {
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
