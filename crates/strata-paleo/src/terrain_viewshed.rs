//! 도보 이동 속도·지구 곡률·가시권(viewshed).
//!
//! `terrain_edt_tpi` 에서 분리했다.

/// 지구 평균 반경 (m, C++ `HGIS_EARTH_RADIUS_M`)
pub const HGIS_EARTH_RADIUS_M: f64 = 6_371_000.0;

/// 대기 굴절 보정 기본 계수 (C++ `HGIS_REFRACTION_INDEX_K`)
pub const HGIS_REFRACTION_INDEX_K: f64 = 0.13;

/// 토블러 보행 함수 최대 속도 (km/h, C++ `HGIS_TOBLER_MAX_SPEED_KMH`)
pub const HGIS_TOBLER_MAX_SPEED_KMH: f64 = 6.0;

/// 최대 속도 최적 경사도 (탄젠트, C++ `HGIS_TOBLER_OPTIMAL_SLOPE`)
pub const HGIS_TOBLER_OPTIMAL_SLOPE: f64 = -0.05;

/// 통행 불가 단애 경사 임계치 (C++ `HGIS_IMPASSABLE_CLIFF_SLOPE`)
pub const HGIS_IMPASSABLE_CLIFF_SLOPE: f64 = 1.5;

/// 토블러 보행 함수(Tobler's Hiking Function) 보행 속도(`km/h`) 산출 (`W = 6 * exp(-3.5 * |S + 0.05|)`)
/// (C++ `KaToblerHikingFunction` 1:1 대응)
#[must_use]
pub fn compute_tobler_hiking_speed_kmh(slope_tan: f64) -> f64 {
    if !slope_tan.is_finite() || slope_tan.abs() >= HGIS_IMPASSABLE_CLIFF_SLOPE {
        return 0.0001;
    }
    (HGIS_TOBLER_MAX_SPEED_KMH * (-3.5 * (slope_tan - HGIS_TOBLER_OPTIMAL_SLOPE).abs()).exp())
        .max(0.0001)
}

/// 지구 곡률 및 대기 굴절(`k = 0.13`)을 반영한 수평 거리별 지형 겉보기 침하량(`m`) 산출
/// (C++ `KaViewshedAnalyzer` 1:1 대응)
#[must_use]
pub fn compute_earth_curvature_drop_m(distance_m: f64, refraction_k: f64) -> f64 {
    if distance_m <= 0.0 || !distance_m.is_finite() {
        return 0.0;
    }
    (distance_m * distance_m * (1.0 - refraction_k)) / (2.0 * HGIS_EARTH_RADIUS_M)
}

/// 유적·봉수·산성 간 상호 조망(Intervisibility) 기선 검증 결과 (C++ `IntervisibilityEdge`)
#[derive(Debug, Clone, PartialEq)]
pub struct IntervisibilityEdge {
    pub from_site_id: String,
    pub to_site_id: String,
    pub is_visible: bool,
    pub distance_m: f64,
    pub min_clearance_m: f64,
}

/// 출발점과 목표점 사이의 중간 지형 표고 샘플(`(dist_m, terrain_elev_m)`)에 대한 가시선(LOS) 차폐 및 여유 고도 판정
/// (C++ `KaIntervisibilityNetwork::buildIntervisibilityNetwork` 1:1 대응)
#[must_use]
pub fn evaluate_line_of_sight_profile(
    from_site_id: &str,
    to_site_id: &str,
    obs_elev_m: f64,
    target_elev_m: f64,
    distance_m: f64,
    intermediate_samples: &[(f64, f64)],
    refraction_k: f64,
) -> IntervisibilityEdge {
    if distance_m <= 1e-6 || !distance_m.is_finite() {
        return IntervisibilityEdge {
            from_site_id: from_site_id.to_owned(),
            to_site_id: to_site_id.to_owned(),
            is_visible: true,
            distance_m: 0.0,
            min_clearance_m: 0.0,
        };
    }
    let adj_target_el = target_elev_m - compute_earth_curvature_drop_m(distance_m, refraction_k);
    let mut min_clearance_m = f64::INFINITY;
    for &(d, z_terr) in intermediate_samples {
        if d <= 0.0 || d >= distance_m {
            continue;
        }
        let frac = d / distance_m;
        let los_z = obs_elev_m + (adj_target_el - obs_elev_m) * frac;
        let adj_terr_z = z_terr - compute_earth_curvature_drop_m(d, refraction_k);
        let clearance = los_z - adj_terr_z;
        min_clearance_m = min_clearance_m.min(clearance);
    }
    if !min_clearance_m.is_finite() {
        min_clearance_m = 0.0;
    }
    IntervisibilityEdge {
        from_site_id: from_site_id.to_owned(),
        to_site_id: to_site_id.to_owned(),
        is_visible: min_clearance_m >= 0.0,
        distance_m,
        min_clearance_m,
    }
}
