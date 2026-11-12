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
                    // C++ 실물 지도 탭 옆판 바탕(cpp-start-maptab.png 실측 245,244,237).
                    .fill(Color32::from_rgb(245, 244, 237))
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
                    if self.left_layers.entries.is_empty() {
                        // MainWindow.cpp:798-811, MainWindowSession.cpp:752-760 refreshLayerEmptyState —
                        // 레이어가 없으면 목록 대신 가운데 맞춘 안내. 「표시 설정」은 찾기 칸 바로 아래.
                        self.paint_layer_display_settings(ui);
                        // Qt AlignCenter — 줄마다 가운데.
                        let rest = Rect::from_min_max(ui.cursor().min, layer_card_rect.max);
                        let lines: Vec<&str> = super::gui_layer_panel_row_spec::layer_panel_empty_hint().lines().collect();
                        let line_h = 17.0;
                        let top = rest.center().y - line_h * (lines.len() as f32) * 0.5;
                        for (i, line) in lines.iter().enumerate() {
                            ui.painter().text(
                                Pos2::new(rest.center().x, top + line_h * (i as f32 + 0.5)),
                                egui::Align2::CENTER_CENTER,
                                *line,
                                egui::FontId::proportional(12.0),
                                Color32::from_rgb(75, 74, 70),
                            );
                        }
                    } else {
                        self.paint_layer_tree(ui, layer_card_rect);
                        self.paint_layer_display_settings(ui);
                    }
                });

                let (div_r, _) =
                    ui.allocate_exact_size(Vec2::new(panel_w, 3.0), egui::Sense::hover());
                ui.painter()
                    .rect_filled(div_r, 0.0, Color32::from_rgb(135, 134, 127));

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
            let (toggle_all_label, has_layers) = self.toggle_all_state();
            if ui
                .add_enabled(
                    has_layers,
                    egui::Button::new(egui::RichText::new(toggle_all_label).size(12.0)).frame(false),
                )
                .clicked()
            {
                self.toggle_all_layers();
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

impl StrataAppState {
    /// C++ `refreshLayerCheckAllButton`(MainWindow.cpp:1272-1287) — 켜진 레이어가 하나라도 있으면
    /// 「전체 끄기」, 아니면 「전체 켜기」. 레이어가 없으면 단추를 흐리게(false).
    pub(crate) fn toggle_all_state(&self) -> (&'static str, bool) {
        let rs = &self.home_map.real_survey;
        let any_on = self.left_layers.entries.iter().any(|e| rs.is_layer_visible(&e.label_ko));
        (if any_on { "전체 끄기" } else { "전체 켜기" }, !self.left_layers.entries.is_empty())
    }

    /// C++ `toggleAllLayersChecked`(MainWindow.cpp:1242-1269) — 하나라도 켜져 있으면 모두 끄고, 모두
    /// 꺼져 있을 때만 모두 켠다. 상태줄에 「레이어 N개를 모두 켰습니다/껐습니다」.
    pub(crate) fn toggle_all_layers(&mut self) {
        let (label, has_layers) = self.toggle_all_state();
        if !has_layers {
            return;
        }
        let turn_on = label == "전체 켜기";
        let names: Vec<String> = self.left_layers.entries.iter().map(|e| e.label_ko.clone()).collect();
        let count = names.len();
        for n in names {
            self.home_map.real_survey.layer_visible.insert(n, turn_on);
        }
        self.home_map.real_survey.set_all_layers_visible(turn_on);
        self.left_layers.set_all_visible(turn_on);
        self.home_map.draw_guide_message = Some(if turn_on {
            format!("레이어 {count}개를 모두 켰습니다")
        } else {
            format!("레이어 {count}개를 모두 껐습니다")
        });
    }
}
