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
        // 워커가 받아 둔 타일을 한 번에 꺼낸다(프레임당 한 번만).
        // 결과가 있으면 그때만 다시 그리기를 요청한다 — 타일마다 알리면 렌더 루프가 죽는다.
        let ready = self.basemap_worker.drain();
        if !ready.is_empty() {
            ctx.request_repaint();
        }

        let n = crate::basemap_paint::paint_basemap_tiles(
            ctx,
            p,
            rect,
            &crate::basemap_paint::TileRequest {
                kind,
                z,
                view: &view,
            },
            &mut self.basemap_textures,
            &mut self.basemap_worker,
        );
        self.basemap_tiles_painted = n;
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
        // C++ BasemapOps.cpp:712,969 — 1:80,000 보다 넓게 보면 지적선을 그리지 않는다.
        let scale_denom = self.home_map.viewport.meters_per_pixel * 3780.0;
        if !crate::basemap_cadastral::cadastral_visible_at_scale(scale_denom) {
            return;
        }
        let key = crate::basemap_fetch::current_key();
        if key.trim().is_empty() {
            return;
        }
        let Some(bbox) = crate::basemap_coords::screen_bbox_3857(
            &self.home_map.viewport,
            rect,
            self.work_crs,
        ) else {
            return;
        };

        // **화면 한 장으로 받는다.** 격자 칸마다 따로 받으면 칸 경계에서 선이 끊기고
        // 겹쳐서 지적도가 격자처럼 보인다. C++ 은 WMS 레이어 하나로 화면 전체를 그린다.
        let size = crate::basemap_cadastral_view::cadastral_view_size(rect.width(), rect.height());
        let Some(path) = crate::basemap_cadastral_view::fetch_cadastral_view(&key, bbox, size)
        else {
            return;
        };

        let name = format!(
            "cadview_{}_{}",
            size,
            path.file_stem().and_then(|s| s.to_str()).unwrap_or("0")
        );
        let tex = if let Some(h) = self.cadastral_view_tex.get(&name) {
            h.clone()
        } else {
            let Some(t) = crate::basemap_coords::load_texture_from_path(ctx, &name, &path) else {
                return;
            };
            self.cadastral_view_tex.insert(name, t.clone());
            t
        };

        // WMS 이미지는 요청한 bbox 를 정확히 덮으므로 화면 사각형에 그대로 그린다.
        // **색을 입히지 않는다**(흰색 불투명 tint). 예전엔 알파 190 을 곱해
        // 지적선이 흐려지고 조사구역에 덮였다. C++ 은 레이어를 원본대로 그린다.
        let tint = egui::Color32::WHITE;
        p.image(
            tex.id(),
            rect,
            egui::Rect::from_min_max(egui::Pos2::new(0.0, 0.0), egui::Pos2::new(1.0, 1.0)),
            tint,
        );
    }
}