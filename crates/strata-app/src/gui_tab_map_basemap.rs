//! 지도 캔버스의 배경지도 타일 그리기.
//!
//! C++ 은 QGIS 가 위성·일반도 타일을 그려 주지만 Rust 는 직접 받아 칠해야 한다
//! (target/cpp-ref/src/app/MainWindow.cpp:514,526 — background rendering keeps last preview visible).
//! gui_tab_map_canvas 가 300줄을 넘어 분리했다.

use std::collections::HashSet;

use eframe::egui::{self, Color32, Rect};

use crate::app_state::StrataAppState;
use crate::basemap_cadastral_jobs::{CadKey, MAX_UPLOADS_PER_FRAME};
use crate::basemap_mesh::{paint_tile_mesh, screen_center_lat, screen_tile_range, tile_mesh_points};
use crate::basemap_paint::{paint_tile_stand_in, tm_params_for, TileView, FULL_UV};
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
    pub(crate) fn basemap_kind_to_paint(&self) -> Option<TileKind> {
        use strata_net::tile_fetcher::BasemapSource;
        let (kind, key) = match self.home_map.active_basemap {
            BasemapSource::VworldSatellite => (TileKind::Satellite, "vworld_satellite"),
            BasemapSource::VworldBase => (TileKind::Base, "vworld_base"),
            _ => return None,
        };
        let entry = self.left_layers.entries.iter().find(|e| e.layer_key == key)?;
        self.home_map.real_survey.is_layer_visible(&entry.label_ko).then_some(kind)
    }

    /// 그 배경지도 줄의 투명도(applyLayerOpacity) — 타일 색에 곱한다.
    fn basemap_opacity(&self, kind: TileKind) -> f32 {
        let key = if kind == TileKind::Satellite { "vworld_satellite" } else { "vworld_base" };
        self.left_layers.entries.iter().find(|e| e.layer_key == key).map_or(1.0, |e| e.opacity.clamp(0.0, 1.0))
    }

    /// 범례에 VWorld 지적 그림 레이어가 있는가(없으면 지적을 칠하지 않는다).
    pub(crate) fn cadastral_layer_listed(&self) -> bool {
        self.left_layers.entries.iter().any(|e| e.layer_key == "vworld_cadastral_wms")
    }

    /// Returns the cached VWorld API key; if it changed since the previous handout, clears failed
    /// marks of both fetchers and asks egui for a repaint so the view retries immediately.
    fn current_vworld_api_key(&mut self, ctx: &egui::Context) -> String {
        let (key, changed) = self.vworld_api_key.get_with_change();
        let out = key.to_owned();
        if changed {
            self.basemap_fetch.clear_failed();
            self.cadastral_fetch.clear_failed();
            ctx.request_repaint();
        }
        out
    }

    /// 배경지도 타일을 칠한다(위성·일반도). 지적 WMS 는 별도 경로다.
    pub(crate) fn paint_basemap_layer(
        &mut self,
        ctx: &egui::Context,
        p: &egui::Painter,
        rect: egui::Rect,
    ) {
        self.paint_basemap_layer_with(ctx, p, rect, |kind, api_key, z, tx, ty| {
            let key = api_key.to_owned();
            move || crate::basemap_cadastral_jobs::load_basemap_image(kind, &key, z, tx, ty)
        });
    }

    pub(crate) fn paint_basemap_layer_with<M, F>(
        &mut self,
        ctx: &egui::Context,
        p: &egui::Painter,
        rect: egui::Rect,
        mut make_job: M,
    ) where
        M: FnMut(TileKind, &str, u8, u32, u32) -> F,
        F: FnOnce() -> Option<egui::ColorImage> + Send + 'static,
    {
        let Some(kind) = self.basemap_kind_to_paint() else {
            self.basemap_tiles_painted = 0;
            return;
        };
        let api_key = self.current_vworld_api_key(ctx);
        let mpp = self.home_map.viewport.meters_per_pixel.max(1e-6);
        let z = crate::basemap_tiles::basemap_zoom_for(mpp, rect);
        let view = TileView::from_viewport(&self.home_map.viewport, rect, self.work_crs);
        let tint = egui::Color32::WHITE.gamma_multiply(self.basemap_opacity(kind));
        let n = crate::basemap_paint::paint_basemap_tiles_with(
            ctx,
            p,
            rect,
            kind,
            z,
            &view,
            &mut self.basemap_textures,
            &self.basemap_fetch,
            tint,
            |tk, tz, tx, ty| make_job(tk, &api_key, tz, tx, ty),
        );
        self.basemap_tiles_painted = n;
    }

    /// 연속지적도(WMS)를 지도 위에 겹쳐 칠한다.
    pub(crate) fn paint_cadastral_layer(
        &mut self,
        ctx: &egui::Context,
        p: &egui::Painter,
        rect: egui::Rect,
    ) {
        self.paint_cadastral_layer_with(ctx, p, rect, |api_key, z, tx, ty| {
            let key = api_key.to_owned();
            move || crate::basemap_cadastral_jobs::load_cadastral_image(&key, z, tx, ty)
        });
    }

    pub(crate) fn paint_cadastral_layer_with<M, F>(
        &mut self,
        ctx: &egui::Context,
        p: &egui::Painter,
        rect: egui::Rect,
        mut make_job: M,
    ) where
        M: FnMut(&str, u8, u32, u32) -> F,
        F: FnOnce() -> Option<egui::ColorImage> + Send + 'static,
    {
        if !self.cadastral_layer_listed()
            || !self.home_map.real_survey.is_layer_visible("연속지적도 (VWorld WMS)")
        {
            return;
        }
        let scale_denom = self.home_map.viewport.meters_per_pixel * crate::status_bar_text::POINTS_PER_METER;
        if !crate::basemap_cadastral::cadastral_visible_at_scale(scale_denom) {
            return;
        }
        let key = self.current_vworld_api_key(ctx);
        if key.trim().is_empty() {
            return;
        }
        let Some(params) = tm_params_for(self.work_crs) else {
            return;
        };
        let view = TileView::from_viewport(&self.home_map.viewport, rect, self.work_crs);
        let ppp = f64::from(ctx.pixels_per_point().max(0.5));
        let ground_mpp = self.home_map.viewport.meters_per_pixel / ppp;
        let lat = screen_center_lat(&view, &params, rect);
        let Some(z) = crate::basemap_cadastral::cadastral_zoom_for(ground_mpp, lat) else {
            return;
        };
        let Some((x0, y0, x1, y1)) = screen_tile_range(&view, &params, rect, z) else {
            return;
        };
        let count = (x1 - x0 + 1) as usize * (y1 - y0 + 1) as usize;
        if count > crate::basemap_cadastral::CADASTRAL_MAX_TILES {
            return;
        }

        let (done, more_done) = self.cadastral_fetch.take_done_limited(MAX_UPLOADS_PER_FRAME);
        for (k, img) in done {
            let name = format!("cad_{}_{}_{}", k.0, k.1, k.2);
            let tex = ctx.load_texture(name, img, cadastral_texture_options());
            self.cadastral_cells.insert(k, tex);
        }
        if more_done {
            ctx.request_repaint();
        }

        let mut used: HashSet<CadKey> = HashSet::new();
        let mut missing: Vec<(f32, u32, u32)> = Vec::new();
        let center = rect.center();
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
                paint_tile_stand_in(
                    p,
                    &view,
                    &params,
                    rect,
                    k,
                    &pts,
                    crate::basemap_cadastral::CADASTRAL_MAX_ZOOM,
                    Color32::WHITE,
                    |sz, sx, sy| {
                        let sk = (sz, sx, sy);
                        self.cadastral_cells.get(&sk).map(|h| {
                            used.insert(sk);
                            h.id()
                        })
                    },
                );
                let dist_sq = (pts[pts.len() / 2] - center).length_sq();
                missing.push((dist_sq, tx, ty));
            }
        }
        missing.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| (a.2, a.1).cmp(&(b.2, b.1))));
        for &(_, tx, ty) in &missing {
            let k = (z, tx, ty);
            self.cadastral_fetch.request_with(k, Some(ctx), make_job(&key, z, tx, ty));
        }
        if let Some(delay) = self.cadastral_fetch.earliest_retry_delay(missing.iter().map(|&(_, tx, ty)| (z, tx, ty))) {
            ctx.request_repaint_after(delay);
        }
        if self.cadastral_cells.len() > CADASTRAL_TEXTURE_LIMIT {
            self.cadastral_cells.retain_keys(&used);
        }
    }
}
