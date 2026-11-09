//! `render_ri_body` 의 하단(비고·출처 안내) 구간.
//!
//! `right_inspector.rs` 가 300줄을 넘어 옮겼다. 실행 순서는 호출부가 보장한다.
use eframe::egui::{self, Color32, Pos2, Stroke, Vec2};
use strata_net::tile_fetcher::BasemapSource;
use crate::app_state::StrataAppState;

impl StrataAppState {
    pub(crate) fn render_ri_body_bottom(&mut self, ui: &mut egui::Ui) {
    ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            for (lbl, src) in [
                ("위성", BasemapSource::VworldSatellite),
                ("지형", BasemapSource::VworldBase),
                ("옛 지도 ▼", BasemapSource::Daedongyeojido),
            ] {
                let btn =
                    egui::Button::new(egui::RichText::new(lbl).size(12.0).strong())
                        .fill(Color32::WHITE)
                        .stroke(Stroke::new(1.0, Color32::from_rgb(185, 182, 172)))
                        .corner_radius(5.0)
                        .min_size(Vec2::new(66.0, 30.0));
                let resp = ui.add(btn);
                if resp.clicked() {
                    if lbl.starts_with("옛 지도") {
                        self.ribbon.old_map_menu_open = !self.ribbon.old_map_menu_open;
                        self.ribbon.context_menu_anchor =
                            Pos2::new((resp.rect.right() - 204.0).max(16.0), resp.rect.top() - 62.0);
                    } else {
                        self.home_map.active_basemap = src;
                    }
                }
            }
        });
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new("배경 지도")
                .size(11.5)
                .color(Color32::from_rgb(95, 94, 88)),
        );
        ui.separator();
    });
    }
}
