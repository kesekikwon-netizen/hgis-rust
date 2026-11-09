//! 용지 장식띠(축척막대·축척글·좌표계·방위표) — C++ `KaDrawingNorth::paintMark` 1:1.
//!
//! `gui_tab_drawing_chrome` 에서 300줄을 지키려고 나눴다. 표제란·장식띠 좌표 계산은
//! 저쪽에 있고, 이 파일은 그 계산값을 인자로 받아 그리기만 한다.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

use strata_canvas2d::viewport::format_scale_denominator;
use strata_layout::sheet_composer::{nice_scale_bar_segment_meters, NorthArrowStyle, ScaleBarStyle};

use crate::app_state::StrataAppState;

use super::gui_tab_drawing_frame::{ChromeRects, SheetFrame};

impl StrataAppState {
    /// 축척막대·축척글·좌표계·방위표 (`KaDrawingNorth::paintMark` 1:1).
    pub(crate) fn paint_sheet_decoration(
        &mut self,
        f: &SheetFrame,
        s: f32,
        ink: Color32,
        r: ChromeRects,
    ) {
        let north_rect = r.north;
        let chrome_crs_r = r.crs;
        let chrome_bar_r = r.bar;
        let chrome_lbl_r = r.label;
        let chrome = f.chrome;
        let map_rect = f.map_rect;
        let px_per_mm = f.px_per_mm;
        let px_per_meter = f.px_per_meter;
        let scale_denom = f.scale_denom;
        let sheet_p = f.sheet_p.clone();
        let seg_m = nice_scale_bar_segment_meters(
            chrome.map.width_mm,
            scale_denom,
            4,
        );
        let seg_count = 4usize;
        let max_bar_w = (chrome_crs_r.left() - chrome_bar_r.left() - 6.0 * px_per_mm).max(36.0 * s);
        let raw_bar_w = ((seg_m * (seg_count as f64) * px_per_meter) as f32)
            .clamp(38.0 * s, max_bar_w.min(chrome_bar_r.width().max(68.0 * s)));
        let seg_w = raw_bar_w / (seg_count as f32);
        let bar_x0 = chrome_bar_r.left();
        let bar_y0 = map_rect.bottom() + 9.5 * px_per_mm;
        let bar_h = (4.0 * px_per_mm).clamp(2.8, 7.2);

        match self.drawing.sheet.scale_bar_style {
            ScaleBarStyle::DoubleBox => {
                let half_h = bar_h * 0.5;
                for row in 0..2 {
                    for i in 0..seg_count {
                        let r_box = Rect::from_min_size(
                            Pos2::new(
                                bar_x0 + (i as f32) * seg_w,
                                bar_y0 + (row as f32) * half_h,
                            ),
                            Vec2::new(seg_w, half_h),
                        );
                        let dark = (i + row) % 2 == 0;
                        sheet_p.rect_filled(
                            r_box,
                            0.0,
                            if dark { ink } else { Color32::WHITE },
                        );
                        sheet_p.rect_stroke(
                            r_box,
                            0.0,
                            Stroke::new(0.7, ink),
                            egui::StrokeKind::Middle,
                        );
                    }
                }
            }
            ScaleBarStyle::SingleBox => {
                for i in 0..seg_count {
                    let r_box = Rect::from_min_size(
                        Pos2::new(bar_x0 + (i as f32) * seg_w, bar_y0),
                        Vec2::new(seg_w, bar_h),
                    );
                    sheet_p.rect_filled(
                        r_box,
                        0.0,
                        if i % 2 == 0 { ink } else { Color32::WHITE },
                    );
                    sheet_p.rect_stroke(
                        r_box,
                        0.0,
                        Stroke::new(0.8, ink),
                        egui::StrokeKind::Middle,
                    );
                }
            }
            ScaleBarStyle::SteppedTicks => {
                let by = bar_y0 + bar_h;
                sheet_p.line_segment(
                    [Pos2::new(bar_x0, by), Pos2::new(bar_x0 + raw_bar_w, by)],
                    Stroke::new(1.2, ink),
                );
                for i in 0..=seg_count {
                    let tx = bar_x0 + (i as f32) * seg_w;
                    sheet_p.line_segment(
                        [Pos2::new(tx, by), Pos2::new(tx, bar_y0)],
                        Stroke::new(1.0, ink),
                    );
                }
            }
        }
        let dist_font_sz = (4.2 * px_per_mm).clamp(3.6, 7.0);
        for i in [0usize, 2, seg_count] {
            let tx = bar_x0 + (i as f32) * seg_w;
            let dist_val = seg_m * (i as f64);
            let dist_lbl = if i == seg_count {
                if dist_val >= 1.0 {
                    format!("{dist_val:.0} m")
                } else {
                    format!("{dist_val:.1} m")
                }
            } else if dist_val >= 1.0 {
                format!("{dist_val:.0}")
            } else {
                format!("{dist_val:.1}")
            };
            sheet_p.text(
                Pos2::new(tx, bar_y0 - 2.6 * px_per_mm),
                egui::Align2::CENTER_CENTER,
                dist_lbl,
                egui::FontId::proportional(dist_font_sz),
                ink,
            );
        }
        let scale_font_sz = (4.8 * px_per_mm).clamp(4.0, 8.2);
        sheet_p.text(
            Pos2::new(
                bar_x0 + raw_bar_w * 0.5,
                chrome_lbl_r.center().y.max(bar_y0 + bar_h + 4.5 * px_per_mm),
            ),
            egui::Align2::CENTER_CENTER,
            format!(
                "축척 1 : {}",
                format_scale_denominator(f64::from(self.drawing.sheet.scale_denominator))
            ),
            egui::FontId::proportional(scale_font_sz),
            ink,
        );

        // 좌표계 라벨 (chrome.crs)
        let crs_font_sz = (4.5 * px_per_mm).clamp(3.8, 7.5);
        sheet_p.text(
            Pos2::new(north_rect.left() - 3.0 * px_per_mm, map_rect.bottom() + 4.5 * px_per_mm),
            egui::Align2::RIGHT_CENTER,
            match self.work_crs {
                strata_types::crs::WorkCrs::Epsg5186 => "중부원점(GRS80) · EPSG:5186",
                _ => "동부원점(GRS80) · EPSG:5187",
            },
            egui::FontId::proportional(crs_font_sz),
            ink,
        );

        // (4) 방위표 (도북 화살, KaDrawingNorth::paintMark 1:1)
        let nc = north_rect.center();
        let nr = north_rect.width() * 0.44;
        match self.drawing.sheet.north_arrow_style {
            NorthArrowStyle::LetterOnly => {
                let tri = vec![
                    Pos2::new(nc.x, north_rect.top() + 2.0),
                    Pos2::new(nc.x + nr * 0.55, nc.y),
                    Pos2::new(nc.x - nr * 0.55, nc.y),
                ];
                sheet_p.add(egui::Shape::convex_polygon(tri, ink, Stroke::NONE));
                sheet_p.text(
                    Pos2::new(nc.x, nc.y + nr * 0.52),
                    egui::Align2::CENTER_CENTER,
                    "N",
                    egui::FontId::proportional(9.5),
                    ink,
                );
            }
            NorthArrowStyle::SingleArrow => {
                sheet_p.text(
                    Pos2::new(nc.x, north_rect.top() + north_rect.height() * 0.16),
                    egui::Align2::CENTER_CENTER,
                    "N",
                    egui::FontId::proportional((8.5 * s).clamp(4.8, 9.5)),
                    ink,
                );
                let tip = Pos2::new(nc.x, north_rect.top() + north_rect.height() * 0.34);
                let tail = Pos2::new(nc.x, north_rect.bottom() - north_rect.height() * 0.14);
                let wing_y = north_rect.bottom() - north_rect.height() * 0.05;
                let hw = nr * 0.42;
                let left_half = vec![tip, Pos2::new(nc.x - hw, wing_y), tail];
                let right_half = vec![tip, tail, Pos2::new(nc.x + hw, wing_y)];
                sheet_p.add(egui::Shape::convex_polygon(
                    left_half,
                    ink,
                    Stroke::new(0.8, ink),
                ));
                sheet_p.add(egui::Shape::convex_polygon(
                    right_half,
                    Color32::WHITE,
                    Stroke::new(0.8, ink),
                ));
            }
            NorthArrowStyle::CompassRose => {
                let c_pos = Pos2::new(nc.x, nc.y + 2.0);
                sheet_p.circle_stroke(c_pos, nr * 0.78, Stroke::new(1.1, ink));
                sheet_p.circle_stroke(c_pos, nr * 0.46, Stroke::new(0.8, ink));
                let needle = vec![
                    Pos2::new(nc.x, north_rect.top() + 1.5),
                    Pos2::new(nc.x + 3.2, c_pos.y - nr * 0.45),
                    Pos2::new(nc.x - 3.2, c_pos.y - nr * 0.45),
                ];
                sheet_p.add(egui::Shape::convex_polygon(needle, ink, Stroke::NONE));
                sheet_p.text(
                    c_pos,
                    egui::Align2::CENTER_CENTER,
                    "N",
                    egui::FontId::proportional(7.5),
                    ink,
                );
            }
            NorthArrowStyle::WindRose => {
                let v_diamond = vec![
                    Pos2::new(nc.x, north_rect.top() + 2.0),
                    Pos2::new(nc.x + 3.0, nc.y + 1.0),
                    Pos2::new(nc.x, north_rect.bottom() - 2.0),
                    Pos2::new(nc.x - 3.0, nc.y + 1.0),
                ];
                let h_diamond = vec![
                    Pos2::new(north_rect.left() + 2.0, nc.y + 1.0),
                    Pos2::new(nc.x, nc.y - 1.8),
                    Pos2::new(north_rect.right() - 2.0, nc.y + 1.0),
                    Pos2::new(nc.x, nc.y + 3.8),
                ];
                sheet_p.add(egui::Shape::convex_polygon(
                    h_diamond,
                    Color32::from_rgb(75, 85, 99),
                    Stroke::NONE,
                ));
                sheet_p.add(egui::Shape::convex_polygon(v_diamond, ink, Stroke::NONE));
                sheet_p.text(
                    Pos2::new(nc.x + 6.5, north_rect.top() + 5.0),
                    egui::Align2::CENTER_CENTER,
                    "N",
                    egui::FontId::proportional(7.5),
                    ink,
                );
            }
        }
    }
}
