//! 정점·선분 스냅 색인 — C++ `QgsPointLocator` + `QgsSnappingUtils` 자리.
//!
//! C++ 은 `snappingUtils()->snapToMap(pos)` 한 번으로 커서 주변의 정점·선분을
//! 찾아 준다(`KaVertexEditTool.cpp:216` `snapMapPoint`). Rust 대응이 이 파일이다.
//!
//! ## GeoRust 먼저 (설계 문서 Phase 6 · 8대 수칙 8)
//!
//! 공간 질의는 GeoRust 의 `rstar`(R*-tree)를 그대로 쓴다. 정점은 `[f64; 2]` 를,
//! 선분은 `rstar::primitives::Line` 을 트리에 넣고 `nearest_neighbor` 로 묻는다.
//! 직접 격자 해시나 전수 비교를 새로 만들지 않는다.
//!
//! ## 우선순위 — 정점 > 선분
//!
//! 설계 문서 Phase 6: "정점과 끝점에 선분보다 높은 우선순위를 부여". 그래서
//! 허용치 안에 정점이 하나라도 있으면 선분은 보지 않고 정점을 돌려준다.
//! (같은 등급 안에서는 거리가 가까운 것을 고른다 — `rstar` 가 보장한다.)
//!
//! ## 호출부
//!
//! `strata-canvas2d::snap_locator::locate` 가 이 색인을 만들어 쓴다.
//! 여기서는 "어느 레이어·피처·정점인지"(`SnapSource`)를 실어 보내기만 하고,
//! 무엇을 넣고 뺄지(숨긴 레이어·자기 자신 제외·허용 종류)는 호출부가 정한다.
//! 그래야 색인은 공간 질의만 하고, 정책은 한 곳(`snap_locator`)에 모인다.

use rstar::primitives::{GeomWithData, Line};
use rstar::{PointDistance, RTree};

/// 스냅 대상의 종류. C++ `QgsPointLocator::Type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapKind {
    /// 정점(꼭짓점·끝점).
    Vertex,
    /// 선분(변) 위의 한 점.
    Segment,
}

/// 이 스냅 대상이 어디서 왔는지.
///
/// 레이어·피처는 **호출부가 매긴 순서 인덱스**다(`LayerStage::all()` 의 순서).
/// `index` 는 [`SnapKind::Vertex`] 면 링 안의 정점 번호, [`SnapKind::Segment`] 면
/// 선분 앞쪽 정점의 번호다(선분 `i` 는 정점 `i` → `i + 1`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnapSource {
    /// 레이어 순서 인덱스.
    pub layer_index: usize,
    /// 레이어 안의 피처 순서 인덱스.
    pub feature_index: usize,
    /// 정점 번호 또는 선분 앞쪽 정점 번호.
    pub index: usize,
}

/// 스냅 결과. C++ `QgsPointLocator::Match`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SnapHit {
    /// 붙을 지도 좌표.
    pub point: [f64; 2],
    /// 정점인지 선분인지.
    pub kind: SnapKind,
    /// 커서에서 `point` 까지의 거리(지도 단위, 보통 m).
    pub distance: f64,
    /// 어느 레이어·피처의 무엇인지.
    pub source: SnapSource,
}

/// 정점·선분 두 그루의 R*-tree.
#[derive(Debug, Clone, Default)]
pub struct SnapIndex {
    vertices: RTree<GeomWithData<[f64; 2], SnapSource>>,
    segments: RTree<GeomWithData<Line<[f64; 2]>, SnapSource>>,
}

impl SnapIndex {
    /// 빈 색인.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 색인을 새로 채운다. 색인은 늘 통째로 다시 만들어 쓴다(편집 중 정점이
    /// 움직이면 부분 갱신보다 재구축이 단순하고 빨리 끝난다 — C++ 도 편집 시
    /// `QgsPointLocator` 를 무효화한다).
    ///
    /// 길이 0 인 선분과 `NaN`/무한 좌표는 조용히 버린다(트리에 넣으면 거리
    /// 비교가 무너진다). 버린 개수는 호출부가 알 필요가 없어 돌려주지 않는다.
    pub fn rebuild(
        &mut self,
        vertices: Vec<([f64; 2], SnapSource)>,
        segments: Vec<([[f64; 2]; 2], SnapSource)>,
    ) {
        let vertex_objs: Vec<GeomWithData<[f64; 2], SnapSource>> = vertices
            .into_iter()
            .filter(|(p, _)| is_finite_point(*p))
            .map(|(p, src)| GeomWithData::new(p, src))
            .collect();
        let segment_objs: Vec<GeomWithData<Line<[f64; 2]>, SnapSource>> = segments
            .into_iter()
            .filter(|(seg, _)| {
                is_finite_point(seg[0])
                    && is_finite_point(seg[1])
                    && segment_length_2(seg[0], seg[1]) > 1e-12
            })
            .map(|(seg, src)| GeomWithData::new(Line::new(seg[0], seg[1]), src))
            .collect();
        self.vertices = RTree::bulk_load(vertex_objs);
        self.segments = RTree::bulk_load(segment_objs);
    }

    /// 넣은 정점 수(진단·시험용).
    #[must_use]
    pub fn vertex_count(&self) -> usize {
        self.vertices.size()
    }

    /// 넣은 선분 수(진단·시험용).
    #[must_use]
    pub fn segment_count(&self) -> usize {
        self.segments.size()
    }

    /// 아무것도 없으면 `true`.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.vertices.size() == 0 && self.segments.size() == 0
    }

    /// 커서에서 `tolerance`(지도 단위) 안의 가장 가까운 스냅 대상을 찾는다.
    ///
    /// 정점이 먼저다. 정점이 허용치 안에 없을 때만 선분을 본다.
    /// `tolerance` 가 음수이거나 유한하지 않으면 `None`.
    #[must_use]
    pub fn query(&self, cursor: [f64; 2], tolerance: f64) -> Option<SnapHit> {
        if !cursor[0].is_finite() || !cursor[1].is_finite() {
            return None;
        }
        if !tolerance.is_finite() || tolerance < 0.0 {
            return None;
        }
        if let Some(v) = self.vertices.nearest_neighbor(&cursor) {
            let distance = v.distance_2(&cursor).sqrt();
            if distance <= tolerance {
                return Some(SnapHit {
                    point: *v.geom(),
                    kind: SnapKind::Vertex,
                    distance,
                    source: v.data,
                });
            }
        }
        let s = self.segments.nearest_neighbor(&cursor)?;
        let distance = s.distance_2(&cursor).sqrt();
        if distance > tolerance {
            return None;
        }
        Some(SnapHit {
            point: s.geom().nearest_point(&cursor),
            kind: SnapKind::Segment,
            distance,
            source: s.data,
        })
    }
}

/// 좌표가 유한한지.
fn is_finite_point(p: [f64; 2]) -> bool {
    p[0].is_finite() && p[1].is_finite()
}

/// 선분 길이의 제곱.
fn segment_length_2(a: [f64; 2], b: [f64; 2]) -> f64 {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    dx * dx + dy * dy
}

#[cfg(test)]
mod tests {
    use super::*;

    fn src(layer: usize, feature: usize, index: usize) -> SnapSource {
        SnapSource {
            layer_index: layer,
            feature_index: feature,
            index,
        }
    }

    /// 정점이 선분보다 우선한다 — **선분이 더 가까워도**.
    #[test]
    fn vertex_beats_nearer_segment() {
        let mut idx = SnapIndex::new();
        // 정점 (0,0) 과, 그보다 커서에 가까운 선분 (10,0)-(20,0).
        idx.rebuild(
            vec![([0.0, 0.0], src(0, 0, 0))],
            vec![([[10.0, 0.0], [20.0, 0.0]], src(1, 0, 0))],
        );
        assert_eq!(idx.vertex_count(), 1);
        assert_eq!(idx.segment_count(), 1);

        // 커서 (9,0): 선분까지 1m, 정점까지 9m. 그래도 정점이 이긴다.
        let hit = idx.query([9.0, 0.0], 10.0).expect("허용치 안");
        assert_eq!(hit.kind, SnapKind::Vertex);
        assert_eq!(hit.point, [0.0, 0.0]);
        assert!((hit.distance - 9.0).abs() < 1e-9);
        assert_eq!(hit.source, src(0, 0, 0));
    }

    /// 정점이 허용치 밖이면 선분으로 내려간다. 선분 위 점은 투영점이다.
    #[test]
    fn segment_is_used_when_no_vertex_is_in_tolerance() {
        let mut idx = SnapIndex::new();
        idx.rebuild(
            vec![([0.0, 0.0], src(0, 0, 0))],
            vec![([[10.0, 0.0], [20.0, 0.0]], src(1, 0, 0))],
        );
        // 커서 (15,2): 정점까지 15m(허용치 5 밖), 선분까지 2m.
        let hit = idx.query([15.0, 2.0], 5.0).expect("선분");
        assert_eq!(hit.kind, SnapKind::Segment);
        assert_eq!(hit.point, [15.0, 0.0]);
        assert!((hit.distance - 2.0).abs() < 1e-9);
    }

    /// 허용치 밖, 빈 색인, 잘못된 입력은 `None`(패닉 없음).
    #[test]
    fn out_of_tolerance_and_bad_input_return_none() {
        let mut idx = SnapIndex::new();
        assert!(idx.is_empty());
        assert!(idx.query([0.0, 0.0], 10.0).is_none());

        idx.rebuild(vec![([0.0, 0.0], src(0, 0, 0))], Vec::new());
        assert!(idx.query([0.0, 5.0], 4.0).is_none(), "허용치 밖");
        assert!(idx.query([0.0, 5.0], 5.0).is_some(), "허용치 경계는 포함");
        assert!(idx.query([f64::NAN, 0.0], 10.0).is_none());
        assert!(idx.query([0.0, 0.0], f64::NAN).is_none());
        assert!(idx.query([0.0, 0.0], -1.0).is_none());
    }

    /// 길이 0 인 선분과 깨진 좌표는 넣지 않는다(거리 비교가 무너진다).
    #[test]
    fn degenerate_vertices_and_segments_are_dropped() {
        let mut idx = SnapIndex::new();
        idx.rebuild(
            vec![([0.0, 0.0], src(0, 0, 0)), ([f64::NAN, 1.0], src(0, 0, 1))],
            vec![
                ([[5.0, 5.0], [5.0, 5.0]], src(1, 0, 0)),
                ([[0.0, 0.0], [f64::INFINITY, 0.0]], src(1, 0, 1)),
            ],
        );
        assert_eq!(idx.vertex_count(), 1, "NaN 정점은 빠진다");
        assert_eq!(idx.segment_count(), 0, "길이 0·무한 선분은 빠진다");
        assert!(!idx.is_empty());
    }
}
