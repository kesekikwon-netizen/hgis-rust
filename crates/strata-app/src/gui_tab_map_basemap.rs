//! 지도 캔버스의 배경지도 타일 그리기.
//!
//! C++ 은 QGIS 가 위성·일반도 타일을 그려 주지만 Rust 는 직접 받아 칠해야 한다.
//! gui_tab_map_canvas 가 300줄을 넘어 분리했다.

use std::collections::HashSet;

use eframe::egui::{self, Color32, Pos2, Rect};

use crate::app_state::StrataAppState;
use crate::basemap_cadastral_jobs::CadKey;
use crate::basemap_mesh::{paint_tile_mesh, screen_center_lat, screen_tile_range, tile_mesh_points};
use crate::basemap_paint::{tm_params_for, TileView, FULL_UV};
use crate::basemap_tiles::TileKind;

/// 지적 텍스처를 이보다 많이 들고 있으면 이번 화면에 쓴 것만 남긴다(512px 한 장 ≈ 1 MB).
const CADASTRAL_TEXTURE_LIMIT: usize = 160;

/// 지적 그림 텍스처 옵션. 화면과 거의 같은 크기로 그릴 때는 원본(mip 0)을 그대로 쓰고,
/// 멀리 볼 때(z16 고정, 줄여 그림)만 작은 단계를 써서 가는 선이 깨지지 않게 한다.
fn cadastral_texture_options() -> egui::TextureOptions {
    egui::TextureOptions {
        mipmap_mode: Some(egui::TextureFilter::Nearest),
        ..egui::TextureOptions::LINEAR
    }
}

impl StrataAppState {
    /// 지금 칠할 배경지도 종류. 레이어 패널에서 그 행의 체크를 끄면 `None`.
    ///
    /// 사용자 지적(2026-11-12): 「위성지도를 꺼도 위성지도가 남아 있다」 —
    /// 예전에는 `active_basemap` 만 보고 체크 상태를 보지 않았다.
    pub(crate) fn basemap_kind_to_paint(&self) -> Option<TileKind> {
        use strata_net::tile_fetcher::BasemapSource;
        let (kind, key, row) = match self.home_map.active_basemap {
            BasemapSource::VworldSatellite => (TileKind::Satellite, "vworld_satellite", "위성지도 (VWorld)"),
            BasemapSource::VworldBase => (TileKind::Base, "vworld_base", "일반지도 (VWorld)"),
            _ => return None,
        };
        // 범례에 그 배경지도 레이어가 있을 때만 — C++ 은 조사를 열거나 만들 때만 배경지도를 올린다
        // (MainWindowSession.cpp:400, :728). 켠 직후 지도는 흰 바탕이다(지시 S1).
        let listed = self.left_layers.entries.iter().any(|e| e.layer_key == key);
        (listed && self.home_map.real_survey.is_layer_visible(row)).then_some(kind)
    }

    /// 범례에 VWorld 지적 그림 레이어가 있는가(없으면 지적을 칠하지 않는다).
    pub(crate) fn cadastral_layer_listed(&self) -> bool {
        self.left_layers.entries.iter().any(|e| e.layer_key == "vworld_cadastral_wms")
    }

    /// 배경지도 타일을 칠한다(위성·일반도). 지적 WMS 는 별도 경로다.
    pub(crate) fn paint_basemap_layer(
        &mut self,
        ctx: &egui::Context,
        p: &egui::Painter,
        rect: egui::Rect,
    ) {
        let Some(kind) = self.basemap_kind_to_paint() else {
            self.basemap_tiles_painted = 0;
            return;
        };
        let mpp = self.home_map.viewport.meters_per_pixel.max(1e-6);
        let z = crate::basemap_tiles::basemap_zoom_for(mpp, rect);
        let view = TileView::from_viewport(&self.home_map.viewport, rect, self.work_crs);
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

    /// 연속지적도(WMS)를 지도 위에 겹쳐 칠한다.
    ///
    /// C++ `addVworldCadastralMap` 은 QGIS WMS 로 화면 해상도 그대로 받아 불투명(1.0)으로
    /// 얹는다(`BasemapOps.cpp:1306` `setOpacity(1.0)`). VWorld 그림 자체가 배경이 투명이다.
    /// 지적 그림은 스냅 대상이 아니므로 그리기만 한다.
    pub(crate) fn paint_cadastral_layer(
        &mut self,
        ctx: &egui::Context,
        p: &egui::Painter,
        rect: egui::Rect,
    ) {
        if !self.cadastral_layer_listed()
            || !self.home_map.real_survey.is_layer_visible("연속지적도 (VWorld WMS)")
        {
            return;
        }
        // C++ `BasemapOps.cpp:712,969` — 1:80,000 보다 넓게 보면 지적선을 그리지 않는다.
        // 넓게 보면 필지선이 뭉개져 오히려 방해가 되고, 타일 요청도 폭증한다.
        let scale_denom = self.home_map.viewport.meters_per_pixel * crate::status_bar_text::POINTS_PER_METER;
        if !crate::basemap_cadastral::cadastral_visible_at_scale(scale_denom) {
            return;
        }
        let key = crate::basemap_fetch::current_key();
        if key.trim().is_empty() {
            return;
        }
        let Some(params) = tm_params_for(self.work_crs) else {
            return;
        };
        let view = TileView::from_viewport(&self.home_map.viewport, rect, self.work_crs);
        // 화면 물리 픽셀 해상도에 맞는 줌(200% 배율이면 논리 픽셀의 절반 크기).
        let ppp = f64::from(ctx.pixels_per_point().max(0.5));
        let ground_mpp = self.home_map.viewport.meters_per_pixel / ppp;
        let lat = screen_center_lat(&view, &params, rect);
        let Some(z) = crate::basemap_cadastral::cadastral_zoom_for(ground_mpp, lat) else {
            return; // VWorld 가 빈 그림을 주는 넓은 축척(C++ 도 비어 보인다)
        };
        let Some((x0, y0, x1, y1)) = screen_tile_range(&view, &params, rect, z) else {
            return;
        };
        let count = (x1 - x0 + 1) as usize * (y1 - y0 + 1) as usize;
        if count > crate::basemap_cadastral::CADASTRAL_MAX_TILES {
            return; // 너무 넓게 보면 지적선을 생략한다(성능 보호)
        }

        // 뒤에서 다 받은 그림을 텍스처로 올린다.
        for (k, img) in self.cadastral_fetch.take_done() {
            let name = format!("cad_{}_{}_{}", k.0, k.1, k.2);
            let tex = ctx.load_texture(name, img, cadastral_texture_options());
            self.cadastral_cells.insert(k, tex);
        }

        let mut used: HashSet<CadKey> = HashSet::new();
        for ty in y0..=y1 {
            for tx in x0..=x1 {
                let Some(pts) = tile_mesh_points(&view, &params, rect, z, tx, ty) else {
                    continue;
                };
                if !Rect::from_points(&pts).intersects(rect) {
                    continue;
                }
                let k = (z, tx, ty);
                if let Some(h) = self.cadastral_cells.get(&k) {
                    paint_tile_mesh(p, h.id(), &pts, FULL_UV, Color32::WHITE);
                    used.insert(k);
                    continue;
                }
                let api_key = key.clone();
                self.cadastral_fetch.request_with(k, Some(ctx), move || {
                    crate::basemap_cadastral_jobs::load_cadastral_image(&api_key, z, tx, ty)
                });
                // 받는 동안 이미 가진 다른 줌 그림으로 자리를 채운다(확대·축소 때 깜빡임 방지).
                self.paint_cadastral_stand_in(p, &view, &params, rect, k, &pts, &mut used);
            }
        }
        if self.cadastral_cells.len() > CADASTRAL_TEXTURE_LIMIT {
            self.cadastral_cells.retain_keys(&used);
        }
    }

    /// 아직 받지 못한 타일 자리에 윗단계(최대 3단계) 그림의 해당 부분을 늘려 그리거나,
    /// 그게 없으면 이미 가진 한 단계 아래 그림 4장을 그린다.
    #[allow(clippy::too_many_arguments)]
    fn paint_cadastral_stand_in(
        &self,
        p: &egui::Painter,
        view: &TileView,
        params: &strata_geo::crs_transform::TmParams,
        rect: Rect,
        (z, tx, ty): CadKey,
        pts: &[Pos2],
        used: &mut HashSet<CadKey>,
    ) {
        for up in 1..=3_u8 {
            let Some(pz) = z.checked_sub(up) else {
                break;
            };
            let pk = (pz, tx >> up, ty >> up);
            if let Some(h) = self.cadastral_cells.get(&pk) {
                let frac = 1.0 / f32::from(1_u16 << up);
                let u0 = (tx - (pk.1 << up)) as f32 * frac;
                let v0 = (ty - (pk.2 << up)) as f32 * frac;
                let uv = Rect::from_min_size(Pos2::new(u0, v0), egui::vec2(frac, frac));
                paint_tile_mesh(p, h.id(), pts, uv, Color32::WHITE);
                used.insert(pk);
                return;
            }
        }
        if z >= crate::basemap_cadastral::CADASTRAL_MAX_ZOOM {
            return;
        }
        for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
            let ck = (z + 1, tx * 2 + dx, ty * 2 + dy);
            let Some(h) = self.cadastral_cells.get(&ck) else {
                continue;
            };
            if let Some(cpts) = tile_mesh_points(view, params, rect, ck.0, ck.1, ck.2) {
                paint_tile_mesh(p, h.id(), &cpts, FULL_UV, Color32::WHITE);
                used.insert(ck);
            }
        }
    }
}
