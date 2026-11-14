//! Tab body dispatch for the right inspector (`속성` vs `스타일`).
//! Wraps the `스타일` tab in a vertical scroll area (`target/cpp-ref/src/app/KaInspectorPanel.cpp:96-104`).
use eframe::egui;
use crate::app_state::StrataAppState;

impl StrataAppState {
    pub(crate) fn render_ri_body_cards(&mut self, ui: &mut egui::Ui, is_attr: bool) {
        if is_attr {
            self.render_ri_attribute_card(ui);
        } else {
            let max_h = (ui.available_height() - 72.0).max(60.0);
            egui::ScrollArea::vertical()
                .id_salt("inspector_style_scroll")
                .auto_shrink([false, true])
                .max_height(max_h)
                .show(ui, |ui| self.render_ri_layer_card(ui));
        }
    }
}
