//! 라벨이 겹칠 때 무엇을 남기는지 — C++ 화면(docs/senior/shots/cpp-3-survey-closeup.png)에서
//! 조사구역 「12,000.00 ㎡」는 남고 그 위에 겹친 작은 유구(면) 「42.00 ㎡」는 빠지며, 단면선
//! 「A-A'」가 선 위에 있고 유물 「토기편」은 비켜서 오른쪽 아래에 놓인다.

use eframe::egui::{Pos2, Rect, Vec2};

use super::{place_labels, LabelReq};

fn r(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, h))
}

fn one(rect: Rect, fits_inside: bool, area: f64) -> LabelReq {
    LabelReq { candidates: vec![rect], fits_inside, area }
}

/// 면 안에 들어가는 큰 면의 라벨이, 면 밖으로 넘치는 작은 면의 라벨보다 먼저 자리를 잡는다
/// (작은 면이 범례에서 위에 있어도).
#[test]
fn a_label_that_fits_its_polygon_wins_over_a_spilling_one() {
    let small_spilling = one(r(100.0, 90.0, 60.0, 14.0), false, 42.0);
    let big_fitting = one(r(110.0, 100.0, 80.0, 14.0), true, 12_000.0);
    assert_eq!(place_labels(&[small_spilling, big_fitting]), vec![None, Some(0)]);
}

/// 넘치는 라벨끼리는 큰 면 먼저.
#[test]
fn among_spilling_labels_the_bigger_polygon_wins() {
    let small = one(r(0.0, 0.0, 50.0, 10.0), false, 42.0);
    let big = one(r(10.0, 5.0, 50.0, 10.0), false, 108.0);
    assert_eq!(place_labels(&[small, big]), vec![None, Some(0)]);
}

/// 자리가 하나뿐인 선 라벨이 먼저 놓이고, 점 라벨은 남은 후보 자리 중 첫 빈 곳으로 간다.
#[test]
fn point_labels_take_the_first_free_candidate_after_lines() {
    let point = LabelReq {
        candidates: vec![r(10.0, 0.0, 40.0, 12.0), r(-40.0, 0.0, 40.0, 12.0), r(10.0, 20.0, 40.0, 12.0)],
        fits_inside: true,
        area: 0.0,
    };
    let line = one(r(-30.0, 2.0, 90.0, 12.0), true, 0.0);
    assert_eq!(place_labels(&[point, line]), vec![Some(2), Some(0)]);
}

/// 어느 자리도 비지 않으면 그리지 않는다.
#[test]
fn a_label_with_no_free_place_is_dropped() {
    let first = one(r(0.0, 0.0, 100.0, 100.0), true, 1.0);
    let point = LabelReq { candidates: vec![r(10.0, 10.0, 5.0, 5.0), r(50.0, 50.0, 5.0, 5.0)], fits_inside: true, area: 0.0 };
    assert_eq!(place_labels(&[first, point]), vec![Some(0), None]);
}
