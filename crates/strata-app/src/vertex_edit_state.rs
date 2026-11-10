//! 꼭짓점 편집 상태 — 어느 피처의 어느 점을 고치는 중인지 들고 있다.
//!
//! ## C++ 대응
//!
//! C++ 은 `KaVertexEditTool`(QgsMapTool 상속)이 선택 피처의 정점에 사각 핸들을,
//! 선분 중점에 원형 핸들을 띄우고, 드래그로 이동·삽입, Del 로 삭제한다
//! (`KaVertexEditTool.cpp:190-204`, `:243` `vertexNear`, `:262` `segmentNear`,
//! `:334` `moveVertexTo`, `:357` `deleteVertexAt`, `:381` `insertVertexAt`).
//!
//! Rust 는 `QgsMapTool` 이 없으므로 상태만 들고, 그리기·판정은
//! `strata_canvas2d::{vertex_handle, snap_locator}` 가 맡는다(엔진은 egui 를 모른다).

use strata_canvas2d::vertex_handle::{self, HandleKind, VertexEdit};

/// 꼭짓점 편집 세션.
#[derive(Debug, Clone, PartialEq)]
pub struct VertexEditState {
    /// 고친 뒤 `survey_rings` 에서의 위치.
    pub feature_index: usize,
    /// 지금 끌고 있는 핸들(없으면 대기).
    pub dragging: Option<HandleKind>,
}

impl VertexEditState {
    /// 편집을 시작한다.
    #[must_use]
    pub fn new(feature_index: usize) -> Self {
        Self {
            feature_index,
            dragging: None,
        }
    }

    /// 이 세션이 끝난 뒤 결과 링을 돌려준다(없으면 빈 벡터).
    ///
    /// 편집은 `apply` 로 링 사본에 적용하고, 확정은 호출부가 한다.
    /// 그래야 실패했을 때 원본이 안 망가진다(취소 가능).
    pub fn apply_to(
        &self,
        ring: &mut Vec<[f64; 2]>,
        edit: &VertexEdit,
    ) -> Result<(), strata_types::error::StrataError> {
        vertex_handle::apply(ring, edit, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square() -> Vec<[f64; 2]> {
        vec![
            [0.0, 0.0],
            [10.0, 0.0],
            [10.0, 10.0],
            [0.0, 10.0],
        ]
    }

    #[test]
    fn move_vertex_shifts_one_point() {
        let st = VertexEditState::new(0);
        let mut ring = square();
        st.apply_to(&mut ring, &VertexEdit::Move { index: 0, to: [1.0, 1.0] })
            .expect("이동은 성공해야");
        assert_eq!(ring[0], [1.0, 1.0]);
        assert_eq!(ring.len(), 4, "점 수는 그대로");
    }

    #[test]
    fn insert_adds_a_vertex() {
        let st = VertexEditState::new(0);
        let mut ring = square();
        st.apply_to(&mut ring, &VertexEdit::Insert { segment: 0, at: [5.0, 0.0] })
            .expect("삽입은 성공해야");
        assert_eq!(ring.len(), 5, "점이 하나 늘어야 한다");
    }

    #[test]
    fn delete_below_minimum_is_rejected() {
        let st = VertexEditState::new(0);
        // 삼각형에서 하나 지우면 2점 — 폴리곤 최소 3점에 못 미친다.
        let mut tri = vec![[0.0, 0.0], [10.0, 0.0], [5.0, 10.0]];
        let before = tri.clone();
        let r = st.apply_to(&mut tri, &VertexEdit::Delete { index: 0 });
        assert!(r.is_err(), "최소 점 미만으로 지우면 Err 여야");
        assert_eq!(tri, before, "실패하면 링이 안 바뀌어야 한다");
    }

    #[test]
    fn delete_above_minimum_works() {
        let st = VertexEditState::new(0);
        let mut ring = square();
        st.apply_to(&mut ring, &VertexEdit::Delete { index: 0 })
            .expect("4점에서 하나 지우는 것은 성공해야");
        assert_eq!(ring.len(), 3);
    }
}
