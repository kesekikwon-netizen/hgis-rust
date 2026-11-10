//! 지도 캔버스의 배경지도 타일 그리기.
//!
//! C++ 은 QGIS 가 위성·일반도 타일을 그려 주지만 Rust 는 직접 받아 칠해야 한다.
//! gui_tab_map_canvas 가 300줄을 넘어 분리했다.

use eframe::egui;

use crate::app_state::StrataAppState;

impl StrataAppState {
    /// 배경지도 타일을 칠한다(위성·일반도). 지적 WMS 는 별도 경로다.
    pub(crate) fn paint_basemap_layer(
        &mut self,
        ctx: &egui::Context,
        p: &egui::Painter,
        rect: egui::Rect,
    ) {
        use strata_net::tile_fetcher::BasemapSource;
        let kind = match self.home_map.active_basemap {
            BasemapSource::VworldSatellite => crate::basemap_tiles::TileKind::Satellite,
            BasemapSource::VworldBase => crate::basemap_tiles::TileKind::Base,
            _ => return,
        };
        let mpp = self.home_map.viewport.meters_per_pixel.max(1e-6);
        let z = crate::basemap_tiles::basemap_zoom_for(mpp, rect);
        let view = crate::basemap_paint::TileView::from_viewport(
            &self.home_map.viewport,
            rect,
            self.work_crs,
        );
        let n = crate::basemap_paint::paint_basemap_tiles(
            ctx,
            p,
            rect,
            kind,
            z,
            &view,
            &mut self.basemap_textures,
        );
        self.basemap_tiles_painted = n;
    }
}
