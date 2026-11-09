//! `cad_interaction` 는 CAD 편집·스냅·측정의 재수출 창구다.
//!
//! 원래 한 파일(1198줄)을 300줄 규칙에 맞춰 주제별로 나눴다.
//! 기존 호출부의 `strata_canvas2d::cad_interaction::<이름>` 경로는 그대로 동작한다.

pub use crate::cad_boundary_graph::*;
pub use crate::cad_measure::*;
pub use crate::cad_point_grid::*;
pub use crate::cad_session::*;
pub use crate::cad_snap_trace::*;
pub use crate::cad_ring_snap::*;

#[cfg(test)]
#[path = "cad_interaction_tests.rs"]
mod cad_interaction_tests;
