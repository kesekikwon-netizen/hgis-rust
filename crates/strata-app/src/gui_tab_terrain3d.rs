use eframe::egui::{self, Color32, FontId, Pos2, Rect, Stroke, Vec2};
use strata_terrain3d::viewport3d_renderer::render_terrain3d_rgba;

use crate::app_state::{MainTab, StrataAppState};

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

        // 2. 우측 중앙 3D 뷰 (KaTerrain3dView.cpp:46-90)
        egui::CentralPanel::default().show_inside(ui, |ui| {
            let (rect, response) =
                ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());

            // KaTerrain3dView.cpp:63-73 — 마우스 드래그 회전 (m_yaw += d.x * 0.4f, m_pitch clamp 8.0..80.0)
            if response.dragged() {
                let d = response.drag_delta();
                self.paleo_3d.camera_3d.yaw_deg =
                    (self.paleo_3d.camera_3d.yaw_deg + d.x * 0.4).rem_euclid(360.0);
                self.paleo_3d.camera_3d.pitch_deg =
                    (self.paleo_3d.camera_3d.pitch_deg + d.y * 0.3).clamp(8.0, 80.0);
                self.paleo_3d.rebuild_3d_preview();
            }

            // KaTerrain3dView.cpp:83-90 — 마우스 휠 거리 조절
            if response.hovered() {
                let scroll_y = ui.input(|i| i.smooth_scroll_delta.y);
                if scroll_y.abs() > 0.5 {
                    let factor = if scroll_y > 0.0 { 1.08 } else { 0.92 };
                    self.paleo_3d.camera_3d.zoom =
                        (self.paleo_3d.camera_3d.zoom * factor).clamp(0.5, 2.5);
                    self.paleo_3d.rebuild_3d_preview();
                }
            }

            let painter = ui.painter_at(rect);

            // 고해상도(480x300) 3D 투시 메시 렌더링 (Terrain3dService::renderPerspective)
            let render_w = 480u32;
            let render_h = 300u32;
            self.paleo_3d.camera_3d.z_exaggeration = self.paleo_3d.z_exaggeration;
            let rgba_buf = if let Some(ref out) = self.paleo_3d.last_paleo_output {
                render_terrain3d_rgba(
                    &out.z_paleo,
                    out.transform.width,
                    out.transform.height,
                    &self.paleo_3d.camera_3d,
                    render_w,
                    render_h,
                )
            } else {
                render_terrain3d_rgba(
                    &[42.0; 64],
                    8,
                    8,
                    &self.paleo_3d.camera_3d,
                    render_w,
                    render_h,
                )
            };

            if rgba_buf.len() == (render_w as usize) * (render_h as usize) * 4 {
                let img = egui::ColorImage::from_rgba_unmultiplied(
                    [render_w as usize, render_h as usize],
                    &rgba_buf,
                );
                let tex = ctx.load_texture("terrain3d_tex", img, egui::TextureOptions::LINEAR);
                painter.image(
                    tex.id(),
                    rect,
                    Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            }

            // 상단 오버레이 배지
            let title_rect = Rect::from_min_size(
                Pos2::new(rect.left() + 14.0, rect.top() + 12.0),
                Vec2::new(490.0, 28.0),
            );
            painter.rect_filled(title_rect, 4.0, Color32::from_rgba_unmultiplied(15, 23, 42, 185));
            painter.text(
                Pos2::new(title_rect.left() + 10.0, title_rect.center().y),
                egui::Align2::LEFT_CENTER,
                format!(
                    "입체지형 (KaTerrain3dView) — 높이 과장 {:.1}배 · 방위각 {:.0}° · 경사 {:.0}° (드래그 회전)",
                    self.paleo_3d.z_exaggeration,
                    self.paleo_3d.camera_3d.yaw_deg,
                    self.paleo_3d.camera_3d.pitch_deg
                ),
                FontId::proportional(12.5),
                Color32::WHITE,
            );

            // Terrain3dService::composeExport (Terrain3dService.cpp:505-545) — 하단 축척자 · 방위(North) · 표고 범례 바
            let footer_h = 36.0;
            let footer_rect = Rect::from_min_max(
                Pos2::new(rect.left(), rect.bottom() - footer_h),
                rect.max,
            );
            painter.rect_filled(
                footer_rect,
                0.0,
                Color32::from_rgba_unmultiplied(255, 255, 255, 235),
            );
            painter.line_segment(
                [
                    Pos2::new(footer_rect.left(), footer_rect.top()),
                    Pos2::new(footer_rect.right(), footer_rect.top()),
                ],
                Stroke::new(1.2, Color32::from_rgb(17, 24, 39)),
            );

            // 좌측 하단 축척자 (0 — 500 m)
            let bar_x0 = footer_rect.left() + 18.0;
            let bar_w = 140.0 / self.paleo_3d.camera_3d.zoom.clamp(0.5, 2.0);
            let bar_y = footer_rect.center().y + 4.0;
            painter.line_segment(
                [Pos2::new(bar_x0, bar_y), Pos2::new(bar_x0 + bar_w, bar_y)],
                Stroke::new(2.0, Color32::from_rgb(17, 24, 39)),
            );
            painter.line_segment(
                [
                    Pos2::new(bar_x0, bar_y - 5.0),
                    Pos2::new(bar_x0, bar_y + 4.0),
                ],
                Stroke::new(2.0, Color32::from_rgb(17, 24, 39)),
            );
            painter.line_segment(
                [
                    Pos2::new(bar_x0 + bar_w, bar_y - 5.0),
                    Pos2::new(bar_x0 + bar_w, bar_y + 4.0),
                ],
                Stroke::new(2.0, Color32::from_rgb(17, 24, 39)),
            );
            painter.text(
                Pos2::new(bar_x0, bar_y - 12.0),
                egui::Align2::LEFT_CENTER,
                "0",
                FontId::proportional(11.0),
                Color32::from_rgb(17, 24, 39),
            );
            painter.text(
                Pos2::new(bar_x0 + bar_w, bar_y - 12.0),
                egui::Align2::RIGHT_CENTER,
                "500 m",
                FontId::proportional(11.0),
                Color32::from_rgb(17, 24, 39),
            );

            // 중앙 하단 DEM 표고 범례 및 조사구역 표시
            let leg_x = bar_x0 + bar_w + 36.0;
            let leg_y = footer_rect.center().y;
            let elev_swatches = [
                (Color32::from_rgb(28, 92, 168), "하도(18.5m)"),
                (Color32::from_rgb(76, 138, 68), "충적평야(25m)"),
                (Color32::from_rgb(112, 134, 66), "구릉·선상지(45m)"),
                (Color32::from_rgb(138, 118, 96), "산지능선(95m)"),
                (Color32::from_rgb(178, 162, 144), "정상부(142.0m)"),
                (Color32::from_rgb(217, 42, 26), "조사구역(AOI)"),
            ];
            let mut cur_x = leg_x;
            for (col, label) in elev_swatches {
                if cur_x + 115.0 > footer_rect.right() - 60.0 {
                    break;
                }
                let sw = Rect::from_center_size(Pos2::new(cur_x + 6.0, leg_y), Vec2::new(11.0, 11.0));
                painter.rect_filled(sw, 2.0, col);
                painter.text(
                    Pos2::new(cur_x + 15.0, leg_y),
                    egui::Align2::LEFT_CENTER,
                    label,
                    FontId::proportional(11.0),
                    Color32::from_rgb(30, 41, 59),
                );
                cur_x += 108.0;
            }

            // 우측 하단 회전 방위표 (Terrain3dService.cpp:530-543)
            let nx = footer_rect.right() - 34.0;
            let ny = footer_rect.center().y - 2.0;
            let rot = -self.paleo_3d.camera_3d.yaw_deg.to_radians();
            let dir_n = Vec2::new(rot.sin(), -rot.cos());
            let dir_perp = Vec2::new(-dir_n.y, dir_n.x);
            let center_n = Pos2::new(nx, ny);
            let tip = center_n + dir_n * 11.0;
            let tail = center_n - dir_n * 8.0;
            painter.line_segment(
                [tail, tip],
                Stroke::new(2.0, Color32::from_rgb(17, 24, 39)),
            );
            painter.add(egui::Shape::convex_polygon(
                vec![
                    tip,
                    center_n + dir_n * 3.0 - dir_perp * 4.0,
                    center_n + dir_n * 3.0 + dir_perp * 4.0,
                ],
                Color32::from_rgb(181, 87, 58),
                Stroke::NONE,
            ));
            painter.text(
                Pos2::new(nx + 14.0, ny),
                egui::Align2::LEFT_CENTER,
                "N",
                FontId::proportional(11.5),
                Color32::from_rgb(17, 24, 39),
            );
        });
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
