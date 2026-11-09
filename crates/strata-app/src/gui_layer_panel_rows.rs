//! 레이어 카드 스크롤 영역 안의 레이어 행들.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

use crate::app_state::StrataAppState;

use super::gui_layer_panel_paint::{paint_checkbox_state, paint_symbol_swatch};
use super::gui_layer_panel_row_spec::{layer_rows, LayerRowSpec};

impl StrataAppState {
    /// 스크롤 영역 안의 레이어 행들 (C++ `KaLayerInformationPanel` 트리).
    pub(crate) fn paint_layer_rows(&mut self, ui: &mut egui::Ui, avail_w: f32) {
        let dark = Color32::from_rgb(35, 35, 32);
        let blue = Color32::from_rgb(38, 72, 132);
        let purple_grp = Color32::from_rgb(98, 28, 118);
        let purple_leaf = Color32::from_rgb(112, 36, 138);
        let rows: [LayerRowSpec; 14] = layer_rows(dark, blue, purple_grp, purple_leaf);
        let filter = self.home_map.layer_filter.trim().to_owned();
        for (
            sec_title,
            top_rule,
            indent,
            arrow,
            sw_kind,
            name,
            cnt,
            right_lbl,
            alt_bg,
            txt_col,
                            ) in &rows
        {
            if !filter.is_empty() && !name.contains(&filter) {
                continue;
            }
            let band_h = if !sec_title.is_empty() {
                15.0
            } else if *top_rule {
                4.0
            } else {
                0.0
            };
            let row_h = 21.5;
            let (full_r, resp) = ui.allocate_exact_size(
                Vec2::new(avail_w, band_h + row_h),
                egui::Sense::click(),
            );
            let right_col_x = full_r.right() - 52.0;
            if resp.clicked() {
                let mut newly_selected: Option<String> = None;
                if let Some(pos) = resp.interact_pointer_pos() {
                    if pos.x < full_r.left() + 42.0 + *indent {
                        self.home_map.real_survey.toggle_layer_visible(name);
                        newly_selected = Some((*name).to_owned());
                    } else if pos.x >= right_col_x && right_lbl.is_some() {
                        self.home_map.real_survey.toggle_label_visible(name);
                    } else if self.right_inspector.selected_layer_key.as_deref()
                        == Some(*name)
                    {
                        self.right_inspector.selected_layer_key = None;
                    } else {
                        newly_selected = Some((*name).to_owned());
                    }
                } else {
                    newly_selected = Some((*name).to_owned());
                }
                if let Some(sel_name) = newly_selected {
                    let (op, br) = ui.ctx().data_mut(|d| {
                        d.get_temp::<(f32, f32)>(egui::Id::new((
                            "ka_layer_op_br",
                            sel_name.as_str(),
                        )))
                        .unwrap_or((1.0, 0.5))
                    });
                    self.home_map.real_survey.layer_opacity = op;
                    self.home_map.real_survey.layer_brightness = br;
                    self.right_inspector.selected_layer_key = Some(sel_name);
                }
            }
            // MainWindowContextMenus.cpp:176-448 layerContextMenu
            if resp.secondary_clicked() {
                let anchor = resp
                    .interact_pointer_pos()
                    .unwrap_or_else(|| full_r.center());
                self.right_inspector.selected_layer_key =
                    Some((*name).to_owned());
                self.layer_context_menu_open = true;
                self.layer_context_menu_pos = [anchor.x, anchor.y];
                self.layer_context_menu_key = Some((*name).to_owned());
                self.layer_menu_sub = 0;
            }
            // 더블클릭 → 면·외곽선 색 (KaDrawingStudio.cpp:738-744)
            if resp.double_clicked() {
                self.right_inspector.selected_layer_key =
                    Some((*name).to_owned());
                self.modals.active =
                    Some(crate::dialogs_modal::ActiveModalDialog::LineColor);
            }
            let is_sel =
                self.right_inspector.selected_layer_key.as_deref() == Some(*name);
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
            } else if *alt_bg {
                p.rect_filled(
                    Rect::from_min_max(
                        Pos2::new(full_r.left(), full_r.top() + band_h),
                        full_r.max,
                    ),
                    0.0,
                    Color32::from_rgb(239, 236, 228),
                );
            }
            if *top_rule {
                p.line_segment(
                    [
                        Pos2::new(full_r.left(), full_r.top() + 1.0),
                        Pos2::new(full_r.right(), full_r.top() + 1.0),
                    ],
                    Stroke::new(1.0, Color32::from_rgb(205, 202, 194)),
                );
            }
            if !sec_title.is_empty() {
                p.text(
                    Pos2::new(full_r.left() + 4.0, full_r.top() + 9.0),
                    egui::Align2::LEFT_CENTER,
                    *sec_title,
                    egui::FontId::proportional(10.5),
                    Color32::from_rgb(110, 108, 102),
                );
            }
            let ry = full_r.top() + band_h + row_h * 0.5;
            let mut x = full_r.left() + 6.0 + *indent;
            if let Some(arr) = arrow {
                p.text(
                    Pos2::new(x, ry),
                    egui::Align2::LEFT_CENTER,
                    *arr,
                    egui::FontId::proportional(10.0),
                    Color32::from_rgb(95, 94, 88),
                );
            }
            x += 12.0;
            let l_state = self.home_map.real_survey.layer_check_state(name);
            paint_checkbox_state(&p, Pos2::new(x + 6.0, ry), l_state);
            x += 18.0;
            paint_symbol_swatch(&p, Pos2::new(x + 7.5, ry), *sw_kind);
            x += 20.0;
            p.text(
                Pos2::new(x, ry),
                egui::Align2::LEFT_CENTER,
                *name,
                egui::FontId::proportional(11.0),
                *txt_col,
            );

            if let Some(c_str) = cnt {
                let dyn_cnt = if *name == "조사구역" {
                    self.home_map.real_survey.survey_rings.len().to_string()
                } else if *name == "시굴격자" {
                    self.home_map
                        .real_survey
                        .trenches
                        .len()
                        .max(232)
                        .to_string()
                } else {
                    (*c_str).to_owned()
                };
                p.text(
                    Pos2::new(right_col_x - 8.0, ry),
                    egui::Align2::RIGHT_CENTER,
                    dyn_cnt,
                    egui::FontId::proportional(10.5),
                    Color32::from_rgb(115, 113, 106),
                );
            }
            if let Some(rl) = right_lbl {
                let lbl_vis = self.home_map.real_survey.is_label_visible(name);
                paint_checkbox_state(
                    &p,
                    Pos2::new(right_col_x + 6.0, ry),
                    if lbl_vis { 2 } else { 0 },
                );
                p.text(
                    Pos2::new(right_col_x + 16.0, ry),
                    egui::Align2::LEFT_CENTER,
                    *rl,
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
}
