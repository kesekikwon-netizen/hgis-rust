//! `trench_grid` 는 시굴 트렌치 기능의 재수출 창구다.
//!
//! 원래 511줄이던 파일을 300줄 규칙에 맞춰 나눴다.
//! 기존 `strata_geo::trench_grid::<이름>` 경로는 그대로 동작한다.

pub use crate::trench_edit::*;
pub use crate::trench_generate::*;
pub use crate::trench_lshape::*;
pub use crate::trench_stats::*;
pub use crate::trench_types::*;
