//! Cursor snap preview helpers for `CadSessionController`
//! (wheel zoom lives in `strata-app::map_wheel_zoom`, `target/cpp-ref/src/core/LayerOps.h:44`).

use crate::cad_session::CadSessionController;
use crate::cad_snap_trace::nearby_vertices;
use crate::painter_scene::MapSceneModel;

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
}
