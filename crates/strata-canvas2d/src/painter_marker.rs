//! 유물 표식 크기·SVG 자원 경로.
//!
//! `canvas_painter` 에서 분리했다.

use crate::painter_hatch::HGIS_MM_TO_INCH;

pub const HGIS_DEFAULT_MARKER_SIZE_MM: f64 = 4.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizingMode {
    ScreenFixedMm,
    WorldProportionalM,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MarkerRenderContext {
    pub world_coord: [f64; 2],
    pub size_value: f64,
    pub sizing_mode: SizingMode,
    pub rotation_angle_deg: f64,
    pub magnetic_declination_deg: f64,
    pub override_fill_rgba: Option<[u8; 4]>,
}

/// C++ `KaSymbolLibraryManager` — 6대 유물 재질별 표준 SVG 리소스 식별자 매핑 (`HGIS_ARTIFACT_SVG_PATHS`)
#[must_use]
pub fn resolve_artifact_svg_resource_path(symbol_key: &str) -> Option<&'static str> {
    match symbol_key.trim().to_ascii_lowercase().as_str() {
        "pottery" | "pottery_comb" | "pottery_vessel" => {
            Some(":/symbols/artifacts/pottery_vessel.svg")
        }
        "stone" | "stone_axe" => Some(":/symbols/artifacts/stone_axe.svg"),
        "bronze" | "bronze_dagger" => Some(":/symbols/artifacts/bronze_dagger.svg"),
        "iron" | "iron_sickle" => Some(":/symbols/artifacts/iron_sickle.svg"),
        "tile" | "roof_tile" => Some(":/symbols/artifacts/roof_tile.svg"),
        "wood" | "wooden_craft" => Some(":/symbols/artifacts/wooden_craft.svg"),
        _ => None,
    }
}

/// C++ `KaArchaeologicalMarkerRenderer::renderArtifactMarker` — `ScreenFixedMm` (`sizeMm * dpi / 25.4`) 또는 `WorldProportionalM` (`sizeM / metersPerPixel`) 픽셀 크기 산출
#[must_use]
pub fn compute_marker_size_px(
    size_value: f64,
    sizing_mode: SizingMode,
    target_dpi: f64,
    meters_per_pixel: f64,
) -> f64 {
    let val = if size_value > 0.0 && size_value.is_finite() {
        size_value
    } else {
        HGIS_DEFAULT_MARKER_SIZE_MM
    };
    match sizing_mode {
        SizingMode::ScreenFixedMm => {
            let dpi = if target_dpi > 0.0 && target_dpi.is_finite() {
                target_dpi
            } else {
                96.0
            };
            val * (dpi * HGIS_MM_TO_INCH)
        }
        SizingMode::WorldProportionalM => {
            let mpp = if meters_per_pixel > 1e-9 && meters_per_pixel.is_finite() {
                meters_per_pixel
            } else {
                1.0
            };
            val / mpp
        }
    }
}
