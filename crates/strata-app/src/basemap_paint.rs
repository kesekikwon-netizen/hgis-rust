//! 지도 캔버스에 VWorld 타일을 칠한다.
//!
//! 지금까지 지도 탭은 단색만 칠했다(사용자 지적 2026-11-09: 위성·지적이
//! 레이어 목록에 켜져 있는데 화면은 회색). 여기서 실제 타일을 받아 그린다.
//!
//! 좌표 변환 경로:
//!   화면 px → work-CRS 미터(TM) → 경위도(WGS84) → XYZ 타일 번호
//! 타일은 `basemap_mesh` 로 기울기까지 따라 펴서 그린다(이음매 없음).


use eframe::egui::{self, Color32, Pos2, Rect};
use strata_geo::crs_transform::TmParams;

use crate::basemap_cadastral_jobs::CadKey;
use crate::basemap_fetch::{fetch_tile, tile_filename_key};
use crate::basemap_mesh::{paint_tile_mesh, screen_tile_range, tile_mesh_points};
use crate::basemap_tiles::TileKind;

/// 타일 텍스처 캐시 키.
type TexKey = (TileKind, u8, u32, u32);

/// 타일 텍스처 캐시. egui::TextureHandle 은 Debug 가 아니므로 직접 구현한다.
#[derive(Default)]
pub struct TileTextureCache(pub std::collections::HashMap<TexKey, egui::TextureHandle>);

impl std::fmt::Debug for TileTextureCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TileTextureCache")
            .field("len", &self.0.len())
            .finish()
    }
}

impl Clone for TileTextureCache {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl TileTextureCache {
    /// 비어 있는지.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// 지적 타일 `(z, x, y)` 별 텍스처 캐시.
#[derive(Clone, Default)]
pub struct CadastralCellCache(pub std::collections::HashMap<CadKey, egui::TextureHandle>);

impl std::fmt::Debug for CadastralCellCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CadastralCellCache")
            .field("len", &self.0.len())
            .finish()
    }
}

impl CadastralCellCache {
    /// 칸 텍스처를 얻는다.
    #[must_use]
    pub fn get(&self, k: &CadKey) -> Option<&egui::TextureHandle> {
        self.0.get(k)
    }

    /// 칸 텍스처를 넣는다.
    pub fn insert(&mut self, k: CadKey, v: egui::TextureHandle) {
        self.0.insert(k, v);
    }

    /// 들고 있는 텍스처 수.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// `keep` 에 없는 텍스처를 버린다(그래픽 메모리 보호).
    pub fn retain_keys(&mut self, keep: &std::collections::HashSet<CadKey>) {
        self.0.retain(|k, _| keep.contains(k));
    }
}

/// 화면에 보이는 타일을 칠한다. 칠한 장수를 돌려준다.
#[allow(clippy::too_many_arguments)]
pub fn paint_basemap_tiles(
    ctx: &egui::Context,
    painter: &egui::Painter,
    rect: Rect,
    kind: TileKind,
    z: u8,
    view: &TileView,
    textures: &mut TileTextureCache,
    tint: Color32,
) -> usize {
    if !kind.is_xyz() || !rect.is_positive() {
        return 0;
    }
    // WorkCrs → CrsCode → TM 파라미터.
    let Some(params) = tm_params_for(view.work_crs) else {
        return 0;
    };
    let key = crate::basemap_fetch::current_key();
    let mut painted = 0usize;

    // 화면 네 모서리 → 경위도 → 타일 범위.
    let Some((x0, y0, x1, y1)) = screen_tile_range(view, &params, rect, z) else {
        return 0;
    };
    let count = (x1.saturating_sub(x0) + 1) as u64 * (y1.saturating_sub(y0) + 1) as u64;
    if count == 0 || count > 256 {
        return 0; // 너무 넓거나 좁으면 타일을 그리지 않는다(성능 보호)
    }

    for ty in y0..=y1 {
        for tx in x0..=x1 {
            let Some(pts) = tile_mesh_points(view, &params, rect, z, tx, ty) else {
                continue;
            };
            if !Rect::from_points(&pts).intersects(rect) {
                continue;
            }
            let k = (kind, z, tx, ty);
            let handle = if let Some(h) = textures.0.get(&k) {
                h.clone()
            } else {
                let Some(tex) = load_tile_texture(ctx, kind, &key, z, tx, ty) else {
                    continue;
                };
                textures.0.insert(k, tex.clone());
                tex
            };
            paint_tile_mesh(painter, handle.id(), &pts, FULL_UV, tint);
            painted += 1;
        }
    }
    painted
}

fn load_tile_texture(
    ctx: &egui::Context,
    kind: TileKind,
    key: &str,
    z: u8,
    x: u32,
    y: u32,
) -> Option<egui::TextureHandle> {
    let path = fetch_tile(kind, key, z, x, y)?;
    let img = image::open(&path).ok()?;
    let rgba = img.to_rgba8();
    let (w, h) = (rgba.width() as usize, rgba.height() as usize);
    if w == 0 || h == 0 {
        return None;
    }
    let color = egui::ColorImage::from_rgba_unmultiplied([w, h], rgba.as_raw());
    Some(ctx.load_texture(
        format!("{}_{}_{}_{}", tile_filename_key(kind), z, x, y),
        color,
        egui::TextureOptions::LINEAR,
    ))
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
    /// 지도 뷰포트(중심·축척)에서 만든다.
    ///
    /// ViewportTransform.center_xy 는 화면 중심이 나타내는 미터 좌표이고
    /// meters_per_pixel 은 화면 1px 당 미터다. 좌상단 원점을 계산해 둔다.
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
