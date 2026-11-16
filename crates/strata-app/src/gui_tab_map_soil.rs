//! 토양도(흙토람) 타일을 받아 지도에 칠한다. C++ `SoilMapService` 대응.
//!
//! ## 배경
//!
//! 2026-11-10: 리본 「토양」 을 눌러도 레이어만 켜지고 **아무 그림도 안 나왔다**.
//! C++ 은 흙토람 서버에서 ArcGIS 캐시 타일을 받아 그린다.
//! 경로가 16진수 0채움(`L13/R00000c95/C00001b7c.png`)이라 10진수로 요청하면 404 다
//! — 이걸 몰라 "토양도가 안 된다"로 보였다. URL 규칙은 `strata_net::soil_map` 에 있다.

use eframe::egui;

use crate::app_state::StrataAppState;

/// 토양 타일 텍스처 캐시. 키 = `(z, x, y)`.
///
/// `TileTextureCache` 는 `TileKind` 를 키에 넣는데 토양도는 그 열거형에 없다.
/// 별도 타입으로 두어 위성·지적 캐시와 섞이지 않게 한다.
#[derive(Default, Clone)]
pub struct SoilTextureCache(pub std::collections::HashMap<(u8, u32, u32), egui::TextureHandle>);

impl std::fmt::Debug for SoilTextureCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SoilTextureCache")
            .field("len", &self.0.len())
            .finish()
    }
}

impl SoilTextureCache {
    #[must_use]
    pub fn get(&self, k: &(u8, u32, u32)) -> Option<&egui::TextureHandle> {
        self.0.get(k)
    }

    pub fn insert(&mut self, k: (u8, u32, u32), v: egui::TextureHandle) {
        self.0.insert(k, v);
    }
}

impl StrataAppState {
    /// 지금 화면에 필요한 토양 타일을 미리 받아 둔다. 받은 장수를 돌려준다.
    ///
    /// 네이티브 창을 띄우지 않고 동작한다(버튼을 누른 직후 한 번 부른다).
    pub(crate) fn prefetch_soil_tiles(&self) -> usize {
        let Some((z, x0, y0, x1, y1)) = self.soil_tile_range() else {
            return 0;
        };
        let mut got = 0usize;
        for ty in y0..=y1 {
            for tx in x0..=x1 {
                if strata_net::soil_map::fetch_soil_tile(z, tx, ty).is_some() {
                    got += 1;
                }
                // 한 번에 너무 많이 받지 않는다(버튼 누른 직후 화면이 멈추지 않게).
                if got >= 12 {
                    return got;
                }
            }
        }
        got
    }

    /// 화면이 덮는 타일 범위 `(z, x0, y0, x1, y1)`.
    fn soil_tile_range(&self) -> Option<(u8, u32, u32, u32, u32)> {
        let rect = self.last_canvas_rect?;
        if !rect.is_positive() {
            return None;
        }
        let mpp = self.home_map.viewport.meters_per_pixel.max(1e-6);
        // 화면 256px 타일 하나가 덮는 거리로 줌을 고른다.
        let z = crate::basemap_tiles::basemap_zoom_for(mpp, rect);
        let z = z.clamp(6, 15); // 흙토람 캐시 제공 범위
        let corners = [
            rect.left_top(),
            rect.right_top(),
            rect.left_bottom(),
            rect.right_bottom(),
        ];
        let (mut x0, mut y0) = (u32::MAX, u32::MAX);
        let (mut x1, mut y1) = (0u32, 0u32);
        for c in corners {
            let w = super::gui_tab_map_geom::s2w(rect, &self.home_map.viewport, c);
            let (tx, ty) = crate::basemap_tiles::lon_lat_to_tile(w[0], w[1], z);
            x0 = x0.min(tx);
            y0 = y0.min(ty);
            x1 = x1.max(tx);
            y1 = y1.max(ty);
        }
        Some((z, x0, y0, x1, y1))
    }

    /// 토양도를 지도 위에 칠한다.
    pub(crate) fn paint_soil_layer(
        &mut self,
        ctx: &egui::Context,
        p: &egui::Painter,
        rect: egui::Rect,
    ) {
        if !self
            .home_map
            .real_survey
            .is_layer_visible(strata_net::soil_map::SOIL_PICTURE_TITLE)
        {
            return;
        }
        let Some((z, x0, y0, x1, y1)) = self.soil_tile_range() else {
            return;
        };
        // 너무 넓으면 건너뛴다(성능 보호 — 위성과 같은 규칙).
        let count = u64::from(x1.saturating_sub(x0) + 1) * u64::from(y1.saturating_sub(y0) + 1);
        if count == 0 || count > 256 {
            return;
        }

        let tint = egui::Color32::WHITE;
        for ty in y0..=y1 {
            for tx in x0..=x1 {
                let key = (z, tx, ty);
                let tex = if let Some(h) = self.soil_textures.get(&key) {
                    h.clone()
                } else {
                    // 캐시에 있는 것만 그린다(그리기 함수에서 네트워크를 타지 않는다).
                    let path = strata_net::soil_map::soil_tile_path(z, tx, ty);
                    if !path.is_file() {
                        continue;
                    }
                    let name = format!("soil_{z}_{tx}_{ty}");
                    let Some(t) = crate::basemap_coords::load_texture_from_path(ctx, &name, &path)
                    else {
                        continue;
                    };
                    self.soil_textures.insert(key, t.clone());
                    t
                };
                let Some(tile_rect) = self.soil_tile_screen_rect(rect, z, tx, ty) else {
                    continue;
                };
                if !tile_rect.intersect(rect).is_positive() {
                    continue;
                }
                p.image(
                    tex.id(),
                    tile_rect,
                    egui::Rect::from_min_max(
                        egui::Pos2::new(0.0, 0.0),
                        egui::Pos2::new(1.0, 1.0),
                    ),
                    tint,
                );
            }
        }
    }

    /// 타일 하나가 화면에서 차지하는 사각형.
    fn soil_tile_screen_rect(
        &self,
        rect: egui::Rect,
        z: u8,
        tx: u32,
        ty: u32,
    ) -> Option<egui::Rect> {
        let (lon_w, lat_n) = crate::basemap_tiles::tile_lon_lat(z, tx, ty);
        let (lon_e, lat_s) = crate::basemap_tiles::tile_lon_lat(z, tx + 1, ty + 1);
        let a = super::gui_tab_map_geom::w2s(rect, &self.home_map.viewport, [lon_w, lat_n]);
        let b = super::gui_tab_map_geom::w2s(rect, &self.home_map.viewport, [lon_e, lat_s]);
        let r = egui::Rect::from_min_max(
            egui::Pos2::new(a.x.min(b.x), a.y.min(b.y)),
            egui::Pos2::new(a.x.max(b.x), a.y.max(b.y)),
        );
        r.is_positive().then_some(r)
    }
}
