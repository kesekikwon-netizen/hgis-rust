//! Responsive explorer home and existing survey-open routes.
//! Owner home redesign; navigation follows target/cpp-ref/src/app/KaFileBrowserPanel.cpp:87-357.

use eframe::egui;

use crate::app_state::StrataAppState;

#[path = "home_explorer.rs"]
pub(crate) mod explorer;

#[path = "home_explorer_ui.rs"]
mod explorer_ui;
#[path = "home_explorer_rows.rs"]
mod explorer_rows;
#[path = "home_explorer_connection.rs"]
mod explorer_connection;
#[path = "gui_tab_home_open.rs"]
mod gui_tab_home_open;
#[path = "gui_tab_home_survey.rs"]
mod gui_tab_home_survey;

pub(crate) use gui_tab_home_open::saved_gpkg_layer_title;

impl StrataAppState {
    pub(crate) fn render_home_tab_ui(&mut self, ui: &mut egui::Ui) {
        self.render_home_explorer(ui);
    }
}

#[cfg(test)]
#[path = "gui_tab_home_tests.rs"]
mod tests_new_survey;
