//! `render_cpp_right_inspector` 의 본문(오른쪽 검사기 카드 전체).
//!
//! `right_inspector.rs` 가 300줄을 넘어 한 메서드의 본문을 옮겼다.
//! 실행 순서는 호출부가 보장한다(진입 메서드가 이 함수만 부른다).
use eframe::egui::{self, Color32, Pos2, Stroke, Vec2};
use crate::app_state::StrataAppState;
use crate::right_inspector::{InspectorCardTab};

impl StrataAppState {
    pub(crate) fn render_ri_body(&mut self, ui: &mut egui::Ui) {
        ui.add_space(2.0);
        // KaInspectorPanel.cpp:152-173 — 제목은 카드의 것(비면 「선택한 유구」), 고른 수 칩 「%1개 선택」.
        let title = self.card_view_now().map_or("선택한 유구", |v| v.title);
        let selected = usize::from(self.home_map.selected_shape.is_some());
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(title).size(13.5).strong());
            if selected > 0 {
                ui.label(egui::RichText::new(format!("{selected}개 선택")).size(11.5).color(Color32::from_rgb(105, 103, 96)));
            }
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
                let (tr, resp) = cols[ci].allocate_exact_size(
                    Vec2::new(cols[ci].available_width(), 24.0),
                    egui::Sense::click(),
                );
                let tp = cols[ci].painter_at(tr);
                tp.text(
                    tr.center(),
                    egui::Align2::CENTER_CENTER,
                    lbl,
                    egui::FontId::proportional(12.5),
                    if active {
                        Color32::from_rgb(30, 30, 28)
                    } else {
                        Color32::from_rgb(105, 103, 96)
                    },
                );
                let line_col = if active {
                    Color32::from_rgb(35, 35, 32)
                } else {
                    Color32::from_rgb(212, 209, 200)
                };
                let line_w = if active { 2.0 } else { 1.0 };
                tp.line_segment(
                    [
                        Pos2::new(tr.left(), tr.bottom()),
                        Pos2::new(tr.right(), tr.bottom()),
                    ],
                    Stroke::new(line_w, line_col),
                );
                if resp.clicked() {
                    self.right_inspector.active_card = target;
                }
            }
        });
        ui.add_space(4.0);

        self.render_ri_body_cards(ui, is_attr);

        self.render_ri_body_bottom(ui);
    }
}
