//! 굴착 안전성 평가와 단면선 투영.
//!
//! `borehole_strata` 에서 분리했다.

use crate::borehole_types::BoreholeRawInput;

pub const HGIS_BOREHOLE_SEARCH_RADIUS_M: f64 = 100.0;

pub const HGIS_WATER_TABLE_WARN_THRESH_M: f64 = 0.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExcavationSafetyReport {
    pub has_water_table_breach: bool,
    pub max_breach_depth_m: f64,
    pub min_excavation_elevation_m: f64,
    pub interpolated_water_table_m: f64,
}

/// C++ `KaExcavationSafetyChecker::evaluateExcavationSafety` — 주변 시추공 지하수위 IDW 보간 및 굴착 계획 바닥 표고 지하수위 침범(`waterTableBreachM = Z_wt - Z_plan > 0.0`) 판정
#[must_use]
pub fn evaluate_excavation_safety(
    target_xy: [f64; 2],
    plan_bottom_elevation_m: f64,
    boreholes: &[BoreholeRawInput],
    search_radius_m: f64,
) -> Option<ExcavationSafetyReport> {
    if boreholes.is_empty() {
        return None;
    }
    let r1 = if search_radius_m > 0.0 && search_radius_m.is_finite() {
        search_radius_m
    } else {
        HGIS_BOREHOLE_SEARCH_RADIUS_M
    };
    let interpolate_within = |radius: f64| -> Option<f64> {
        let mut sum_w = 0.0;
        let mut sum_wv = 0.0;
        for bh in boreholes {
            let wt_el = bh.ground_elev_m - bh.water_depth_m;
            let d = (target_xy[0] - bh.x).hypot(target_xy[1] - bh.y);
            if d < 1e-6 {
                return Some(wt_el);
            }
            if d <= radius {
                let w = 1.0 / (d * d);
                sum_w += w;
                sum_wv += w * wt_el;
            }
        }
        if sum_w > 0.0 {
            Some(sum_wv / sum_w)
        } else {
            None
        }
    };
    let wt_el = interpolate_within(r1).or_else(|| interpolate_within(500.0))?;
    let breach = wt_el - plan_bottom_elevation_m;
    Some(ExcavationSafetyReport {
        has_water_table_breach: breach > HGIS_WATER_TABLE_WARN_THRESH_M,
        max_breach_depth_m: breach.max(0.0),
        min_excavation_elevation_m: plan_bottom_elevation_m,
        interpolated_water_table_m: wt_el,
    })
}

/// 단면선 최대 허용 시추공 수직 이격 거리 (50.0m, C++ `HGIS_BOREHOLE_MAX_PERP_DIST_M`)
pub const HGIS_BOREHOLE_MAX_PERP_DIST_M: f64 = 50.0;

/// 단면선 상 시추공 정사영 결과 (C++ `KaPaleoCrossSectionWidget` 투영 레코드)
#[derive(Debug, Clone, PartialEq)]
pub struct BoreholeSectionProjection {
    pub hole_id: String,
    pub proj_distance_m: f64,
    pub perp_distance_m: f64,
    pub within_tolerance: bool,
}

/// 시추공 좌표(`borehole_xy`)를 단면선(`line_a` -> `line_b`)에 정사영하여 투영 거리(`d_proj`)와 수직 이격 거리(`d_perp`) 산출
/// (C++ `KaPaleoCrossSectionWidget::setCrossSectionData` 1:1 대응)
#[must_use]
pub fn project_borehole_onto_section_line(
    hole_id: &str,
    borehole_xy: [f64; 2],
    line_a: [f64; 2],
    line_b: [f64; 2],
    max_perp_dist_m: f64,
) -> BoreholeSectionProjection {
    let vx = line_b[0] - line_a[0];
    let vy = line_b[1] - line_a[1];
    let len = vx.hypot(vy);
    if len <= 1e-9 || !len.is_finite() {
        return BoreholeSectionProjection {
            hole_id: hole_id.to_owned(),
            proj_distance_m: 0.0,
            perp_distance_m: f64::INFINITY,
            within_tolerance: false,
        };
    }
    let dx = borehole_xy[0] - line_a[0];
    let dy = borehole_xy[1] - line_a[1];
    let d_proj = (dx * vx + dy * vy) / len;
    let d_perp = (dx * vy - dy * vx).abs() / len;
    let tol = if max_perp_dist_m.is_finite() && max_perp_dist_m > 0.0 {
        max_perp_dist_m
    } else {
        HGIS_BOREHOLE_MAX_PERP_DIST_M
    };
    BoreholeSectionProjection {
        hole_id: hole_id.to_owned(),
        proj_distance_m: d_proj,
        perp_distance_m: d_perp,
        within_tolerance: (0.0..=len).contains(&d_proj) && d_perp <= tol,
    }
}
