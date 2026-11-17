//! Print tab actions, target/cpp-ref/src/app/KaPrintDialog.cpp:193-204.
use crate::app_state::{MainTab, StrataAppState};
use eframe::egui::{self, Color32};

pub(crate) fn render_print_actions(app: &mut StrataAppState, ui: &mut egui::Ui) {
    ui.horizontal_wrapped(|ui| {
        if ui.add_sized([140.0, 36.0], egui::Button::new("PDF 내보내기")).clicked() {
            app.export_sheet_pdf();
        }
        if !app.modals.print_single_sheet {
            ui.add_enabled(false, egui::Button::new("나눈 장을 PDF로 저장…")).on_disabled_hover_text("아직 Rust 판에서 연결하지 않았습니다 (지시 W-50)");
        }
        ui.add_enabled(false, egui::Button::new("인쇄").fill(Color32::from_rgb(181, 87, 58))).on_disabled_hover_text("아직 Rust 판에서 연결하지 않았습니다 (지시 UI3_PRINT)");
        if ui.add_sized([80.0, 36.0], egui::Button::new("닫기")).clicked() {
            app.close_studio_tab(MainTab::Print);
        }
    });
}

#[cfg(test)]
#[path = "dlg_drawing_print_actions_tests.rs"]
mod tests;
