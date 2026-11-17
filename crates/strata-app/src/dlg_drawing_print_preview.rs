//! The real current sheet, paper-clipped (target/cpp-ref/src/app/KaPrintDialog.cpp:455-456).
use crate::app_state::StrataAppState;
use eframe::egui::{self, Color32, Rect, Stroke, Vec2};

pub(crate) fn render_print_preview(app: &mut StrataAppState, ui: &mut egui::Ui, height: f32) {
    let size = Vec2::new(ui.available_width().max(1.0), height.max(1.0));
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 8.0, Color32::from_rgb(232, 230, 222));
    let drawing = [app.drawing.sheet.width_mm, app.drawing.sheet.height_mm];
    let plan = app.printing.plan(drawing).ok();
    let dimensions = plan.as_ref().filter(|_| app.printing.single_sheet).map_or(drawing, |p| p.paper_mm);
    let [width, height] = dimensions.map(|v| v as f32);
    let room = rect.shrink(24.0);
    let px_per_mm = (room.width() / width).min(room.height() / height).max(0.01);
    let paper = Rect::from_center_size(room.center(), Vec2::new(width, height) * px_per_mm);
    painter.rect_filled(paper.translate(Vec2::new(3.0, 4.0)), 0.0, Color32::from_black_alpha(30));
    painter.rect_filled(paper, 0.0, Color32::WHITE);
    let target = if let Some(plan) = &plan {
        if app.printing.single_sheet {
            let [x, y, w, h] = plan.pages[0].target_mm.map(|v| v as f32 * px_per_mm);
            Rect::from_min_size(paper.min + Vec2::new(x, y), Vec2::new(w, h))
        } else {
            paper
        }
    } else {
        paper
    };
    app.paint_scaled_sheet(painter.with_clip_rect(paper), target);
    if let Some(plan) = plan.filter(|_| !app.printing.single_sheet) {
        for (index, page) in plan.pages.iter().enumerate() {
            let [x, y, w, h] = page.source_mm.map(|v| v as f32 * px_per_mm);
            let tile = Rect::from_min_size(paper.min + Vec2::new(x, y), Vec2::new(w, h)).intersect(paper);
            let p = painter.with_clip_rect(paper);
            p.rect_stroke(tile, 0.0, Stroke::new(1.0, Color32::from_rgb(181, 87, 58)), egui::StrokeKind::Inside);
            p.text(tile.left_top() + Vec2::splat(4.0), egui::Align2::LEFT_TOP, (index + 1).to_string(), egui::FontId::proportional(12.0), Color32::BLACK);
        }
    }
    painter.rect_stroke(paper, 0.0, Stroke::new(1.0, Color32::from_rgb(180, 177, 167)), egui::StrokeKind::Outside);
}
