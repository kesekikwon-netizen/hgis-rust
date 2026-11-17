//! Shared real sheet scene for print/export, target/cpp-ref/src/app/KaDrawingStudio.cpp:3758-3855.
use crate::app_state::StrataAppState;
use eframe::egui::{self, Color32, Painter, Rect};

impl StrataAppState {
    // Scale the same scene typography as the output, rather than relayout at thumbnail font sizes.
    // target/cpp-ref/src/app/KaPrintDialog.cpp:74-81,455-456.
    pub(crate) fn paint_scaled_sheet(&mut self, mut painter: Painter, target: Rect) {
        use egui::{emath::TSTransform, layers::ShapeIdx, Pos2, Vec2};
        let design = self.drawing.screen_px_per_mm.unwrap_or(1.65);
        let source = Rect::from_min_size(Pos2::ZERO, Vec2::new(self.drawing.sheet.width_mm as f32, self.drawing.sheet.height_mm as f32) * design);
        let clip = painter.clip_rect();
        let (ctx, layer) = (painter.ctx().clone(), painter.layer_id());
        let start = ctx.graphics_mut(|layers| layers.entry(layer).next_idx());
        painter.set_clip_rect(source);
        self.paint_sheet_on(painter, source, design);
        let transform = TSTransform::new(target.min.to_vec2(), target.width() / source.width());
        ctx.graphics_mut(|layers| {
            let shapes = layers.entry(layer);
            let end = shapes.next_idx();
            shapes.transform_range(start, end, transform);
            for index in start.0..end.0 {
                shapes.mutate_shape(ShapeIdx(index), |shape| shape.clip_rect = shape.clip_rect.intersect(clip));
            }
        });
    }

    pub(crate) fn paint_sheet_for_print(&mut self, ctx: &egui::Context, paper: Rect, px_per_mm: f32) {
        self.paint_sheet_on(ctx.layer_painter(egui::LayerId::background()), paper, px_per_mm);
    }

    pub(crate) fn paint_sheet_on(&mut self, painter: Painter, paper: Rect, px_per_mm: f32) {
        painter.rect_filled(paper, 0.0, Color32::WHITE);
        let screen_items = std::mem::take(&mut self.drawing.item_rects);
        let f = self.sheet_frame_for(paper, paper, px_per_mm, painter, false);
        self.paint_sheet_map_layers(&f);
        self.paint_sheet_grid(&f);
        self.paint_sheet_legend(&f);
        self.paint_sheet_chrome(&f);
        self.paint_sheet_coord_points(&f);
        self.drawing.item_rects = screen_items;
    }
}
