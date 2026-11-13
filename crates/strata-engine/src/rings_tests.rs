//! 주변 거리 경계 — C++ tests/test_buffer.cpp 의 기댓값.

use super::*;

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> String {
    format!("POLYGON (({x0} {y0}, {x1} {y0}, {x1} {y1}, {x0} {y1}, {x0} {y0}))")
}

/// ported from tests/test_buffer.cpp:ringHasGapAndLabel — 경계선 하나, 거리 글자 넷, 경계 상자 ±12 m.
#[test]
fn ring_has_one_line_and_four_labels() {
    let rings = distance_rings(&[rect(200_000.0, 450_000.0, 200_080.0, 450_060.0)], 500.0).expect("rings");
    assert_eq!(rings.len(), 1);
    assert_eq!(ring_label_points(&rings[0]).expect("labels").len(), 4);
    let xs = rings[0].iter().map(|p| p[0]);
    let ys = rings[0].iter().map(|p| p[1]);
    let (x0, x1) = (xs.clone().fold(f64::INFINITY, f64::min), xs.fold(f64::NEG_INFINITY, f64::max));
    let (y0, y1) = (ys.clone().fold(f64::INFINITY, f64::min), ys.fold(f64::NEG_INFINITY, f64::max));
    for (got, want) in [(x0, 199_500.0), (x1, 200_580.0), (y0, 449_500.0), (y1, 450_560.0)] {
        assert!((got - want).abs() < 12.0, "{got} vs {want}");
    }
}

/// ported from tests/test_buffer.cpp:offsetIs500mFromSource — 경계선은 원래 면에서 490~510 m(점들은 498~502 m).
#[test]
fn ring_is_500m_from_the_source() {
    let src = rect(200_000.0, 450_000.0, 200_026.0, 450_026.0);
    let rings = distance_rings(std::slice::from_ref(&src), 500.0).expect("rings");
    let source = Geometry::new_from_wkt(&src).expect("source");
    let coords: Vec<String> = rings[0].iter().map(|p| format!("{} {}", p[0], p[1])).collect();
    let line = Geometry::new_from_wkt(&format!("LINESTRING ({})", coords.join(", "))).expect("line");
    let min_d = source.distance(&line).expect("distance");
    assert!(min_d > 490.0 && min_d < 510.0, "minD={min_d}");
    let len = line.length().expect("length");
    assert!(len > 100.0);
    let mut max_d = 0.0_f64;
    for i in 0..64 {
        let p = line.interpolate(len * f64::from(i) / 64.0).expect("point");
        max_d = max_d.max(p.distance(&source).expect("distance"));
    }
    assert!(max_d > 498.0 && max_d < 502.0, "maxD={max_d}");
}

/// ported from tests/test_buffer.cpp:multiPartSurveyArea_ringsEveryPart — 1 km 안의 조각은 한 경계, 3 km 떨어진 조각은
/// 따로(경계 둘, 글자 여덟), 모든 조각이 경계에서 490~510 m.
#[test]
fn multi_part_area_rings_every_part() {
    let a = rect(200_000.0, 450_000.0, 200_050.0, 450_050.0);
    let b = rect(203_000.0, 450_000.0, 203_050.0, 450_050.0);
    let c = rect(200_350.0, 450_000.0, 200_400.0, 450_050.0);
    let multi = format!("MULTIPOLYGON ({}, {})", &a["POLYGON ".len()..], &b["POLYGON ".len()..]);
    let rings = distance_rings(&[multi, c.clone()], 500.0).expect("rings");
    assert_eq!(rings.len(), 2);
    let labels: usize = rings.iter().map(|r| ring_label_points(r).expect("labels").len()).sum();
    assert_eq!(labels, 8);
    let lines: Vec<String> = rings
        .iter()
        .map(|r| format!("({})", r.iter().map(|p| format!("{} {}", p[0], p[1])).collect::<Vec<_>>().join(", ")))
        .collect();
    let all = Geometry::new_from_wkt(&format!("MULTILINESTRING ({})", lines.join(", "))).expect("all");
    for part in [a, b, c] {
        let d = Geometry::new_from_wkt(&part).expect("part").distance(&all).expect("distance");
        assert!(d > 490.0 && d < 510.0, "part ring distance={d}");
    }
}
