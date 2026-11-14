//! Right 3D view of the Terrain 3D tab without a scene
//! (`target/cpp-ref/src/app/KaTerrain3dView.cpp:42-52`,
//! `target/cpp-ref/src/core/Terrain3dService.cpp:237-242`).

use eframe::egui::{self, Color32};

use crate::app_state::StrataAppState;

impl StrataAppState {
    /// Without a built mesh, `Terrain3dService::renderPerspective`
    /// (`target/cpp-ref/src/core/Terrain3dService.cpp:237-242`) fills the view with plain white.
    pub(crate) fn render_terrain3d_view(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(Color32::WHITE))
            .show_inside(ui, |ui| {
                let (rect, _response) =
                    ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());
                ui.painter_at(rect).rect_filled(rect, 0.0, Color32::WHITE);
            });
    }
}
