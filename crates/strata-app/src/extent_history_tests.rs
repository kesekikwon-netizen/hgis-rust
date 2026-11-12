//! 지도 범위 이력이 QGIS `QgsMapCanvas`(setExtent :1629-1650, zoomToPreviousExtent :1745-1758,
//! zoomToNextExtent :1760-1774, clearExtentHistory :1776-1784)와 같은지. 기댓값은 qgis-dev
//! 4.3.0-Master 를 화면 없이 돌려 본 zoomLastStatusChanged/zoomNextStatusChanged 순서다.

use super::*;

fn r(a: f64, b: f64, c: f64, d: f64) -> [f64; 4] {
    [a, b, c, d]
}

/// 작업 파일을 읽기 전(이력이 비었을 때)에는 범위를 아무리 바꿔도 이전·다음이 없다.
#[test]
fn nothing_is_recorded_before_a_project_is_read() {
    let mut h = ExtentHistory::default();
    for e in [r(0.0, 0.0, 1000.0, 1000.0), r(0.0, 0.0, 500.0, 500.0), r(125.0, 125.0, 375.0, 375.0)] {
        h.note_extent(e);
        assert!(!h.can_previous() && !h.can_next());
    }
    assert_eq!(h.previous(), None);
    assert_eq!(h.next(), None);
}

/// 읽은 뒤: 바뀔 때마다 쌓고(같은 범위는 쌓지 않음), 뒤로·앞으로, 뒤로 간 뒤 새 범위는 앞 이력을 지운다.
#[test]
fn after_a_project_read_history_follows_qgis() {
    let mut h = ExtentHistory::default();
    let seed = r(0.0, 0.0, 10.0, 10.0);
    h.seed(seed);
    assert!(!h.can_previous() && !h.can_next());
    let d = r(100.0, 100.0, 300.0, 300.0);
    h.note_extent(d);
    assert!(h.can_previous() && !h.can_next());
    h.note_extent(d);
    assert!(h.can_previous() && !h.can_next());
    let e = r(0.0, 0.0, 50.0, 50.0);
    h.note_extent(e);
    assert_eq!(h.previous(), Some(d));
    assert!(h.can_previous() && h.can_next());
    assert_eq!(h.previous(), Some(seed));
    assert!(!h.can_previous() && h.can_next());
    assert_eq!(h.next(), Some(d));
    assert!(h.can_previous() && h.can_next());
    h.note_extent(r(10.0, 10.0, 60.0, 60.0));
    assert!(h.can_previous() && !h.can_next());
    assert_eq!(h.next(), None, "a new extent after going back drops the forward history");
}

/// 100개까지만 남긴다(가장 오래된 것부터 버림).
#[test]
fn history_keeps_at_most_one_hundred_extents() {
    let mut h = ExtentHistory::default();
    h.seed(r(0.0, 0.0, 1.0, 1.0));
    for i in 1..=150 {
        let f = f64::from(i);
        h.note_extent(r(f, f, f + 1.0, f + 1.0));
    }
    let mut back = 0;
    while h.previous().is_some() {
        back += 1;
    }
    assert_eq!(back, 99);
}
