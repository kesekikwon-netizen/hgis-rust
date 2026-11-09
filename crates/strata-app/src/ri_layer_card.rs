//! 레이어 카드(layer card) 본문.
//!
//! `right_inspector.rs` 가 300줄을 넘어 옮겼다.
use eframe::egui::{self, Color32, Stroke, Vec2};
use crate::app_state::StrataAppState;
use crate::right_inspector::{KA_STYLE_PALETTE};

impl StrataAppState {
    pub(crate) fn render_ri_layer_card(&mut self, ui: &mut egui::Ui) {
let layer_name = self
    .right_inspector
    .selected_layer_key
    .as_deref()
    .unwrap_or("조사구역");
let stroke_col = self.right_inspector.active_stroke_color();
ui.horizontal(|ui| {
    let (sr, _) =
        ui.allocate_exact_size(Vec2::new(16.0, 12.0), egui::Sense::hover());
    let sp = ui.painter_at(sr);
    sp.rect_filled(
        sr,
        1.0,
        Color32::from_rgba_unmultiplied(
            stroke_col.r(),
            stroke_col.g(),
            stroke_col.b(),
            70,
        ),
    );
    sp.rect_stroke(
        sr,
        1.0,
        Stroke::new(2.0, stroke_col),
        egui::StrokeKind::Middle,
    );
    ui.label(
        egui::RichText::new(layer_name)
            .size(12.5)
            .color(Color32::from_rgb(35, 35, 32)),
    );
});
// KaFeatureCard.cpp — 유형별 추가 편집 필드
egui::Grid::new("ka_feature_card_extra")
    .num_columns(2)
    .spacing([6.0, 5.0])
    .show(ui, |ui| {
        let layer_kind = self
            .right_inspector
            .selected_layer_key
            .clone()
            .unwrap_or_else(|| "조사구역".to_owned());
        if layer_kind.contains("기준점") {
            ui.label(egui::RichText::new("기준점 번호").size(11.5));
            if ui
                .add(
                    egui::TextEdit::singleline(
                        &mut self.right_inspector.control_point_id,
                    )
                    .desired_width(150.0),
                )
                .changed()
            {
                self.is_dirty = true;
            }
            ui.end_row();
            ui.label(egui::RichText::new("X(북)").size(11.5));
            ui.add(
                egui::DragValue::new(&mut self.right_inspector.control_x)
                    .speed(0.01)
                    .fixed_decimals(3),
            );
            ui.end_row();
            ui.label(egui::RichText::new("Y(동)").size(11.5));
            ui.add(
                egui::DragValue::new(&mut self.right_inspector.control_y)
                    .speed(0.01)
                    .fixed_decimals(3),
            );
            ui.end_row();
            ui.label(egui::RichText::new("Z(표고)").size(11.5));
            ui.add(
                egui::DragValue::new(&mut self.right_inspector.control_z)
                    .speed(0.01)
                    .fixed_decimals(2)
                    .suffix(" m"),
            );
            ui.end_row();
        } else if layer_kind.contains("단면") {
            ui.label(egui::RichText::new("단면 번호").size(11.5));
            if ui
                .add(
                    egui::TextEdit::singleline(
                        &mut self.right_inspector.section_id,
                    )
                    .desired_width(150.0)
                    .hint_text("A-A'"),
                )
                .changed()
            {
                self.is_dirty = true;
            }
            ui.end_row();
        } else if layer_kind.contains("조사구역") {
            ui.label(egui::RichText::new("유적명").size(11.5));
            if ui
                .add(
                    egui::TextEdit::singleline(
                        &mut self.right_inspector.site_name,
                    )
                    .desired_width(150.0),
                )
                .changed()
            {
                self.is_dirty = true;
            }
            ui.end_row();
        } else {
            ui.label(egui::RichText::new("이름").size(11.5));
            if ui
                .add(
                    egui::TextEdit::singleline(
                        &mut self.right_inspector.feature_name,
                    )
                    .desired_width(150.0),
                )
                .changed()
            {
                self.is_dirty = true;
            }
            ui.end_row();
        }
    });
// KaFeatureCard.cpp 하단 액션 버튼
ui.horizontal_wrapped(|ui| {
    let save_btn = egui::Button::new(
        egui::RichText::new("저장 (Enter)")
            .color(Color32::WHITE)
            .strong()
            .size(11.5),
    )
    .fill(Color32::from_rgb(181, 87, 58))
    .corner_radius(5.0)
    .min_size(Vec2::new(92.0, 24.0));
    if ui
        .add(save_btn)
        .on_hover_text("지금 카드에 적은 값을 도형 기록에 저장합니다.")
        .clicked()
    {
        self.is_dirty = false;
        self.home_map.draw_guide_message =
            Some("도형 기록을 저장했습니다.".to_owned());
    }
    if ui
        .button(egui::RichText::new("되돌리기").size(11.0))
        .on_hover_text("마지막 편집을 되돌립니다 (Ctrl+Z).")
        .clicked()
    {
        self.home_map.draw_guide_message =
            Some("마지막 편집을 되돌렸습니다.".to_owned());
    }
    if ui
        .button(egui::RichText::new("도형 삭제 (Del)").size(11.0))
        .on_hover_text("이 도형을 지웁니다.")
        .clicked()
    {
        if let Some(idx) = self.home_map.selected_feature_idx {
            if idx < self.home_map.real_survey.survey_rings.len() {
                self.home_map.real_survey.survey_rings.remove(idx);
                self.home_map.selected_feature_idx = None;
                self.is_dirty = true;
                self.home_map.draw_guide_message =
                    Some("도형을 지웠습니다.".to_owned());
            }
        }
    }
    if ui
        .button(egui::RichText::new("면·선 색상…").size(11.0))
        .on_hover_text("이 도형의 면·외곽선 색과 굵기를 고칩니다.")
        .clicked()
    {
        self.modals.active =
            Some(crate::dialogs_modal::ActiveModalDialog::LineColor);
    }
});
ui.add_space(6.0);
ui.label(
    egui::RichText::new("색")
        .size(12.0)
        .color(Color32::from_rgb(80, 79, 74)),
);
ui.add_space(2.0);
egui::Grid::new("ka_inspector_style_palette")
    .spacing([5.0, 5.0])
    .show(ui, |ui| {
        for (idx, (r, g, b, name)) in KA_STYLE_PALETTE.iter().enumerate() {
            let (rect, resp) = ui.allocate_exact_size(
                Vec2::new(46.0, 24.0),
                egui::Sense::click(),
            );
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
let mut nf = self.right_inspector.no_fill;
if ui
    .add(egui::Checkbox::new(&mut nf, "채우기 없음 (선만)"))
    .changed()
{
    self.right_inspector.no_fill = nf;
    if nf {
        self.right_inspector.no_stroke = false;
    }
}
let mut ns = self.right_inspector.no_stroke;
if ui
    .add(egui::Checkbox::new(&mut ns, "선 없음"))
    .changed()
{
    self.right_inspector.no_stroke = ns;
    if ns {
        self.right_inspector.no_fill = false;
    }
}
ui.add_space(6.0);
ui.horizontal(|ui| {
    ui.label(
        egui::RichText::new("외곽선 굵기")
            .size(12.0)
            .color(Color32::from_rgb(80, 79, 74)),
    );
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        // KaInspectorStyle.cpp:87-94 — QDoubleSpinBox 0.2..=12.0
        ui.add(
            egui::DragValue::new(&mut self.right_inspector.stroke_width_mm)
                .range(0.2..=12.0)
                .speed(0.2)
                .fixed_decimals(1)
                .suffix(" mm"),
        );
    });
});
    }
}
