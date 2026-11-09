//! 용지 위 팝업 — 지도 조정 안내 띠, 좌표점 콜아웃, 도면 목록.
//!
//! C++ `KaDrawingStudio.cpp:822-838`, `KaDrawingCoordCallout.cpp`,
//! `KaDrawingSheetSet.cpp:135-160`.

use eframe::egui::{self, Color32, FontId, Pos2, Rect, Stroke, Vec2};

use crate::app_state::StrataAppState;

use super::gui_tab_drawing_frame::SheetFrame;

impl StrataAppState {
    /// 지도 조정 안내 띠, 좌표점 콜아웃(A, B, C …), 도면 목록 팝업.
    pub(crate) fn paint_sheet_popups(&mut self, ui: &mut egui::Ui, f: &SheetFrame) {
        let sheet_rect = f.sheet_rect;
        let rect = f.rect;
        let p = f.p.clone();
        // KaDrawingStudio.cpp:822-838 m_adjustBar — 지도 조정 중 상단 안내 띠
        if self.drawing.map_adjust_active {
            let bar_w = 420.0_f32.min(rect.width() - 24.0);
            let bar_r = Rect::from_min_size(
                Pos2::new(rect.center().x - bar_w * 0.5, rect.top() + 8.0),
                Vec2::new(bar_w, 30.0),
            );
            p.rect_filled(bar_r, 7.0, Color32::from_rgba_unmultiplied(255, 253, 249, 244));
            p.rect_stroke(
                bar_r,
                7.0,
                Stroke::new(1.0, Color32::from_rgb(0xC5, 0xBF, 0xB2)),
                egui::StrokeKind::Middle,
            );
            p.text(
                Pos2::new(bar_r.left() + 10.0, bar_r.center().y),
                egui::Align2::LEFT_CENTER,
                "지도 조정 중 — 드래그로 이동, 휠로 확대. 끝나면 「조정끝」.",
                FontId::proportional(11.5),
                Color32::from_rgb(28, 25, 23),
            );
            let done_r = Rect::from_min_size(
                Pos2::new(bar_r.right() - 62.0, bar_r.top() + 4.0),
                Vec2::new(56.0, 22.0),
            );
            if ui
                .interact(done_r, ui.id().with("drawing_adjust_done"), egui::Sense::click())
                .on_hover_text("지도 조정을 끝냅니다.")
                .clicked()
            {
                self.drawing.map_adjust_active = false;
                self.home_map.draw_guide_message =
                    Some("지도 조정을 끝냈습니다.".to_owned());
            }
            p.text(
                done_r.center(),
                egui::Align2::CENTER_CENTER,
                "조정끝",
                FontId::proportional(11.0),
                Color32::from_rgb(28, 25, 23),
            );
        }

        // KaDrawingCoordCallout.cpp — 용지 위 좌표점 콜아웃(A, B, C …)
        {
            let mut callout_pick: Option<[f32; 2]> = None;
            if self.drawing.coord_callout_mode {
                let resp = ui.interact(
                    sheet_rect,
                    ui.id().with("drawing_callout_pick"),
                    egui::Sense::click(),
                );
                if resp.clicked() {
                    if let Some(pos) = ui.ctx().pointer_interact_pos() {
                        if sheet_rect.contains(pos) {
                            callout_pick = Some([pos.x, pos.y]);
                        }
                    }
                }
            }
            if let Some(pos) = callout_pick {
                let tag =
                    ((b'A' + (self.drawing.coord_callouts.len() % 26) as u8) as char)
                        .to_string();
                let n = f64::from(pos[1] - sheet_rect.top()) * 0.5
                    + self.home_map.real_survey.survey_center[1];
                let e = f64::from(pos[0] - sheet_rect.left()) * 0.5
                    + self.home_map.real_survey.survey_center[0];
                self.drawing
                    .coord_callouts
                    .push(crate::tab_drawing::CoordCallout {
                        tag,
                        pos,
                        world: [n, e],
                    });
            }
            for c in &self.drawing.coord_callouts {
                let cp = Pos2::new(c.pos[0], c.pos[1]);
                let label = format!("{}  X {:.2}  Y {:.2}", c.tag, c.world[1], c.world[0]);
                let lw = (label.chars().count() as f32) * 6.6 + 22.0;
                let pill = Rect::from_min_size(
                    Pos2::new(cp.x + 8.0, cp.y - 22.0),
                    Vec2::new(lw, 18.0),
                );
                p.circle_filled(cp, 3.2, Color32::from_rgb(181, 87, 58));
                p.line_segment(
                    [cp, Pos2::new(pill.left(), pill.center().y)],
                    Stroke::new(1.0, Color32::from_rgb(181, 87, 58)),
                );
                p.rect_filled(pill, 3.0, Color32::WHITE);
                p.rect_stroke(
                    pill,
                    3.0,
                    Stroke::new(1.0, Color32::from_rgb(0xC5, 0xBF, 0xB2)),
                    egui::StrokeKind::Middle,
                );
                p.text(
                    Pos2::new(pill.left() + 5.0, pill.center().y),
                    egui::Align2::LEFT_CENTER,
                    &label,
                    FontId::monospace(9.5),
                    Color32::from_rgb(28, 25, 23),
                );
            }
        }

        // KaDrawingSheetSet.cpp:135-160 — 도면 목록 팝업
        if self.drawing.sheet_set_menu_open {
            let menu_w = 300.0_f32;
            let row_h = 26.0_f32;
            let head_h = 30.0_f32;
            let n = self.drawing.stored_sheets.len();
            let menu_h = head_h + row_h * (n as f32).max(1.0) + 34.0;
            let menu_r = Rect::from_min_size(
                Pos2::new(rect.right() - menu_w - 16.0, rect.bottom() - menu_h - 60.0),
                Vec2::new(menu_w, menu_h),
            );
            p.rect_filled(menu_r, 6.0, Color32::WHITE);
            p.rect_stroke(
                menu_r,
                6.0,
                Stroke::new(1.0, Color32::from_rgb(185, 182, 172)),
                egui::StrokeKind::Middle,
            );
            p.text(
                Pos2::new(menu_r.left() + 10.0, menu_r.top() + head_h * 0.5),
                egui::Align2::LEFT_CENTER,
                "도면 목록",
                FontId::proportional(12.0),
                Color32::from_rgb(28, 25, 23),
            );
            let mut pick_store = false;
            let mut pick_open: Option<usize> = None;
            for (idx, st) in self.drawing.stored_sheets.iter().enumerate() {
                let row = Rect::from_min_size(
                    Pos2::new(
                        menu_r.left() + 6.0,
                        menu_r.top() + head_h + row_h * idx as f32,
                    ),
                    Vec2::new(menu_w - 12.0, row_h - 2.0),
                );
                let resp = ui.interact(
                    row,
                    ui.id().with(("drawing_sheet_row", idx)),
                    egui::Sense::click(),
                );
                p.text(
                    Pos2::new(row.left() + 4.0, row.center().y),
                    egui::Align2::LEFT_CENTER,
                    format!("{}  (1:{})", st.title, st.scale_denominator),
                    FontId::proportional(11.0),
                    Color32::from_rgb(45, 44, 40),
                );
                if resp.clicked() {
                    pick_open = Some(idx);
                }
            }
            if n == 0 {
                p.text(
                    Pos2::new(menu_r.left() + 10.0, menu_r.top() + head_h + row_h * 0.5),
                    egui::Align2::LEFT_CENTER,
                    "보관한 도면이 없습니다",
                    FontId::proportional(11.0),
                    Color32::from_rgb(135, 133, 126),
                );
            }
            let store_r = Rect::from_min_size(
                Pos2::new(menu_r.left() + 6.0, menu_r.bottom() - 32.0),
                Vec2::new(menu_w - 12.0, 26.0),
            );
            if ui
                .interact(store_r, ui.id().with("drawing_sheet_store"), egui::Sense::click())
                .on_hover_text("지금 도면을 이름 붙여 보관합니다.")
                .clicked()
            {
                pick_store = true;
            }
            p.text(
                store_r.center(),
                egui::Align2::CENTER_CENTER,
                "지금 도면 보관…",
                FontId::proportional(11.5),
                Color32::from_rgb(181, 87, 58),
            );
            if pick_store {
                self.drawing.stored_sheets.push(crate::tab_drawing::StoredSheet {
                    title: self.drawing.title_ko.clone(),
                    scale_denominator: self.drawing.sheet.scale_denominator,
                    paper_w_mm: self.drawing.sheet.width_mm,
                    paper_h_mm: self.drawing.sheet.height_mm,
                    legend_visible: self.drawing.sheet.legend_visible,
                    title_block_visible: self.drawing.sheet.title_block_visible,
                });
                self.home_map.draw_guide_message = Some(format!(
                    "도면을 보관했습니다: 「{}」",
                    self.drawing.title_ko
                ));
                self.drawing.sheet_set_menu_open = false;
            }
            if let Some(idx) = pick_open {
                if let Some(st) = self.drawing.stored_sheets.get(idx).cloned() {
                    self.drawing.title_ko = st.title.clone();
                    self.drawing.sheet.scale_denominator = st.scale_denominator;
                    self.drawing.sheet.legend_visible = st.legend_visible;
                    self.drawing.sheet.title_block_visible = st.title_block_visible;
                    self.home_map.draw_guide_message =
                        Some(format!("보관한 도면을 열었습니다: 「{}」", st.title));
                }
                self.drawing.sheet_set_menu_open = false;
            }
        }
    }
}
