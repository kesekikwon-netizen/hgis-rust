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
}

impl StrataAppState {
    /// 격자·지형도·기준점·러버밴드 겹침 (원문 캔버스 본문 뒤쪽).
    pub(crate) fn paint_heritage_overlays(&self, g: HeritageOverlayCtx<'_>) {
        let HeritageOverlayCtx {
            ctx,
            p,
            rect,
            vp,
        } = g;
        // KaCanvasGridOverlay.cpp:101-194 — 좌표격자가 켜져 있으면 지도 위에 격자와 도곽 좌표 띠를 그린다
        if self.home_map.grid_enabled {
            let cfg = ctx.data_mut(|d| {
                d.get_temp::<ShellGridConfig>(egui::Id::new("ka_shell_grid_cfg"))
                    .unwrap_or_default()
            });
            let labels = cfg.show_labels;
            // 격자 원점은 C++ 기본 좌표계 원점(0, 0)(KaGridOriginButton.cpp:36) — 「원점」 단추는 아직 없다.
            paint_projected_grid(p, rect, vp, &cfg, [0.0, 0.0], cfg.is_geo, labels);
        }

        // F1: 「수치지형도 (합침)」을 넣으면 경주 기본점 둘레에 지어낸 회색 선 여섯 줄을 긋던 것을 없앴다 —
        // 받은 도엽을 제자리에 그리는 일은 따로(C++ styleTopographic 은 실제 레이어).

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
