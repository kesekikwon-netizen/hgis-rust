//! Terrain 3D tab (`target/cpp-ref/src/app/KaTerrain3dStudio.cpp:84-149`, `:464-469`, `:541-546`).

use eframe::egui::{self, Color32};

use crate::app_state::StrataAppState;

#[path = "gui_tab_terrain3d_view.rs"]
mod gui_tab_terrain3d_view;

/// Deferred-feature text until order W-56 ports DEM clip, mesh and drape.
const DEFERRED_W56: &str = "아직 Rust 판에서 연결하지 않았습니다 (지시 W-56)";

/// No-scene notice (`target/cpp-ref/src/app/KaTerrain3dStudio.cpp:466-468`, `:543-545`).
const NO_SCENE_MESSAGE: &str = "먼저 지도 화면을 입체로 만드세요.";

impl StrataAppState {
    /// Left `terrain3dPanel` (`target/cpp-ref/src/app/KaTerrain3dStudio.cpp:84-126`) and right
    /// `KaTerrain3dView` (`target/cpp-ref/src/app/KaTerrain3dStudio.cpp:127-136`) without a scene.
    pub(crate) fn render_terrain3d_tab_ui(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) {
        egui::SidePanel::left("ka_terrain3d_left_panel")
            .resizable(true)
            .default_width(230.0)
            .width_range(200.0..=260.0)
            .frame(
                egui::Frame::NONE
                    .fill(Color32::from_rgb(244, 246, 248))
                    .inner_margin(egui::Margin::same(10)),
            )
            .show_inside(ui, |ui| {
                // target/cpp-ref/src/app/KaTerrain3dStudio.cpp:91-100 — caption and hint.
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("입체지형")
                        .size(15.0)
                        .strong()
                        .color(Color32::from_rgb(28, 25, 23)),
                );
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(
                        "지금 지도에 보이는 화면을 고해상 입체로 만듭니다. 탭을 열면 자동으로 시작합니다.",
                    )
                    .size(12.0)
                    .color(Color32::from_rgb(94, 93, 89)),
                );
                ui.add_space(8.0);

                // target/cpp-ref/src/app/KaTerrain3dStudio.cpp:102-110 — action buttons.
                let btn_fill = egui::Button::new(
                    egui::RichText::new("화면을 입체로")
                        .color(Color32::WHITE)
                        .strong(),
                )
                .fill(Color32::from_rgb(181, 87, 58))
                .min_size(egui::vec2(ui.available_width(), 30.0));
                ui.add_enabled(false, btn_fill)
                    .on_disabled_hover_text(DEFERRED_W56);

                if ui
                    .add_sized([ui.available_width(), 28.0], egui::Button::new("이미지 저장"))
                    .clicked()
                {
                    // target/cpp-ref/src/app/KaTerrain3dStudio.cpp:464-469 — saveImage without a scene.
                    self.show_status_message(NO_SCENE_MESSAGE);
                }
                if ui
                    .add_sized(
                        [ui.available_width(), 28.0],
                        egui::Button::new("입체지형 도면출력"),
                    )
                    .clicked()
                {
                    // target/cpp-ref/src/app/KaTerrain3dStudio.cpp:541-546 — runExportSheet without a scene.
                    self.show_status_message(NO_SCENE_MESSAGE);
                }

                // target/cpp-ref/src/app/KaTerrain3dStudio.cpp:111-119 — height exaggeration spin box.
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new("높이 과장")
                        .size(12.5)
                        .strong()
                        .color(Color32::from_rgb(41, 37, 36)),
                );
                ui.add_sized(
                    [ui.available_width(), 24.0],
                    egui::DragValue::new(&mut self.paleo_3d.z_exaggeration)
                        .range(0.2..=12.0)
                        .speed(0.1)
                        .fixed_decimals(1)
                        .suffix(" 배"),
                );

                // target/cpp-ref/src/app/KaTerrain3dStudio.cpp:120-123 — status label (deferred until W-56).
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(DEFERRED_W56)
                        .size(12.0)
                        .color(Color32::from_rgb(94, 93, 89)),
                );
            });
        self.render_terrain3d_view(ui);
    }
}

#[cfg(test)]
#[path = "gui_tab_terrain3d_tests.rs"]
mod tests;
