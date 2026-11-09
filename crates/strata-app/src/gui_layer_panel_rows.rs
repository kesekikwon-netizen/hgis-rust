//! 레이어 카드 스크롤 영역 안의 레이어 행들.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke};

use crate::app_state::StrataAppState;

use super::gui_layer_panel_paint::{paint_checkbox_state, paint_symbol_swatch};
use super::gui_layer_panel_row_spec::{layer_panel_empty_hint, layer_rows, LayerRowSpec};

impl StrataAppState {
    /// 스크롤 영역 안의 레이어 행들 (C++ `KaLayerInformationPanel` 트리).
    ///
    /// 실제 `left_layers.entries` 만큼만 그린다 — 레이어가 없으면 안내문 하나.
    pub(crate) fn paint_layer_rows(&mut self, ui: &mut egui::Ui, avail_w: f32) {
        let dark = Color32::from_rgb(35, 35, 32);
        let blue = Color32::from_rgb(38, 72, 132);
        let purple_grp = Color32::from_rgb(98, 28, 118);
        let purple_leaf = Color32::from_rgb(112, 36, 138);
        let rows: Vec<LayerRowSpec> =
            layer_rows(&self.left_layers, dark, blue, purple_grp, purple_leaf);
        let filter = self.home_map.layer_filter.trim().to_owned();

        if rows.is_empty() {
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(layer_panel_empty_hint())
                    .size(11.0)
                    .color(Color32::from_rgb(125, 123, 116)),
            );
            return;
        }

        for spec in &rows {
            if !filter.is_empty()
                && !spec.name.is_empty()
                && !spec.name.contains(&filter)
            {
                continue;
            }
            let band_h = if spec.section.is_empty() {
                if spec.top_rule {
                    4.0
                } else {
                    0.0
                }
            } else {
                15.0
            };
            let row_h = 21.5;
            let (full_r, resp) = ui.allocate_exact_size(
                egui::Vec2::new(avail_w, band_h + row_h),
                egui::Sense::click(),
            );
            let right_col_x = full_r.right() - 52.0;
            let is_section = !spec.section.is_empty();

            if !is_section {
                if resp.clicked() {
                    self.on_layer_row_clicked(&spec.name, full_r, &resp, right_col_x);
                }
                if resp.secondary_clicked() {
                    self.on_layer_row_secondary(&spec.name, full_r, &resp);
                }
                if resp.double_clicked() {
                    self.on_layer_row_double_clicked(&spec.name);
                }
            }

            let is_sel = !is_section
                && self.right_inspector.selected_layer_key.as_deref() == Some(spec.name.as_str());
            let p = ui.painter_at(full_r);
            if is_sel {
                p.rect_filled(
                    Rect::from_min_max(
                        Pos2::new(full_r.left(), full_r.top() + band_h),
                        full_r.max,
                    ),
                    0.0,
                    Color32::from_rgb(219, 212, 196),
                );
            } else if spec.alt_bg {
                p.rect_filled(
                    Rect::from_min_max(
                        Pos2::new(full_r.left(), full_r.top() + band_h),
                        full_r.max,
                    ),
                    0.0,
                    Color32::from_rgb(239, 236, 228),
                );
            }
            if spec.top_rule {
                p.line_segment(
                    [
                        Pos2::new(full_r.left(), full_r.top() + 1.0),
                        Pos2::new(full_r.right(), full_r.top() + 1.0),
                    ],
                    Stroke::new(1.0, Color32::from_rgb(205, 202, 194)),
                );
            }
            if is_section {
                p.text(
                    Pos2::new(full_r.left() + 4.0, full_r.top() + 9.0),
                    egui::Align2::LEFT_CENTER,
                    spec.section,
                    egui::FontId::proportional(10.5),
                    Color32::from_rgb(110, 108, 102),
                );
                continue;
            }

            let ry = full_r.top() + band_h + row_h * 0.5;
            let mut x = full_r.left() + 6.0 + spec.indent;
            if let Some(arr) = spec.arrow {
                p.text(
                    Pos2::new(x, ry),
                    egui::Align2::LEFT_CENTER,
                    arr,
                    egui::FontId::proportional(10.0),
                    Color32::from_rgb(95, 94, 88),
                );
            }
            x += 12.0;
            let l_state = self.home_map.real_survey.layer_check_state(&spec.name);
            paint_checkbox_state(&p, Pos2::new(x + 6.0, ry), l_state);
            x += 18.0;
            paint_symbol_swatch(&p, Pos2::new(x + 7.5, ry), spec.swatch);
            x += 20.0;
            p.text(
                Pos2::new(x, ry),
                egui::Align2::LEFT_CENTER,
                &spec.name,
                egui::FontId::proportional(11.0),
                spec.ink,
            );

            if spec.show_count {
                let cnt = self
                    .left_layers
                    .entries
                    .iter()
                    .find(|e| e.label_ko == spec.name)
                    .map_or(0, |e| e.feature_count);
                p.text(
                    Pos2::new(right_col_x - 8.0, ry),
                    egui::Align2::RIGHT_CENTER,
                    cnt.to_string(),
                    egui::FontId::proportional(10.5),
                    Color32::from_rgb(115, 113, 106),
                );
            }
            if let Some(rl) = spec.right_label {
                let lbl_vis = self.home_map.real_survey.is_label_visible(&spec.name);
                paint_checkbox_state(
                    &p,
                    Pos2::new(right_col_x + 6.0, ry),
                    if lbl_vis { 2 } else { 0 },
                );
                p.text(
                    Pos2::new(right_col_x + 16.0, ry),
                    egui::Align2::LEFT_CENTER,
                    rl,
                    egui::FontId::proportional(10.5),
                    Color32::from_rgb(75, 74, 70),
                );
            } else {
                p.text(
                    Pos2::new(right_col_x + 4.0, ry),
                    egui::Align2::LEFT_CENTER,
                    "—",
                    egui::FontId::proportional(10.5),
                    Color32::from_rgb(135, 133, 126),
                );
            }
        }
    }

    /// 행 클릭 — 왼쪽 체크박스는 표시 토글, 오른쪽 칸은 라벨 토글, 나머지는 선택.
    fn on_layer_row_clicked(
        &mut self,
        name: &str,
        full_r: Rect,
        resp: &egui::Response,
        right_col_x: f32,
    ) {
        let spec_indent = 0.0_f32;
        let mut newly_selected: Option<String> = None;
        if let Some(pos) = resp.interact_pointer_pos() {
            if pos.x < full_r.left() + 42.0 + spec_indent {
                self.home_map.real_survey.toggle_layer_visible(name);
                newly_selected = Some(name.to_owned());
            } else if pos.x >= right_col_x {
                self.home_map.real_survey.toggle_label_visible(name);
            } else if self.right_inspector.selected_layer_key.as_deref() == Some(name) {
                self.right_inspector.selected_layer_key = None;
            } else {
                newly_selected = Some(name.to_owned());
            }
        } else {
            newly_selected = Some(name.to_owned());
        }
        if let Some(sel_name) = newly_selected {
            let (op, br) = ui_temp_opacity(resp);
            self.home_map.real_survey.layer_opacity = op;
            self.home_map.real_survey.layer_brightness = br;
            self.right_inspector.selected_layer_key = Some(sel_name);
        }
    }

    /// 우클릭 — C++ `MainWindowContextMenus.cpp:176-448` layerContextMenu.
    fn on_layer_row_secondary(&mut self, name: &str, full_r: Rect, resp: &egui::Response) {
        let anchor = resp
            .interact_pointer_pos()
            .unwrap_or_else(|| full_r.center());
        self.right_inspector.selected_layer_key = Some(name.to_owned());
        self.layer_context_menu_open = true;
        self.layer_context_menu_pos = [anchor.x, anchor.y];
        self.layer_context_menu_key = Some(name.to_owned());
        self.layer_menu_sub = 0;
    }

    /// 더블클릭 — 면·외곽선 색 (`KaDrawingStudio.cpp:738-744`).
    fn on_layer_row_double_clicked(&mut self, name: &str) {
        self.right_inspector.selected_layer_key = Some(name.to_owned());
        self.modals.active = Some(crate::dialogs_modal::ActiveModalDialog::LineColor);
    }
}

/// 행마다 저장해 둔 불투명도·밝기 값(없으면 기본).
fn ui_temp_opacity(resp: &egui::Response) -> (f32, f32) {
    resp.ctx.data_mut(|d| {
        d.get_temp::<(f32, f32)>(egui::Id::new(("ka_layer_op_br", resp.id)))
            .unwrap_or((1.0, 0.5))
    })
}
