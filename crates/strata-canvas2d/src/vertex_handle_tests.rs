//! `vertex_handle` 시험 — 설계 문서 Phase 6 정점 편집 핸들.
//!
//! 화면 변환은 `origin = [0, 100]`, `1 m/px` 로 두었다. 지도 `(x, y)` 는 화면
//! `(x, 100 - y)` 가 된다(화면 y 는 아래로 증가).

use super::*;
use strata_geo::snap_vertex_ops::{HGIS_MIN_LINE_VERTICES, HGIS_MIN_POLYGON_VERTICES};

fn xf() -> ScreenTransform {
    ScreenTransform::new([0.0, 100.0], 1.0, [0.0, 0.0])
}

fn square() -> Vec<[f64; 2]> {
    vec![[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]]
}

/// 정점 4개 + 선분 중점만큼 핸들이 나온다.
///
/// 닫는 점을 저장하지 않은 링은 [`handles`] 만으로는 오각형인지 폴리곤인지 알 수
/// 없어 닫는 선분을 넣지 않는다(중점 3개 → 7개). 폴리곤임을 아는 호출부는
/// [`handles_of`] 로 닫는 선분(4번째)까지 받는다(8개).
#[test]
fn handles_list_vertices_and_segment_midpoints() {
    let hs = handles(&square(), &xf());
    assert_eq!(hs.len(), 7, "정점 4 + 열린 선분 중점 3");

    let hs_closed = handles_of(&square(), &xf(), true);
    assert_eq!(hs_closed.len(), 8, "정점 4 + 중점 4(닫는 선분 포함)");
    let vertices: Vec<usize> = hs_closed
        .iter()
        .filter_map(|(k, _)| match k {
            HandleKind::Vertex(i) => Some(*i),
            HandleKind::Midpoint(_) => None,
        })
        .collect();
    assert_eq!(vertices, vec![0, 1, 2, 3]);
    let mids: Vec<usize> = hs_closed
        .iter()
        .filter_map(|(k, _)| match k {
            HandleKind::Midpoint(i) => Some(*i),
            HandleKind::Vertex(_) => None,
        })
        .collect();
    assert_eq!(mids, vec![0, 1, 2, 3], "닫는 선분까지 중점이 있어야 한다");

    // 닫는 선분(3번) 의 중점은 (0, 50) → 화면 (0, 50).
    let last = hs_closed
        .iter()
        .find(|(k, _)| *k == HandleKind::Midpoint(3))
        .map(|(_, p)| *p);
    assert_eq!(last, Some([0.0, 50.0]));

    // 선은 닫는 선분이 없다.
    let open = vec![[0.0, 0.0], [100.0, 0.0], [100.0, 100.0]];
    let hs_open = handles(&open, &xf());
    assert_eq!(hs_open.len(), 3 + 2, "정점 3 + 중점 2");

    // 닫는 점을 저장한 링이면 [`handles`] 도 폴리곤으로 알아본다.
    let mut closed_ring = square();
    closed_ring.push([0.0, 0.0]);
    assert_eq!(handles(&closed_ring, &xf()).len(), 8);
}

/// 중점 핸들 위치와 범위 밖 번호.
#[test]
fn midpoint_world_is_computed_and_bounds_checked() {
    assert_eq!(midpoint_world(&square(), 0, true), Some([50.0, 0.0]));
    assert_eq!(midpoint_world(&square(), 3, true), Some([0.0, 50.0]));
    assert_eq!(midpoint_world(&square(), 4, true), None);
    let open = vec![[0.0, 0.0], [100.0, 0.0], [100.0, 100.0]];
    assert_eq!(midpoint_world(&open, 1, false), Some([100.0, 50.0]));
    assert_eq!(midpoint_world(&open, 2, false), None, "선은 닫는 선분이 없다");
}

/// 정점이 중점보다 우선한다 — **중점이 더 가까워도**.
#[test]
fn hit_handle_prefers_vertex_even_when_midpoint_is_closer() {
    let ring = square();
    // (45, 0) 은 정점 (0,0) 에서 45px, 선분 0 의 중점 (50,0) 에서 5px.
    let hit = hit_handle(&ring, [45.0, 100.0], &xf(), 50.0);
    assert_eq!(hit, Some(HandleKind::Vertex(0)), "정점이 이겨야 한다");

    // 정점 허용치 밖이면 중점이 잡힌다.
    let hit = hit_handle(&ring, [50.0, 100.0], &xf(), 5.0);
    assert_eq!(hit, Some(HandleKind::Midpoint(0)));

    // 아무것도 허용치 밖이면 None.
    assert_eq!(hit_handle(&ring, [500.0, 500.0], &xf(), 10.0), None);
    assert_eq!(hit_handle(&ring, [0.0, 0.0], &xf(), f32::NAN), None);
}

/// 정점 이동.
#[test]
fn move_edit_relocates_one_vertex() {
    let mut ring = square();
    let r = apply(
        &mut ring,
        &VertexEdit::Move {
            index: 2,
            to: [120.0, 130.0],
        },
        true,
    );
    assert!(r.is_ok(), "정점 이동은 성공해야 한다: {r:?}");
    assert_eq!(ring[2], [120.0, 130.0]);
    assert_eq!(ring.len(), 4, "점 개수는 그대로");

    // 범위 밖 번호와 잘못된 좌표는 링을 건드리지 않고 Err.
    let before = ring.clone();
    assert!(apply(&mut ring, &VertexEdit::Move { index: 9, to: [0.0, 0.0] }, true).is_err());
    assert!(apply(
        &mut ring,
        &VertexEdit::Move {
            index: 0,
            to: [f64::NAN, 0.0]
        },
        true
    )
    .is_err());
    assert_eq!(ring, before);
}

/// 정점 삽입 — 선분 사이에 들어가고 좌표가 맞는다.
#[test]
fn insert_edit_adds_vertex_between_segment_ends() {
    let mut ring = square();
    let r = apply(
        &mut ring,
        &VertexEdit::Insert {
            segment: 1,
            at: [100.0, 50.0],
        },
        true,
    );
    assert!(r.is_ok(), "삽입은 성공해야 한다: {r:?}");
    assert_eq!(ring.len(), 5);
    // 선분 1 = 정점 1 → 2, 사이(번호 2)에 들어간다.
    assert_eq!(ring[2], [100.0, 50.0]);
    assert_eq!(ring[1], [100.0, 0.0]);
    assert_eq!(ring[3], [100.0, 100.0]);

    // 닫는 선분(3번) 에 넣으면 끝에 붙어 첫 점과의 사이를 만든다.
    let mut ring2 = square();
    assert!(apply(
        &mut ring2,
        &VertexEdit::Insert {
            segment: 3,
            at: [0.0, 50.0]
        },
        true
    )
    .is_ok());
    assert_eq!(ring2, vec![
        [0.0, 0.0],
        [100.0, 0.0],
        [100.0, 100.0],
        [0.0, 100.0],
        [0.0, 50.0]
    ]);

    // 선은 닫는 선분이 없으므로 3번 선분은 범위 밖.
    let mut line = vec![[0.0, 0.0], [100.0, 0.0], [200.0, 0.0]];
    assert!(apply(
        &mut line,
        &VertexEdit::Insert {
            segment: 3,
            at: [300.0, 0.0]
        },
        false
    )
    .is_err());
    assert_eq!(line.len(), 3);
}

/// 중점 핸들을 잡으면 그 선분에 정점이 삽입된다(사용자 동작 그대로).
#[test]
fn grabbing_midpoint_handle_inserts_a_vertex() {
    let mut ring = square();
    let mid = midpoint_world(&ring, 0, true).expect("선분 0 의 중점");
    // 화면에서 중점 핸들을 잡는다.
    let at = xf().world_to_screen(mid);
    let hit = hit_handle(&ring, at, &xf(), 6.0).expect("핸들이 잡혀야 한다");
    assert_eq!(hit, HandleKind::Midpoint(0), "중점 핸들이 잡혀야 한다");
    let segment = 0;
    // 끌어 놓은 자리(조금 옮긴 중점)에 넣는다.
    assert!(apply(&mut ring, &VertexEdit::Insert { segment, at: mid }, true).is_ok());
    assert_eq!(ring.len(), 5);
    assert_eq!(ring[1], [50.0, 0.0]);
}

/// 정점 삭제 — 폴리곤·선 모두 동작한다.
#[test]
fn delete_edit_removes_vertex() {
    let mut ring = square();
    assert!(apply(&mut ring, &VertexEdit::Delete { index: 1 }, true).is_ok());
    assert_eq!(ring, vec![[0.0, 0.0], [100.0, 100.0], [0.0, 100.0]]);

    // 닫는 점을 저장한 링에서도 마찬가지.
    let mut closed = vec![
        [0.0, 0.0],
        [100.0, 0.0],
        [100.0, 100.0],
        [0.0, 100.0],
        [0.0, 0.0],
    ];
    assert!(apply(&mut closed, &VertexEdit::Delete { index: 2 }, true).is_ok());
    assert_eq!(closed, vec![
        [0.0, 0.0],
        [100.0, 0.0],
        [0.0, 100.0],
        [0.0, 0.0]
    ]);

    let mut line = vec![[0.0, 0.0], [50.0, 50.0], [100.0, 0.0]];
    assert!(apply(&mut line, &VertexEdit::Delete { index: 1 }, false).is_ok());
    assert_eq!(line, vec![[0.0, 0.0], [100.0, 0.0]]);

    // 범위 밖 (선: 2점에서 1번) — 그 밖의 번호는 바로 Err.
    let mut two = vec![[0.0, 0.0], [100.0, 0.0]];
    assert!(apply(&mut two, &VertexEdit::Delete { index: 5 }, false).is_err());
}

/// 최소 정점 수 미만으로 줄이려 하면 Err — 선 2점, 폴리곤 3점.
#[test]
fn delete_below_minimum_returns_err_for_line_and_polygon() {
    assert_eq!(HGIS_MIN_LINE_VERTICES, 2);
    assert_eq!(HGIS_MIN_POLYGON_VERTICES, 4);

    // 선 2점 — 더 줄일 수 없다.
    let mut line = vec![[0.0, 0.0], [100.0, 0.0]];
    let err = apply(&mut line, &VertexEdit::Delete { index: 0 }, false);
    assert!(err.is_err(), "선은 2점 아래로 못 줄인다");
    assert_eq!(line.len(), 2, "실패하면 그대로 둔다");

    // 폴리곤 삼각형(열린 링 3점) — 더 줄일 수 없다.
    let mut tri = vec![[0.0, 0.0], [100.0, 0.0], [50.0, 80.0]];
    let err = apply(&mut tri, &VertexEdit::Delete { index: 0 }, true);
    assert!(err.is_err(), "면은 꼭짓점 3개 아래로 못 줄인다");
    assert_eq!(tri.len(), 3, "실패하면 그대로 둔다");

    // 닫는 점을 저장한 삼각형(4점)도 마찬가지.
    let mut closed_tri = vec![[0.0, 0.0], [100.0, 0.0], [50.0, 80.0], [0.0, 0.0]];
    assert!(apply(&mut closed_tri, &VertexEdit::Delete { index: 0 }, true).is_err());
    assert_eq!(closed_tri.len(), 4);

    // 사각형은 3점까지 줄어든다(경계 안쪽).
    let mut square_ring = square();
    assert!(apply(&mut square_ring, &VertexEdit::Delete { index: 0 }, true).is_ok());
    assert_eq!(square_ring.len(), 3);
    assert!(apply(&mut square_ring, &VertexEdit::Delete { index: 0 }, true).is_err());
}

/// 빈 링·1점 링에서 패닉하지 않는다.
#[test]
fn empty_and_single_point_rings_do_not_panic() {
    let empty: Vec<[f64; 2]> = Vec::new();
    assert!(handles(&empty, &xf()).is_empty());
    assert_eq!(hit_handle(&empty, [0.0, 0.0], &xf(), 10.0), None);
    assert_eq!(midpoint_world(&empty, 0, true), None);

    let one = vec![[10.0, 10.0]];
    let hs = handles(&one, &xf());
    assert_eq!(hs.len(), 1, "정점 하나만 나온다");
    assert_eq!(hit_handle(&one, [10.0, 90.0], &xf(), 5.0), Some(HandleKind::Vertex(0)));
    assert_eq!(midpoint_world(&one, 0, true), None, "선분이 없다");

    let mut empty_mut: Vec<[f64; 2]> = Vec::new();
    assert!(apply(&mut empty_mut, &VertexEdit::Delete { index: 0 }, true).is_err());
    assert!(apply(&mut empty_mut, &VertexEdit::Move { index: 0, to: [1.0, 1.0] }, true).is_err());
    assert!(apply(
        &mut empty_mut,
        &VertexEdit::Insert {
            segment: 0,
            at: [1.0, 1.0]
        },
        true
    )
    .is_err());
    assert!(empty_mut.is_empty());

    // 1점 폴리곤에서 삭제·이동.
    let mut one_mut = one.clone();
    assert!(apply(&mut one_mut, &VertexEdit::Delete { index: 0 }, true).is_err());
    assert!(apply(
        &mut one_mut,
        &VertexEdit::Move {
            index: 0,
            to: [20.0, 20.0]
        },
        true
    )
    .is_ok());
    assert_eq!(one_mut, vec![[20.0, 20.0]]);
}
