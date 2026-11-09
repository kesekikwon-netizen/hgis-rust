//! `dialogs_download_print` 에서 옮긴 「PdfPreparing」 대화상자.
//!
//! 분할 전에는 `render_download_and_print_dialogs` 의 match arm 이었다.

use eframe::egui::{self, Color32, Rect, Stroke, Vec2};

use crate::app_state::StrataAppState;
use crate::dialogs_modal::render_strata_modal_header;

pub(crate) fn render_pdf_preparing(app: &mut StrataAppState, ctx: &egui::Context, open: &mut bool) {
    egui::Window::new("ka_modal_pdf_prep")
        .title_bar(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .fixed_size([
            360.0_f32.min(ctx.screen_rect().width() - 32.0),
            128.0_f32.min(ctx.screen_rect().height() - 32.0),
        ])
        .show(ctx, |ui| {
            render_strata_modal_header(ui, "인쇄 준비 - Strata", open);
            ui.label(
                egui::RichText::new("PDF를 그리는 중... (용지 210x297 mm · 300 DPI)")
                    .size(13.0),
            );
            ui.label(
                egui::RichText::new("취소하면 그리기가 끝난 뒤 파일을 만들지 않습니다.")
                    .size(12.0)
                    .color(Color32::from_rgb(95, 94, 88)),
            );
            ui.add_space(6.0);
            let (bar_r, _) =
                ui.allocate_exact_size(Vec2::new(340.0, 18.0), egui::Sense::hover());
            let p = ui.painter_at(bar_r);
            p.rect_filled(bar_r, 3.0, Color32::WHITE);
            p.rect_stroke(
                bar_r,
                3.0,
                Stroke::new(1.0, Color32::from_rgb(160, 158, 150)),
                egui::StrokeKind::Middle,
            );
            p.rect_filled(
                Rect::from_center_size(bar_r.center(), Vec2::new(160.0, 14.0)),
                2.0,
                Color32::from_rgb(25, 25, 24),
            );
            ui.add_space(6.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let cb = egui::Button::new("취소")
                    .fill(Color32::WHITE)
                    .stroke(Stroke::new(1.0, Color32::from_rgb(155, 152, 142)))
                    .corner_radius(5.0)
                    .min_size(Vec2::new(58.0, 28.0));
                if ui.add(cb).clicked() {
                    app.modals.active = None;
                }
            });
        });
}
