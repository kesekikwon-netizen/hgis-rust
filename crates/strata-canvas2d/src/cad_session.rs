//! CAD 세션 조작(`CadSessionController`) — 클릭·스냅·미리보기.
//!
//! `impl CadSessionController` 가 266줄이라 두 갈래로 나눴다.
//! 미리보기·줌은 `cad_cursor_preview` 에 있다.

use crate::canvas_painter::{MapSceneModel, WorldPolygonItem};
use crate::map_tools::MapToolKind;

use crate::cad_measure::MeasureReadout;
use crate::cad_snap_trace::trace_between;

#[derive(Debug, Clone)]
pub struct CadSessionController {
    pub last_measure: Option<MeasureReadout>,
    pub snap_enabled: bool,
    pub snap_trace_enabled: bool,
    pub snap_preview_enabled: bool,
    pub snap_tolerance_px: f64,
    /// `KaSnapSettingsWidget.cpp:28-30` — 켜면 맞닿은 유구의 같은 꼭짓점을 같이 옮긴다
    pub snap_topological: bool,
    /// `KaSnapSettingsWidget.cpp:24-27` — `false` = 「모든 조사 레이어」, `true` = 「현재 레이어」
    pub snap_target_current_only: bool,
    pub preview_markers_world: Vec<[f64; 2]>,
}

impl Default for CadSessionController {
    fn default() -> Self {
        Self {
            last_measure: None,
            snap_enabled: true,
            snap_trace_enabled: true,
            snap_preview_enabled: true,
            snap_tolerance_px: 12.0,
            snap_topological: true,
            snap_target_current_only: false,
            preview_markers_world: Vec::new(),
        }
    }
}

impl CadSessionController {

    #[must_use]
    pub fn snap_world_point(world_xy: [f64; 2], scene: &MapSceneModel, tol_m: f64) -> [f64; 2] {
        let mut best = world_xy;
        let mut best_d = tol_m;
        for item in &scene.polygons {
            for &pt in &item.world_pts {
                let d = (pt[0] - world_xy[0]).hypot(pt[1] - world_xy[1]);
                if d < best_d {
                    best_d = d;
                    best = pt;
                }
            }
        }
        best
    }

    pub fn on_left_click(
        &mut self,
        world_xy: [f64; 2],
        tool: MapToolKind,
        scene: &mut MapSceneModel,
    ) {
        self.on_left_click_with_refs(world_xy, tool, scene, &[], 0.5);
    }

    /// `KaSnapSettingsWidget.cpp:24-27` — 「현재 레이어」일 때는 그 레이어 도형에만 붙는다.
    /// `current_only` 판단은 호출한 쪽의 레이어 고르기를 따른다(`snap_target_current_only`가 켜졌을 때만).
    #[must_use]
    fn snap_rings_for<'a>(
        &self,
        scene: &'a MapSceneModel,
        extra_rings: &'a [Vec<[f64; 2]>],
        current_layer_key: Option<&str>,
    ) -> Vec<Vec<[f64; 2]>> {
        let mut rings: Vec<Vec<[f64; 2]>> = scene
            .polygons
            .iter()
            .filter(|p| {
                !(self.snap_target_current_only
                    && current_layer_key.is_some_and(|k| p.layer_key != k))
            })
            .map(|p| p.world_pts.clone())
            .collect();
        rings.extend_from_slice(extra_rings);
        rings
    }

    pub fn on_left_click_with_refs(
        &mut self,
        world_xy: [f64; 2],
        tool: MapToolKind,
        scene: &mut MapSceneModel,
        extra_rings: &[Vec<[f64; 2]>],
        mupp: f64,
    ) {
        self.on_left_click_with_refs_and_layer(world_xy, tool, scene, extra_rings, mupp, None);
    }

    pub fn on_left_click_with_refs_and_layer(
        &mut self,
        world_xy: [f64; 2],
        tool: MapToolKind,
        scene: &mut MapSceneModel,
        extra_rings: &[Vec<[f64; 2]>],
        mupp: f64,
        current_layer_key: Option<&str>,
    ) {
        let tol_m = if self.snap_enabled {
            (self.snap_tolerance_px * mupp.max(0.05)).max(2.5)
        } else {
            0.0
        };
        let snap_rings = self.snap_rings_for(scene, extra_rings, current_layer_key);
        let snapped = if self.snap_enabled {
            Self::snap_world_point_with_rings(world_xy, scene, &snap_rings, tol_m)
        } else {
            world_xy
        };
        scene.snap_marker_world = Some(snapped);
        match tool {
            MapToolKind::DrawPolygon | MapToolKind::DrawLine => {
                if self.snap_enabled && self.snap_trace_enabled {
                    if let Some(&last_pt) = scene.active_rubberband.last() {
                        let mut all_rings: Vec<Vec<[f64; 2]>> =
                            scene.polygons.iter().map(|p| p.world_pts.clone()).collect();
                        all_rings.extend_from_slice(extra_rings);
                        let mids = trace_between(last_pt, snapped, &all_rings, mupp, 20);
                        scene.active_rubberband.extend(mids);
                    }
                }
                scene.active_rubberband.push(snapped);
            }
            MapToolKind::PlacePoint => {
                let s = 1.5;
                scene.polygons.push(WorldPolygonItem {
                    layer_key: "control_points".to_owned(),
                    world_pts: vec![
                        [snapped[0] - s, snapped[1] - s],
                        [snapped[0] + s, snapped[1] - s],
                        [snapped[0] + s, snapped[1] + s],
                        [snapped[0] - s, snapped[1] + s],
                    ],
                    fill_rgba: [220, 38, 38, 220],
                    stroke_rgba: [153, 27, 27, 255],
                });
            }
            MapToolKind::TrenchPlace => {
                scene.polygons.push(WorldPolygonItem {
                    layer_key: "trial_trench".to_owned(),
                    world_pts: vec![
                        [snapped[0] - 1.0, snapped[1] - 10.0],
                        [snapped[0] + 1.0, snapped[1] - 10.0],
                        [snapped[0] + 1.0, snapped[1] + 10.0],
                        [snapped[0] - 1.0, snapped[1] + 10.0],
                    ],
                    fill_rgba: [37, 99, 235, 95],
                    stroke_rgba: [29, 78, 216, 255],
                });
            }
            MapToolKind::Measure => {
                scene.active_rubberband.push(snapped);
                self.recompute_measure(&scene.active_rubberband);
            }
            _ => {}
        }
    }

    #[must_use]
    pub fn snap_world_point_with_rings(
        world_xy: [f64; 2],
        scene: &MapSceneModel,
        extra_rings: &[Vec<[f64; 2]>],
        tol_m: f64,
    ) -> [f64; 2] {
        let mut best = Self::snap_world_point(world_xy, scene, tol_m);
        let mut best_d = (best[0] - world_xy[0]).hypot(best[1] - world_xy[1]);
        if best == world_xy {
            best_d = tol_m;
        }
        for ring in extra_rings {
            for &pt in ring {
                let d = (pt[0] - world_xy[0]).hypot(pt[1] - world_xy[1]);
                if d < best_d {
                    best_d = d;
                    best = pt;
                }
            }
        }
        best
    }

    pub fn on_right_click(&mut self, tool: MapToolKind, scene: &mut MapSceneModel) -> bool {
        if tool == MapToolKind::DrawPolygon && scene.active_rubberband.len() >= 3 {
            let pts: Vec<[f64; 2]> = std::mem::take(&mut scene.active_rubberband);
            let n = pts.len() as f64;
            let cx = pts.iter().map(|p| p[0]).sum::<f64>() / n;
            let cy = pts.iter().map(|p| p[1]).sum::<f64>() / n;
            scene.polygons.push(WorldPolygonItem {
                layer_key: "survey_area".to_owned(),
                world_pts: pts,
                fill_rgba: [181, 87, 58, 52],
                stroke_rgba: [181, 87, 58, 240],
            });
            scene.buffer_500m_ring = (0..=36)
                .map(|i| {
                    let th = (i as f64 / 36.0) * std::f64::consts::TAU;
                    [cx + 500.0 * th.cos(), cy + 500.0 * th.sin()]
                })
                .collect();
            scene.buffer_1000m_ring = (0..=36)
                .map(|i| {
                    let th = (i as f64 / 36.0) * std::f64::consts::TAU;
                    [cx + 1000.0 * th.cos(), cy + 1000.0 * th.sin()]
                })
                .collect();
            return true;
        }
        scene.active_rubberband.clear();
        false
    }

    pub fn on_escape(&mut self, scene: &mut MapSceneModel) {
        scene.active_rubberband.clear();
        self.last_measure = None;
    }

    pub fn on_backspace(&mut self, scene: &mut MapSceneModel) {
        let _ = scene.active_rubberband.pop();
        self.recompute_measure(&scene.active_rubberband);
    }

    fn recompute_measure(&mut self, pts: &[[f64; 2]]) {
        if pts.len() < 2 {
            self.last_measure = None;
            return;
        }
        let mut total_distance_m = 0.0;
        for w in pts.windows(2) {
            total_distance_m += (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1]);
        }
        let mut polygon_area_m2 = 0.0;
        let mut polygon_perimeter_m = 0.0;
        if pts.len() >= 3 {
            let mut shoelace = 0.0;
            for i in 0..pts.len() {
                let j = (i + 1) % pts.len();
                shoelace += pts[i][0] * pts[j][1] - pts[j][0] * pts[i][1];
                polygon_perimeter_m += (pts[j][0] - pts[i][0]).hypot(pts[j][1] - pts[i][1]);
            }
            polygon_area_m2 = 0.5 * shoelace.abs();
        }
        self.last_measure = Some(MeasureReadout {
            total_distance_m,
            polygon_area_m2,
            polygon_perimeter_m,
        });
    }
}
