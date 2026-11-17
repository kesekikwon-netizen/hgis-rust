//! Print tab actions, target/cpp-ref/src/app/KaPrintDialog.cpp:193-204.
use crate::app_state::{MainTab, StrataAppState};
use eframe::egui::{self, Color32};

pub(crate) fn render_print_actions(app: &mut StrataAppState, ui: &mut egui::Ui) {
    if let Some(words) = app.printing.job.words() {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(words);
            if ui.button("멈추기").clicked() {
                app.printing.job.cancel();
            }
        });
    }
    ui.horizontal_wrapped(|ui| {
        if ui.add_sized([140.0, 36.0], egui::Button::new("PDF 내보내기")).clicked() {
            app.export_sheet_pdf();
        }
        if !app.printing.single_sheet {
            ui.add_enabled(false, egui::Button::new("나눈 장을 PDF로 저장…")).on_disabled_hover_text("아직 Rust 판에서 연결하지 않았습니다 (지시 W-50)");
        }
        let request = app.printing.request(&app.drawing.title_ko, [app.drawing.sheet.width_mm, app.drawing.sheet.height_mm]);
        let words = request.as_ref().map(|r| format!("인쇄 · {}장", r.plan.pages.len() + usize::from(r.plan.overview))).unwrap_or_else(|_| "인쇄".to_owned());
        let ready = request.is_ok() && app.printing.job.0.is_none();
        let button = egui::Button::new(egui::RichText::new(words).color(Color32::WHITE)).fill(Color32::from_rgb(181, 87, 58)).min_size(egui::vec2(132.0, 36.0));
        let response = ui.add_enabled(ready, button);
        if let Err(why) = &request {
            response.clone().on_disabled_hover_text(why);
        }
        if response.clicked() {
            if let Err(why) = app.start_device_print_using(Box::new(super::print_device::dispatch)) {
                app.show_status_message(format!("인쇄하지 못했습니다 · {why}"));
            }
        }
        if ui.add_sized([80.0, 36.0], egui::Button::new("닫기")).clicked() {
            app.close_studio_tab(MainTab::Print);
        }
    });
}

#[cfg(test)]
#[path = "dlg_drawing_print_actions_tests.rs"]
mod tests;
