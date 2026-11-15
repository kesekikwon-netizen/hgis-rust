//! Dragging a row of the legend up or down — the drag and drop of the C++ layer list (`MainWindow.cpp:
//! 658-665`: QTreeView, MoveAction, drop indicator) and its undo step (`KaLayerTreeUndo`, `MainWindowUndo.cpp:
//! 391-393`). A row stays in its own part of the legend; the map and a sheet draw the rows in their order.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke};
use strata_types::schema::LegendGroup;

use super::UndoStep;
use crate::app_state::StrataAppState;

/// A row of the legend held with the mouse, and where it would go if let go now: above the named row, or
/// below it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LegendDrag {
    pub(crate) key: String,
    pub(crate) onto: Option<(String, bool)>,
}

impl StrataAppState {
    /// Puts the row `key` above the row `onto` of the same group, or below it. `false` when that changes
    /// nothing. One undo step.
    pub(crate) fn move_layer_row(&mut self, key: &str, onto: &str, below: bool) -> bool {
        let entries = &mut self.left_layers.entries;
        let (Some(from), Some(target)) = (entries.iter().position(|e| e.layer_key == key), entries.iter().position(|e| e.layer_key == onto)) else {
            return false;
        };
        if from == target || entries[from].group != entries[target].group {
            return false;
        }
        let before: Vec<String> = entries.iter().map(|e| e.layer_key.clone()).collect();
        let row = entries.remove(from);
        // The place of `onto` after the row was taken out, and one further for below it.
        let at = entries.iter().position(|e| e.layer_key == onto).map_or(entries.len(), |i| i + usize::from(below));
        entries.insert(at, row);
        if entries.iter().map(|e| &e.layer_key).eq(before.iter()) {
            return false;
        }
        self.redo_tables.clear();
        self.undo_tables.push(UndoStep::RowsMoved(before));
        self.is_dirty = true;
        true
    }

    /// Ctrl+Z of a moved row: the rows that are still there stand in the order they had (`restoreInto`:
    /// rows added since stay where they are).
    pub(crate) fn restore_layer_order(&mut self, before: &[String]) {
        let place = |key: &str| before.iter().position(|k| k == key);
        let entries = &mut self.left_layers.entries;
        let slots: Vec<usize> = (0..entries.len()).filter(|i| place(&entries[*i].layer_key).is_some()).collect();
        let mut known: Vec<_> = slots.iter().map(|i| entries[*i].clone()).collect();
        known.sort_by_key(|e| place(&e.layer_key));
        for (slot, row) in slots.into_iter().zip(known) {
            entries[slot] = row;
        }
        self.is_dirty = true;
    }

    /// The row that is held with the mouse, over the rows as they were painted in this frame: the row under
    /// the pointer shows by a line where the held row will go, and letting go puts it there.
    pub(crate) fn drag_layer_rows(&mut self, ui: &egui::Ui, rows: &[(String, LegendGroup, Rect)]) {
        let Some(mut drag) = self.home_map.legend_drag.take() else { return };
        let group = rows.iter().find(|(key, ..)| *key == drag.key).map(|(_, group, _)| *group);
        let pointer = ui.ctx().pointer_latest_pos();
        let under = pointer.and_then(|at| rows.iter().find(|(_, _, row)| row.contains(at)).map(|row| (row, at)));
        drag.onto = under.filter(|((key, of, _), _)| *key != drag.key && Some(*of) == group).map(|((key, _, row), at)| (key.clone(), at.y > row.center().y));
        if let Some(((_, _, row), _)) = under.filter(|_| drag.onto.is_some()) {
            let y = if drag.onto.as_ref().is_some_and(|(_, below)| *below) { row.bottom() } else { row.top() };
            ui.painter().line_segment([Pos2::new(row.left(), y), Pos2::new(row.right(), y)], Stroke::new(2.0, Color32::from_rgb(181, 87, 58)));
        }
        if ui.input(|i| i.pointer.primary_down()) {
            self.home_map.legend_drag = Some(drag);
        } else if let Some((onto, below)) = drag.onto {
            self.move_layer_row(&drag.key, &onto, below);
        }
    }
}

#[cfg(test)]
#[path = "app_state_layer_order_tests.rs"]
mod tests;
