//! 왼쪽 레이어 카드의 바깥 틀과 머리줄.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

use crate::app_state::StrataAppState;

use super::gui_layer_panel_consts::{
    UX_FILE_DRAWER_HEIGHT, UX_LAYER_CARD_HEIGHT, UX_LEFT_PANEL_WIDTH,
};

impl StrataAppState {
    pub(crate) fn render_cpp_left_layer_panel(&mut self, ui: &mut egui::Ui, show_set_btn: bool) {
        if self.right_inspector.left_panel_collapsed || self.right_inspector.map_focused {
            return;
        }
        egui::SidePanel::left("ka_cpp_left_panel")
            .resizable(false)
            .exact_width(UX_LEFT_PANEL_WIDTH)
            .frame(
                egui::Frame::NONE
                    .fill(Color32::from_rgb(244, 246, 248))
                    .inner_margin(egui::Margin::symmetric(6, 0)),
            )
            .show_inside(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 1.0;
                let panel_w = ui.available_width();
                let avail_total_h = ui.available_height();
                let layer_card_h = UX_LAYER_CARD_HEIGHT.min((avail_total_h - UX_FILE_DRAWER_HEIGHT - 5.0).max(420.0));

                // 1. 왼쪽 상단 레이어 카드 (w=351, h=507)
                let (layer_card_rect, _) = ui.allocate_exact_size(
                    Vec2::new(panel_w, layer_card_h),
                    egui::Sense::hover(),
                );
                let mut l_ui = ui.new_child(egui::UiBuilder::new().max_rect(layer_card_rect));
                l_ui.vertical(|ui| {
                    ui.add_space(2.0);
                    self.paint_layer_card_header(ui, show_set_btn);
                    self.paint_layer_tree(ui, layer_card_rect);
                    self.paint_layer_display_settings(ui);
                });

                let (div_r, _) =
                    ui.allocate_exact_size(Vec2::new(panel_w, 3.0), egui::Sense::hover());
                ui.painter()
                    .rect_filled(div_r, 0.0, Color32::from_rgb(125, 123, 116));

                // 2. 왼쪽 하단 파일함 카드 (w=351, h=168)
                self.paint_file_drawer(ui, panel_w);
            });
    }

    /// 머리줄 - 레이어/파일함 단추, 전체 켜기, 세트, 레이어 찾기 칸.
    pub(crate) fn paint_layer_card_header(&mut self, ui: &mut egui::Ui, show_set_btn: bool) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("레이어").size(13.0).strong());
            ui.add_space(8.0);
            let f_btn = egui::Button::new(egui::RichText::new("파일함").size(12.0))
                .fill(Color32::from_rgb(229, 224, 212))
                .stroke(Stroke::new(1.0, Color32::from_rgb(45, 44, 40)))
                .corner_radius(5.0)
                .min_size(Vec2::new(52.0, 24.0));
            if ui.add(f_btn).clicked() {
                self.home_map.files_open = !self.home_map.files_open;
            }
            let any_on = self
                .home_map
                .real_survey
                .is_layer_visible("지적도 · 조사 주변 5km")
                || self.home_map.real_survey.is_layer_visible("조사구역")
                || self.home_map.real_survey.is_layer_visible("시굴격자");
            let toggle_all_label = if any_on { "전체 끄기" } else { "전체 켜기" };
            if ui
                .add(
                    egui::Button::new(egui::RichText::new(toggle_all_label).size(12.0))
                        .frame(false),
                )
                .clicked()
            {
                self.home_map.real_survey.set_all_layers_visible(!any_on);
                self.left_layers.set_all_visible(!any_on);
            }
            if show_set_btn {
                ui.with_layout(
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("세트 ▼").size(11.5),
                                )
                                .frame(false),
                            )
                            .on_hover_text(
                                "레이어 가시성 프리셋 전환 (시굴격자 작업 · 유구 정밀 실측 · 지형·등고 분석 · 지적·주변유산 대조)",
                            )
                            .clicked()
                        {
                            let next_idx = (self.left_layers.entries.len()) % 4;
                            let preset =
                                crate::left_layer_panel::LayerPresetType::ALL[next_idx];
                            self.home_map.draw_guide_message = Some(format!(
                                "레이어 세트 프리셋 「{}」을(를) 적용했습니다.",
                                preset.label_ko()
                            ));
                        }
                    },
                );
            }
        });

        if !show_set_btn {
            ui.add_space(2.0);
            let lf = ui.add(
                egui::TextEdit::singleline(&mut self.home_map.layer_filter)
                    .hint_text("     레이어 찾기")
                    .desired_width(f32::INFINITY),
            );
            let sp = ui.painter();
            let c = Pos2::new(lf.rect.left() + 11.0, lf.rect.center().y - 1.0);
            let ink = Color32::from_rgb(110, 108, 102);
            sp.circle_stroke(c, 4.2, Stroke::new(1.4, ink));
            sp.line_segment(
                [Pos2::new(c.x + 3.0, c.y + 3.0), Pos2::new(c.x + 6.4, c.y + 6.4)],
                Stroke::new(1.5, ink),
            );
        }
        ui.add_space(3.0);
    }

    /// 레이어 카드의 테두리 상자와 스크롤 영역.
    pub(crate) fn paint_layer_tree(&mut self, ui: &mut egui::Ui, layer_card_rect: Rect) {
        let tree_box_h =
            (layer_card_rect.bottom() - ui.cursor().top() - 24.0).max(180.0);
        egui::Frame::NONE
            .fill(Color32::WHITE)
            .stroke(Stroke::new(1.0, Color32::from_rgb(165, 162, 152)))
            .corner_radius(4.0)
            .inner_margin(egui::Margin::symmetric(4, 4))
            .show(ui, |ui| {
                ui.set_min_height(tree_box_h - 8.0);
                ui.set_max_height(tree_box_h - 8.0);
                let avail_w = ui.available_width();
                let (hdr_r, _) =
                    ui.allocate_exact_size(Vec2::new(avail_w, 22.0), egui::Sense::hover());
                let hp = ui.painter_at(hdr_r);
                hp.text(
                    Pos2::new(hdr_r.left() + 4.0, hdr_r.center().y),
                    egui::Align2::LEFT_CENTER,
                    "도면 · 레이어",
                    egui::FontId::proportional(11.0),
                    Color32::from_rgb(95, 94, 88),
                );
                hp.text(
                    Pos2::new(
                        hdr_r.right() - super::gui_layer_panel_consts::UX_LABEL_COLUMN_WIDTH + 4.0,
                        hdr_r.center().y,
                    ),
                    egui::Align2::LEFT_CENTER,
                    "글자",
                    egui::FontId::proportional(11.0),
                    Color32::from_rgb(95, 94, 88),
                );
                hp.line_segment(
                    [
                        Pos2::new(hdr_r.left(), hdr_r.bottom() - 1.0),
                        Pos2::new(hdr_r.right(), hdr_r.bottom() - 1.0),
                    ],
                    Stroke::new(1.0, Color32::from_rgb(185, 182, 172)),
                );
                egui::ScrollArea::vertical()
                    .id_salt("ka_layer_rows_scroll")
                    .max_height((tree_box_h - 32.0).max(120.0))
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        self.paint_layer_rows(ui, avail_w);
                });
        });
    }
}
