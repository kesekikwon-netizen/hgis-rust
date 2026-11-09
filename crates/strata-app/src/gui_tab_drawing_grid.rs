//! (g) 도곽 좌표격자 및 TM 좌표 주기 — C++ `LayoutService::applySurveyFrameGrid`.

use eframe::egui::{self, Color32, Pos2, Stroke};

use strata_layout::sheet_composer::nice_grid_interval_meters;

use crate::app_state::StrataAppState;

use super::gui_tab_drawing_frame::SheetFrame;

impl StrataAppState {
    /// 격자선·좌표 눈금과 지도 칸 테두리.
    pub(crate) fn paint_sheet_grid(&mut self, f: &SheetFrame) {
        let map_rect = f.map_rect;
        let px_per_meter = f.px_per_meter;
        let scale_denom = f.scale_denom;
        let scx = f.scx;
        let scy = f.scy;
        let ink = f.ink;
        let sheet_p = f.sheet_p.clone();
        let map_p = f.map_p.clone();
        // (g) 도곽 좌표격자 및 TM 좌표 주기 (C++ LayoutService::applySurveyFrameGrid)
        if self.drawing.sheet.grid_enabled || self.drawing.sheet.grid_follow_map {
            let step_m = if self.drawing.sheet.grid_follow_map {
                20.0
            } else {
                nice_grid_interval_meters(scale_denom, self.drawing.sheet.map_frame.width_mm)
            };
            let step_px = (step_m * px_per_meter) as f32;
            if step_px >= 18.0 {
                let min_wx = scx - f64::from(map_rect.width() * 0.5) / px_per_meter.max(1e-6);
                let max_wx = scx + f64::from(map_rect.width() * 0.5) / px_per_meter.max(1e-6);
                let min_wy = scy - f64::from(map_rect.height() * 0.5) / px_per_meter.max(1e-6);
                let max_wy = scy + f64::from(map_rect.height() * 0.5) / px_per_meter.max(1e-6);
                let first_gx = (min_wx / step_m).ceil() as i64;
                let last_gx = (max_wx / step_m).floor() as i64;
                let first_gy = (min_wy / step_m).ceil() as i64;
                let last_gy = (max_wy / step_m).floor() as i64;
                let grid_stroke =
                    Stroke::new(0.5, Color32::from_rgba_unmultiplied(17, 24, 39, 95));

                for ix in (first_gx..=last_gx).take(24) {
                    let wx = (ix as f64) * step_m;
                    let sx = f.w2m([wx, scy]).x;
                    if sx > map_rect.left() + 4.0 && sx < map_rect.right() - 4.0 {
                        if self.drawing.sheet.grid_follow_map {
                            map_p.line_segment(
                                [
                                    Pos2::new(sx, map_rect.top()),
                                    Pos2::new(sx, map_rect.bottom()),
                                ],
                                grid_stroke,
                            );
                        } else {
                            map_p.line_segment(
                                [
                                    Pos2::new(sx, map_rect.top()),
                                    Pos2::new(sx, map_rect.top() + 4.0),
                                ],
                                grid_stroke,
                            );
                            map_p.line_segment(
                                [
                                    Pos2::new(sx, map_rect.bottom() - 4.0),
                                    Pos2::new(sx, map_rect.bottom()),
                                ],
                                grid_stroke,
                            );
                        }
                        if step_px >= 42.0 {
                            sheet_p.text(
                                Pos2::new(sx, map_rect.top() - 5.0),
                                egui::Align2::CENTER_CENTER,
                                format!("{wx:.0}"),
                                egui::FontId::proportional(6.5),
                                Color32::from_rgb(31, 41, 55),
                            );
                        }
                    }
                }
                for iy in (first_gy..=last_gy).take(24) {
                    let wy = (iy as f64) * step_m;
                    let sy = f.w2m([scx, wy]).y;
                    if sy > map_rect.top() + 4.0 && sy < map_rect.bottom() - 4.0 {
                        if self.drawing.sheet.grid_follow_map {
                            map_p.line_segment(
                                [
                                    Pos2::new(map_rect.left(), sy),
                                    Pos2::new(map_rect.right(), sy),
                                ],
                                grid_stroke,
                            );
                        } else {
                            for ix in (first_gx..=last_gx).take(24) {
                                let sx = f.w2m([(ix as f64) * step_m, wy]).x;
                                if sx > map_rect.left() + 4.0 && sx < map_rect.right() - 4.0 {
                                    map_p.line_segment(
                                        [Pos2::new(sx - 2.5, sy), Pos2::new(sx + 2.5, sy)],
                                        grid_stroke,
                                    );
                                    map_p.line_segment(
                                        [Pos2::new(sx, sy - 2.5), Pos2::new(sx, sy + 2.5)],
                                        grid_stroke,
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
        sheet_p.rect_stroke(
            map_rect,
            0.0,
            Stroke::new(1.3, ink),
            egui::StrokeKind::Middle,
        );
    }
}
