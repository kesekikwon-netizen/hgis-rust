//! 정점 편집 핸들 — C++ `KaVertexEditTool` 의 수정점(네모/원) 그리기·집기 자리.
//!
//! ## C++ 대응
//!
//! | Rust | C++ |
//! |---|---|
//! | [`handles`] | `KaVertexEditTool.cpp:showVertexMarkers` — 정점마다 `QgsVertexMarker` 를 세운다 |
//! | [`hit_handle`] | `KaVertexEditTool.cpp:251` `vertexNear`, `:267` `segmentNear` |
//! | [`apply`] `Move` | `KaVertexEditTool.cpp:334` `moveVertexTo` |
//! | [`apply`] `Insert` | `KaVertexEditTool.cpp:381` `insertVertexAt` (선분 중점 핸들 끌기) |
//! | [`apply`] `Delete` | `KaVertexEditTool.cpp:357` `deleteVertexAt` |
//!
//! C++ 은 정점을 **사각형** 마커, 선분 중점을 **원형** 마커로 그린다(설계 문서
//! Phase 6). 여기서는 `HandleKind` 로 그 구분을 들고 다니고, 그리는 일은 호출부가
//! 한다.
//!
//! ## 최소 정점 수
//!
//! 선 2점, 폴리곤 꼭짓점 3점. 이 아래로 줄이면 도형이 깨진다. 가드는
//! [`strata_geo::snap_vertex_ops`] 의 `delete_line_vertex_checked` /
//! `delete_polygon_vertex_checked` 를 그대로 쓴다(C++ `deleteVertexAt` 의
//! `minCount` 와 같은 규칙 — 폴리곤은 닫는 점 포함 4점 = 꼭짓점 3점).

use strata_geo::snap_vertex_ops::{delete_line_vertex_checked, delete_polygon_vertex_checked};
use strata_types::error::StrataError;

use crate::layer_engine::ScreenTransform;

/// 편집 실패 사유. 워크스페이스 공통 오류 타입을 그대로 쓴다.
pub type GisError = StrataError;

/// 편집 가능한 수정점 종류.
///
/// `Vertex(i)` 는 사각형(끌면 그 정점이 움직이고, 고른 뒤 Delete 면 지운다),
/// `Midpoint(i)` 는 원형(끌면 선분 `i` 위에 새 정점이 들어간다).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandleKind {
    /// 정점 핸들(사각형). 값은 링 안의 정점 번호.
    Vertex(usize),
    /// 선분 중점 핸들(원형). 값은 선분 번호(= 앞쪽 정점 번호).
    Midpoint(usize),
}

/// 적용할 편집 한 건. C++ 의 `moveVertexTo` / `insertVertexAt` / `deleteVertexAt`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VertexEdit {
    /// 정점 `index` 를 `to` 로 옮긴다.
    Move {
        /// 정점 번호.
        index: usize,
        /// 옮길 지도 좌표.
        to: [f64; 2],
    },
    /// 선분 `segment` 위 `at` 에 정점을 넣는다(중점 핸들 끌기).
    Insert {
        /// 선분 번호.
        segment: usize,
        /// 넣을 지도 좌표.
        at: [f64; 2],
    },
    /// 정점 `index` 를 지운다.
    Delete {
        /// 정점 번호.
        index: usize,
    },
}

/// 링이 폐합(첫 점 == 끝 점)인지. C++ `QgsCurvePolygon` 은 닫는 점을 저장한다.
fn is_closed_ring(ring: &[[f64; 2]]) -> bool {
    ring.len() >= 2 && ring.first() == ring.last()
}

/// 꼭짓점 개수 — 닫는 점(첫 점 복제)은 세지 않는다.
fn corner_count(ring: &[[f64; 2]]) -> usize {
    let n = ring.len();
    if is_closed_ring(ring) {
        n - 1
    } else {
        n
    }
}

/// 선분 개수. 폴리곤은 마지막 점 → 첫 점을 잇는 닫는 선분까지 센다. 선은 안 센다.
fn segment_count(ring: &[[f64; 2]], is_polygon: bool) -> usize {
    let d = corner_count(ring);
    if is_polygon {
        if d >= 2 {
            d
        } else {
            0
        }
    } else {
        d.saturating_sub(1)
    }
}

/// 선분 `segment` 의 중점(지도 좌표). 번호가 범위 밖이면 `None`.
#[must_use]
pub fn midpoint_world(ring: &[[f64; 2]], segment: usize, is_polygon: bool) -> Option<[f64; 2]> {
    if segment >= segment_count(ring, is_polygon) {
        return None;
    }
    let d = corner_count(ring);
    let a = ring[segment];
    let b = ring[(segment + 1) % d];
    Some([(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5])
}

/// 핸들 화면 위치 목록(정점 + 선분 중점).
///
/// 닫힌 링(첫 점 == 끝 점)이면 폴리곤으로 보아 닫는 선분의 중점 핸들까지 낸다.
/// 열린 폴리곤 링을 다룰 때는 [`handles_of`] 로 `closed_segment` 를 직접 준다.
#[must_use]
pub fn handles(ring: &[[f64; 2]], xf: &ScreenTransform) -> Vec<(HandleKind, [f32; 2])> {
    handles_of(ring, xf, is_closed_ring(ring))
}

/// 폴리곤 여부를 아는 호출부용 [`handles`]. `closed_segment` 가 `true` 면
/// 마지막 점 → 첫 점을 잇는 닫는 선분의 중점 핸들도 넣는다.
#[must_use]
pub fn handles_of(
    ring: &[[f64; 2]],
    xf: &ScreenTransform,
    closed_segment: bool,
) -> Vec<(HandleKind, [f32; 2])> {
    let d = corner_count(ring);
    if d == 0 {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(d * 2);
    for (i, &pt) in ring.iter().take(d).enumerate() {
        out.push((HandleKind::Vertex(i), xf.world_to_screen(pt)));
    }
    let segs = segment_count(ring, closed_segment);
    for i in 0..segs {
        if let Some(m) = midpoint_world(ring, i, closed_segment) {
            out.push((HandleKind::Midpoint(i), xf.world_to_screen(m)));
        }
    }
    out
}

/// 화면 좌표에서 핸들 하나를 고른다.
///
/// 허용치 안에서 **정점이 중점보다 우선**하고, 같은 종류끼리는 가까운 것이 이긴다
/// (C++ `vertexNear` 를 먼저 보고 `segmentNear` 를 보는 순서와 같다).
#[must_use]
pub fn hit_handle(
    ring: &[[f64; 2]],
    at: [f32; 2],
    xf: &ScreenTransform,
    tol_px: f32,
) -> Option<HandleKind> {
    hit_handle_of(ring, at, xf, tol_px, is_closed_ring(ring))
}

/// 폴리곤 여부를 아는 호출부용 [`hit_handle`].
#[must_use]
pub fn hit_handle_of(
    ring: &[[f64; 2]],
    at: [f32; 2],
    xf: &ScreenTransform,
    tol_px: f32,
    closed_segment: bool,
) -> Option<HandleKind> {
    if !tol_px.is_finite() || tol_px < 0.0 {
        return None;
    }
    let mut best_vertex: Option<(HandleKind, f32)> = None;
    let mut best_mid: Option<(HandleKind, f32)> = None;
    for (kind, pos) in handles_of(ring, xf, closed_segment) {
        let dx = pos[0] - at[0];
        let dy = pos[1] - at[1];
        let d = (dx * dx + dy * dy).sqrt();
        if d > tol_px {
            continue;
        }
        match kind {
            HandleKind::Vertex(_) => {
                if best_vertex.is_none_or(|(_, bd)| d < bd) {
                    best_vertex = Some((kind, d));
                }
            }
            HandleKind::Midpoint(_) => {
                if best_mid.is_none_or(|(_, bd)| d < bd) {
                    best_mid = Some((kind, d));
                }
            }
        }
    }
    best_vertex.or(best_mid).map(|(kind, _)| kind)
}

/// 편집을 적용한다. 최소 정점 수(선 2, 폴리곤 3)를 지킨다.
///
/// 실패하면 링을 **건드리지 않고** `Err` 를 돌려준다(호출부가 상태를 그대로 둔다).
pub fn apply(
    ring: &mut Vec<[f64; 2]>,
    edit: &VertexEdit,
    is_polygon: bool,
) -> Result<(), GisError> {
    match *edit {
        VertexEdit::Move { index, to } => {
            if !to[0].is_finite() || !to[1].is_finite() {
                return Err(GisError::InvalidInput("옮길 좌표가 올바르지 않습니다.".into()));
            }
            let Some(pt) = ring.get_mut(index) else {
                return Err(GisError::InvalidInput(format!(
                    "정점 번호 {index} 가 범위를 벗어났습니다(정점 {}개).",
                    ring.len()
                )));
            };
            *pt = to;
            Ok(())
        }
        VertexEdit::Insert { segment, at } => {
            if !at[0].is_finite() || !at[1].is_finite() {
                return Err(GisError::InvalidInput("넣을 좌표가 올바르지 않습니다.".into()));
            }
            if segment >= segment_count(&ring[..], is_polygon) {
                return Err(GisError::InvalidInput(format!(
                    "선분 번호 {segment} 가 범위를 벗어났습니다."
                )));
            }
            // 선분 i 는 정점 i → i+1 이므로 사이(= i+1)에 넣는다.
            // 닫는 선분이면 위치가 닫는 점 자리와 같아, 그 점이 자연히 뒤로 밀린다.
            let pos = segment + 1;
            if pos > ring.len() {
                return Err(GisError::InvalidInput(format!(
                    "선분 {segment} 에 정점을 넣을 자리가 없습니다."
                )));
            }
            ring.insert(pos, at);
            Ok(())
        }
        VertexEdit::Delete { index } => {
            if index >= ring.len() {
                return Err(GisError::InvalidInput(format!(
                    "정점 번호 {index} 가 범위를 벗어났습니다(정점 {}개).",
                    ring.len()
                )));
            }
            if is_polygon {
                delete_polygon_corner(ring, index)
            } else if delete_line_vertex_checked(ring, index) {
                Ok(())
            } else {
                Err(GisError::InvalidInput(
                    "선은 꼭짓점 2개보다 줄일 수 없습니다.".into(),
                ))
            }
        }
    }
}

/// 폴리곤 정점 삭제. 닫힌 링이면 열린 링과 저장 모양이 달라 처리를 나눈다.
///
/// 닫는 점을 저장하지 않는 링(이 워크스페이스의 `Feature::polygon` 이 그렇다)은
/// `strata_geo` 의 가드(꼭짓점 3개 = 점 4개)를 그대로 쓰기 위해 사본에 닫는 점을
/// 붙여 검사한다. **성공했을 때만** 원래 `ring` 을 바꾼다.
fn delete_polygon_corner(ring: &mut Vec<[f64; 2]>, index: usize) -> Result<(), GisError> {
    let too_few = || GisError::InvalidInput("면은 꼭짓점 3개보다 줄일 수 없습니다.".into());
    if is_closed_ring(ring) {
        // 닫는 점을 저장한 링: 최소 4점(꼭짓점 3개)은 가드가 막는다.
        return if delete_polygon_vertex_checked(ring, index) {
            Ok(())
        } else {
            Err(too_few())
        };
    }
    let Some(&first) = ring.first() else {
        return Err(too_few());
    };
    let mut probe = ring.clone();
    probe.push(first);
    if !delete_polygon_vertex_checked(&mut probe, index) {
        return Err(too_few());
    }
    let _ = probe.pop();
    *ring = probe;
    Ok(())
}

#[cfg(test)]
#[path = "vertex_handle_tests.rs"]
mod vertex_handle_tests;
