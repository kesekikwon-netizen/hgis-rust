use crate::canvas_painter::{MapSceneModel, WorldPolygonItem};
use crate::map_tools::MapToolKind;
use crate::viewport::ViewportTransform;

#[derive(Debug, Clone, PartialEq)]
pub struct MeasureReadout {
    pub total_distance_m: f64,
    pub polygon_area_m2: f64,
}

#[derive(Debug, Clone, Default)]
pub struct CadSessionController {
    pub last_measure: Option<MeasureReadout>,
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
        let snapped = Self::snap_world_point(world_xy, scene, 2.5);
        scene.snap_marker_world = Some(snapped);
        match tool {
            MapToolKind::DrawPolygon | MapToolKind::DrawLine => {
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

    pub fn on_right_click(&mut self, tool: MapToolKind, scene: &mut MapSceneModel) -> bool {
        if tool == MapToolKind::DrawPolygon && scene.active_rubberband.len() >= 3 {
            let pts: Vec<[f64; 2]> = scene.active_rubberband.drain(..).collect();
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

    pub fn on_wheel_zoom(
        &mut self,
        scroll_delta_y: f32,
        cursor_px: [f64; 2],
        vp: &mut ViewportTransform,
    ) {
        if scroll_delta_y.abs() < 0.01 {
            return;
        }
        let factor = if scroll_delta_y > 0.0 { 0.85 } else { 1.18 };
        vp.zoom_at(factor, cursor_px);
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
        if pts.len() >= 3 {
            let mut shoelace = 0.0;
            for i in 0..pts.len() {
                let j = (i + 1) % pts.len();
                shoelace += pts[i][0] * pts[j][1] - pts[j][0] * pts[i][1];
            }
            polygon_area_m2 = 0.5 * shoelace.abs();
        }
        self.last_measure = Some(MeasureReadout {
            total_distance_m,
            polygon_area_m2,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cad_left_clicks_and_right_click_commit_new_polygon_and_compute_measure() {
        let mut cad = CadSessionController::default();
        let mut scene = MapSceneModel::default();

        cad.on_left_click([100.0, 100.0], MapToolKind::DrawPolygon, &mut scene);
        cad.on_left_click([200.0, 100.0], MapToolKind::DrawPolygon, &mut scene);
        cad.on_left_click([200.0, 200.0], MapToolKind::DrawPolygon, &mut scene);
        cad.on_left_click([100.0, 200.0], MapToolKind::DrawPolygon, &mut scene);
        assert_eq!(scene.active_rubberband.len(), 4);

        let committed = cad.on_right_click(MapToolKind::DrawPolygon, &mut scene);
        assert!(committed);
        assert!(scene.active_rubberband.is_empty());
        assert_eq!(scene.polygons.len(), 1);
        assert_eq!(scene.buffer_500m_ring.len(), 37);

        cad.on_left_click([0.0, 0.0], MapToolKind::Measure, &mut scene);
        cad.on_left_click([100.0, 0.0], MapToolKind::Measure, &mut scene);
        cad.on_left_click([100.0, 100.0], MapToolKind::Measure, &mut scene);
        cad.on_left_click([0.0, 100.0], MapToolKind::Measure, &mut scene);
        let m = cad.last_measure.as_ref();
        assert!(m.is_some_and(|r| (r.total_distance_m - 300.0).abs() < 1e-4));
        assert!(m.is_some_and(|r| (r.polygon_area_m2 - 10_000.0).abs() < 1e-2));
    }
}
