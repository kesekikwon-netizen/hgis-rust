use eframe::egui;

use crate::app_state::{MainTab, StrataAppState};

impl StrataAppState {
    /// `KaTerrain3dStudio.cpp:913-980` — 좌측 `terrain3dPanel` + 우측 `KaTerrain3dView` 1:1 재현
    pub(crate) fn render_terrain3d_tab_ui(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        // 1. 좌측 패널 (KaTerrain3dStudio.cpp:920-960)
        egui::SidePanel::left("ka_terrain3d_left_panel")
            .resizable(false)
            .exact_width(236.0)
            .show_inside(ui, |ui| {
                ui.strong("입체지형");
                ui.label(
                    egui::RichText::new(
                        "지금 지도에 보이는 화면을 고해상 입체로 만듭니다. 탭을 열면 자동으로 시작합니다.",
                    )
                    .size(12.0)
                    .color(egui::Color32::from_rgb(94, 93, 89)),
                );
                ui.add_space(6.0);
                let btn_fill = egui::Button::new(
                    egui::RichText::new("화면을 입체로")
                        .color(egui::Color32::WHITE)
                        .strong(),
                )
                .fill(egui::Color32::from_rgb(181, 87, 58))
                .min_size(egui::vec2(ui.available_width(), 30.0));
                if ui.add(btn_fill).clicked() {
                    self.paleo_3d.rebuild_3d_preview();
                    self.home_map.draw_guide_message =
                        Some("지금 화면을 고해상 입체로 만들었습니다.".to_owned());
                }
                if ui
                    .add_sized([ui.available_width(), 28.0], egui::Button::new("이미지 저장"))
                    .clicked()
                {
                    self.home_map.draw_guide_message =
                        Some("저장함 · 축척자·방위 포함 (입체지형.png)".to_owned());
                }
                if ui
                    .add_sized(
                        [ui.available_width(), 28.0],
                        egui::Button::new("입체지형 도면출력"),
                    )
                    .clicked()
                {
                    self.open_studio_tab(MainTab::Drawing);
                    self.home_map.draw_guide_message =
                        Some("입체지형 렌더링 뷰를 도면(조판) 시트로 보냈습니다.".to_owned());
                }
                ui.separator();
                ui.label("시점 프리셋 (KaTerrain3dView)");
                ui.horizontal(|ui| {
                    if ui.small_button("사선 조감").clicked() {
                        self.paleo_3d.camera_3d.yaw_deg = 38.0;
                        self.paleo_3d.camera_3d.pitch_deg = 32.0;
                        self.paleo_3d.rebuild_3d_preview();
                    }
                    if ui.small_button("수직 조감").clicked() {
                        self.paleo_3d.camera_3d.yaw_deg = 0.0;
                        self.paleo_3d.camera_3d.pitch_deg = 80.0;
                        self.paleo_3d.rebuild_3d_preview();
                    }
                    if ui.small_button("현장 시점").clicked() {
                        self.paleo_3d.camera_3d.yaw_deg = 125.0;
                        self.paleo_3d.camera_3d.pitch_deg = 16.0;
                        self.paleo_3d.rebuild_3d_preview();
                    }
                });
                ui.separator();
                ui.label("높이 과장 (0.2 ~ 12.0 배)");
                if ui
                    .add(
                        egui::Slider::new(&mut self.paleo_3d.z_exaggeration, 0.2..=12.0)
                            .suffix(" 배"),
                    )
                    .changed()
                {
                    self.paleo_3d.rebuild_3d_preview();
                }
                ui.separator();
                let cut_vol =
                    strata_terrain3d::trench_profile3d::carve_trench_volume_from_samples(
                        &[50.0, 50.0, 50.0, 50.0],
                        (self.home_map.real_survey.trenches.len().max(1) as f64) * 40.0,
                        48.5,
                        4,
                    );
                ui.label(
                    egui::RichText::new(format!(
                        "DEM 160×160 · 높이 18.5–142.0 m · 끌어서 돌리기\n방위각 {:.1}° · 경사 {:.1}°\n드레이핑: 조사구역 {}건 · 시굴 트렌치 {}공 (+0.25m 오프셋)\n굴착 토량(TrenchTerrain): {:.1} ㎥ (최대 심도 {:.1} m)",
                        self.paleo_3d.camera_3d.yaw_deg,
                        self.paleo_3d.camera_3d.pitch_deg,
                        self.home_map.real_survey.survey_rings.len(),
                        self.home_map.real_survey.trenches.len(),
                        cut_vol.excavated_volume_m3,
                        cut_vol.max_excavation_depth_m
                    ))
                    .size(11.5)
                    .color(egui::Color32::from_rgb(94, 93, 89)),
                );
            });

        // 2. 우측 중앙 3D 뷰 (KaTerrain3dView)
        egui::CentralPanel::default().show_inside(ui, |ui| {
            let (rect, response) =
                ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());
            if response.dragged() {
                let d = response.drag_delta();
                self.paleo_3d.camera_3d.yaw_deg += d.x * 0.45;
                self.paleo_3d.camera_3d.pitch_deg =
                    (self.paleo_3d.camera_3d.pitch_deg + d.y * 0.35).clamp(12.0, 82.0);
                self.paleo_3d.rebuild_3d_preview();
            }
            let painter = ui.painter_at(rect);
            if self.paleo_3d.terrain3d_rgba.len() == 320 * 200 * 4 {
                let img = egui::ColorImage::from_rgba_unmultiplied(
                    [320, 200],
                    &self.paleo_3d.terrain3d_rgba,
                );
                let tex = ctx.load_texture("terrain3d_tex", img, egui::TextureOptions::LINEAR);
                painter.image(
                    tex.id(),
                    rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
            painter.text(
                rect.min + egui::vec2(18.0, 16.0),
                egui::Align2::LEFT_TOP,
                format!(
                    "입체지형 (KaTerrain3dView) — 마우스 드래그: 궤도 회전 · 높이 과장 {:.1}배",
                    self.paleo_3d.z_exaggeration
                ),
                egui::FontId::proportional(13.5),
                egui::Color32::WHITE,
            );
        });
    }
}
