//! 오른쪽 「속성」 카드 — C++ `KaFeatureCard` 새 모양(KaFeatureCard.cpp:70-125 틀, KaFeatureCardPlate.cpp:22-110 머리판)과
//! 패널 문장(KaInspectorPanel.cpp:164-173). 값·고침은 feature_card.rs·app_state_feature_card.rs, 줄은 ri_attr_card_rows.rs.
//! 창(모달)이 열려 있으면 카드는 흐려 누를 수 없다(C++ 창 뒤의 카드).

use eframe::egui::{self, Color32, RichText, Stroke, Vec2};

use crate::app_state::StrataAppState;
use crate::feature_card::{CardState, CardView};
use crate::ri_attr_card_rows::{control_rows, ledger, period_band, INK, MUTED};

/// 「저장 안 됨」 점 #D97757, 못 바꿈 빨강.
const CLAY: Color32 = Color32::from_rgb(0xD9, 0x77, 0x57);
const DANGER: Color32 = Color32::from_rgb(0xB4, 0x23, 0x18);
const ACCENT: Color32 = Color32::from_rgb(181, 87, 58);

fn sentence(ui: &mut egui::Ui, text: &str) {
    ui.add_space(16.0);
    ui.add(egui::Label::new(RichText::new(text).size(12.0).color(MUTED)).wrap());
    ui.add_space(16.0);
}

impl StrataAppState {
    pub(crate) fn render_ri_attribute_card(&mut self, ui: &mut egui::Ui) {
        // C++ `KaFeatureCard.cpp:202-278` (`d7afcc8`) — 시추공을 고르면 주상도 층서 카드를 유구 카드 위에 먼저 보여 준다.
        if self.right_inspector.borehole.has_borehole_fields() {
            crate::borehole_card::render_borehole_card(ui, &self.right_inspector.borehole);
            ui.add_space(6.0);
        }
        self.drop_card_if_gone();
        let drawing = self.active_tool == strata_canvas2d::map_tools::MapToolKind::DrawPolygon;
        let Some(view) = self.card_view_now() else {
            sentence(ui, if drawing { "도형을 다 그리면 그 기록이 여기에 나옵니다." } else { "지도에서 도형 하나를 고르면 기록이 여기에 나옵니다." });
            return;
        };
        if drawing {
            sentence(ui, "그리는 중에도 여기서 바로 고칠 수 있습니다. 지도를 누르면 이어서 그립니다.");
        }
        let enabled = !self.modal_window_open();
        ui.add_enabled_ui(enabled, |ui| self.render_card_body(ui, &view));
    }

    fn render_card_body(&mut self, ui: &mut egui::Ui, v: &CardView) {
        ui.spacing_mut().item_spacing.y = 8.0;
        // 머리판 — 도형 모양 견본 28×20(1 px 먹 테두리), 이름 20 px, 아랫줄 12 px.
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            let (r, _) = ui.allocate_exact_size(Vec2::new(28.0, 20.0), egui::Sense::hover());
            let style = self.card_swatch_style();
            let rgba = |c: [u8; 4]| Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]);
            ui.painter().rect_filled(r.shrink(3.0), 0.0, rgba(style.fill));
            ui.painter().rect_stroke(r.shrink(3.0), 0.0, Stroke::new(1.5, rgba(style.stroke)), egui::StrokeKind::Middle);
            ui.painter().rect_stroke(r, 0.0, Stroke::new(1.0, INK), egui::StrokeKind::Inside);
            ui.add(egui::Label::new(RichText::new(&v.header).size(20.0).color(INK)).truncate());
        });
        if !v.sub_line.is_empty() {
            ui.label(RichText::new(&v.sub_line).size(12.0).color(MUTED));
        }
        if let Some(c) = &v.control {
            control_rows(ui, c);
        }
        ledger(ui, v);
        self.card_number_row(ui, v);
        for (field, word, value) in &v.name_rows {
            self.card_text_row(ui, field, word, value, "");
        }
        if v.has_kind || v.has_period {
            let table = self.home_map.card.as_ref().map(|c| c.table.clone()).unwrap_or_default();
            let items = |field: &str| self.survey_tables.iter().find(|t| t.table == table).map(|t| crate::feature_form::choices(t, field)).unwrap_or_default();
            let (kinds, periods) = (items("kind"), items("period"));
            if v.has_kind {
                self.card_choice_row(ui, "kind", "종류", &v.kind, &kinds);
            }
            if v.has_period {
                self.card_choice_row(ui, "period", "시대", &v.period, &periods);
                period_band(ui, &v.period);
            }
        }
        for (field, word, value) in &v.work_rows {
            let hint = if *field == "surv_date" { "예: 2026-10-03" } else { "" };
            self.card_text_row(ui, field, word, value, hint);
        }
        if v.has_note {
            self.card_memo(ui, &v.note);
        }
        // 상태 칩 — 저장됨은 흐린 글만, 저장 안 됨은 진흙색 점 + 먹 글, 못 바꿈은 ⊗ + 빨강.
        let (dot, words, color) = match &v.state {
            CardState::Saved => (None, v.state.text(), MUTED),
            CardState::Changed => (Some(CLAY), v.state.text(), INK),
            CardState::Failed(_) => (Some(DANGER), v.state.text(), DANGER),
        };
        let chip = ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            if let Some(c) = dot {
                let (r, _) = ui.allocate_exact_size(Vec2::new(6.0, 22.0), egui::Sense::hover());
                ui.painter().circle_filled(r.center(), 3.0, c);
            }
            ui.label(RichText::new(words).size(12.0).color(color));
        });
        if v.state != CardState::Saved {
            chip.response.on_hover_text("「저장」 Ctrl+S로 파일에 씁니다.");
        }
        let open = egui::Button::new(RichText::new("조사카드 열기").size(12.0).strong().color(Color32::WHITE)).fill(ACCENT).corner_radius(5.0).min_size(Vec2::new(ui.available_width(), 36.0));
        if ui.add(open).on_hover_text("이 도형의 전체 조사카드(모든 칸)를 창으로 엽니다.").clicked() {
            // MainWindowFeatureCard.cpp:65-67 formRequested → 「도형 속성」.
            if let Some(c) = self.home_map.card.clone() {
                self.open_feature_record(&c.table, c.fid);
            }
        }
    }

    /// 견본 모양 — 그 레이어에 정한 모양(조사구역 그리기 설정), 없으면 레이어 키의 기본 모양.
    fn card_swatch_style(&self) -> crate::survey_style::DomainStyle {
        let table = self.home_map.card.as_ref().map(|c| c.table.as_str()).unwrap_or_default();
        let key = if crate::survey_area_setup::is_survey_area_key(table) { "survey_area" } else { table };
        self.home_map.real_survey.layer_styles.get(table).cloned().unwrap_or_else(|| crate::survey_style::domain_style_for_key(key))
    }
}

#[cfg(test)]
#[path = "ri_attr_card_tests.rs"]
mod tests;
