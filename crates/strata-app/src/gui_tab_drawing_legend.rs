//! (1) 용지 범례 — `legend_visible` 토글 연동, `KaDrawingStudio::defaultItemRect(kIdLegend)`.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

use crate::app_state::StrataAppState;

use super::gui_tab_drawing_frame::SheetFrame;

impl StrataAppState {
    /// 범례 상자와 항목들.
    pub(crate) fn paint_sheet_legend(&mut self, f: &SheetFrame) {
        let map_rect = f.map_rect;
        let px_per_mm = f.px_per_mm;
        let ink = f.ink;
        let sheet_p = f.sheet_p.clone();
        let rs = &self.home_map.real_survey;
        // 3. 용지 내 4요소 실물 조판: (1) 범례, (2) 표제란, (3) 축척막대·축척글·좌표계, (4) 방위표(도북 화살)

        // (1) 범례 (legend_visible 토글 연동, KaDrawingStudio::defaultItemRect(kIdLegend))
        if self.drawing.sheet.legend_visible {
            let mut legend_items: Vec<(Color32, Color32, &str)> = Vec::new();
            if rs.is_layer_visible("조사구역") {
                legend_items.push((
                    Color32::from_rgba_unmultiplied(214, 92, 32, 95),
                    Color32::from_rgb(214, 78, 22),
                    "조사구역",
                ));
            }
            if rs.is_layer_visible("시굴격자") {
                legend_items.push((
                    Color32::from_rgba_unmultiplied(156, 62, 32, 80),
                    Color32::from_rgb(156, 62, 32),
                    "시굴격자",
                ));
            }
            if rs.is_layer_visible("주변 500m") {
                legend_items.push((
                    Color32::TRANSPARENT,
                    Color32::from_rgb(184, 58, 24),
                    "주변 500m",
                ));
            }
            if rs.is_layer_visible("참조 지도") && rs.is_layer_visible("지정유산") {
                if rs.is_layer_visible("국가지정유산") || rs.is_layer_visible("시도지정유산")
                {
                    legend_items.push((
                        Color32::from_rgba_unmultiplied(142, 68, 173, 60),
                        Color32::from_rgb(162, 32, 118),
                        "지정유산",
                    ));
                }
                if rs.is_layer_visible("국가지정유산보호구역")
                    || rs.is_layer_visible("시도지정유산보호구역")
                {
                    legend_items.push((
                        Color32::from_rgba_unmultiplied(39, 174, 96, 60),
                        Color32::from_rgb(34, 148, 52),
                        "보호·유존지역",
                    ));
                }
            }
            if rs.is_layer_visible("지적도 · 조사 주변 5km") {
                legend_items.push((
                    Color32::TRANSPARENT,
                    Color32::from_rgb(68, 66, 60),
                    "지적도",
                ));
            }

            let s = (px_per_mm / 1.65).clamp(0.42, 1.3);
            let base_leg_r = f.mm_to_rect(self.drawing.sheet.legend_rect);
            let row_h = 12.0 * s;
            let leg_w = base_leg_r.width().clamp(42.0 * s, map_rect.width() * 0.42);
            let leg_h = (20.0 * s + (legend_items.len() as f32) * row_h)
                .clamp(24.0 * s, base_leg_r.height().max(68.0 * s));
            let leg_r = Rect::from_min_size(
                Pos2::new(
                    (map_rect.right() - leg_w - 5.0 * s).max(map_rect.left() + 4.0),
                    base_leg_r.top(),
                ),
                Vec2::new(leg_w, leg_h),
            );
            sheet_p.rect_filled(
                leg_r,
                1.0,
                Color32::from_rgba_unmultiplied(255, 255, 255, 245),
            );
            sheet_p.rect_stroke(leg_r, 1.0, Stroke::new(0.9, ink), egui::StrokeKind::Middle);
            let title_pt = ((self.drawing.sheet.legend_font_pt as f32) * s).clamp(4.5, 11.5);
            sheet_p.text(
                Pos2::new(leg_r.left() + 5.0 * s, leg_r.top() + 9.0 * s),
                egui::Align2::LEFT_CENTER,
                &self.drawing.sheet.legend_title,
                egui::FontId::proportional(title_pt),
                ink,
            );
            sheet_p.line_segment(
                [
                    Pos2::new(leg_r.left() + 3.5 * s, leg_r.top() + 16.5 * s),
                    Pos2::new(leg_r.right() - 3.5 * s, leg_r.top() + 16.5 * s),
                ],
                Stroke::new(0.5, Color32::from_rgb(200, 198, 190)),
            );
            for (idx, (fill_c, stroke_c, name)) in legend_items.into_iter().enumerate() {
                let ry = leg_r.top() + 23.5 * s + (idx as f32) * row_h;
                if ry + 3.0 * s > leg_r.bottom() {
                    break;
                }
                let sw_r = Rect::from_min_size(
                    Pos2::new(leg_r.left() + 5.0 * s, ry - 3.5 * s),
                    Vec2::new(10.0 * s, 7.0 * s),
                );
                if fill_c != Color32::TRANSPARENT {
                    sheet_p.rect_filled(sw_r, 1.0, fill_c);
                }
                sheet_p.rect_stroke(
                    sw_r,
                    1.0,
                    Stroke::new(1.0, stroke_c),
                    egui::StrokeKind::Middle,
                );
                sheet_p.text(
                    Pos2::new(sw_r.right() + 4.0 * s, ry),
                    egui::Align2::LEFT_CENTER,
                    name,
                    egui::FontId::proportional((title_pt - 1.2 * s).max(4.0)),
                    ink,
                );
            }
        }
    }
}
