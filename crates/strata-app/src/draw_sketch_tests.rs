//! 그리던 면 — C++ KaCaptureMapTool.cpp 누름(:260-298)·더블클릭(:305-314)·되돌리기(:364-377)·
//! 마치기(:393-456)·취소(:458-467).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

/// 1 px = 0.02 m 일 때 허용 거리는 0.2 m, 아주 크게 보면 최소 0.05 m, 모르면 0.15 m.
#[test]
fn vertex_tolerance_follows_cpp() {
    assert!((vertex_tolerance(0.02) - 0.2).abs() < 1e-12);
    assert!((vertex_tolerance(0.001) - 0.05).abs() < 1e-12);
    assert!((vertex_tolerance(0.0) - 0.15).abs() < 1e-12);
}

/// 같은 자리를 다시 누르면(허용 거리 안) 점을 더하지 않는다.
#[test]
fn pressing_on_the_last_point_adds_nothing() {
    let mut s = DrawSketch::default();
    s.press_left([0.0, 0.0], 0.02);
    s.press_left([0.1, 0.1], 0.02);
    s.press_left([10.0, 0.0], 0.02);
    assert_eq!(s.points, vec![[0.0, 0.0], [10.0, 0.0]]);
}

/// 더블클릭으로 마치기: 첫 누름이 점을 넣고, 둘째 누름은 겹쳐 빠지고, 더블클릭 자리는 앞 점과 같아
/// 더하지 않는다 — 찍은 꼭짓점 그대로, 겹친 점 없음.
#[test]
fn double_click_finish_keeps_exactly_the_clicked_vertices() {
    let mut s = DrawSketch::default();
    for p in [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]] {
        s.press_left(p, 0.02);
    }
    s.press_left([10.0, 10.0], 0.02); // 더블클릭의 둘째 누름
    s.double_click([10.0, 10.0]);
    assert_eq!(s.points.len(), 3);
    let closed = s.close(0.02).expect("polygon");
    assert_eq!(closed.rings.unwrap(), vec![vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 0.0]]]);
}

/// 더블클릭 자리가 조금 움직였으면 점은 더해지지만 면을 닫을 때 앞 점 간격 안이라 빠진다.
#[test]
fn double_click_that_moved_a_little_is_dropped_when_closing() {
    let mut s = DrawSketch::default();
    for p in [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]] {
        s.press_left(p, 0.02);
    }
    s.double_click([10.01, 10.0]);
    assert_eq!(s.points.len(), 4);
    assert_eq!(s.close(0.02).unwrap().rings.unwrap()[0].len(), 4);
}

/// 되돌리기는 마지막 점 하나, 취소는 모두. 빈 스케치에서는 둘 다 false.
#[test]
fn undo_drops_the_last_vertex_and_cancel_drops_the_sketch() {
    let mut s = DrawSketch::default();
    assert!(!s.undo_last());
    assert!(!s.cancel());
    s.press_left([0.0, 0.0], 0.02);
    s.press_left([5.0, 0.0], 0.02);
    assert!(s.undo_last());
    assert_eq!(s.points, vec![[0.0, 0.0]]);
    assert!(s.cancel());
    assert!(s.points.is_empty());
}

/// 점이 모자라면 마치지 못하고 점은 남는다. 점이 없으면 아무 일도 없다.
#[test]
fn finishing_with_too_few_points_keeps_the_sketch() {
    let mut s = DrawSketch::default();
    assert_eq!(s.close(0.02), Err(FinishShort::Empty));
    s.press_left([0.0, 0.0], 0.02);
    s.press_left([5.0, 0.0], 0.02);
    assert_eq!(s.close(0.02), Err(FinishShort::TooFew));
    assert_eq!(s.points.len(), 2);
}
