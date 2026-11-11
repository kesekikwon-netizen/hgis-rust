//! 속성 카드(attribute card) 본문.
//!
//! `right_inspector.rs` 가 300줄을 넘어 옮겼다.
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use crate::app_state::StrataAppState;
use crate::right_inspector::{next_feature_number, KA_BUILTIN_PERIODS};

impl StrataAppState {
    pub(crate) fn render_ri_attribute_card(&mut self, ui: &mut egui::Ui) {
// C++ `KaFeatureCard.cpp:202-278` (`d7afcc8`) — 시추공을 고르면 주상도 층서 카드를
// 유구 카드 위에 먼저 보여 준다. `hole_code` 가 없으면 예전처럼 유구 카드만 그린다.
if self.right_inspector.borehole.has_borehole_fields() {
    crate::borehole_card::render_borehole_card(
        ui,
        &self.right_inspector.borehole,
    );
    ui.add_space(6.0);
}
// 고른 도형(조사 파일의 유구·선·점, 또는 조사구역)의 크기 두 줄. 없으면 빈 안내를 보인다.
if let Some((line1, line2)) = self.selection_summary_lines() {
    egui::Frame::NONE
        .fill(Color32::from_rgb(245, 243, 236))
        .stroke(Stroke::new(1.0, Color32::from_rgb(212, 209, 200)))
        .corner_radius(5.0)
        .inner_margin(egui::Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(line1.clone())
                .size(12.0)
                .strong()
                .color(Color32::from_rgb(30, 30, 28)),
            );
            ui.label(
                egui::RichText::new(line2.clone())
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
                .add(
                    egui::TextEdit::singleline(
                        &mut self.right_inspector.survey_name,
                    )
                    .desired_width(155.0),
                )
                .changed()
            {
                self.is_dirty = true;
            }
            ui.end_row();
            ui.label(egui::RichText::new("유구번호").size(11.5));
            ui.horizontal(|ui| {
                if ui
                    .add(
                        egui::TextEdit::singleline(
                            &mut self.right_inspector.feature_no,
                        )
                        .desired_width(96.0),
                    )
                    .changed()
                {
                    self.is_dirty = true;
                }
                // KaFeatureCardEditors.cpp:157-170
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("다음 번호").size(11.0),
                        )
                        .min_size(Vec2::new(50.0, 20.0)),
                    )
                    .on_hover_text("지금까지 쓴 번호 다음 값을 넣습니다.")
                    .clicked()
                {
                    let next =
                        next_feature_number(&self.right_inspector.feature_no);
                    self.right_inspector.feature_no = next;
                    self.is_dirty = true;
                }
            });
            ui.end_row();
            ui.label(egui::RichText::new("종류").size(11.5));
            egui::ComboBox::from_id_salt("ka_feat_type_combo")
                .selected_text(&self.right_inspector.feature_type)
                .width(155.0)
                .show_ui(ui, |ui| {
                    for ft in [
                        "조사구역",
                        "주거지",
                        "수혈",
                        "분묘",
                        "구",
                        "주초",
                        "유물포함층",
                    ] {
                        if ui
                            .selectable_value(
                                &mut self.right_inspector.feature_type,
                                ft.to_owned(),
                                ft,
                            )
                            .changed()
                        {
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
                    for pd in [
                        "미상",
                        "구석기",
                        "신석기",
                        "청동기",
                        "원삼국",
                        "삼국",
                        "통일신라",
                        "고려",
                        "조선",
                    ] {
                        if ui
                            .selectable_value(
                                &mut self.right_inspector.feature_period,
                                pd.to_owned(),
                                pd,
                            )
                            .changed()
                        {
                            self.is_dirty = true;
                        }
                    }
                });
            ui.end_row();
        });
        // KaFeatureCardEditors.cpp:38-75 — 10칸 시대 띠 (168x13)
        {
            let (band_r, _) = ui.allocate_exact_size(
                Vec2::new(168.0, 13.0),
                egui::Sense::hover(),
            );
            let bp = ui.painter_at(band_r);
            bp.rect_stroke(
                band_r,
                2.0,
                Stroke::new(1.0, Color32::from_rgb(31, 35, 40)),
                egui::StrokeKind::Middle,
            );
            let cell_w = 168.0 / KA_BUILTIN_PERIODS.len() as f32;
            let sel_idx = KA_BUILTIN_PERIODS
                .iter()
                .position(|(n, _)| *n == self.right_inspector.feature_period);
            for (i, (nm, rgb)) in KA_BUILTIN_PERIODS.iter().enumerate() {
                let x0 = band_r.left() + cell_w * (i as f32);
                let cell = Rect::from_min_size(
                    Pos2::new(x0 + 1.0, band_r.top() + 2.0),
                    Vec2::new(cell_w - 2.0, 8.0),
                );
                let col = match rgb {
                    Some((r, g, b)) => Color32::from_rgb(*r, *g, *b),
                    None => Color32::from_rgb(230, 228, 220),
                };
                let cell_resp = ui.interact(
                    cell,
                    egui::Id::new(("ka_period_band", i)),
                    egui::Sense::click(),
                );
                bp.rect_filled(cell, 1.0, col);
                if sel_idx == Some(i) {
                    bp.line_segment(
                        [
                            Pos2::new(x0 + 2.0, band_r.bottom() - 3.0),
                            Pos2::new(x0 + cell_w - 2.0, band_r.bottom() - 3.0),
                        ],
                        Stroke::new(2.0, Color32::from_rgb(31, 35, 40)),
                    );
                }
                if cell_resp.on_hover_text(*nm).clicked() {
                    self.right_inspector.feature_period = (*nm).to_owned();
                    self.is_dirty = true;
                }
            }
        }
    ui.add_space(4.0);
    // 단면 표고: 그 유구에 상면·바닥 표고(top_el·bottom_el)가 적혀 있을 때만 보인다.
    if let Some((top_el, bottom_el)) = self.selected_elevations() {
        let (pit_depth, pit_caption) =
            strata_terrain3d::trench_profile3d::compute_pit_elevation_depth_and_caption(
                Some(top_el),
                Some(bottom_el),
                None,
            );
        egui::Frame::NONE
            .fill(Color32::from_rgb(245, 240, 232))
            .stroke(Stroke::new(1.0, Color32::from_rgb(212, 204, 191)))
            .corner_radius(5.0)
            .inner_margin(egui::Margin::symmetric(8, 5))
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("단면 표고 (KaElevationFigure)")
                        .size(11.0)
                        .strong()
                        .color(Color32::from_rgb(107, 78, 61)),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "상면 EL. {top_el:.2} · 바닥 EL. {bottom_el:.2} · 깊이 {:.2} m",
                        pit_depth.unwrap_or(0.0)
                    ))
                    .size(11.0)
                    .color(Color32::from_rgb(47, 42, 38)),
                )
                .on_hover_text(pit_caption);
            });
    }
    ui.add_space(2.0);
    ui.label(egui::RichText::new("메모").size(11.5));
    if ui
        .add(
            egui::TextEdit::multiline(&mut self.right_inspector.feature_memo)
                .desired_rows(2)
                .desired_width(f32::INFINITY),
        )
        .changed()
    {
        self.is_dirty = true;
    }
    ui.add_space(4.0);
    // KaFeatureCard.cpp:240-244,298 — 저장 상태 칩
    if self.is_dirty {
        ui.label(
            egui::RichText::new("● 변경됨 · 아직 저장 안 됨")
                .size(11.0)
                .color(Color32::from_rgb(181, 87, 58)),
        )
        .on_hover_text("저장(Ctrl+S)으로 파일에 씁니다.");
    } else {
        ui.label(
            egui::RichText::new("● 저장된 값입니다")
                .size(11.0)
                .color(Color32::from_rgb(105, 103, 96)),
        );
    }
    let card_btn = egui::Button::new(
        egui::RichText::new("조사카드 열기")
            .size(12.0)
            .strong()
            .color(Color32::WHITE),
    )
    .fill(Color32::from_rgb(181, 87, 58))
    .corner_radius(5.0)
    .min_size(Vec2::new(ui.available_width(), 28.0));
    if ui
        .add(card_btn)
        .on_hover_text("이 유구의 전체 조사카드(모든 칸)를 창으로 엽니다.")
        .clicked()
    {
        self.right_inspector.survey_card_modal_open = true;
    }
} else {
    let is_drawing = self.ribbon.sub_toolbar_visible
        && self.ribbon.sub_mode == crate::ribbon::SubToolsMode::Draw;
    let empty_msg = if is_drawing {
        "도형을 다 그리면 그 기록이 여기에\n나옵니다."
    } else {
        "지도에서 도형 하나를 고르면 기록이\n여기에 나옵니다."
    };
    ui.add_space(4.0);
    ui.label(
        egui::RichText::new(empty_msg)
            .size(12.0)
            .color(Color32::from_rgb(95, 94, 88)),
    );
}
    }
}
