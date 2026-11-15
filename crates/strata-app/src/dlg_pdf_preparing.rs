//! The progress window of 「PDF 내보내기」 — C++ `KaDrawingPdfProgress` (KaDrawingPdfProgress.cpp:10-58): the
//! title 「PDF 내보내기」, the step being done, a bar that runs while the length is not known, and 「취소」.

use std::sync::atomic::Ordering;

use eframe::egui::{self, Color32, Rect, Stroke, Vec2};

use crate::app_state::StrataAppState;
use crate::dialogs_modal::render_strata_modal_header;

pub(crate) fn render_pdf_preparing(app: &mut StrataAppState, ctx: &egui::Context, open: &mut bool) {
    // The step of the PDF being made; the first words of the C++ window before any (`:20`).
    let words = app.drawing.pdf.0.as_ref().map_or_else(|| "준비하는 중…".to_owned(), |job| job.words());
    let mut cancel = false;
    egui::Window::new("ka_modal_pdf_prep")
        .title_bar(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .fixed_size([440.0_f32.min(ctx.screen_rect().width() - 32.0), 132.0_f32.min(ctx.screen_rect().height() - 32.0)])
        .show(ctx, |ui| {
            render_strata_modal_header(ui, "PDF 내보내기", open);
            let mut lines = words.lines();
            ui.label(egui::RichText::new(lines.next().unwrap_or_default()).size(13.0));
            for note in lines {
                ui.label(egui::RichText::new(note).size(12.0).color(Color32::from_rgb(95, 94, 88)));
            }
            ui.add_space(6.0);
            // A thin bar whose block runs back and forth (QProgressDialog with the range 0, 0).
            let (bar, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 6.0), egui::Sense::hover());
            let p = ui.painter_at(bar);
            p.rect_filled(bar, 3.0, Color32::from_rgb(232, 230, 222));
            let phase = (ctx.input(|i| i.time) * 0.8).fract() as f32;
            let swing = if phase < 0.5 { phase * 2.0 } else { 2.0 - phase * 2.0 };
            let block = Rect::from_min_size(bar.min + Vec2::new((bar.width() - 90.0) * swing, 0.0), Vec2::new(90.0, bar.height()));
            p.rect_filled(block, 3.0, Color32::from_rgb(181, 87, 58));
            ui.add_space(8.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let cancel_button = egui::Button::new("취소")
                    .fill(Color32::WHITE)
                    .stroke(Stroke::new(1.0, Color32::from_rgb(155, 152, 142)))
                    .corner_radius(5.0)
                    .min_size(Vec2::new(58.0, 28.0));
                cancel = ui.add(cancel_button).clicked();
            });
        });
    // 취소, or the window's ×: the PDF stops, and no file is made (the window closes when it has stopped).
    if cancel || !*open {
        *open = true;
        match app.drawing.pdf.0.as_ref() {
            Some(job) => job.cancel.store(true, Ordering::Release),
            None => app.modals.active = None,
        }
    }
    ctx.request_repaint();
}
