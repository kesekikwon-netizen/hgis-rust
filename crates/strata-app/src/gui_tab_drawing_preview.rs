//! Shared real sheet scene for print/export, target/cpp-ref/src/app/KaDrawingStudio.cpp:3758-3855.
use eframe::egui::{self, Color32, Painter, Rect};
use crate::app_state::StrataAppState;

impl StrataAppState {
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
