//! The real current sheet, paper-clipped (target/cpp-ref/src/app/KaPrintDialog.cpp:455-456).
use crate::app_state::StrataAppState;
use eframe::egui::{self, Color32, Rect, Stroke, Vec2};

pub(crate) fn render_print_preview(app: &mut StrataAppState, ui: &mut egui::Ui, height: f32) {
    let size = Vec2::new(ui.available_width().max(1.0), height.max(1.0));
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 8.0, Color32::from_rgb(232, 230, 222));
    let [width, height] = [app.drawing.sheet.width_mm as f32, app.drawing.sheet.height_mm as f32];
    let room = rect.shrink(24.0);
    let px_per_mm = (room.width() / width).min(room.height() / height).max(0.01);
    let paper = Rect::from_center_size(room.center(), Vec2::new(width, height) * px_per_mm);
    painter.rect_filled(paper.translate(Vec2::new(3.0, 4.0)), 0.0, Color32::from_black_alpha(30));
    app.paint_sheet_on(painter.clone(), paper, px_per_mm);
    painter.rect_stroke(paper, 0.0, Stroke::new(1.0, Color32::from_rgb(180, 177, 167)), egui::StrokeKind::Outside);
}
