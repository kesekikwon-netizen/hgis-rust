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
        self.paint_terrain_overlay(ctx, p, rect);
    }

    /// 「지형」 단추로 켠 지형맵(OpenTopoMap)을 배경지도 위에 칠한다.
    /// C++ 은 참조 지도 그룹에 XYZ 레이어 「지형맵」을 올린다(zmax 17).
    fn paint_terrain_overlay(&mut self, ctx: &egui::Context, p: &egui::Painter, rect: egui::Rect) {
        let rs = &self.home_map.real_survey;
        if !rs.layer_visible.contains_key("지형맵") || !rs.is_layer_visible("지형맵") {
            return;
        }
        let mpp = self.home_map.viewport.meters_per_pixel.max(1e-6);
        let z = crate::basemap_tiles::basemap_zoom_for(mpp, rect)
            .min(crate::basemap_tiles::TERRAIN_MAX_ZOOM);
        let view = crate::basemap_paint::TileView::from_viewport(
            &self.home_map.viewport,
            rect,
            self.work_crs,
        );
        let _ = crate::basemap_paint::paint_basemap_tiles(
            ctx,
            p,
            rect,
            crate::basemap_tiles::TileKind::Terrain,
            z,
            &view,
            &mut self.basemap_textures,
        );
    }

    /// 연속지적도(WMS)를 지도 위에 겹쳐 칠한다.
    ///
    /// C++ `addVworldCadastralMap` 은 위성 위에 지적 PNG 를 반투명으로 얹는다.
    /// 지적 그림은 스냅 대상이 아니므로 그리기만 한다.
    pub(crate) fn paint_cadastral_layer(
        &mut self,
        ctx: &egui::Context,
        p: &egui::Painter,
        rect: egui::Rect,
    ) {
        if !self
            .home_map
            .real_survey
            .is_layer_visible("연속지적도 (VWorld WMS)")
        {
            return;
        }
        // C++ `BasemapOps.cpp:712,969` — 1:80,000 보다 넓게 보면 지적선을 그리지 않는다.
        // 넓게 보면 필지선이 뭉개져 오히려 방해가 되고, 타일 요청도 폭증한다.
        let scale_denom = self.home_map.viewport.meters_per_pixel * 3780.0;
        if !crate::basemap_cadastral::cadastral_visible_at_scale(scale_denom) {
            return;
        }
        let key = crate::basemap_fetch::current_key();
        if key.trim().is_empty() {
            return;
        }
        // 화면 영역을 EPSG:3857 미터 경계로 바꾼다.
        let Some(bbox) = crate::basemap_coords::screen_bbox_3857(
            &self.home_map.viewport,
            rect,
            self.work_crs,
        ) else {
            return;
        };
        // 위성과 같은 방식으로 **격자 칸 단위**로 받아 칠한다.
        // 화면 전체를 한 장으로 받으면 1px 만 움직여도 새로 받아 화면이 비어 버린다.
        let (x0, y0, x1, y1) = crate::basemap_cadastral::cadastral_grid_range(bbox);
        let count = (x1 - x0 + 1) * (y1 - y0 + 1);
        if count <= 0 || count > 36 {
            return; // 너무 넓게 보면 지적선을 생략한다(성능 보호)
        }
        let tint = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 190);
        for cy in y0..=y1 {
            for cx in x0..=x1 {
                let cell = [cx, cy];
                let handle = if let Some(h) = self.cadastral_cells.get(&cell) {
                    h.clone()
                } else {
                    // 지적 그림 칸도 뒤에서 받는다(그리는 동안 기다리지 않는다).
                    let key_owned = key.clone();
                    let state = crate::tile_fetch_queue::request_tile(
                        ctx,
                        format!("cadastral/{cx}/{cy}"),
                        || crate::basemap_cadastral::cached_cadastral_cell(cx, cy),
                        move || crate::basemap_cadastral::fetch_cadastral_cell(&key_owned, cx, cy),
                    );
                    let crate::tile_fetch_queue::TileState::Ready(path) = state else {
                        continue;
                    };
                    let name = format!("cad_{cx}_{cy}");
                    let Some(tex) =
                        crate::basemap_coords::load_texture_from_path(ctx, &name, &path)
                    else {
                        continue;
                    };
                    self.cadastral_cells.insert(cell, tex.clone());
                    tex
                };
                let cell_bbox = crate::basemap_cadastral::cadastral_cell_bbox(cx, cy);
                let Some(draw_rect) = crate::basemap_coords::bbox_3857_to_screen(
                    &self.home_map.viewport,
                    rect,
                    self.work_crs,
                    cell_bbox,
                ) else {
                    continue;
                };
                if !draw_rect.intersect(rect).is_positive() {
                    continue;
                }
                p.image(
                    handle.id(),
                    draw_rect,
                    egui::Rect::from_min_max(egui::Pos2::new(0.0, 0.0), egui::Pos2::new(1.0, 1.0)),
                    tint,
                );
            }
        }
    }
}
