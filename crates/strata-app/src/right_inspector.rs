use eframe::egui::{self, Color32, Pos2, Stroke, Vec2};
use strata_net::tile_fetcher::BasemapSource;

use crate::app_state::StrataAppState;

/// C++ `KaStylePalette.h` 8대 표준 제도 색상 (빨, 주, 노, 초, 파, 남, 보, 갈)
pub const KA_STYLE_PALETTE: [(u8, u8, u8, &str); 8] = [
    (0xDC, 0x26, 0x26, "빨강"),
    (0xEA, 0x58, 0x0C, "주황"),
    (0xCA, 0x8A, 0x04, "노랑"),
    (0x16, 0xA3, 0x4A, "초록"),
    (0x25, 0x63, 0xEB, "파랑"),
    (0x1E, 0x3A, 0x8A, "남색"),
    (0x7C, 0x3A, 0xED, "보라"),
    (0x92, 0x40, 0x0E, "갈색"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InspectorCardTab {
    #[default]
    FeatureAttributeCard,
    LayerStyleCard,
    BasemapQuickCard,
}

#[derive(Debug, Clone, Default)]
pub struct RightInspectorState {
    pub active_card: InspectorCardTab,
    pub selected_layer_key: Option<String>,
    pub selected_feature_id: Option<i64>,
    pub survey_name: String,
    pub feature_no: String,
    pub feature_type: String,
    pub feature_period: String,
    pub feature_memo: String,
    pub survey_card_modal_open: bool,
    pub style_color_index: usize,
    pub no_fill: bool,
    pub no_stroke: bool,
    pub stroke_width_mm: f32,
    pub fill_opacity: f32,
    pub show_draw_tooltip: bool,
    pub left_panel_collapsed: bool,
    pub right_panel_collapsed: bool,
    pub map_focused: bool,
}

impl RightInspectorState {
    #[must_use]
    pub fn new_default() -> Self {
        Self {
            active_card: InspectorCardTab::FeatureAttributeCard,
            selected_layer_key: Some("조사구역".to_owned()),
            selected_feature_id: Some(1),
            survey_name: "경주 월성북편 시굴조사".to_owned(),
            feature_no: "1호 주거지".to_owned(),
            feature_type: "주거지".to_owned(),
            feature_period: "삼국".to_owned(),
            feature_memo: "경주 인왕동 76번지 일원 지표·시굴조사 구역 및 유구 기록".to_owned(),
            survey_card_modal_open: false,
            style_color_index: 1,
            no_fill: false,
            no_stroke: false,
            stroke_width_mm: 1.5,
            fill_opacity: 0.35,
            show_draw_tooltip: false,
            left_panel_collapsed: false,
            right_panel_collapsed: false,
            map_focused: false,
        }
    }

    #[must_use]
    pub fn active_stroke_color(&self) -> Color32 {
        let (r, g, b, _) = KA_STYLE_PALETTE[self.style_color_index.min(7)];
        Color32::from_rgb(r, g, b)
    }

    /// C++ `KaFeatureCard::bindFeature` 면적(`#define PYEONG_FACTOR 3.305785`) 및 둘레 포맷팅 1:1
    #[must_use]
    pub fn format_bound_ring_metrics(ring: &[[f64; 2]]) -> (String, String) {
        if ring.len() < 3 {
            return ("0.00 ㎡ (약 0.0 평)".to_owned(), "0.00 m".to_owned());
        }
        let area_m2 = crate::real_survey::ring_area_m2(ring);
        let pyeong = area_m2 / 3.305_785;
        let perim_m = crate::real_survey::ring_perimeter_m(ring);
        (
            format!("{area_m2:.2} ㎡ (약 {pyeong:.1} 평)"),
            format!("{perim_m:.2} m"),
        )
    }

    /// C++ `KaFeatureCard::saveCurrentAttributes() -> bool` 필수 항목(`FEAT_NO`, `FEAT_TYPE`) 검증 1:1
    #[must_use]
    pub fn save_current_attributes(&self) -> bool {
        !self.feature_no.trim().is_empty() && !self.feature_type.trim().is_empty()
    }
}

impl StrataAppState {
    /// C++ `KaInspectorPanel.cpp` + `KaInspectorStyle.cpp` + `KaStylePalette.h` 1:1 재현 (Images 2, 16)
    pub(crate) fn render_cpp_right_inspector(&mut self, ui: &mut egui::Ui) {
        if self.right_inspector.right_panel_collapsed {
            return;
        }
        egui::SidePanel::right("ka_map_right_inspector")
            .resizable(true)
            .default_width(242.0)
            .width_range(220.0..=300.0)
            .show_inside(ui, |ui| {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("선택한 유구").size(13.5).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let b = egui::Button::new("›")
                            .fill(Color32::WHITE)
                            .stroke(Stroke::new(1.0, Color32::from_rgb(185, 182, 172)))
                            .corner_radius(4.0)
                            .min_size(Vec2::new(22.0, 22.0));
                        if ui.add(b).on_hover_text("오른쪽 패널 접기 (F10)").clicked() {
                            self.right_inspector.right_panel_collapsed = true;
                        }
                    });
                });
                ui.add_space(4.0);

                let is_attr = self.right_inspector.active_card != InspectorCardTab::LayerStyleCard;
                ui.columns(2, |cols| {
                    for (ci, (lbl, active, target)) in [
                        ("속성", is_attr, InspectorCardTab::FeatureAttributeCard),
                        ("스타일", !is_attr, InspectorCardTab::LayerStyleCard),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let (tr, resp) = cols[ci].allocate_exact_size(Vec2::new(cols[ci].available_width(), 24.0), egui::Sense::click());
                        let tp = cols[ci].painter_at(tr);
                        tp.text(
                            tr.center(),
                            egui::Align2::CENTER_CENTER,
                            lbl,
                            egui::FontId::proportional(12.5),
                            if active { Color32::from_rgb(30, 30, 28) } else { Color32::from_rgb(105, 103, 96) },
                        );
                        let line_col = if active { Color32::from_rgb(35, 35, 32) } else { Color32::from_rgb(212, 209, 200) };
                        let line_w = if active { 2.0 } else { 1.0 };
                        tp.line_segment([Pos2::new(tr.left(), tr.bottom()), Pos2::new(tr.right(), tr.bottom())], Stroke::new(line_w, line_col));
                        if resp.clicked() {
                            self.right_inspector.active_card = target;
                        }
                    }
                });
                ui.add_space(8.0);

                if is_attr {
                    let active_ring = self
                        .home_map
                        .selected_feature_idx
                        .and_then(|idx| self.home_map.real_survey.survey_rings.get(idx))
                        .or_else(|| self.home_map.real_survey.survey_rings.last());
                    if let Some(ring) = active_ring {
                        let area_m2 = crate::real_survey::ring_area_m2(ring);
                        let pyeong = area_m2 / 3.305_785;
                        let perim_m = crate::real_survey::ring_perimeter_m(ring);
                        egui::Frame::NONE
                            .fill(Color32::from_rgb(245, 243, 236))
                            .stroke(Stroke::new(1.0, Color32::from_rgb(212, 209, 200)))
                            .corner_radius(5.0)
                            .inner_margin(egui::Margin::symmetric(8, 6))
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(format!("면적: {area_m2:.2} m² ({pyeong:.1} 평)"))
                                        .size(12.0)
                                        .strong()
                                        .color(Color32::from_rgb(30, 30, 28)),
                                );
                                ui.label(
                                    egui::RichText::new(format!("둘레: {perim_m:.2} m · {}", self.work_crs.auth_id()))
                                        .size(11.5)
                                        .color(Color32::from_rgb(80, 79, 74)),
                                );
                            });
                        ui.add_space(6.0);
                        egui::Grid::new("ka_feature_card_grid")
                            .num_columns(2)
                            .spacing([6.0, 5.0])
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("조사명").size(11.5));
                                if ui
                                    .add(egui::TextEdit::singleline(&mut self.right_inspector.survey_name).desired_width(155.0))
                                    .changed()
                                {
                                    self.is_dirty = true;
                                }
                                ui.end_row();
                                ui.label(egui::RichText::new("유구번호").size(11.5));
                                if ui
                                    .add(egui::TextEdit::singleline(&mut self.right_inspector.feature_no).desired_width(155.0))
                                    .changed()
                                {
                                    self.is_dirty = true;
                                }
                                ui.end_row();
                                ui.label(egui::RichText::new("종류").size(11.5));
                                egui::ComboBox::from_id_salt("ka_feat_type_combo")
                                    .selected_text(&self.right_inspector.feature_type)
                                    .width(155.0)
                                    .show_ui(ui, |ui| {
                                        for ft in ["조사구역", "주거지", "수혈", "분묘", "구", "주초", "유물포함층"] {
                                            if ui.selectable_value(&mut self.right_inspector.feature_type, ft.to_owned(), ft).changed() {
                                                self.is_dirty = true;
                                            }
                                        }
                                    });
                                ui.end_row();
                                ui.label(egui::RichText::new("시대").size(11.5));
                                egui::ComboBox::from_id_salt("ka_feat_period_combo")
                                    .selected_text(&self.right_inspector.feature_period)
                                    .width(155.0)
                                    .show_ui(ui, |ui| {
                                        for pd in ["미상", "구석기", "신석기", "청동기", "원삼국", "삼국", "통일신라", "고려", "조선"] {
                                            if ui.selectable_value(&mut self.right_inspector.feature_period, pd.to_owned(), pd).changed() {
                                                self.is_dirty = true;
                                            }
                                        }
                                    });
                                ui.end_row();
                            });
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new("메모").size(11.5));
                        if ui
                            .add(
                                egui::TextEdit::multiline(&mut self.right_inspector.feature_memo)
                                    .desired_rows(3)
                                    .desired_width(f32::INFINITY),
                            )
                            .changed()
                        {
                            self.is_dirty = true;
                        }
                        ui.add_space(6.0);
                        let card_btn = egui::Button::new(
                            egui::RichText::new("조사카드 열기")
                                .size(12.0)
                                .strong()
                                .color(Color32::WHITE),
                        )
                        .fill(Color32::from_rgb(181, 87, 58))
                        .corner_radius(5.0)
                        .min_size(Vec2::new(ui.available_width(), 28.0));
                        if ui.add(card_btn).clicked() {
                            self.right_inspector.survey_card_modal_open = true;
                        }
                    } else {
                        ui.label(
                            egui::RichText::new("지도에서 도형 하나를 고르면 기록이\n여기에 나옵니다.")
                                .size(12.0)
                                .color(Color32::from_rgb(95, 94, 88)),
                        );
                    }
                } else {
                    let layer_name = self.right_inspector.selected_layer_key.as_deref().unwrap_or("조사구역");
                    let stroke_col = self.right_inspector.active_stroke_color();
                    ui.horizontal(|ui| {
                        let (sr, _) = ui.allocate_exact_size(Vec2::new(16.0, 12.0), egui::Sense::hover());
                        let sp = ui.painter_at(sr);
                        sp.rect_filled(sr, 1.0, Color32::from_rgba_unmultiplied(stroke_col.r(), stroke_col.g(), stroke_col.b(), 70));
                        sp.rect_stroke(sr, 1.0, Stroke::new(2.0, stroke_col), egui::StrokeKind::Middle);
                        ui.label(egui::RichText::new(layer_name).size(12.5).color(Color32::from_rgb(35, 35, 32)));
                    });
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new("색").size(12.0).color(Color32::from_rgb(80, 79, 74)));
                    ui.add_space(2.0);
                    egui::Grid::new("ka_inspector_style_palette").spacing([5.0, 5.0]).show(ui, |ui| {
                        for (idx, (r, g, b, name)) in KA_STYLE_PALETTE.iter().enumerate() {
                            let (rect, resp) = ui.allocate_exact_size(Vec2::new(46.0, 24.0), egui::Sense::click());
                            let p = ui.painter_at(rect);
                            let col = Color32::from_rgb(*r, *g, *b);
                            p.rect_filled(rect, 4.0, col);
                            let chosen = self.right_inspector.style_color_index == idx;
                            let border = if chosen {
                                Stroke::new(2.6, Color32::from_rgb(28, 28, 26))
                            } else {
                                Stroke::new(1.0, Color32::from_rgb(175, 172, 162))
                            };
                            p.rect_stroke(rect, 4.0, border, egui::StrokeKind::Middle);
                            if resp.on_hover_text(*name).clicked() {
                                self.right_inspector.style_color_index = idx;
                            }
                            if idx % 4 == 3 {
                                ui.end_row();
                            }
                        }
                    });
                    ui.add_space(6.0);
                    if ui.radio(self.right_inspector.no_fill, "채우기 없음 (선만)").clicked() {
                        self.right_inspector.no_fill = !self.right_inspector.no_fill;
                        if self.right_inspector.no_fill {
                            self.right_inspector.no_stroke = false;
                        }
                    }
                    if ui.radio(self.right_inspector.no_stroke, "선 없음").clicked() {
                        self.right_inspector.no_stroke = !self.right_inspector.no_stroke;
                        if self.right_inspector.no_stroke {
                            self.right_inspector.no_fill = false;
                        }
                    }
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("외곽선 굵기").size(12.0).color(Color32::from_rgb(80, 79, 74)));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let w_txt = format!("{:.1} mm    ↕", self.right_inspector.stroke_width_mm);
                            let wb = egui::Button::new(egui::RichText::new(w_txt).size(12.0))
                                .fill(Color32::WHITE)
                                .stroke(Stroke::new(1.0, Color32::from_rgb(175, 172, 162)))
                                .corner_radius(5.0)
                                .min_size(Vec2::new(88.0, 26.0));
                            if ui.add(wb).clicked() {
                                self.right_inspector.stroke_width_mm = if self.right_inspector.stroke_width_mm >= 2.5 {
                                    0.5
                                } else {
                                    self.right_inspector.stroke_width_mm + 0.5
                                };
                            }
                        });
                    });
                }

                ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        for (lbl, src) in [
                            ("위성", BasemapSource::VworldSatellite),
                            ("지형", BasemapSource::VworldBase),
                            ("옛 지도 ▼", BasemapSource::Daedongyeojido),
                        ] {
                            let btn = egui::Button::new(egui::RichText::new(lbl).size(12.0).strong())
                                .fill(Color32::WHITE)
                                .stroke(Stroke::new(1.0, Color32::from_rgb(185, 182, 172)))
                                .corner_radius(5.0)
                                .min_size(Vec2::new(66.0, 30.0));
                            if ui.add(btn).clicked() {
                                self.home_map.active_basemap = src;
                            }
                        }
                    });
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("바탕 지도").size(11.5).color(Color32::from_rgb(95, 94, 88)));
                    ui.separator();
                });
            });

        if self.right_inspector.survey_card_modal_open {
            let mut open = self.right_inspector.survey_card_modal_open;
            let mut close_requested = false;
            egui::Window::new("조사카드 상세 기록 — Strata")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .default_width(380.0)
                .show(ui.ctx(), |ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "조사명: {} · 유구번호: {}",
                            self.right_inspector.survey_name, self.right_inspector.feature_no
                        ))
                        .size(13.0)
                        .strong(),
                    );
                    ui.label(format!(
                        "종류: {} · 시대: {}",
                        self.right_inspector.feature_type, self.right_inspector.feature_period
                    ));
                    ui.separator();
                    ui.label("현장 조사 메모 및 층위·출토유물 기록:");
                    if ui
                        .add(
                            egui::TextEdit::multiline(&mut self.right_inspector.feature_memo)
                                .desired_rows(5)
                                .desired_width(f32::INFINITY),
                        )
                        .changed()
                    {
                        self.is_dirty = true;
                    }
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        let save_btn = egui::Button::new(
                            egui::RichText::new("조사카드 저장").color(Color32::WHITE).strong(),
                        )
                        .fill(Color32::from_rgb(181, 87, 58))
                        .corner_radius(5.0);
                        if ui.add(save_btn).clicked() {
                            self.home_map.draw_guide_message = Some(format!(
                                "「{}」 조사카드를 저장했습니다.",
                                self.right_inspector.feature_no
                            ));
                            self.is_dirty = false;
                            close_requested = true;
                        }
                        if ui.button("닫기").clicked() {
                            close_requested = true;
                        }
                    });
                });
            self.right_inspector.survey_card_modal_open = open && !close_requested;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_card_area_calculation_10x20_rect_formats_exact_200m2_605_pyeong_and_60m_perim() {
        let rect_10x20 = [
            [0.0, 0.0],
            [10.0, 0.0],
            [10.0, 20.0],
            [0.0, 20.0],
            [0.0, 0.0],
        ];
        let (area_label, perim_label) = RightInspectorState::format_bound_ring_metrics(&rect_10x20);
        assert_eq!(area_label, "200.00 ㎡ (약 60.5 평)");
        assert_eq!(perim_label, "60.00 m");

        let mut card = RightInspectorState::new_default();
        assert!(card.save_current_attributes());
        card.feature_no.clear();
        assert!(!card.save_current_attributes());
    }
}
