//! The items of a sheet under the mouse. The sheet places its items by itself (legend, title block, north
//! arrow, scale bar, scale text, coordinate system); the user takes one and puts it somewhere else on the
//! paper, as with the select tool of the C++ studio, which is on whenever no other tool is
//! (`KaDrawingStudio.cpp:1891-1897`). Owner 2026-11-15: 「조판에서 자동으로 배치를 하지만 모든것을 사용자가
//! 움직일수있게하라」. What is kept is the way from the sheet's own place, in millimetres of paper.

use eframe::egui::{self, emath::TSTransform, layers::ShapeIdx, Color32, Pos2, Rect, Stroke, Vec2};

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

/// The frame of the selected item with a handle at each corner (the C++ selection of a layout item).
fn paint_selection(painter: &egui::Painter, item: Rect) {
    let (clay, frame) = (Color32::from_rgb(181, 87, 58), item.expand(3.0));
    painter.rect_stroke(frame, 1.0, Stroke::new(1.4, clay), egui::StrokeKind::Middle);
    for corner in [frame.left_top(), frame.right_top(), frame.left_bottom(), frame.right_bottom()] {
        let handle = Rect::from_center_size(corner, Vec2::splat(6.0));
        painter.rect_filled(handle, 0.0, Color32::WHITE);
        painter.rect_stroke(handle, 0.0, Stroke::new(1.2, clay), egui::StrokeKind::Middle);
    }
}

impl StrataAppState {
    /// How far the user moved an item, in screen points; an item nobody moved stays exactly where the sheet
    /// places it.
    pub(crate) fn sheet_item_offset(&self, item: SheetItem, placed: Rect, f: &SheetFrame) -> Vec2 {
        self.drawing.item_offsets_mm.get(&item).map_or(Vec2::ZERO, |[x, y]| {
            let wanted = Vec2::new(*x, *y) * f.px_per_mm;
            if item == SheetItem::Legend {
                wanted
            } else {
                kept_on_paper(wanted, placed, f.sheet_rect)
            }
        })
    }

    /// The shapes painted since `start` are the item, which the sheet placed at `placed`: moves them to
    /// where the user put it, and remembers the item for the mouse.
    pub(crate) fn place_sheet_item(&mut self, f: &SheetFrame, item: SheetItem, start: ShapeIdx, placed: Rect) {
        if !self.sheet_item_on(item) {
            // Taken off with Delete: what was painted for it is wiped, and the mouse does not find it.
            f.sheet_p.ctx().graphics_mut(|layers| {
                let shapes = layers.entry(f.sheet_p.layer_id());
                for index in start.0..shapes.next_idx().0 {
                    shapes.reset_shape(ShapeIdx(index));
                }
            });
            return;
        }
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

    /// The items as they were painted last, under the mouse: a click selects an item, a drag moves it, kept
    /// on the paper; a press on bare paper lets go of the selection. Not while the map is adjusted or
    /// coordinate points are placed (one tool of the sheet at a time).
    pub(crate) fn move_sheet_items(&mut self, ui: &egui::Ui, paper: Rect, px_per_mm: f32) {
        let painted = std::mem::take(&mut self.drawing.item_rects);
        if self.drawing.map_adjust_active || self.drawing.coord_callout_mode {
            return;
        }
        let pressed: Option<Pos2> = ui.input(|i| i.pointer.press_origin().filter(|_| i.pointer.primary_pressed()));
        if pressed.is_some_and(|at| {
            paper.contains(at)
                && !painted.iter().any(|(item, placed, moved)| {
                    let rect = placed.translate(*moved);
                    if self.drawing.selected_item == Some(*item) {
                        rect.expand(9.0).contains(at)
                    } else {
                        rect.contains(at)
                    }
                })
        }) {
            self.drawing.selected_item = None;
        }
        for (item, placed, moved) in painted {
            let now = placed.translate(moved);
            if item == SheetItem::Legend && self.drawing.selected_item == Some(item) {
                self.resize_legend_handles(ui, now, moved, px_per_mm);
            }
            // Drags only: a drag starts at the press, so a click selects and a move is followed at once.
            let response = ui.interact(now, ui.id().with(("sheet_item", item)), egui::Sense::drag());
            if response.drag_started() {
                self.drawing.selected_item = Some(item);
            }
            if self.drawing.selected_item == Some(item) {
                paint_selection(ui.painter(), now);
            }
            if response.hovered() || response.dragged() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Move);
                ui.painter().rect_stroke(now.expand(2.0), 2.0, Stroke::new(1.0, Color32::from_rgb(181, 87, 58)), egui::StrokeKind::Middle);
            }
            if response.dragged_by(egui::PointerButton::Primary) {
                let wanted = moved + response.drag_delta();
                let kept = if item == SheetItem::Legend { wanted } else { kept_on_paper(wanted, placed, paper) } / px_per_mm;
                self.drawing.item_offsets_mm.insert(item, [kept.x, kept.y]);
            }
        }
    }

    // Four independent corner drags, as the C++ select tool (target/cpp-ref/src/app/KaDrawingStudio.cpp:3370-3407).
    fn resize_legend_handles(&mut self, ui: &egui::Ui, now: Rect, moved: Vec2, px_per_mm: f32) {
        for (index, corner) in [now.left_top(), now.right_top(), now.left_bottom(), now.right_bottom()].into_iter().enumerate() {
            let sign = Vec2::new(if index % 2 == 0 { -1.0 } else { 1.0 }, if index < 2 { -1.0 } else { 1.0 });
            let response = ui.interact(Rect::from_center_size(corner + sign * 3.0, Vec2::splat(12.0)), ui.id().with(("legend_resize", index)), egui::Sense::drag());
            if response.hovered() || response.dragged() {
                ui.ctx().set_cursor_icon(if index == 0 || index == 3 { egui::CursorIcon::ResizeNwSe } else { egui::CursorIcon::ResizeNeSw });
            }
            if response.dragged_by(egui::PointerButton::Primary) {
                let delta = response.drag_delta();
                let minimum = Vec2::new(16.0, 12.0) * px_per_mm;
                let size = (now.size() + delta * sign).max(minimum);
                let change = size - now.size();
                let offset = moved + Vec2::new(if index % 2 == 0 { -change.x } else { 0.0 }, if index < 2 { -change.y } else { 0.0 });
                self.drawing.legend_size_mm = Some([size.x / px_per_mm, size.y / px_per_mm]);
                self.drawing.item_offsets_mm.insert(SheetItem::Legend, [offset.x / px_per_mm, offset.y / px_per_mm]);
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
