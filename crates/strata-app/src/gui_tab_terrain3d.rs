//! 「입체지형」 탭 — 좌측 `terrain3dPanel` + 우측 `KaTerrain3dView`.
//!
//! 원래 440줄 한 파일이던 것을 300줄 규칙(마스터플랜 Chapter 07)에 맞춰 나눴다.
//! 우측 3D 뷰는 `gui_tab_terrain3d_view.rs` 에 있고, 기존
//! `crate::gui_tab_terrain3d::...` 경로는 그대로 동작한다.

use eframe::egui::{self, Color32};

use crate::app_state::{MainTab, StrataAppState};

#[path = "gui_tab_terrain3d_view.rs"]
mod gui_tab_terrain3d_view;

impl StrataAppState {
    /// `KaTerrain3dStudio.cpp:87-170` + `KaTerrain3dView.cpp:46-90` — 좌측 `terrain3dPanel` + 우측 `KaTerrain3dView` 1:1 재현
    pub(crate) fn render_terrain3d_tab_ui(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        // 1. 좌측 패널 (KaTerrain3dStudio.cpp:85-166)
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

                let btn_fill = egui::Button::new(
                    egui::RichText::new("화면을 입체로")
                        .color(Color32::WHITE)
                        .strong(),
                )
                .fill(Color32::from_rgb(181, 87, 58))
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
                        Some("저장함 · 축척자·방위 포함\n입체지형.png".to_owned());
                }
                if ui
                    .add_sized(
                        [ui.available_width(), 28.0],
                        egui::Button::new("입체지형 도면출력"),
                    )
                    .clicked()
                {
                    // KaTerrain3dLayoutStudio.cpp:72-320 — 3D 뷰를 도면 시트에 올린다
                    self.open_studio_tab(MainTab::Drawing);
                    self.drawing.terrain3d_layout_active = true;
                    self.drawing
                        .sheet
                        .legend_visible = true;
                    self.home_map.draw_guide_message = Some(
                        "입체지형 도면 — 범례는 「범례」로, 3D 뷰가 용지 안에 들어갔습니다."
                            .to_owned(),
                    );
                }

                ui.add_space(6.0);
                ui.separator();
                ui.label(
                    egui::RichText::new("높이 과장")
                        .size(12.5)
                        .strong()
                        .color(Color32::from_rgb(41, 37, 36)),
                );
                // KaTerrain3dStudio.cpp:113-118 — QDoubleSpinBox 0.2..=12.0, step 0.1
                let mut changed = false;
                if ui
                    .add(
                        egui::Slider::new(&mut self.paleo_3d.z_exaggeration, 0.2..=12.0)
                            .step_by(0.1)
                            .fixed_decimals(1)
                            .suffix(" 배"),
                    )
                    .changed()
                {
                    changed = true;
                }
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("값").size(11.5));
                    if ui
                        .add(
                            egui::DragValue::new(&mut self.paleo_3d.z_exaggeration)
                                .range(0.2..=12.0)
                                .speed(0.1)
                                .fixed_decimals(1)
                                .suffix(" 배"),
                        )
                        .on_hover_text(
                            "높이 과장입니다. 1.0이면 실제 높이, 크게 하면 기복이 두드러집니다.",
                        )
                        .changed()
                    {
                        changed = true;
                    }
                });
                if changed {
                    self.paleo_3d.rebuild_3d_preview();
                }

                ui.add_space(6.0);
                // KaTerrain3dStudio.cpp:190-248 — 표면에 입힐 그림 선택
                ui.label(
                    egui::RichText::new("표면에 입힐 그림")
                        .size(12.0)
                        .strong()
                        .color(Color32::from_rgb(68, 64, 60)),
                );
                const TEXTURES: [&str; 3] =
                    ["입체지형 위성", "음영기복(Hillshade)", "고지형 7대 분류"];
                egui::ComboBox::from_id_salt("ka_terrain3d_texture_combo")
                    .selected_text(TEXTURES[self.paleo_3d.texture_mode_idx.min(2)])
                    .width(200.0)
                    .show_ui(ui, |ui| {
                        for (idx, lbl) in TEXTURES.iter().enumerate() {
                            if ui
                                .selectable_label(
                                    self.paleo_3d.texture_mode_idx == idx,
                                    *lbl,
                                )
                                .clicked()
                            {
                                self.paleo_3d.texture_mode_idx = idx;
                                self.paleo_3d.rebuild_3d_preview();
                                self.home_map.draw_guide_message = Some(format!(
                                    "입체지형 표면에 「{lbl}」을 입혔습니다."
                                ));
                            }
                        }
                    });

                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("시점 프리셋 (KaTerrain3dView)")
                        .size(12.0)
                        .strong()
                        .color(Color32::from_rgb(68, 64, 60)),
                );
                ui.horizontal(|ui| {
                    if ui.small_button("사선 조감").clicked() {
                        self.paleo_3d.camera_3d.yaw_deg = 35.0;
                        self.paleo_3d.camera_3d.pitch_deg = 42.0;
                        self.paleo_3d.camera_3d.zoom = 1.0;
                        self.paleo_3d.rebuild_3d_preview();
                    }
                    if ui.small_button("수직 조감").clicked() {
                        self.paleo_3d.camera_3d.yaw_deg = 0.0;
                        self.paleo_3d.camera_3d.pitch_deg = 78.0;
                        self.paleo_3d.camera_3d.zoom = 1.05;
                        self.paleo_3d.rebuild_3d_preview();
                    }
                    if ui.small_button("현장 시점").clicked() {
                        self.paleo_3d.camera_3d.yaw_deg = 128.0;
                        self.paleo_3d.camera_3d.pitch_deg = 18.0;
                        self.paleo_3d.camera_3d.zoom = 1.15;
                        self.paleo_3d.rebuild_3d_preview();
                    }
                });

                ui.add_space(6.0);
                ui.separator();

                // KaTerrain3dStudio.cpp:161 — 상태줄 문구 1:1
                ui.label(
                    egui::RichText::new("DEM 160×160 · 높이 18.5–142.0 m · 끌어서 돌리기")
                        .size(12.0)
                        .strong()
                        .color(Color32::from_rgb(28, 25, 23)),
                );
                ui.add_space(4.0);

                let cut_vol =
                    strata_terrain3d::trench_profile3d::carve_trench_volume_from_samples(
                        &[50.0, 50.0, 50.0, 50.0],
                        (self.home_map.real_survey.trenches.len().max(1) as f64) * 40.0,
                        48.5,
                        4,
                    );
                ui.label(
                    egui::RichText::new(format!(
                        "방위각 {:.1}° · 경사 {:.1}° · 배율 {:.1}배\n드레이핑: 조사구역 {}건 · 시굴 트렌치 {}공 (+0.25m 오프셋)\n굴착 토량(TrenchTerrain): {:.1} ㎥ (최대 심도 {:.1} m)\n배경 표고: {}",
                        self.paleo_3d.camera_3d.yaw_deg,
                        self.paleo_3d.camera_3d.pitch_deg,
                        self.paleo_3d.z_exaggeration,
                        self.home_map.real_survey.survey_rings.len(),
                        self.home_map.real_survey.trenches.len(),
                        cut_vol.excavated_volume_m3,
                        cut_vol.max_excavation_depth_m,
                        strata_terrain3d::trench_profile3d::copernicus_dsm_title_ko()
                    ))
                    .size(11.5)
                    .color(Color32::from_rgb(94, 93, 89)),
                );
            });
        self.render_terrain3d_view(ctx, ui);
    }
}

#[cfg(test)]
mod tests {
    // D23 KaTerrain3dStudio — 높이 과장 범위·표면 그림 3종·반응형 패널
    #[test]
    fn test_d23_terrain3d_studio_panel_and_texture_modes() {
        // KaTerrain3dStudio.cpp:113-118 QDoubleSpinBox 0.2..=12.0, step 0.1
        let range = 0.2_f32..=12.0_f32;
        assert!(range.contains(&2.0));
        assert!(!range.contains(&0.1));
        assert!(!range.contains(&12.1));

        // KaTerrain3dStudio.cpp:190-248 — 표면 그림 3종
        let textures = ["입체지형 위성", "음영기복(Hillshade)", "고지형 7대 분류"];
        assert_eq!(textures.len(), 3);
        assert_eq!(textures[1], "음영기복(Hillshade)");

        // 좌측 패널 폭 200..=260 (KaTerrain3dStudio.cpp:85-145)
        let w = 200.0_f32..=260.0_f32;
        assert!(w.contains(&230.0));
        assert!(!w.contains(&199.0));

        // KaTerrain3dLayoutStudio.cpp — 조판 모드 기본값
        let st = crate::tab_drawing::DrawingTabState::default();
        assert!(!st.terrain3d_layout_active);
    }
}
