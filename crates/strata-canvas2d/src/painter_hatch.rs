//! 해치 패턴 간격과 점선 패턴 축척.
//!
//! `canvas_painter` 에서 분리했다.

pub const HGIS_MM_TO_INCH: f64 = 1.0 / 25.4;

pub const HGIS_MIN_HATCH_SPACING_MM: f64 = 0.5;

pub const HGIS_DEFAULT_HATCH_ANGLE_DEG: f64 = 45.0;

pub const HGIS_DASH_DOT_PATTERN: [f64; 4] = [4.0, 2.0, 1.0, 2.0];

pub const HGIS_DASH_PATTERN: [f64; 2] = [3.0, 3.0];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HatchPatternType {
    SingleDiagonal,
    CrossHatch,
    DotStipple,
    HorizontalHatch,
}

/// C++ `KaHatchPatternRenderer::renderPolygonHatch` — 해칭 간격 `spacingMm`의 최소값(`0.5mm`) 클램핑 및 목표 DPI 기준 픽셀 간격(`spacingMm * dpi / 25.4`) 산출
#[must_use]
pub fn compute_hatch_spacing_px(spacing_mm: f64, target_dpi: f64) -> f64 {
    let safe_mm = if spacing_mm.is_finite() {
        spacing_mm.max(HGIS_MIN_HATCH_SPACING_MM)
    } else {
        HGIS_MIN_HATCH_SPACING_MM
    };
    let dpi = if target_dpi > 0.0 && target_dpi.is_finite() {
        target_dpi
    } else {
        96.0
    };
    safe_mm * (dpi * HGIS_MM_TO_INCH)
}

/// C++ `KaLineDashStyleEngine::createScaledDashPen` — 물리 선폭(`lineWidthMm`)과 출력 DPI에 비례 스케일링된 대시/갭 픽셀 배열 산출
#[must_use]
pub fn scale_dash_pattern_px(pattern: &[f64], line_width_mm: f64, target_dpi: f64) -> Vec<f64> {
    let line_w_px = (line_width_mm.max(0.05) * (target_dpi.max(1.0) * HGIS_MM_TO_INCH)).max(1.0);
    pattern.iter().map(|&u| u * line_w_px).collect()
}
