//! 면 닫기·고치기 — C++ `EditGeometryRepair::closePolygon`·`notice`(EditGeometryRepair.cpp:38-104).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

fn square() -> Vec<[f64; 2]> {
    vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]]
}

/// 찍은 점을 그대로 닫는다(첫 점을 끝에 덧붙임), 고친 것 없음, 알림 없음.
#[test]
fn plain_sketch_is_closed_without_notice() {
    let r = close_polygon(&square(), 0.15);
    assert_eq!(r.rings, Some(vec![vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0], [0.0, 0.0]]]));
    assert!(!r.repaired);
    assert_eq!(r.dropped_parts, 0);
    assert!((r.kept_area - 100.0).abs() < 1e-9);
    assert_eq!(notice(&r), None);
}

/// 간격 안의 겹친 점은 하나로 — 바로 앞 점과의 거리만 본다(C++ ring.last().sqrDist(p) > tol²).
#[test]
fn points_within_spacing_of_the_previous_point_are_dropped() {
    let pts = vec![[0.0, 0.0], [10.0, 0.0], [10.1, 0.05], [10.0, 10.0], [0.0, 10.0], [0.0, 10.0]];
    let r = close_polygon(&pts, 0.15);
    assert_eq!(r.rings.unwrap()[0].len(), 5);
}

/// 이미 첫 점으로 돌아온 스케치는 한 번 더 닫지 않는다.
#[test]
fn sketch_already_closed_is_not_closed_twice() {
    let mut pts = square();
    pts.push([0.0, 0.0]);
    // 앞 점(0,10)과 0.15 보다 멀어 남는다 — 닫힌 고리 그대로.
    let r = close_polygon(&pts, 0.15);
    assert_eq!(r.rings.unwrap()[0].len(), 5);
}

/// 점이 모자라면(겹친 점을 빼고 3개 미만) 도형이 없다.
#[test]
fn too_few_distinct_points_give_no_geometry() {
    let r = close_polygon(&[[0.0, 0.0], [10.0, 0.0], [10.05, 0.0]], 0.15);
    assert_eq!(r.rings, None);
    assert_eq!(notice(&r), None);
}

/// 꼬인 면(8자) — GEOS makeValid(linework) 뒤 큰 조각 하나만, 빠진 조각 알림(C++ 문구).
#[test]
fn bow_tie_keeps_the_largest_part_and_tells_the_dropped_one() {
    let pts = vec![[0.0, 0.0], [10.0, 10.0], [10.0, 0.0], [0.0, 10.0]];
    let r = close_polygon(&pts, 0.15);
    assert!(r.repaired);
    assert_eq!(r.dropped_parts, 1);
    assert!((r.kept_area - 25.0).abs() < 1e-9);
    assert!((r.parts_area - 50.0).abs() < 1e-9);
    // 같은 넓이면 처음 조각(GEOS 차례) — C++ `area > keptArea` 일 때만 바꾼다.
    let kept = r.rings.clone().unwrap();
    assert_eq!(strata_engine::polygon_area(&kept).unwrap(), 25.0);
    assert_eq!(
        notice(&r).as_deref(),
        Some("선이 서로 꼬인 면이라 가장 큰 조각만 넣었습니다. 빠진 조각 1개(25.00 ㎡)를 도형선택으로 확인하세요.")
    );
}

/// 거의 한 줄 — 넓이가 1 cm² 미만이거나 가늘기(4πA/P²) 1e-3 미만이면 다시 그리라고 알린다.
#[test]
fn nearly_collinear_sketch_is_degenerate() {
    let pts = vec![[0.0, 0.0], [100.0, 0.0], [50.0, 0.01]];
    let r = close_polygon(&pts, 0.001);
    assert!(r.degenerate);
    assert_eq!(notice(&r).as_deref(), Some("점이 거의 한 줄이라 면적이 0.50 ㎡뿐입니다. 제출 검수에서 걸리니 다시 그리세요."));
}

/// 넓은 면의 알림은 ha 로(formatAreaM2).
#[test]
fn dropped_area_uses_hectares_when_large() {
    let pts = vec![[0.0, 0.0], [300.0, 300.0], [300.0, 0.0], [0.0, 300.0]];
    let r = close_polygon(&pts, 0.15);
    assert_eq!(
        notice(&r).as_deref(),
        Some("선이 서로 꼬인 면이라 가장 큰 조각만 넣었습니다. 빠진 조각 1개(2.250 ha (22500.0 ㎡))를 도형선택으로 확인하세요.")
    );
}
