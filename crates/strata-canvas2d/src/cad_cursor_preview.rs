//! CAD 세션의 커서 미리보기와 휠 줌.
//!
//! `impl CadSessionController` 가 266줄이라 여기로 나눴다.

use crate::cad_session::CadSessionController;
use crate::cad_snap_trace::nearby_vertices;
use crate::painter_scene::MapSceneModel;
use crate::viewport::ViewportTransform;

impl CadSessionController {
    #[must_use]
    pub fn snap_world_point_with_refs(
        world_xy: [f64; 2],
        scene: &MapSceneModel,
        extra_rings: &[Vec<[f64; 2]>],
        tol_m: f64,
    ) -> [f64; 2] {
        Self::snap_world_point_with_rings(world_xy, scene, extra_rings, tol_m)
    }

    pub fn update_cursor_preview(
        &mut self,
        cursor_xy: [f64; 2],
        scene: &MapSceneModel,
        extra_rings: &[Vec<[f64; 2]>],
        mupp: f64,
    ) {
        if !self.snap_enabled || !self.snap_preview_enabled {
            self.preview_markers_world.clear();
            return;
        }
        let mut all_rings: Vec<Vec<[f64; 2]>> =
            scene.polygons.iter().map(|p| p.world_pts.clone()).collect();
        all_rings.extend_from_slice(extra_rings);
        self.preview_markers_world = nearby_vertices(
            cursor_xy,
            scene.snap_marker_world,
            &all_rings,
            mupp,
            48.0,
            24,
        );
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
}
