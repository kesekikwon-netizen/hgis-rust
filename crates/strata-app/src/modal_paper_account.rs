//! Account dialog and paper entry facade, target/cpp-ref/src/app/KaPaperDialog.cpp:43-198.
use eframe::egui::{self, Color32, Vec2};
use crate::app_state::StrataAppState;
use crate::dialogs_modal::render_strata_modal_header;

#[path = "modal_paper.rs"]
mod paper;
pub(crate) use paper::{note_paper_trigger, paper_keyboard_entry, render_paper_setup, PaperTrigger};

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
