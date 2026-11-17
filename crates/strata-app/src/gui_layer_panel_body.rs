//! Shared layer contents, target/cpp-ref/src/app/MainWindow.cpp:798-811 and KaFileBrowserPanel.cpp:87-151.
use super::gui_layer_panel_consts::{UX_FILE_DRAWER_HEIGHT, UX_LAYER_CARD_HEIGHT};
use crate::app_state::StrataAppState;
use eframe::egui::{self, Color32, Pos2, Rect, Vec2};
impl StrataAppState {
    pub(crate) fn paint_compact_layer_menu(&mut self, ui: &mut egui::Ui, show_set_btn: bool) {
        ui.menu_button("레이어", |ui| {
            ui.set_width(340.0);
            let height = (ui.ctx().screen_rect().height() - 160.0).clamp(100.0, 440.0);
            egui::ScrollArea::vertical().id_salt("compact_layer_pane").drag_to_scroll(false).max_height(height).auto_shrink([false, false]).show(ui, |ui| {
                ui.set_height(UX_LAYER_CARD_HEIGHT + UX_FILE_DRAWER_HEIGHT + 8.0);
                self.paint_layer_pane_body(ui, show_set_btn);
            });
        });
    }
    pub(super) fn paint_layer_pane_body(&mut self, ui: &mut egui::Ui, show_set_btn: bool) {
        ui.spacing_mut().item_spacing.y = 1.0;
        let panel_w = ui.available_width();
        let avail_total_h = ui.available_height();
        let layer_card_h = UX_LAYER_CARD_HEIGHT.min((avail_total_h - UX_FILE_DRAWER_HEIGHT - 5.0).max(420.0));

        // 1. 왼쪽 상단 레이어 카드 (w=351, h=507)
        let (layer_card_rect, _) = ui.allocate_exact_size(Vec2::new(panel_w, layer_card_h), egui::Sense::hover());
        let mut l_ui = ui.new_child(egui::UiBuilder::new().max_rect(layer_card_rect));
        l_ui.vertical(|ui| {
            ui.add_space(2.0);
            self.paint_layer_card_header(ui, show_set_btn);
            if self.left_layers.entries.is_empty() {
                // MainWindow.cpp:798-811, MainWindowSession.cpp:752-760 refreshLayerEmptyState —
                // 레이어가 없으면 목록 대신 가운데 맞춘 안내. 「표시 설정」은 찾기 칸 바로 아래.
                self.paint_layer_display_settings(ui);
                // Qt AlignCenter — 줄마다 가운데.
                let rest = Rect::from_min_max(ui.cursor().min, layer_card_rect.max);
                let lines: Vec<&str> = super::gui_layer_panel_row_spec::layer_panel_empty_hint().lines().collect();
                let line_h = 17.0;
                let top = rest.center().y - line_h * (lines.len() as f32) * 0.5;
                for (i, line) in lines.iter().enumerate() {
                    ui.painter().text(
                        Pos2::new(rest.center().x, top + line_h * (i as f32 + 0.5)),
                        egui::Align2::CENTER_CENTER,
                        *line,
                        egui::FontId::proportional(12.0),
                        Color32::from_rgb(75, 74, 70),
                    );
                }
            } else {
                self.paint_layer_tree(ui, layer_card_rect);
                self.paint_layer_display_settings(ui);
            }
        });

        let (div_r, _) = ui.allocate_exact_size(Vec2::new(panel_w, 3.0), egui::Sense::hover());
        ui.painter().rect_filled(div_r, 0.0, Color32::from_rgb(135, 134, 127));

        // 2. 왼쪽 하단 파일함 카드 (w=351, h=168)
        self.paint_file_drawer(ui, panel_w);
    }
}
