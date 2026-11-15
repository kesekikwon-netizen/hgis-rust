//! 3857 타일을 작업 좌표계(TM) 화면에 **이음매 없이** 펴서 그린다.
//!
//! 사용자 지적(2026-11-12): 위성·지적이 「갖다 붙여놓은 것 같이 지저분하다」.
//! 3857 타일은 TM(EPSG:5187/5186)에서 자오선 수렴각(≈ Δλ·sin φ, 춘천·5187 에서 약 0.8°)만큼
//! 기운 사각형이다. 예전에는 두 모서리만 변환해 축에 나란한 사각형으로 그려 이웃 타일 사이에
//! 2~4px 흰 줄과 어긋남이 생겼다. C++ 은 QGIS 가 래스터를 재투영해 그리므로 틈이 없다.
//!
//! 여기서는 타일을 격자 메시로 나눠 꼭짓점마다 따로 변환한다. 꼭짓점은 **전역 타일 좌표**
//! (tx + i/n)에서 계산하므로 이웃 타일의 경계 꼭짓점이 비트까지 같다.

use eframe::egui::{self, Color32, Pos2, Rect};
use strata_geo::crs_transform::{lonlat_to_tm, tm_to_lonlat, TmParams};

use crate::basemap_paint::TileView;
use crate::basemap_tiles::{grid_point_lon_lat, grid_tile_of, TileGrid};

/// 타일 한 변을 나누는 칸 수. 넓은 타일(낮은 줌)에서도 TM 곡률을 따라가게 한다.
pub const TILE_MESH_STEPS: usize = 4;

/// 타일 (z, tx, ty) 의 메시 꼭짓점(화면 좌표). 북→남 행, 서→동 열 순서로 (STEPS+1)² 개.
#[must_use]
pub fn tile_mesh_points(
    view: &TileView,
    params: &TmParams,
    rect: Rect,
    z: u8,
    tx: u32,
    ty: u32,
) -> Option<Vec<Pos2>> {
    tile_grid_points(view, params, rect, z, tx, ty, TILE_MESH_STEPS)
}

/// The same grid with `n` cells along a side (a tile on the rim of a clip circle uses a finer one).
#[must_use]
pub fn tile_grid_points(
    view: &TileView,
    params: &TmParams,
    rect: Rect,
    z: u8,
    tx: u32,
    ty: u32,
    n: usize,
) -> Option<Vec<Pos2>> {
    tile_grid_points_in(TileGrid::WebMercator, view, params, rect, (z, tx, ty), n)
}

/// The mesh of a tile of any grid (`tile`: level, column, row), `n` cells along a side.
#[must_use]
pub fn tile_grid_points_in(grid: TileGrid, view: &TileView, params: &TmParams, rect: Rect, tile: (u8, u32, u32), n: usize) -> Option<Vec<Pos2>> {
    let (z, tx, ty) = tile;
    let mut pts = Vec::with_capacity((n + 1) * (n + 1));
    for j in 0..=n {
        for i in 0..=n {
            let fx = f64::from(tx) + i as f64 / n as f64;
            let fy = f64::from(ty) + j as f64 / n as f64;
            let (lon, lat) = grid_point_lon_lat(grid, z, fx, fy);
            let m = lonlat_to_tm(lon, lat, params);
            if !m[0].is_finite() || !m[1].is_finite() {
                return None;
            }
            pts.push(view.meters_to_screen(rect, m));
        }
    }
    Some(pts)
}

/// 메시를 텍스처로 칠한다. `uv` 는 텍스처에서 쓸 부분이다(전체면 0..1).
pub fn paint_tile_mesh(
    painter: &egui::Painter,
    texture: egui::TextureId,
    pts: &[Pos2],
    uv: Rect,
    tint: Color32,
) {
    let n = TILE_MESH_STEPS;
    let w = n + 1;
    if pts.len() != w * w {
        return;
    }
    let mut mesh = egui::Mesh::with_texture(texture);
    mesh.reserve_vertices(pts.len());
    mesh.reserve_triangles(2 * n * n);
    for j in 0..=n {
        for i in 0..=n {
            let u = uv.min.x + uv.width() * i as f32 / n as f32;
            let v = uv.min.y + uv.height() * j as f32 / n as f32;
            mesh.vertices.push(egui::epaint::Vertex {
                pos: pts[j * w + i],
                uv: Pos2::new(u, v),
                color: tint,
            });
        }
    }
    let w32 = w as u32;
    for j in 0..n as u32 {
        for i in 0..n as u32 {
            let a = j * w32 + i;
            let c = a + w32;
            mesh.add_triangle(a, a + 1, c);
            mesh.add_triangle(a + 1, c + 1, c);
        }
    }
    painter.add(egui::Shape::mesh(mesh));
}

/// 화면이 걸치는 타일 번호 범위 `(x0, y0, x1, y1)`.
#[must_use]
pub fn screen_tile_range(
    view: &TileView,
    params: &TmParams,
    rect: Rect,
    z: u8,
) -> Option<(u32, u32, u32, u32)> {
    screen_tile_range_in(TileGrid::WebMercator, view, params, rect, z)
}

/// The same for any grid. A grid that stands askew to longitude and latitude (EPSG:5179) is asked at all
/// four corners of the box, so that no tile of the screen is left out.
#[must_use]
pub fn screen_tile_range_in(grid: TileGrid, view: &TileView, params: &TmParams, rect: Rect, z: u8) -> Option<(u32, u32, u32, u32)> {
    let corners = [
        view.screen_to_meters(rect, rect.left_top()),
        view.screen_to_meters(rect, rect.right_top()),
        view.screen_to_meters(rect, rect.left_bottom()),
        view.screen_to_meters(rect, rect.right_bottom()),
    ];
    let mut lon_min = f64::INFINITY;
    let mut lon_max = f64::NEG_INFINITY;
    let mut lat_min = f64::INFINITY;
    let mut lat_max = f64::NEG_INFINITY;
    for m in corners {
        let [lon, lat] = tm_to_lonlat(m[0], m[1], params);
        if !lon.is_finite() || !lat.is_finite() {
            return None;
        }
        lon_min = lon_min.min(lon);
        lon_max = lon_max.max(lon);
        lat_min = lat_min.min(lat);
        lat_max = lat_max.max(lat);
    }
    let tiles = [(lon_min, lat_max), (lon_max, lat_max), (lon_min, lat_min), (lon_max, lat_min)].map(|(lon, lat)| grid_tile_of(grid, lon, lat, z));
    let (xs, ys) = (tiles.map(|tile| tile.0), tiles.map(|tile| tile.1));
    Some((xs.into_iter().min()?, ys.into_iter().min()?, xs.into_iter().max()?, ys.into_iter().max()?))
}

/// 화면 가운데의 위도(지적 줌 고르기에 쓴다).
#[must_use]
pub fn screen_center_lat(view: &TileView, params: &TmParams, rect: Rect) -> f64 {
    let m = view.screen_to_meters(rect, rect.center());
    tm_to_lonlat(m[0], m[1], params)[1]
}
