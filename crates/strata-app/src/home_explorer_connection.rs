//! Keep connection actions in a compact menu, target/cpp-ref/src/app/KaHomeConnectionCard.cpp:66-72,123-207.
use crate::app_state::StrataAppState;
use crate::home_connection_model::{conn_rows, internet_label, InternetCheck, INTERNET_BUTTON_TIP};
use eframe::egui::{self, Color32};

impl StrataAppState {
    pub(super) fn render_explorer_connection(&mut self, ui: &mut egui::Ui) {
        if self.home_map.internet.poll() || matches!(self.home_map.internet, InternetCheck::Checking(_)) {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
        }
        ui.menu_button("연결 상태", |ui| {
            ui.set_width(300.0);
            ui.label("비밀값은 표시되지 않습니다");
            ui.separator();
            let net = internet_label(&self.home_map.internet);
            ui.horizontal(|ui| {
                if let Some(ready) = net.dot {
                    ui.colored_label(if ready { Color32::DARK_GREEN } else { Color32::DARK_RED }, "●");
                }
                let label = ui.label(net.text);
                if let Some(tip) = net.tip {
                    label.on_hover_text(tip);
                }
            });
            if ui.add_enabled(net.button_enabled, egui::Button::new("인터넷 확인")).on_hover_text(INTERNET_BUTTON_TIP).clicked() {
                self.home_map.internet = InternetCheck::start();
            }
            for row in conn_rows(&self.accounts.status, self.paper_look) {
                ui.separator();
                ui.horizontal(|ui| {
                    if row.missing_dot {
                        ui.colored_label(Color32::from_rgb(185, 90, 57), "●");
                    }
                    ui.strong(row.name).on_hover_text(row.name_tip);
                    let state = ui.colored_label(if row.ready { Color32::DARK_GREEN } else { ui.visuals().text_color() }, row.state);
                    if let Some(tip) = row.state_tip {
                        state.on_hover_text(tip);
                    }
                });
                if ui.button("설정").on_hover_text(row.setup_tip).clicked() {
                    self.modals.active = Some(crate::dialogs_modal::ActiveModalDialog::AccountSettings);
                    ui.close_menu();
                }
            }
        });
    }
}
