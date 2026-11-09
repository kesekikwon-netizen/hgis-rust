//! 등고선 종류 분류와 등고선 변 보간.
//!
//! `dem_synthesizer` 에서 분리했다.

pub const HGIS_CONTOUR_INDEX_FACTOR: i64 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContourType {
    Intermediate,
    Index,
}

/// C++ `KaMarchingSquaresEngine` — 등치선 모서리 선형 내분비 `t = (iso - z0) / (z1 - z0)` 산출 (`|z1 - z0| <= 1e-6` 평탄 모서리는 `0.5` 클램핑)
#[must_use]
pub fn interpolate_contour_edge_fraction(iso_elev_m: f64, z0: f64, z1: f64) -> f64 {
    let diff = z1 - z0;
    if diff.abs() <= 1e-6 || !diff.is_finite() {
        return 0.5;
    }
    ((iso_elev_m - z0) / diff).clamp(0.0, 1.0)
}

/// C++ `KaContourGenerator` — 주곡선 간격 `base_interval_m`의 5배수(`HGIS_CONTOUR_INDEX_FACTOR = 5`) 표고 여부를 판정하여 계곡선(`ContourType::Index`) 분류
#[must_use]
pub fn classify_contour_type(iso_elev_m: f64, base_interval_m: f64) -> ContourType {
    if base_interval_m <= 1e-6 || !base_interval_m.is_finite() {
        return ContourType::Intermediate;
    }
    let step = (iso_elev_m / base_interval_m).round() as i64;
    if step % HGIS_CONTOUR_INDEX_FACTOR == 0 {
        ContourType::Index
    } else {
        ContourType::Intermediate
    }
}
