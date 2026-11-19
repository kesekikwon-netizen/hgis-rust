//! Window chords that C++ puts on `QAction` shortcuts.
//!
//! Ctrl+F: `MainWindowRibbon.cpp:240-244` (`QKeySequence::Find`) → `KaAppBar::focusSearch`
//! (`KaAppBar.cpp:84-87`).
//! F11: `MainWindow.cpp:270-278` (`Qt::Key_F11`, full screen / maximized).
//! Ctrl+F11: `KaShellFocus.h:27`, `MainWindowRibbon.cpp:655-663`, messages `KaShellFocus.cpp:133-143`.
//! F2: `MainWindow.cpp:2379-2381` when the layer list has the key, then `renameSelectedLayer`
//! (`MainWindow.cpp:1634-1650`). Same list-versus-map split as Delete (`app_state_layer_remove.rs`).

use eframe::egui::{self, Key};

use crate::app_state::StrataAppState;

impl StrataAppState {
    /// `Some(on)` is the full-screen state F11 should apply. The caller sends the viewport command
    /// outside `ctx.input`, which holds the context.
    pub(super) fn window_chords(&mut self, i: &egui::InputState, wants_kb: bool) -> Option<bool> {
        if i.modifiers.ctrl
            && !i.modifiers.shift
            && !i.modifiers.alt
            && i.key_pressed(Key::F)
            && !self.right_inspector.map_focused
        {
            // The search row is not on screen while map focus hides the ribbon (KaShellFocus.cpp:123-130).
            self.ribbon.app_bar.focus_search = true;
        }
        if !wants_kb && self.home_map.legend_focus && i.key_pressed(Key::F2) {
            self.rename_legend_row_from_f2();
        }
        let ctrl_f11 = i.modifiers.ctrl && !i.modifiers.shift && !i.modifiers.alt;
        if i.key_pressed(Key::F11) && ctrl_f11 {
            // Same toggle the 더보기 row calls (shell_focus.rs, KaShellFocus.cpp:146).
            self.toggle_map_focus();
            return None;
        }
        if i.key_pressed(Key::F11) && i.modifiers.is_none() {
            return Some(!i.viewport().fullscreen.unwrap_or(false));
        }
        None
    }

    /// `renameSelectedLayer` with no argument: the current legend row, or the information text.
    fn rename_legend_row_from_f2(&mut self) {
        let label = self.right_inspector.selected_layer_key.clone();
        let key = label.as_deref().and_then(|label| {
            self.left_layers
                .entries
                .iter()
                .find(|entry| entry.label_ko == label)
                .map(|entry| entry.layer_key.clone())
        });
        if let Some(key) = key {
            self.open_rename_layer(&key);
        } else {
            // MainWindow.cpp:1648-1649. QMessageBox until that box exists: the status line, as the other
            // rename warnings do (app_state_layer_menu.rs).
            self.show_status_message("이름을 바꿀 레이어 또는 그룹을 선택하세요.");
        }
    }
}
