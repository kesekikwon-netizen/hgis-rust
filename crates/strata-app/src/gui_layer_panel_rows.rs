//! 레이어 카드 스크롤 영역 안의 레이어 행들.

use eframe::egui::{self, Color32, Pos2, Rect};

use crate::app_state::StrataAppState;

use super::gui_layer_panel_paint::{paint_checkbox_state, paint_symbol_swatch};
use super::gui_layer_panel_row_spec::{layer_panel_empty_hint, layer_rows, LayerRowSpec};

impl StrataAppState {
    /// 스크롤 영역 안의 레이어 행들 (C++ `KaLayerInformationPanel` 트리).
    ///
    /// 실제 `left_layers.entries` 만큼만 그린다 — 레이어가 없으면 안내문 하나.
    pub(crate) fn paint_layer_rows(&mut self, ui: &mut egui::Ui, avail_w: f32) {
        use super::gui_layer_panel_consts::{
            UX_LABEL_COLUMN_WIDTH, UX_LEGEND_ALT_ROW, UX_LEGEND_BAND_HEIGHT, UX_LEGEND_BAND_INK,
            UX_LEGEND_BAND_RULE, UX_LEGEND_ROW_HEIGHT,
        };
        let dark = Color32::from_rgb(35, 35, 32);
        let purple_leaf = Color32::from_rgb(112, 36, 138);
        let rows: Vec<LayerRowSpec> = layer_rows(&self.left_layers, dark, purple_leaf);
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

        // C++ 트리처럼 행 사이 틈 없이 붙인다(이 스크롤 영역 안에는 행만 있다).
        ui.spacing_mut().item_spacing.y = 0.0;
        let ppp = ui.ctx().pixels_per_point();
        // 보이는 행 번호 — QTreeView 처럼 한 줄 걸러 배경을 바꾼다(띠도 그 행의 배경을 쓴다).
        let mut painted = 0_usize;
        for spec in &rows {
            if !filter.is_empty()
                && !spec.name.is_empty()
                && !spec.name.contains(&filter)
            {
                continue;
            }
            let band_h = if spec.section.is_empty() { 0.0 } else { UX_LEGEND_BAND_HEIGHT };
            let row_h = UX_LEGEND_ROW_HEIGHT;
            let (full_r, resp) = ui.allocate_exact_size(
                egui::Vec2::new(avail_w, band_h + row_h),
                egui::Sense::click(),
            );
            let right_col_x = full_r.right() - UX_LABEL_COLUMN_WIDTH;

            if resp.clicked() {
                self.on_layer_row_clicked(&spec.name, full_r, &resp, right_col_x);
            }
            if resp.secondary_clicked() {
                self.on_layer_row_secondary(spec, full_r, &resp, band_h);
            }
            if resp.double_clicked() {
                self.start_inline_rename(&spec.key);
            }

            let is_sel =
                self.right_inspector.selected_layer_key.as_deref() == Some(spec.name.as_str());
            let p = ui.painter_at(full_r);
            if painted % 2 == 1 {
                p.rect_filled(full_r, 0.0, UX_LEGEND_ALT_ROW);
            }
            painted += 1;
            if is_sel {
                p.rect_filled(
                    Rect::from_min_max(
                        Pos2::new(full_r.left(), full_r.top() + band_h),
                        full_r.max,
                    ),
                    0.0,
                    Color32::from_rgb(219, 212, 196),
                );
            }
            // C++ `KaLayerSectionDelegate.cpp:87-106` paintBand — 1 px 윗줄과 왼쪽 6 px 의 제목.
            if band_h > 0.0 {
                // 화면 픽셀에 맞춰 칠해야 섞인 회색이 아니라 C++ 과 같은 색으로 보인다.
                let top = (full_r.top() * ppp).round() / ppp;
                p.rect_filled(
                    Rect::from_min_max(
                        Pos2::new(full_r.left(), top),
                        Pos2::new(full_r.right(), top + (ppp.round() / ppp).max(1.0 / ppp)),
                    ),
                    0.0,
                    UX_LEGEND_BAND_RULE,
                );
                p.text(
                    Pos2::new(full_r.left() + 6.0, full_r.top() + band_h * 0.5 + 0.5),
                    egui::Align2::LEFT_CENTER,
                    spec.section,
                    egui::FontId::proportional(11.0),
                    UX_LEGEND_BAND_INK,
                );
            }

            // 가로 자리는 C++ 화면을 잰 값: 체크상자 가운데 27, 견본 41~61, 이름 69.
            let ry = full_r.top() + band_h + row_h * 0.5;
            let x = full_r.left() + spec.indent;
            if let Some(arr) = spec.arrow {
                p.text(
                    Pos2::new(x + 6.0, ry),
                    egui::Align2::LEFT_CENTER,
                    arr,
                    egui::FontId::proportional(10.0),
                    Color32::from_rgb(95, 94, 88),
                );
            }
            let l_state = self.home_map.real_survey.layer_check_state(&spec.name);
            paint_checkbox_state(&p, Pos2::new(x + 27.0, ry), l_state);
            paint_symbol_swatch(&p, Pos2::new(x + 51.0, ry), spec.swatch);
            if self.home_map.inline_rename.as_ref().is_some_and(|r| r.0 == spec.key) {
                self.paint_inline_rename(ui, Rect::from_min_max(Pos2::new(x + 66.0, ry - 10.0), Pos2::new(right_col_x - 8.0, ry + 10.0)));
            } else {
                p.text(Pos2::new(x + 69.0, ry), egui::Align2::LEFT_CENTER, &spec.name, egui::FontId::proportional(11.0), spec.ink);
            }

            if spec.show_count {
                p.text(
                    Pos2::new(right_col_x - 8.0, ry),
                    egui::Align2::RIGHT_CENTER,
                    spec.count.to_string(),
                    egui::FontId::proportional(10.5),
                    Color32::from_rgb(115, 113, 106),
                );
            }
            // 글자 칸은 그 레이어의 지금 라벨 글자(LayerLabelControls::describe — 메뉴로 항목·면적을 바꾸면 따라간다).
            let live = spec.right_label.and_then(|_| self.layer_menu_input(&spec.key)).filter(|i| i.has_labeling).map(|i| i.label.caption);
            if let Some(rl) = live.as_deref().or(spec.right_label) {
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
        // 목록의 빈 곳 우클릭 — 레이어 아닌 줄 메뉴(:195-213).
        let (_, blank) = ui.allocate_exact_size(egui::Vec2::new(avail_w, ui.available_height().max(24.0)), egui::Sense::click());
        if let (true, Some(at)) = (blank.secondary_clicked(), blank.interact_pointer_pos()) {
            self.open_blank_layer_menu(at);
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

    /// 우클릭 — C++ `MainWindowContextMenus.cpp:176-448`. 줄을 먼저 현재 레이어로 하고(:218-219) 그 layer_key 로 메뉴를 연다.
    /// 줄 위 묶음 띠를 누르면 묶음 머리 메뉴(:195-213).
    fn on_layer_row_secondary(&mut self, spec: &LayerRowSpec, full_r: Rect, resp: &egui::Response, band_h: f32) {
        let anchor = resp.interact_pointer_pos().unwrap_or_else(|| full_r.center());
        self.layer_context_menu_open = true;
        self.layer_context_menu_pos = [anchor.x, anchor.y];
        if anchor.y < full_r.top() + band_h {
            self.layer_context_menu_key = None;
            self.home_map.layer_menu_group = Some(spec.group);
        } else {
            self.right_inspector.selected_layer_key = Some(spec.name.clone());
            self.layer_context_menu_key = Some(spec.key.clone());
            self.home_map.layer_menu_group = None;
        }
    }

    /// 그 자리 이름 고치기 상자(QTreeView 편집기) — Enter·칸을 떠나면 바꾸고, Esc 는 그대로.
    fn paint_inline_rename(&mut self, ui: &mut egui::Ui, rect: Rect) {
        let Some((_, text, first)) = self.home_map.inline_rename.as_mut() else { return };
        let r = ui.put(rect, egui::TextEdit::singleline(text).font(egui::FontId::proportional(11.0)));
        if *first {
            r.request_focus();
            *first = false;
        }
        let cancel = ui.input(|i| i.key_pressed(egui::Key::Escape));
        if r.lost_focus() || cancel {
            self.finish_inline_rename(!cancel);
        }
    }

    /// 목록의 빈 곳 우클릭 — 레이어 아닌 줄 메뉴(「참고 자료 불러오기…」).
    pub(crate) fn open_blank_layer_menu(&mut self, at: egui::Pos2) {
        self.layer_context_menu_open = true;
        self.layer_context_menu_pos = [at.x, at.y];
        self.layer_context_menu_key = None;
        self.home_map.layer_menu_group = None;
    }
}

/// 행마다 저장해 둔 불투명도·밝기 값(없으면 기본).
fn ui_temp_opacity(resp: &egui::Response) -> (f32, f32) {
    resp.ctx.data_mut(|d| {
        d.get_temp::<(f32, f32)>(egui::Id::new(("ka_layer_op_br", resp.id)))
            .unwrap_or((1.0, 0.5))
    })
}
