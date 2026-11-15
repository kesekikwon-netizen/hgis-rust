//! The items of a sheet under the mouse. The sheet places its items by itself (legend, title block, north
//! arrow, scale bar, scale text, coordinate system); the user takes one and puts it somewhere else on the
//! paper, as with the select tool of the C++ studio, which is on whenever no other tool is
//! (`KaDrawingStudio.cpp:1891-1897`). Owner 2026-11-15: 「조판에서 자동으로 배치를 하지만 모든것을 사용자가
//! 움직일수있게하라」. What is kept is the way from the sheet's own place, in millimetres of paper.

use eframe::egui::{self, emath::TSTransform, layers::ShapeIdx, Color32, Rect, Stroke, Vec2};

use super::gui_tab_drawing_frame::SheetFrame;
use crate::app_state::StrataAppState;
use crate::tab_drawing::SheetItem;

/// The offset that keeps `placed` on the paper when it is moved by `wanted`.
pub(crate) fn kept_on_paper(wanted: Vec2, placed: Rect, paper: Rect) -> Vec2 {
    let along = |wanted: f32, low: f32, high: f32| wanted.max(low).min(high.max(low));
    Vec2::new(along(wanted.x, paper.left() - placed.left(), paper.right() - placed.right()), along(wanted.y, paper.top() - placed.top(), paper.bottom() - placed.bottom()))
}

/// How many shapes the sheet has painted so far: the start of an item.
pub(crate) fn sheet_shapes(f: &SheetFrame) -> ShapeIdx {
    f.sheet_p.ctx().graphics_mut(|layers| layers.entry(f.sheet_p.layer_id()).next_idx())
}

impl StrataAppState {
    /// How far the user moved an item, in screen points; an item nobody moved stays exactly where the sheet
    /// places it.
    pub(crate) fn sheet_item_offset(&self, item: SheetItem, placed: Rect, f: &SheetFrame) -> Vec2 {
        self.drawing.item_offsets_mm.get(&item).map_or(Vec2::ZERO, |[x, y]| kept_on_paper(Vec2::new(*x, *y) * f.px_per_mm, placed, f.sheet_rect))
    }

    /// The shapes painted since `start` are the item, which the sheet placed at `placed`: moves them to
    /// where the user put it, and remembers the item for the mouse.
    pub(crate) fn place_sheet_item(&mut self, f: &SheetFrame, item: SheetItem, start: ShapeIdx, placed: Rect) {
        let moved = self.sheet_item_offset(item, placed, f);
        if moved != Vec2::ZERO {
            f.sheet_p.ctx().graphics_mut(|layers| {
                let shapes = layers.entry(f.sheet_p.layer_id());
                let end = shapes.next_idx();
                shapes.transform_range(start, end, TSTransform::from_translation(moved));
            });
        }
        self.drawing.item_rects.push((item, placed, moved));
    }

    /// The items as they were painted last, under the mouse: a drag moves the item, kept on the paper.
    /// Not while the map is adjusted or coordinate points are placed (one tool of the sheet at a time).
    pub(crate) fn move_sheet_items(&mut self, ui: &egui::Ui, paper: Rect, px_per_mm: f32) {
        let painted = std::mem::take(&mut self.drawing.item_rects);
        if self.drawing.map_adjust_active || self.drawing.coord_callout_mode {
            return;
        }
        for (item, placed, moved) in painted {
            let now = placed.translate(moved);
            let response = ui.interact(now, ui.id().with(("sheet_item", item)), egui::Sense::drag());
            if response.hovered() || response.dragged() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Move);
                ui.painter().rect_stroke(now.expand(2.0), 2.0, Stroke::new(1.0, Color32::from_rgb(181, 87, 58)), egui::StrokeKind::Middle);
            }
            if response.dragged_by(egui::PointerButton::Primary) {
                let kept = kept_on_paper(moved + response.drag_delta(), placed, paper) / px_per_mm;
                self.drawing.item_offsets_mm.insert(item, [kept.x, kept.y]);
            }
        }
    }

    /// 「항목 이동」 goes back to taking items (`useSelectTool`): not while the map is adjusted, and it ends
    /// the placing of coordinate points.
    pub(crate) fn use_sheet_select_tool(&mut self) {
        if self.drawing.map_adjust_active {
            return;
        }
        self.drawing.coord_callout_mode = false;
        self.show_status_message("항목을 눌러 끌면 옮겨집니다.");
    }
}

#[cfg(test)]
#[path = "gui_tab_drawing_items_tests.rs"]
mod tests;
