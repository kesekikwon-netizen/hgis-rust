//! `gui_tab_map` 시험 — 원래 `mod tests_measure_hud` 를 그대로 옮겼다.
//! facade 에서 같은 이름으로 등록하므로 시험 경로가 그대로다.
//!
//! `#[path]` 로 붙은 모듈이라 `use super::*` 가 부모(gui_tab_map)가 아니라
//! `gui_tab_map::tests_measure_hud` 를 가리킨다. 원문의 `use super::*;` 는
//! 그래서 여기서는 `use crate::gui_tab_map::*;` 여야 한다.

use crate::gui_tab_map::{format_area_m2, format_length_m, grid_bearing_degrees, ring_perimeter_m};

/// R9 — `MeasureOps::formatLengthM` `MeasureOps.cpp:163-171` 세 구간과 `—`.
#[test]
fn test_r9_format_length_m_matches_measureops() {
    assert_eq!(format_length_m(f64::NAN), "—");
    assert_eq!(format_length_m(-1.0), "—");
    // 10 m 미만은 소수 3자리
    assert_eq!(format_length_m(9.8765), "9.877 m");
    // 10 m 이상 1 km 미만은 소수 2자리
    assert_eq!(format_length_m(10.0), "10.00 m");
    assert_eq!(format_length_m(123.456), "123.46 m");
    // 1 km 이상은 km 소수 3자리
    assert_eq!(format_length_m(1000.0), "1.000 km");
    assert_eq!(format_length_m(2500.0), "2.500 km");
}

/// R9 — `MeasureOps::formatAreaM2` `MeasureOps.cpp:188-198` 세 구간과 `—`.
#[test]
fn test_r9_format_area_m2_matches_measureops() {
    assert_eq!(format_area_m2(f64::INFINITY), "—");
    assert_eq!(format_area_m2(-5.0), "—");
    // 100 ㎡ 미만은 소수 2자리
    assert_eq!(format_area_m2(42.128), "42.13 ㎡");
    // 100 ㎡ 이상은 소수 1자리
    assert_eq!(format_area_m2(100.0), "100.0 ㎡");
    assert_eq!(format_area_m2(1234.567), "1234.6 ㎡");
    // 1 ha 이상은 `ha (㎡)`
    assert_eq!(format_area_m2(10000.0), "1.000 ha (10000.0 ㎡)");
    assert_eq!(format_area_m2(25000.0), "2.500 ha (25000.0 ㎡)");
}

/// R9 — `MeasureOps::gridBearingDegrees` `MeasureOps.cpp:73-82`.
/// 북 0°, 동 90°, 남 180°, 서 270° 이고, 같은 점이면 NaN.
#[test]
fn test_r9_grid_bearing_degrees_matches_measureops() {
    let o = [0.0, 0.0];
    assert_eq!(grid_bearing_degrees(o, [0.0, 10.0]), 0.0); // 북
    assert_eq!(grid_bearing_degrees(o, [10.0, 0.0]), 90.0); // 동
    assert_eq!(grid_bearing_degrees(o, [0.0, -10.0]), 180.0); // 남
    assert_eq!(grid_bearing_degrees(o, [-10.0, 0.0]), 270.0); // 서
    assert!(grid_bearing_degrees(o, o).is_nan(), "같은 점은 NaN");
}

/// R9 — `MeasureOps::polygonPerimeterMeters` `MeasureOps.cpp:129-147`.
/// 점이 3개 미만이면 0, 3개면 닫는 변까지 더한다.
#[test]
fn test_r9_ring_perimeter_m_matches_measureops() {
    assert_eq!(ring_perimeter_m(&[]), 0.0);
    assert_eq!(ring_perimeter_m(&[[0.0, 0.0], [10.0, 0.0]]), 0.0);
    let square = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
    assert!((ring_perimeter_m(&square) - 40.0).abs() < 1e-9);
    // 삼각형: 닫는 변(마지막→처음)까지 더한다
    let tri = [[0.0, 0.0], [3.0, 0.0], [0.0, 4.0]];
    assert!((ring_perimeter_m(&tri) - 12.0).abs() < 1e-9);
}
