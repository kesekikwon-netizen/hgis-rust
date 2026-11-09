//! 고지형 파이프라인의 주상도 요약을 시추 도면 엔진의 입력으로 바꾸는 다리.
//!
//! C++ 은 `QgsVectorLayer` 속성에서 직접 읽지만(`GeoInfoBoreholeMaps.cpp:203-225`),
//! Rust 는 이미 파싱해 둔 `ParsedBoreholeSummary`(`HgisPaleoBorehole.cpp:1-147` 대응)를
//! `BoreholeRecord` 로 옮겨 쓴다. 두 구조가 같은 값을 다른 이름으로 들고 있기 때문이다.

use strata_net::geoinfo_borehole::{BoreholeRecord, StratumLayer};
use strata_paleo::borehole_strata::ParsedBoreholeSummary;

/// `GeoInfoBoreholeMaps.cpp:203-225` 가 읽는 다섯 값을 `ParsedBoreholeSummary` 에서 채운다.
///
/// C++ 는 `fill_m`, `alluvial_m`, `weathered_m`, `altitude_m`, `water_m`,
/// `hole_code`, `uscs` 속성을 읽는다. 여기서는 같은 값을 가진 필드를 옮긴다.
/// 지층 목록은 C++ `BoreholeRecord::layers` 처럼 매립 → 충적 → 풍화 세 층으로 만든다
/// (`HgisPaleoBorehole.cpp:221-223` 와 같은 구성).
#[must_use]
pub fn borehole_record_from_parsed(summary: &ParsedBoreholeSummary) -> BoreholeRecord {
    let fill_m = summary.fill_thickness_m.max(0.0);
    let alluv_m = summary.alluvial_thickness_m.max(0.0);
    let weathered_m = summary.weathered_top_depth_m;
    let mut layers = Vec::with_capacity(3);
    if fill_m > 0.0 {
        layers.push(StratumLayer {
            from_m: 0.0,
            to_m: fill_m,
            name: "매립토층".to_owned(),
            uscs: "SM".to_owned(),
        });
    }
    if alluv_m > 0.0 {
        layers.push(StratumLayer {
            from_m: fill_m,
            to_m: fill_m + alluv_m,
            name: "충적층".to_owned(),
            uscs: if summary.dominant_uscs.is_empty() {
                "SM".to_owned()
            } else {
                summary.dominant_uscs.clone()
            },
        });
    }
    let weath_top = weathered_m.max(fill_m + alluv_m);
    layers.push(StratumLayer {
        from_m: weath_top,
        to_m: weath_top + 3.0,
        name: "풍화토층".to_owned(),
        uscs: "SM".to_owned(),
    });

    BoreholeRecord {
        hole_code: summary.hole_id.clone(),
        tm_x: summary.x,
        tm_y: summary.y,
        altitude_m: summary.ground_elev_m,
        fill_depth_m: fill_m,
        alluvial_from_m: fill_m,
        alluvial_to_m: fill_m + alluv_m,
        alluvial_thickness_m: alluv_m,
        weathered_depth_m: weathered_m,
        // C++ `:218` 은 `std::abs(water_m)` 을 쓰고 `:222` 에서 `> 0.01` 로 거른다.
        // 여기서도 부호를 지워 넘기고, 거르는 일은 엔진이 한다.
        water_level_m: summary.water_depth_m.abs(),
        dominant_uscs: summary.dominant_uscs.clone(),
        paleo_hint: summary.sed_type.clone(),
        layers,
    }
}

/// 여러 주상도를 한꺼번에 옮긴다.
#[must_use]
pub fn borehole_records_from_parsed(summaries: &[ParsedBoreholeSummary]) -> Vec<BoreholeRecord> {
    summaries.iter().map(borehole_record_from_parsed).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ParsedBoreholeSummary {
        ParsedBoreholeSummary {
            hole_id: "BH-07".to_owned(),
            x: 218_700.0,
            y: 361_300.0,
            ground_elev_m: 38.4,
            fill_thickness_m: 1.2,
            alluvial_thickness_m: 4.2,
            weathered_top_depth_m: 5.4,
            water_depth_m: -2.1,
            paleo_surface_elev_m: 37.2,
            weathered_top_elev_m: 33.0,
            dominant_uscs: "CL".to_owned(),
            sed_type: "실트질점토".to_owned(),
            has_organic_inclusion: false,
            marker_color_hex: "#8BC34A".to_owned(),
        }
    }

    /// C++ `:213-224` 가 읽는 여섯 값이 그대로 넘어간다.
    #[test]
    fn test_bridge_keeps_the_six_cpp_fields() {
        let r = borehole_record_from_parsed(&sample());
        assert_eq!(r.hole_code, "BH-07");
        assert!((r.tm_x - 218_700.0).abs() < 1e-9);
        assert!((r.tm_y - 361_300.0).abs() < 1e-9);
        assert!((r.altitude_m - 38.4).abs() < 1e-9);
        assert!((r.fill_depth_m - 1.2).abs() < 1e-9);
        assert!((r.alluvial_thickness_m - 4.2).abs() < 1e-9);
        assert!((r.weathered_depth_m - 5.4).abs() < 1e-9);
        // `:218` abs()
        assert!((r.water_level_m - 2.1).abs() < 1e-9);
        assert_eq!(r.dominant_uscs, "CL");
    }

    /// `HgisPaleoBorehole.cpp:221-223` 처럼 매립 → 충적 → 풍화 세 층이 만들어진다.
    #[test]
    fn test_bridge_builds_three_strata_layers_like_cpp() {
        let r = borehole_record_from_parsed(&sample());
        assert_eq!(r.layers.len(), 3);
        assert_eq!(r.layers[0].name, "매립토층");
        assert!((r.layers[0].from_m - 0.0).abs() < 1e-9);
        assert!((r.layers[0].to_m - 1.2).abs() < 1e-9);
        assert_eq!(r.layers[1].name, "충적층");
        assert_eq!(r.layers[1].uscs, "CL");
        assert!((r.layers[1].from_m - 1.2).abs() < 1e-9);
        assert!((r.layers[1].to_m - 5.4).abs() < 1e-9);
        assert_eq!(r.layers[2].name, "풍화토층");
        assert!((r.layers[2].from_m - 5.4).abs() < 1e-9);
    }

    /// 매립이 0 이면 그 층을 만들지 않는다(C++ `if (fillM > 0.0)`).
    #[test]
    fn test_bridge_skips_zero_thickness_fill_layer() {
        let mut s = sample();
        s.fill_thickness_m = 0.0;
        let r = borehole_record_from_parsed(&s);
        assert_eq!(r.layers.len(), 2);
        assert_eq!(r.layers[0].name, "충적층");
        assert!((r.alluvial_from_m - 0.0).abs() < 1e-9);
    }

    /// 풍화대 시작이 매립+충적보다 얕으면 그 아래로 올린다(`HgisPaleoBorehole.cpp:216`
    /// `weathM = max(fillM + alluvM, weathered_m)`).
    #[test]
    fn test_bridge_clamps_weathered_start_below_alluvium() {
        let mut s = sample();
        s.weathered_top_depth_m = 0.5;
        let r = borehole_record_from_parsed(&s);
        let weath = r.layers.last().map(|l| l.from_m).unwrap_or(-1.0);
        assert!((weath - 5.4).abs() < 1e-9, "got {weath}");
    }

    /// 여러 공을 한 번에 옮긴다.
    #[test]
    fn test_bridge_maps_all_records() {
        let mut a = sample();
        let mut b = sample();
        b.hole_id = "BH-08".to_owned();
        a.hole_id = "BH-07".to_owned();
        let records = borehole_records_from_parsed(&[a, b]);
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].hole_code, "BH-07");
        assert_eq!(records[1].hole_code, "BH-08");
    }
}
