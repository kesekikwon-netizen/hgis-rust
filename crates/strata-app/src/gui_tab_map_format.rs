//! 지도 탭 HUD 문구와 색 변조.

use eframe::egui::{Color32, Pos2, Rect, Vec2};
pub const GRID_BAND_PX: f32 = 18.0;
pub const GRID_BAND_FILL: Color32 = Color32::from_rgb(0xF5, 0xF4, 0xED);
pub const GRID_BAND_BORDER: Color32 = Color32::from_rgb(0xDE, 0xDC, 0xD1);
pub const GRID_BAND_INK: Color32 = Color32::from_rgb(0x5E, 0x5D, 0x59);

pub fn grid_band_rects(rect: Rect) -> (Rect, Rect) {
    (
        Rect::from_min_size(rect.min, Vec2::new(rect.width(), GRID_BAND_PX)),
        Rect::from_min_size(
            Pos2::new(rect.left(), rect.top() + GRID_BAND_PX),
            Vec2::new(GRID_BAND_PX, rect.height() - GRID_BAND_PX),
        ),
    )
}
pub(crate) fn modulate_color_with_opacity_brightness(base: Color32, opacity: f32, brightness: f32) -> Color32 {
    let op = opacity.clamp(0.0, 1.0);
    let br_shift = ((brightness.clamp(0.0, 1.0) - 0.5) * 2.0 * 255.0).round() as i32;
    let r = (i32::from(base.r()) + br_shift).clamp(0, 255) as u8;
    let g = (i32::from(base.g()) + br_shift).clamp(0, 255) as u8;
    let b = (i32::from(base.b()) + br_shift).clamp(0, 255) as u8;
    let a = (f32::from(base.a()) * op).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgba_unmultiplied(r, g, b, a)
}

/// `KaLayerOpacityRail` 밝기 줄의 세 가지 모양 — 투명도 / 그림 레이어 밝기 / 벡터 「그림만」
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrightRow {
    Opacity,
    Raster,
    Vector,
}
