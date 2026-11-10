//! 배경지도 좌표 변환 — 화면(px) ↔ work-CRS 미터 ↔ EPSG:3857 · 경위도.
//!
//! asemap_paint 가 300줄을 넘어 분리했다.

use eframe::egui;
use strata_geo::crs_transform::tm_to_lonlat;

use crate::basemap_paint::{tm_params_for, TileView};

/// 화면 사각형을 EPSG:3857 미터 경계 (minx, miny, maxx, maxy) 로 바꾼다.
#[must_use]
pub fn screen_bbox_3857(
    vp: &strata_canvas2d::viewport::ViewportTransform,
    rect: egui::Rect,
    work_crs: strata_types::crs::WorkCrs,
) -> Option<[f64; 4]> {
    let view = TileView::from_viewport(vp, rect, work_crs);
    let params = tm_params_for(work_crs)?;
    let corners = [
        view.screen_to_meters(rect, rect.left_top()),
        view.screen_to_meters(rect, rect.right_top()),
        view.screen_to_meters(rect, rect.left_bottom()),
        view.screen_to_meters(rect, rect.right_bottom()),
    ];
    let mut minx = f64::INFINITY;
    let mut miny = f64::INFINITY;
    let mut maxx = f64::NEG_INFINITY;
    let mut maxy = f64::NEG_INFINITY;
    for m in corners {
        let [lon, lat] = tm_to_lonlat(m[0], m[1], &params);
        let (x, y) = lon_lat_to_3857(lon, lat);
        if !x.is_finite() || !y.is_finite() {
            return None;
        }
        minx = minx.min(x);
        maxx = maxx.max(x);
        miny = miny.min(y);
        maxy = maxy.max(y);
    }
    (maxx > minx && maxy > miny).then_some([minx, miny, maxx, maxy])
}

/// 경위도 → EPSG:3857 미터.
fn lon_lat_to_3857(lon: f64, lat: f64) -> (f64, f64) {
    const R: f64 = 6_378_137.0;
    let x = lon.to_radians() * R;
    let y = ((std::f64::consts::FRAC_PI_4 + lat.to_radians() * 0.5).tan()).ln() * R;
    (x, y)
}

/// 디스크의 이미지 파일을 egui 텍스처로 올린다.
#[must_use]
pub fn load_texture_from_path(
    ctx: &egui::Context,
    name: &str,
    path: &std::path::Path,
) -> Option<egui::TextureHandle> {
    let img = image::open(path).ok()?;
    let rgba = img.to_rgba8();
    let (w, h) = (rgba.width() as usize, rgba.height() as usize);
    if w == 0 || h == 0 {
        return None;
    }
    let color = egui::ColorImage::from_rgba_unmultiplied([w, h], rgba.as_raw());
    Some(ctx.load_texture(name.to_owned(), color, egui::TextureOptions::LINEAR))
}