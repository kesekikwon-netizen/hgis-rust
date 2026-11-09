//! C++ `GeoInfoBoreholeMaps.cpp:45-81`(`addIdwMap`)·`:83-166`(`addIsopachContourLayer`)의
//! **화면에 칠하는 부분**을 옮긴 모듈.
//!
//! C++ 은 QGIS 레이어로 등록해 QGIS 가 칠하지만, Rust 에는 레이어 계층이 없으므로
//! `strata_paleo` 가 돌려준 격자를 램프로 색칠해 egui 텍스처로 직접 그린다.
//!
//! 이 모듈이 없으면 파일(`.tif`/`.gpkg`)만 만들어지고 **화면에는 아무 색도 안 나온다.**
//! 그래서 시험은 `rasterize_ramp_rgba` 가 실제 색 픽셀을 내는지까지 확인한다.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke};

use strata_paleo::borehole_color_ramp::{rasterize_ramp_rgba, BoreholeColorRamp};
use strata_paleo::borehole_maps::BoreholeMapBundle;

use crate::app_state::StrataAppState;

/// 칠할 수 있는 도면 하나 — 격자 + 램프 + 표제.
#[derive(Debug, Clone, PartialEq)]
pub struct PaintableBoreholeMap {
    pub title: String,
    pub ramp: BoreholeColorRamp,
    /// 1채널 격자(C++ `writeFloatGeoTiff` 가 쓴 것과 같은 값).
    pub grid: Vec<f64>,
}

/// `BoreholeMapBundle` 에서 화면에 칠할 수 있는 도면만 뽑는다.
///
/// `painted_grids` 와 램프가 있는 레이어의 순서를 짝지어 돌려준다.
/// 마커 맵은 파일·격자가 없으므로 제외한다(화면에는 점 레이어로 따로 그린다).
#[must_use]
pub fn paintable_maps(bundle: &BoreholeMapBundle) -> Vec<PaintableBoreholeMap> {
    bundle
        .layers
        .iter()
        .filter_map(|layer| layer.ramp.clone())
        .zip(bundle.painted_grids.iter())
        .map(|(ramp, grid)| PaintableBoreholeMap {
            title: bundle
                .layers
                .iter()
                .find(|l| l.ramp.as_ref() == Some(&ramp))
                .map(|l| l.title.clone())
                .unwrap_or_else(|| "시추 도면".to_owned()),
            ramp,
            grid: grid.clone(),
        })
        .collect()
}

/// 격자 하나를 `map_rect` 에 맞춰 칠한다. 성공하면 `true`.
///
/// C++ `:71` 의 `setOpacity(0.72)` 는 램프 알파(`BOREHOLE_RASTER_OPACITY`)에 이미 들어 있다.
pub fn paint_borehole_map(
    ctx: &egui::Context,
    painter: &egui::Painter,
    map_rect: Rect,
    map: &PaintableBoreholeMap,
    width: usize,
    height: usize,
    texture_id: &str,
) -> bool {
    if width == 0 || height == 0 || map.grid.len() < width * height {
        return false;
    }
    let rgba = rasterize_ramp_rgba(&map.grid, &map.ramp);
    let img = egui::ColorImage::from_rgba_unmultiplied([width, height], &flatten_rgba(&rgba));
    let tex = ctx.load_texture(texture_id, img, egui::TextureOptions::LINEAR);
    painter.image(
        tex.id(),
        map_rect,
        Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
        Color32::WHITE,
    );
    true
}

/// `[u8; 4]` 목록을 egui 가 먹는 평평한 RGBA8 버퍼로 바꾼다.
#[must_use]
pub fn flatten_rgba(px: &[[u8; 4]]) -> Vec<u8> {
    let mut out = Vec::with_capacity(px.len() * 4);
    for p in px {
        out.extend_from_slice(p);
    }
    out
}

/// 지금 고른 시추 도면을 지도 위에 칠하고, 무엇을 칠했는지 한 줄로 돌려준다.
///
/// `shown_borehole_map_idx` 가 `None` 이거나 칠할 도면이 없으면 아무것도 하지 않는다.
#[must_use]
pub fn render_borehole_map_overlay(
    app: &mut StrataAppState,
    ctx: &egui::Context,
    painter: &egui::Painter,
    map_rect: Rect,
) -> Option<String> {
    let Some(bundle) = app.paleo_3d.last_borehole_maps.as_ref() else {
        return None;
    };
    let idx = app.paleo_3d.shown_borehole_map_idx?;
    let maps = paintable_maps(bundle);
    let map = maps.get(idx)?;
    let width = bundle.grid.width;
    let height = bundle.grid.height;
    let id = format!("borehole_map_overlay_{idx}");
    if !paint_borehole_map(ctx, painter, map_rect.shrink(4.0), map, width, height, &id) {
        return None;
    }
    painter.rect_stroke(
        map_rect.shrink(4.0),
        2.0,
        Stroke::new(1.0, Color32::from_rgb(0x92, 0x40, 0x0E)),
        egui::StrokeKind::Middle,
    );
    Some(map.title.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use strata_net::geoinfo_borehole::BoreholeRecord;
    use strata_paleo::borehole_map_options::BoreholeMapOptions;
    use strata_paleo::borehole_maps::generate_borehole_gis_maps;
    use strata_types::crs::WorkCrs;

    fn hole(x: f64, y: f64, fill: f64, alluv: f64, weath: f64) -> BoreholeRecord {
        BoreholeRecord {
            hole_code: "BH".to_owned(),
            tm_x: x,
            tm_y: y,
            altitude_m: 35.0,
            fill_depth_m: fill,
            alluvial_from_m: fill,
            alluvial_to_m: fill + alluv,
            alluvial_thickness_m: alluv,
            weathered_depth_m: weath,
            water_level_m: 2.0,
            dominant_uscs: "SM".to_owned(),
            paleo_hint: String::new(),
            layers: Vec::new(),
        }
    }

    /// 시험마다 다른 임시 폴더를 쓴다. 같은 이름을 쓰면 병렬 실행에서 두 시험이
    /// 같은 폴더를 지우며 서로를 깨뜨린다(실제로 그렇게 실패했다).
    fn bundle_with_maps(tag: &str) -> BoreholeMapBundle {
        let dir = std::env::temp_dir().join(format!(
            "ka_hgis_paintable_{}_{tag}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let holes = vec![
            hole(200_000.0, 400_000.0, 1.0, 3.0, 8.0),
            hole(200_500.0, 400_000.0, 2.0, 4.0, 9.5),
            hole(200_000.0, 400_500.0, 0.5, 2.0, 6.0),
        ];
        let opts = BoreholeMapOptions {
            radius_meters: 5000,
            marker_map: true,
            alluvial_isopach_map: true,
            fill_thickness_map: true,
            bedrock_depth_map: true,
            bedrock_elev_map: true,
            groundwater_map: true,
        };
        let b = generate_borehole_gis_maps(&holes, &[], WorkCrs::Epsg5187, &dir, &opts)
            .expect("generation");
        let _ = std::fs::remove_dir_all(&dir);
        b
    }

    /// 칠할 수 있는 도면이 다섯 장 나오고 각각 격자·램프·표제를 갖는다.
    #[test]
    fn five_paintable_maps_extracted_from_bundle() {
        let bundle = bundle_with_maps("five");
        let maps = paintable_maps(&bundle);
        assert_eq!(maps.len(), 5);
        for m in &maps {
            assert!(!m.grid.is_empty(), "{}: grid", m.title);
            assert!(!m.ramp.stops.is_empty(), "{}: ramp", m.title);
            assert!(!m.title.is_empty(), "title must not be empty");
        }
    }

    /// 칠하기 경로가 실제로 불투명 픽셀을 낸다 — 화면이 비어 있지 않음을 보장한다.
    #[test]
    fn painting_produces_visible_pixels_not_blank_canvas() {
        let bundle = bundle_with_maps("paint");
        let maps = paintable_maps(&bundle);
        let map = maps.first().expect("at least one map");
        let rgba = rasterize_ramp_rgba(&map.grid, &map.ramp);
        let flat = flatten_rgba(&rgba);
        assert_eq!(flat.len(), map.grid.len() * 4);
        let visible = rgba.iter().filter(|p| p[3] > 0).count();
        assert!(visible > 0, "no visible pixel: canvas would look empty");
        assert_eq!(
            visible,
            map.grid.iter().filter(|v| v.is_finite()).count(),
            "every finite cell must be painted"
        );
    }

    /// `flatten_rgba` 는 채널 순서를 바꾸지 않는다.
    #[test]
    fn flatten_keeps_channel_order() {
        let flat = flatten_rgba(&[[1, 2, 3, 4], [5, 6, 7, 8]]);
        assert_eq!(flat, vec![1, 2, 3, 4, 5, 6, 7, 8]);
    }
}
