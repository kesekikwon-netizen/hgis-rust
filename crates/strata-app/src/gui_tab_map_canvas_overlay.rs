//! 지도 위 보조 겹침 — 좌표격자·도곽 좌표 띠, 수치지형도, GPS 기준점, 러버밴드.
//!
//! `gui_tab_map_heritage` 에서 300줄을 지키려고 나눴다. 원래 `paint_gyeongju_heritage_canvas`
//! 끝부분의 연속 블록이고, 본문은 그대로다.

use eframe::egui::{self, Color32, FontId, Pos2, Rect, Stroke};
use strata_canvas2d::viewport::ViewportTransform;

use crate::app_state::StrataAppState;

use super::gui_tab_map_consts::ShellGridConfig;
use super::gui_tab_map_geom::w2s;
use super::gui_tab_map_grid::paint_projected_grid;

/// paint_heritage_overlays 가 쓰는 값 묶음 — 인자가 8개가 되어 clippy
///     oo_many_arguments 에 걸리므로 넘긴다.
pub(crate) struct HeritageOverlayCtx<'a> {
    pub(crate) ctx: &'a egui::Context,
    pub(crate) p: &'a egui::Painter,
    pub(crate) rect: Rect,
    pub(crate) vp: &'a ViewportTransform,
    pub(crate) rs: &'a crate::real_survey::RealSurveyData,
    pub(crate) center: [f64; 2],
    pub(crate) layer_color: &'a dyn Fn(&str, Color32) -> Color32,
}

impl StrataAppState {
    /// 격자·지형도·기준점·러버밴드 겹침 (원문 캔버스 본문 뒤쪽).
    pub(crate) fn paint_heritage_overlays(&self, g: HeritageOverlayCtx<'_>) {
        let HeritageOverlayCtx {
            ctx,
            p,
            rect,
            vp,
            rs,
            center: [cx, cy],
            layer_color,
        } = g;
        // KaCanvasGridOverlay.cpp:101-194 — 좌표격자가 켜져 있으면 지도 위에 격자와 도곽 좌표 띠를 그린다
        if self.home_map.grid_enabled {
            let cfg = ctx.data_mut(|d| {
                d.get_temp::<ShellGridConfig>(egui::Id::new("ka_shell_grid_cfg"))
                    .unwrap_or_default()
            });
            let labels = cfg.show_labels;
            paint_projected_grid(p, rect, vp, &cfg, rs.survey_center, cfg.is_geo, labels);
        }

        // T9 KaTopographicImportDialog에서 지도에 추가한 수치지형도 레이어 렌더링 (styleTopographic: 회색 0.2mm 선)
        if self.modals.topo_maps_enabled && self.modals.topo_imported_count > 0 {
            let topo_col = layer_color(
                "수치지형도 1:5,000 (합침)",
                Color32::from_rgba_unmultiplied(115, 115, 115, 190),
            );
            for idx in 0..self.modals.topo_imported_count.min(6) {
                let offset = (idx as f64 - 2.5) * 140.0;
                let p1 = w2s(rect, vp, [cx - 880.0, cy + offset - 90.0]);
                let p2 = w2s(rect, vp, [cx + 880.0, cy + offset + 90.0]);
                p.line_segment([p1, p2], Stroke::new(1.0, topo_col));
            }
        }

        // T6 (1) KaControlPointDialog에서 추가한 GPS 기준점 심볼·라벨 렌더링
        for (cp_id, cp_xy, cp_z) in &self.modals.control_points_world {
            let sp = w2s(rect, vp, *cp_xy);
            if rect.contains(sp) {
                let tri = vec![
                    Pos2::new(sp.x, sp.y - 7.0),
                    Pos2::new(sp.x - 6.5, sp.y + 5.0),
                    Pos2::new(sp.x + 6.5, sp.y + 5.0),
                ];
                p.add(egui::Shape::convex_polygon(
                    tri,
                    Color32::from_rgb(220, 38, 38),
                    Stroke::new(1.5, Color32::WHITE),
                ));
                let lbl = match cp_z {
                    Some(z) => format!("{cp_id} ({z:.2}m)"),
                    None => cp_id.clone(),
                };
                p.text(
                    Pos2::new(sp.x + 9.0, sp.y),
                    egui::Align2::LEFT_CENTER,
                    lbl,
                    FontId::proportional(10.5),
                    Color32::from_rgb(185, 28, 28),
                );
            }
        }

        if self.home_map.cad.snap_enabled && self.home_map.cad.snap_preview_enabled {
            for &pt in &self.home_map.cad.preview_markers_world {
                let sp = w2s(rect, vp, pt);
                p.circle_filled(sp, 4.5, Color32::from_rgba_unmultiplied(255, 251, 235, 210));
                p.circle_stroke(
                    sp,
                    4.5,
                    Stroke::new(2.0, Color32::from_rgba_unmultiplied(217, 119, 6, 230)),
                );
            }
        }

        if !self.home_map.scene.active_rubberband.is_empty() {
            let mut active_pts: Vec<Pos2> = self
                .home_map
                .scene
                .active_rubberband
                .iter()
                .map(|&pt| w2s(rect, vp, pt))
                .collect();
            for &v in &active_pts {
                p.circle_filled(v, 4.0, Color32::from_rgb(214, 78, 22));
            }
            active_pts.push(w2s(rect, vp, self.home_map.cursor_world_xy));
            p.add(egui::Shape::line(
                active_pts,
                Stroke::new(2.2, Color32::from_rgb(214, 78, 22)),
            ));
        }
    }
}
