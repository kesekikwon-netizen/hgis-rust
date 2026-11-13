//! 지도 위 투영 격자를 그린다 — C++ `KaShellGridControls` / `LayoutService::applySurveyFrameGrid`.

use eframe::egui::{self, FontId, Pos2, Rect, Stroke};

use strata_canvas2d::viewport::ViewportTransform;

use super::gui_tab_map_consts::{ShellGridConfig, SHELL_GRID_SWATCHES};
use super::gui_tab_map_format::{
    grid_band_rects, GRID_BAND_BORDER, GRID_BAND_FILL, GRID_BAND_INK, GRID_BAND_PX,
};
use super::gui_tab_map_geom::{
    grid_dash_segments, grid_label_text, grid_line_stride,
    grid_stroke_for, nice_grid_step_meters, s2w, w2s,
};
pub fn paint_projected_grid(
    p: &egui::Painter,
    rect: Rect,
    vp: &ViewportTransform,
    cfg: &ShellGridConfig,
    origin: [f64; 2],
    is_geo: bool,
    labels: bool,
) {
    let ink = SHELL_GRID_SWATCHES[cfg.color_idx.min(2)].0;
    let stroke = grid_stroke_for(cfg, ink);
    let world_tl = s2w(rect, vp, rect.left_top());
    let world_br = s2w(rect, vp, rect.right_bottom());
    let min_wx = world_tl[0].min(world_br[0]);
    let max_wx = world_tl[0].max(world_br[0]);
    let min_wy = world_tl[1].min(world_br[1]);
    let max_wy = world_tl[1].max(world_br[1]);
    let turned = cfg.rotation_deg.abs() > 1e-3;
    let step = if cfg.step_m > 0.0 {
        cfg.step_m
    } else {
        nice_grid_step_meters(120.0 * vp.meters_per_pixel.max(1e-9))
    };
    if step.is_nan() || step <= 0.0 {
        return;
    }
    let span = (max_wx - min_wx).max(max_wy - min_wy);
    let stride = grid_line_stride(span, step, 80);
    let draw_step = step * stride as f64;
    let start_x = (min_wx / draw_step).floor() * draw_step;
    let start_y = (min_wy / draw_step).floor() * draw_step;
    let [origin_x, origin_y] = origin;

    let mut gx = start_x;
    while gx <= max_wx + draw_step * 0.5 {
        let sx = w2s(rect, vp, [gx, min_wy]).x;
        if sx >= rect.left() && sx <= rect.right() {
            for seg in grid_dash_segments(
                Pos2::new(sx, rect.top() + GRID_BAND_PX),
                Pos2::new(sx, rect.bottom()),
                cfg.dash_idx,
                stroke.width,
            ) {
                p.line_segment(seg, stroke);
            }
        }
        gx += draw_step;
    }
    let mut gy = start_y;
    while gy <= max_wy + draw_step * 0.5 {
        let sy = w2s(rect, vp, [min_wx, gy]).y;
        if sy >= rect.top() && sy <= rect.bottom() {
            for seg in grid_dash_segments(
                Pos2::new(rect.left() + GRID_BAND_PX, sy),
                Pos2::new(rect.right(), sy),
                cfg.dash_idx,
                stroke.width,
            ) {
                p.line_segment(seg, stroke);
            }
        }
        gy += draw_step;
    }
    if !labels {
        return;
    }
    if is_geo {
        return;
    }

    let (top_band, left_band) = grid_band_rects(rect);
    p.rect_filled(top_band, 0.0, GRID_BAND_FILL);
    p.rect_filled(left_band, 0.0, GRID_BAND_FILL);
    p.line_segment(
        [
            Pos2::new(rect.left() + GRID_BAND_PX, rect.top() + GRID_BAND_PX),
            Pos2::new(rect.right(), rect.top() + GRID_BAND_PX),
        ],
        Stroke::new(1.0, GRID_BAND_BORDER),
    );
    p.line_segment(
        [
            Pos2::new(rect.left() + GRID_BAND_PX, rect.top() + GRID_BAND_PX),
            Pos2::new(rect.left() + GRID_BAND_PX, rect.bottom()),
        ],
        Stroke::new(1.0, GRID_BAND_BORDER),
    );

    let band_font = FontId::monospace(10.0);
    let mut last_x = f32::NEG_INFINITY;
    let mut gx = start_x;
    while gx <= max_wx + draw_step * 0.5 {
        let sx = w2s(rect, vp, [gx, max_wy]).x;
        let lbl = grid_label_text(gx - origin_x, origin_x, draw_step, turned);
        if sx >= rect.left() + GRID_BAND_PX && sx <= rect.right() {
            let need = lbl.chars().count() as f32 * 6.0 + 8.0;
            if sx - last_x >= need {
                p.text(
                    Pos2::new(sx + 2.0, rect.top() + 9.0),
                    egui::Align2::LEFT_CENTER,
                    lbl,
                    band_font.clone(),
                    GRID_BAND_INK,
                );
                last_x = sx;
            }
        }
        gx += draw_step;
    }
    let mut last_y = f32::NEG_INFINITY;
    let mut gy = start_y;
    while gy <= max_wy + draw_step * 0.5 {
        let sy = w2s(rect, vp, [min_wx, gy]).y;
        let lbl = grid_label_text(gy - origin_y, origin_y, draw_step, turned);
        if sy >= rect.top() + GRID_BAND_PX && sy <= rect.bottom() {
            let need = lbl.chars().count() as f32 * 6.0 + 8.0;
            if sy - last_y >= need {
                p.text(
                    Pos2::new(rect.left() + GRID_BAND_PX / 2.0, sy),
                    egui::Align2::CENTER_CENTER,
                    lbl,
                    band_font.clone(),
                    GRID_BAND_INK,
                );
                last_y = sy;
            }
        }
        gy += draw_step;
    }
}
