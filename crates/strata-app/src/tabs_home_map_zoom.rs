//! 지도 범위 옮기기 — QGIS `QgsMapCanvas::setExtent`·`zoomToFeatureExtent`(qgsmapcanvas.cpp :2040-2062)와
//! C++ `LayerOps::zoomToLayerMax`(`BasemapOps.cpp:580-651`)·`zoomCanvasToWorkingScale`(:179-191).
//!
//! 축척은 상태줄과 같은 96 dpi(`app_state_status_bar.rs`) — 1:N 의 한 점은 N × 0.0254 / 96 m.

use strata_types::crs::WorkCrs;

use super::tabs_home_map_korea::korea_outer_extent;
use super::tabs_home_map_state::HomeMapTabState;

const METERS_PER_PIXEL_AT_1: f64 = 0.0254 / 96.0;

/// QgsRectangle::scale — 가운데를 두고 가로세로를 `f` 배.
fn scaled(r: [f64; 4], f: f64) -> [f64; 4] {
    let (cx, cy) = ((r[0] + r[2]) * 0.5, (r[1] + r[3]) * 0.5);
    let (hw, hh) = ((r[2] - r[0]) * 0.5 * f, (r[3] - r[1]) * 0.5 * f);
    [cx - hw, cy - hh, cx + hw, cy + hh]
}

impl HomeMapTabState {
    /// QGIS `setExtent` — 범위를 맞추고 이력에 쌓는다(이력이 시작된 뒤에만, `extent_history.rs`).
    pub(crate) fn set_view_extent(&mut self, r: [f64; 4]) {
        self.show_history_extent(r);
        self.note_view();
    }

    /// 지금 보기를 이력에 — QGIS setExtent 의 뒷부분.
    pub(crate) fn note_view(&mut self) {
        let now = self.viewport.world_bbox();
        self.extent_history.note_extent(now);
    }

    /// 이력의 범위로 옮긴다(QGIS zoomToPreviousExtent·zoomToNextExtent 의 mSettings.setExtent —
    /// 이력에 쌓지 않는다). 범위 전체가 지도 칸에 들어오게, 가운데는 범위 가운데. 비어 있으면 가운데만.
    pub(crate) fn show_history_extent(&mut self, r: [f64; 4]) {
        let [sw, sh] = self.viewport.screen_size_px;
        let (w, h) = (r[2] - r[0], r[3] - r[1]);
        self.viewport.center_xy = [(r[0] + r[2]) * 0.5, (r[1] + r[3]) * 0.5];
        if w > 0.0 && h > 0.0 && sw >= 1.0 && sh >= 1.0 {
            self.viewport.meters_per_pixel = (w / sw).max(h / sh);
        }
    }

    /// C++ `zoomCanvasToWorkingScale` — 지금 보기가 동네 축척(1:50 ~ 1:500,000)이 아니면 한반도 전체에서,
    /// 가운데를 두고 1:`scale`, 그 뒤 한반도 안으로.
    fn zoom_to_working_scale(&mut self, work_crs: WorkCrs, scale: f64) {
        let current = self.viewport.meters_per_pixel / METERS_PER_PIXEL_AT_1;
        let local = self.viewport.meters_per_pixel.is_finite() && current > 50.0 && current < 500_000.0;
        if !local {
            self.zoom_to_korea(work_crs);
            self.note_view();
        }
        self.viewport.meters_per_pixel = scale * METERS_PER_PIXEL_AT_1;
        self.note_view();
        self.clamp_to_korea(work_crs);
    }

    /// The second half of C++ `zoomToProjectDataLayers` (`BasemapOps.cpp:663-715`): the common box of the
    /// layers' boxes (a layer as wide as the country says nothing about where the survey is, `:680-682`),
    /// at least 100 m each way, with a fifth of room; a view smaller than 1:80,000 becomes 1:5,000 around
    /// the same middle. `false`, and the view as it was, without a box.
    pub(crate) fn zoom_to_data_extents(&mut self, layers: impl Iterator<Item = [f64; 4]>, work_crs: WorkCrs) -> bool {
        const MIN_W: f64 = 100.0;
        let korea = korea_outer_extent(work_crs);
        let country_wide = |e: &[f64; 4]| korea.is_some_and(|k| e[2] - e[0] > (k[2] - k[0]) * 0.95 || e[3] - e[1] > (k[3] - k[1]) * 0.95);
        let boxes = layers.filter(|e| !country_wide(e));
        let Some(mut e) = boxes.reduce(|a, b| [a[0].min(b[0]), a[1].min(b[1]), a[2].max(b[2]), a[3].max(b[3])]) else {
            return false;
        };
        // Only a short side is widened. The C++ puts a square of 100 m in place of the whole box when one
        // side is short (`:701-706`), which takes a long and narrow survey out of the view.
        let (cx, cy) = ((e[0] + e[2]) * 0.5, (e[1] + e[3]) * 0.5);
        let (hw, hh) = ((e[2] - e[0]).max(MIN_W) * 0.5, (e[3] - e[1]).max(MIN_W) * 0.5);
        e = scaled([cx - hw, cy - hh, cx + hw, cy + hh], 1.2);
        self.set_view_extent(e);
        self.set_view_extent(scaled(e, 1.05));
        self.clamp_to_korea(work_crs);
        if self.viewport.meters_per_pixel / METERS_PER_PIXEL_AT_1 > 80_000.0 {
            self.viewport.meters_per_pixel = 5_000.0 * METERS_PER_PIXEL_AT_1;
            self.note_view();
        }
        true
    }

    /// C++ `LayerOps::zoomToLayerMax` — `ext` 는 작업 좌표계로 옮긴 레이어 범위(없으면 None),
    /// `is_raster` 는 래스터(배경지도·지적 그림) 레이어. 옮기지 못하면 false.
    pub(crate) fn zoom_to_layer_max(&mut self, ext: Option<[f64; 4]>, is_raster: bool, work_crs: WorkCrs) -> bool {
        let kr = korea_outer_extent(work_crs);
        let too_big = |e: [f64; 4]| {
            kr.is_some_and(|k| e[2] - e[0] > (k[2] - k[0]) * 1.2 || e[3] - e[1] > (k[3] - k[1]) * 1.2)
        };
        let usable = ext.filter(|e| e.iter().all(|v| v.is_finite()) && e[2] >= e[0] && e[3] >= e[1]);
        // :612-619 — 세계 전체 같은 래스터(XYZ·WMS)는 작업 축척 1:50,000 로.
        if is_raster && usable.is_none_or(too_big) {
            self.zoom_to_working_scale(work_crs, 50_000.0);
            return true;
        }
        let Some(mut e) = usable else {
            return false;
        };
        if too_big(e) {
            self.zoom_to_working_scale(work_crs, 50_000.0);
            return true;
        }
        // :630-641 — 좁은 유적·단면은 짧은 변만 80 m 로 채운다.
        const MIN_W: f64 = 80.0;
        if e[2] - e[0] < MIN_W || e[3] - e[1] < MIN_W {
            let (cx, cy) = ((e[0] + e[2]) * 0.5, (e[1] + e[3]) * 0.5);
            let (hw, hh) = ((e[2] - e[0]).max(MIN_W) * 0.5, (e[3] - e[1]).max(MIN_W) * 0.5);
            e = [cx - hw, cy - hh, cx + hw, cy + hh];
        }
        e = scaled(e, 1.15);
        self.set_view_extent(e);
        // QGIS zoomToFeatureExtent — 같은 범위를 5 % 더 넓혀 다시.
        self.set_view_extent(scaled(e, 1.05));
        let after = self.viewport.world_bbox();
        if kr.is_some_and(|k| after[2] - after[0] > (k[2] - k[0]) * 1.12 || after[3] - after[1] > (k[3] - k[1]) * 1.12) {
            self.clamp_to_korea(work_crs);
        }
        true
    }
}

#[cfg(test)]
#[path = "tabs_home_map_zoom_tests.rs"]
mod tests;
