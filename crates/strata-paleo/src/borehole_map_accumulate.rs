//! C++ `GeoInfoBoreholeMaps.cpp:203-241` — 시추공 속성 순회 누적과 KIGAM 지하수위 결합.
//!
//! `generate_borehole_gis_maps` (오케스트레이터) 에서 분리했다.

use strata_net::geoinfo_borehole::BoreholeRecord;

/// C++ `waterGl > 0.01` 임계 (`GeoInfoBoreholeMaps.cpp:222, 236`).
pub const BOREHOLE_WATER_MIN_DEPTH_M: f64 = 0.01;

/// C++ `bedrockElevMap` 의 `mx = std::max(mn + 4.0, *mxIt)` 클램프
/// (`GeoInfoBoreholeMaps.cpp:279`).
pub const BOREHOLE_BEDROCK_ELEV_MIN_RANGE_M: f64 = 4.0;

/// C++ `:203-225` 순회 결과.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BoreholeAccumulation {
    pub points: Vec<[f64; 2]>,
    pub alluvial: Vec<f64>,
    pub fill: Vec<f64>,
    pub bedrock_gl: Vec<f64>,
    pub bedrock_el: Vec<f64>,
    /// `waterGl > 0.01` 인 시추공의 좌표 (`:222` 의 `wx/wy`).
    pub groundwater_points: Vec<[f64; 2]>,
    /// 위 좌표와 같은 순서의 지하수위 깊이 (`:222` 의 `vWater`).
    pub groundwater: Vec<f64>,
}

/// C++ `bed_gl` 폴백 규칙 (`GeoInfoBoreholeMaps.cpp:216`).
///
/// `rawWeathered >= 0.0 ? rawWeathered : (fillM + alluvM)` 이고,
/// `fillM`/`alluvM` 은 각각 `max(0.0, ...)` 로 클램프한 값이다(`:213-214`).
#[must_use]
pub fn bedrock_gl_with_fallback(raw_weathered: f64, fill_m: f64, alluv_m: f64) -> f64 {
    let fill = fill_m.max(0.0);
    let alluv = alluv_m.max(0.0);
    if raw_weathered >= 0.0 {
        raw_weathered
    } else {
        fill + alluv
    }
}

/// C++ `:213-224` 속성 계산을 1:1 로 옮긴다.
///
/// - `fillM = max(0.0, fill_m)` (`:213`)
/// - `alluvM = max(0.0, alluvial_m)` (`:214`)
/// - `bedGl = rawWeathered >= 0.0 ? rawWeathered : (fillM + alluvM)` (`:216`)
/// - `bedEl = altM - bedGl` (`:221`)
/// - `waterGl = abs(water_m)` 이고 `> 0.01` 인 것만 지하수위 집합에 포함 (`:218, 222`)
#[must_use]
pub fn accumulate_boreholes(holes: &[BoreholeRecord]) -> BoreholeAccumulation {
    let mut acc = BoreholeAccumulation::default();
    for h in holes {
        if !h.tm_x.is_finite() || !h.tm_y.is_finite() {
            continue;
        }
        let fill_m = h.fill_depth_m.max(0.0);
        let alluv_m = h.alluvial_thickness_m.max(0.0);
        let bed_gl = bedrock_gl_with_fallback(h.weathered_depth_m, fill_m, alluv_m);
        let alt_m = h.altitude_m;
        let water_gl = h.water_level_m.abs();

        acc.points.push([h.tm_x, h.tm_y]);
        acc.alluvial.push(alluv_m);
        acc.fill.push(fill_m);
        acc.bedrock_gl.push(bed_gl);
        acc.bedrock_el.push(alt_m - bed_gl);
        if water_gl > BOREHOLE_WATER_MIN_DEPTH_M {
            acc.groundwater_points.push([h.tm_x, h.tm_y]);
            acc.groundwater.push(water_gl);
        }
    }
    acc
}

/// KIGAM 결합 지하수위 집합 — C++ `:222` + `:235-239`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GroundwaterSet {
    pub points: Vec<[f64; 2]>,
    pub values: Vec<f64>,
}

/// 시추 지하수위(`:222`) 에 KIGAM 수위(`:235-239`) 를 이어 붙인다.
#[must_use]
pub fn accumulate_groundwater(
    accumulation: &BoreholeAccumulation,
    kigam_wells: &[[f64; 3]],
) -> GroundwaterSet {
    // C++ `:239` — `wx.push_back(pt.x()); wy.push_back(pt.y()); vWater.push_back(w);`
    let mut set = GroundwaterSet {
        points: accumulation.groundwater_points.clone(),
        values: accumulation.groundwater.clone(),
    };
    // C++ `:235-239`: `w = abs(water_gl_m)`, `w <= 0.01` 은 건너뛴다.
    for w in kigam_wells {
        if !w[0].is_finite() || !w[1].is_finite() {
            continue;
        }
        let depth = w[2].abs();
        if depth <= BOREHOLE_WATER_MIN_DEPTH_M {
            continue;
        }
        set.points.push([w[0], w[1]]);
        set.values.push(depth);
    }
    set
}

/// C++ `:278-279` — `mn = min(vBedEl)`, `mx = max(mn + 4.0, max(vBedEl))`.
#[must_use]
pub fn bedrock_elev_range(values: &[f64]) -> (f64, f64) {
    let mut mn = f64::INFINITY;
    let mut mx = f64::NEG_INFINITY;
    for &v in values {
        if v.is_finite() {
            mn = mn.min(v);
            mx = mx.max(v);
        }
    }
    if !mn.is_finite() {
        return (0.0, BOREHOLE_BEDROCK_ELEV_MIN_RANGE_M);
    }
    (mn, mx.max(mn + BOREHOLE_BEDROCK_ELEV_MIN_RANGE_M))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hole(
        x: f64,
        y: f64,
        alt: f64,
        fill: f64,
        alluv: f64,
        weathered: f64,
        water: f64,
    ) -> BoreholeRecord {
        BoreholeRecord {
            hole_code: "BH".to_owned(),
            tm_x: x,
            tm_y: y,
            altitude_m: alt,
            fill_depth_m: fill,
            alluvial_from_m: fill,
            alluvial_to_m: fill + alluv,
            alluvial_thickness_m: alluv,
            weathered_depth_m: weathered,
            water_level_m: water,
            dominant_uscs: "SM".to_owned(),
            paleo_hint: String::new(),
            layers: Vec::new(),
        }
    }

    #[test]
    fn bedrock_gl_fallback_uses_weathered_when_present_else_fill_plus_alluv() {
        // 출현: weathered >= 0 이면 그 값 (`:216`)
        assert_eq!(bedrock_gl_with_fallback(8.0, 1.0, 3.0), 8.0);
        assert_eq!(bedrock_gl_with_fallback(0.0, 5.0, 5.0), 0.0);
        // 미출현(-1): fillM + alluvM
        assert_eq!(bedrock_gl_with_fallback(-1.0, 2.0, 4.0), 6.0);
        // 음수 속성은 0 으로 클램프 (`:213-214`)
        assert_eq!(bedrock_gl_with_fallback(-1.0, -2.0, 4.0), 4.0);
    }

    #[test]
    fn accumulation_matches_cpp_water_filter_and_bedrock_elev() {
        let holes = vec![
            hole(200_000.0, 400_000.0, 35.0, 1.0, 3.0, 8.0, 2.5),
            hole(200_500.0, 400_000.0, 37.0, 2.0, 4.0, 9.5, 3.0),
            hole(200_000.0, 400_500.0, 33.0, 0.5, 2.0, -1.0, 0.0),
        ];
        let acc = accumulate_boreholes(&holes);
        assert_eq!(acc.points.len(), 3);
        // BH-3 은 weathered=-1 이므로 bed_gl = 0.5 + 2.0 = 2.5, bed_el = 33.0 - 2.5 = 30.5
        assert!((acc.bedrock_gl[2] - 2.5).abs() < 1e-12);
        assert!((acc.bedrock_el[2] - 30.5).abs() < 1e-12);
        // BH-1 bed_gl = 8.0, bed_el = 35 - 8 = 27
        assert!((acc.bedrock_el[0] - 27.0).abs() < 1e-12);
        // 지하수위 0.0 인 BH-3 은 제외 (`> 0.01`)
        assert_eq!(acc.groundwater.len(), 2);
        // 지하수위 점은 시추공 좌표와 같은 순서로 대응한다.
        assert_eq!(acc.groundwater_points[0], [200_000.0, 400_000.0]);
        assert_eq!(acc.groundwater_points[1], [200_500.0, 400_000.0]);
    }

    #[test]
    fn groundwater_merges_kigam_wells_and_filters_shallow() {
        let acc = accumulate_boreholes(&[
            hole(0.0, 0.0, 10.0, 1.0, 2.0, 5.0, 2.5),
            hole(100.0, 0.0, 10.0, 1.0, 2.0, 5.0, 3.0),
        ]);
        let kigam = vec![
            [200_250.0, 400_250.0, 4.2],
            [200_750.0, 400_750.0, 0.005], // <= 0.01 -> 제외
            [200_900.0, 400_900.0, -5.5],  // abs -> 5.5 포함
            [f64::NAN, 0.0, 9.9],          // 좌표 비유한 -> 제외
        ];
        let gw = accumulate_groundwater(&acc, &kigam);
        // 시추 2점 + KIGAM 유효 2점
        assert_eq!(gw.points.len(), 4);
        assert_eq!(gw.values.len(), 4);
        assert!((gw.values[0] - 2.5).abs() < 1e-12);
        assert!((gw.values[2] - 4.2).abs() < 1e-12);
        assert!((gw.values[3] - 5.5).abs() < 1e-12);
    }

    #[test]
    fn bedrock_elev_range_applies_min_4m_clamp() {
        // mx - mn < 4 이면 mx = mn + 4 (`:279`)
        let (mn, mx) = bedrock_elev_range(&[27.0, 27.5, 28.0]);
        assert_eq!(mn, 27.0);
        assert!((mx - 31.0).abs() < 1e-12);
        // 이미 4m 이상이면 그대로
        let (mn, mx) = bedrock_elev_range(&[10.0, 30.0]);
        assert_eq!(mn, 10.0);
        assert_eq!(mx, 30.0);
        // 빈 입력 / 전부 비유한 방어
        let (mn, mx) = bedrock_elev_range(&[]);
        assert!((mx - mn - 4.0).abs() < 1e-12);
        let (mn, mx) = bedrock_elev_range(&[f64::NAN, f64::INFINITY]);
        assert!((mx - mn - 4.0).abs() < 1e-12);
    }

    #[test]
    fn non_finite_coordinates_are_skipped() {
        let acc = accumulate_boreholes(&[
            hole(f64::NAN, 0.0, 10.0, 1.0, 2.0, 5.0, 2.5),
            hole(0.0, f64::INFINITY, 10.0, 1.0, 2.0, 5.0, 2.5),
            hole(50.0, 60.0, 10.0, 1.0, 2.0, 5.0, 2.5),
        ]);
        assert_eq!(acc.points.len(), 1);
        assert_eq!(acc.points[0], [50.0, 60.0]);
    }
}
