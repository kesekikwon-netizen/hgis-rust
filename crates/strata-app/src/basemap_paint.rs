//! 지도 캔버스에 VWorld 타일을 칠한다.
//!
//! Background tiles load on worker threads while the canvas draws cached textures and multi-zoom
//! stand-ins without blocking the UI thread (target/cpp-ref/src/app/MainWindow.cpp:514,526).
//!
//! 좌표 변환 경로:
//!   화면 px → work-CRS 미터(TM) → 경위도(WGS84) → XYZ 타일 번호
//! 타일은 `basemap_mesh` 로 기울기까지 따라 펴서 그린다(이음매 없음).

use std::collections::{HashMap, HashSet};

use eframe::egui::{self, Color32, Pos2, Rect};
use strata_geo::crs_transform::TmParams;

use crate::basemap_cadastral_jobs::{BasemapFetcher, CadKey, TexKey, MAX_UPLOADS_PER_FRAME};
use crate::basemap_fetch::tile_filename_key;
use crate::basemap_mesh::{paint_tile_mesh, screen_tile_range, tile_mesh_points};
use crate::basemap_tiles::TileKind;

/// Keep up to 512 256x256 RGBA tile textures (~0.26 MB each, ~134 MB total) before trimming
/// unused off-screen entries, leaving headroom for several screens and stand-in zoom levels.
pub const BASEMAP_TEXTURE_LIMIT: usize = 512;

/// Highest XYZ zoom level used by `basemap_zoom_for` (`5..=19`).
pub const BASEMAP_MAX_ZOOM: u8 = 19;

/// 타일 텍스처 캐시. egui::TextureHandle 은 Debug 가 아니므로 직접 구현한다.
#[derive(Clone, Default)]
pub struct TileTextureCache(pub HashMap<TexKey, egui::TextureHandle>);

impl std::fmt::Debug for TileTextureCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TileTextureCache").field("len", &self.0.len()).finish()
    }
}

impl TileTextureCache {
    #[must_use]
    pub fn is_empty(&self) -> bool { self.0.is_empty() }
    #[must_use]
    pub fn len(&self) -> usize { self.0.len() }
    #[must_use]
    pub fn get(&self, k: &TexKey) -> Option<&egui::TextureHandle> { self.0.get(k) }
    pub fn insert(&mut self, k: TexKey, v: egui::TextureHandle) { self.0.insert(k, v); }
    pub fn retain_keys(&mut self, keep: &HashSet<TexKey>) { self.0.retain(|k, _| keep.contains(k)); }
}

/// 지적 타일 `(z, x, y)` 별 텍스처 캐시.
#[derive(Clone, Default)]
pub struct CadastralCellCache(pub HashMap<CadKey, egui::TextureHandle>);

impl std::fmt::Debug for CadastralCellCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CadastralCellCache").field("len", &self.0.len()).finish()
    }
}

impl CadastralCellCache {
    #[must_use]
    pub fn get(&self, k: &CadKey) -> Option<&egui::TextureHandle> { self.0.get(k) }
    pub fn insert(&mut self, k: CadKey, v: egui::TextureHandle) { self.0.insert(k, v); }
    #[must_use]
    pub fn len(&self) -> usize { self.0.len() }
    pub fn retain_keys(&mut self, keep: &HashSet<CadKey>) { self.0.retain(|k, _| keep.contains(k)); }
}

/// Draws ready XYZ tiles, fills missing slots from other zoom levels in memory, and schedules
/// missing tiles nearest to the view centre on `fetcher` (target/cpp-ref/src/app/MainWindow.cpp:514,526).
#[allow(clippy::too_many_arguments)]
pub fn paint_basemap_tiles_with<M, F>(
    ctx: &egui::Context,
    painter: &egui::Painter,
    rect: Rect,
    kind: TileKind,
    z: u8,
    view: &TileView,
    textures: &mut TileTextureCache,
    fetcher: &BasemapFetcher,
    tint: Color32,
    mut make_job: M,
) -> usize
where
    M: FnMut(TileKind, u8, u32, u32) -> F,
    F: FnOnce() -> Option<egui::ColorImage> + Send + 'static,
{
    if !kind.is_xyz() || !rect.is_positive() {
        return 0;
    }
    let Some(params) = tm_params_for(view.work_crs) else {
        return 0;
    };
    let Some((x0, y0, x1, y1)) = screen_tile_range(view, &params, rect, z) else {
        return 0;
    };
    let count = (x1.saturating_sub(x0) + 1) as u64 * (y1.saturating_sub(y0) + 1) as u64;
    if count == 0 || count > 256 {
        return 0;
    }

    let (done, more_done) = fetcher.take_done_limited(MAX_UPLOADS_PER_FRAME);
    for (dk, color) in done {
        let tex = ctx.load_texture(
            format!("{}_{}_{}_{}", tile_filename_key(dk.0), dk.1, dk.2, dk.3),
            color,
            egui::TextureOptions::LINEAR,
        );
        textures.insert(dk, tex);
    }
    if more_done {
        ctx.request_repaint();
    }

    let mut painted = 0usize;
    let mut used: HashSet<TexKey> = HashSet::new();
    let mut missing: Vec<(f32, u32, u32)> = Vec::new();
    let center = rect.center();

    for ty in y0..=y1 {
        for tx in x0..=x1 {
            let Some(pts) = tile_mesh_points(view, &params, rect, z, tx, ty) else {
                continue;
            };
            if !Rect::from_points(&pts).intersects(rect) {
                continue;
            }
            let k = (kind, z, tx, ty);
            if let Some(h) = textures.get(&k) {
                paint_tile_mesh(painter, h.id(), &pts, FULL_UV, tint);
                used.insert(k);
                painted += 1;
                continue;
            }
            paint_tile_stand_in(
                painter,
                view,
                &params,
                rect,
                (z, tx, ty),
                &pts,
                BASEMAP_MAX_ZOOM,
                tint,
                |sz, sx, sy| {
                    let sk = (kind, sz, sx, sy);
                    textures.get(&sk).map(|h| {
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
    for (_, tx, ty) in missing {
        let k = (kind, z, tx, ty);
        fetcher.request_with(k, Some(ctx), make_job(kind, z, tx, ty));
    }

    if textures.len() > BASEMAP_TEXTURE_LIMIT {
        textures.retain_keys(&used);
    }
    painted
}

/// Draws a stand-in from an ancestor tile (up to 3 zoom levels up) or direct children (1 level down)
/// already in memory while `(z, tx, ty)` is still loading.
#[allow(clippy::too_many_arguments)]
pub(crate) fn paint_tile_stand_in(
    painter: &egui::Painter,
    view: &TileView,
    params: &TmParams,
    rect: Rect,
    (z, tx, ty): CadKey,
    pts: &[Pos2],
    max_zoom: u8,
    tint: Color32,
    mut use_tile: impl FnMut(u8, u32, u32) -> Option<egui::TextureId>,
) {
    for up in 1..=3_u8 {
        let Some(pz) = z.checked_sub(up) else {
            break;
        };
        let (px, py) = (tx >> up, ty >> up);
        if let Some(tex_id) = use_tile(pz, px, py) {
            let frac = 1.0 / f32::from(1_u16 << up);
            let u0 = (tx - (px << up)) as f32 * frac;
            let v0 = (ty - (py << up)) as f32 * frac;
            let uv = Rect::from_min_size(Pos2::new(u0, v0), egui::vec2(frac, frac));
            paint_tile_mesh(painter, tex_id, pts, uv, tint);
            return;
        }
    }
    if z >= max_zoom {
        return;
    }
    for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
        let (cz, cx, cy) = (z + 1, tx * 2 + dx, ty * 2 + dy);
        let Some(tex_id) = use_tile(cz, cx, cy) else {
            continue;
        };
        if let Some(cpts) = tile_mesh_points(view, params, rect, cz, cx, cy) {
            paint_tile_mesh(painter, tex_id, &cpts, FULL_UV, tint);
        }
    }
}

/// 텍스처 전체(0..1).
pub const FULL_UV: Rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0));

/// 화면 ↔ work-CRS 미터 변환기.
#[derive(Debug, Clone, Copy)]
pub struct TileView {
    /// 화면 사각형 좌상단이 나타내는 work-CRS 미터 좌표.
    pub world_origin: [f64; 2],
    /// 화면 1px 이 나타내는 미터.
    pub meters_per_px: f64,
    /// work CRS.
    pub work_crs: strata_types::crs::WorkCrs,
}

impl TileView {
    #[must_use]
    pub fn from_viewport(
        vp: &strata_canvas2d::viewport::ViewportTransform,
        rect: egui::Rect,
        work_crs: strata_types::crs::WorkCrs,
    ) -> Self {
        let cx = f64::from(rect.center().x - rect.left());
        let cy = f64::from(rect.center().y - rect.top());
        Self {
            world_origin: [
                vp.center_xy[0] - cx * vp.meters_per_pixel,
                vp.center_xy[1] + cy * vp.meters_per_pixel,
            ],
            meters_per_px: vp.meters_per_pixel,
            work_crs,
        }
    }

    pub(crate) fn screen_to_meters(&self, rect: Rect, px: Pos2) -> [f64; 2] {
        [
            self.world_origin[0] + f64::from(px.x - rect.left()) * self.meters_per_px,
            self.world_origin[1] - f64::from(px.y - rect.top()) * self.meters_per_px,
        ]
    }

    pub(crate) fn meters_to_screen(&self, rect: Rect, m: [f64; 2]) -> Pos2 {
        let sx = (m[0] - self.world_origin[0]) / self.meters_per_px;
        let sy = (self.world_origin[1] - m[1]) / self.meters_per_px;
        Pos2::new(rect.left() + sx as f32, rect.top() + sy as f32)
    }
}

/// WorkCrs 에 맞는 TM 투영 파라미터.
pub(crate) fn tm_params_for(crs: strata_types::crs::WorkCrs) -> Option<TmParams> {
    use strata_types::crs::WorkCrs;
    let code = match crs {
        WorkCrs::Epsg5186 => strata_types::crs::CrsCode::Epsg5186,
        WorkCrs::Epsg5187 => strata_types::crs::CrsCode::Epsg5187,
    };
    TmParams::for_crs(code)
}

#[cfg(test)]
#[path = "basemap_async_tests_a.rs"]
mod async_tests_a;
#[cfg(test)]
#[path = "basemap_async_tests_b.rs"]
mod async_tests_b;
