//! 한반도 전체 보기 — C++ `LayerOps::zoomToKorea`(`BasemapOps.cpp:2559-2591`)와 그 범위
//! `satelliteFillExtentForCrs`(:2426-2457)·`extentFittedInside`(:2481-2493).
//!
//! C++ 은 조사 데이터만 연 뒤 applyStartupMap -> ensureStartupViewReady
//! (`MainWindow.cpp:1378-1396`, :1493-1508)가 지도 칸 크기가 정해졌을 때 한 번 이 범위로 맞춘다.
//! 그 뒤 clampCanvasToKorea 는 이 범위 안쪽이라 바꾸는 것이 없다.

use strata_geo::crs_transform::{lonlat_to_tm, TmParams};
use strata_types::crs::{CrsCode, WorkCrs};

use super::tabs_home_map_state::HomeMapTabState;

/// 한반도 경위도 범위(124.5~132°E, 33~39.5°N)의 네 모서리를 작업 좌표계로 옮겨 그 안쪽 직사각형
/// [xmin, ymin, xmax, ymax] 를 구한다. 경위도 직사각형은 웹 메르카토르에서도 직사각형이므로
/// C++ 의 4326 -> 3857 -> 작업 좌표계 경로와 같은 모서리다.
fn korea_fill_extent(work_crs: WorkCrs) -> Option<[f64; 4]> {
    let code = match work_crs {
        WorkCrs::Epsg5186 => CrsCode::Epsg5186,
        WorkCrs::Epsg5187 => CrsCode::Epsg5187,
    };
    let p = TmParams::for_crs(code)?;
    let sw = lonlat_to_tm(124.5, 33.0, &p);
    let se = lonlat_to_tm(132.0, 33.0, &p);
    let ne = lonlat_to_tm(132.0, 39.5, &p);
    let nw = lonlat_to_tm(124.5, 39.5, &p);
    let ext = [sw[0].max(nw[0]), sw[1].max(se[1]), se[0].min(ne[0]), nw[1].min(ne[1])];
    (ext[2] > ext[0] && ext[3] > ext[1]).then_some(ext)
}

/// C++ extentFittedInside — 화면 가로세로 비에 맞춰 범위 안쪽으로 줄인다.
fn fitted_inside(ext: [f64; 4], view_aspect: f64) -> [f64; 4] {
    let (w, h) = (ext[2] - ext[0], ext[3] - ext[1]);
    if view_aspect <= 0.05 {
        return ext;
    }
    if view_aspect > w / h {
        let half = w / view_aspect * 0.5;
        let cy = (ext[1] + ext[3]) * 0.5;
        [ext[0], cy - half, ext[2], cy + half]
    } else {
        let half = h * view_aspect * 0.5;
        let cx = (ext[0] + ext[2]) * 0.5;
        [cx - half, ext[1], cx + half, ext[3]]
    }
}

impl HomeMapTabState {
    /// 지금 지도 칸 크기로 한반도 전체를 보인다(C++ zoomToKorea + canvasViewAspect).
    pub fn zoom_to_korea(&mut self, work_crs: WorkCrs) {
        let Some(ext) = korea_fill_extent(work_crs) else {
            return;
        };
        let [sw, sh] = self.viewport.screen_size_px;
        let aspect = if sw >= 2.0 && sh >= 2.0 { sw / sh } else { 1.0 };
        let f = fitted_inside(ext, aspect);
        self.viewport.center_xy = [(f[0] + f[2]) * 0.5, (f[1] + f[3]) * 0.5];
        if sw >= 2.0 {
            self.viewport.meters_per_pixel = (f[2] - f[0]) / sw;
        }
    }

    /// 지도 칸 크기가 정해진 뒤 한 번만 다시 맞춘다(C++ ensureStartupViewReady 의 m_startupViewApplied).
    pub fn apply_pending_korea_view(&mut self, work_crs: WorkCrs) {
        if self.korea_view_pending {
            self.korea_view_pending = false;
            self.zoom_to_korea(work_crs);
        }
    }
}
