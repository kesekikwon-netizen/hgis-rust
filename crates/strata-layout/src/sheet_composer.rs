//! `sheet_composer` 는 도면 조판 기능의 재수출 창구다.
//!
//! 원래 한 파일(1647줄)을 300줄 규칙에 맞춰 주제별로 나눴다.
//! 기존 호출부의 `strata_layout::sheet_composer::<이름>` 경로는 그대로 동작한다.

pub use crate::sheet_decorations::*;
pub use crate::sheet_drawing::*;
pub use crate::sheet_geometry::*;
pub use crate::sheet_heritage::*;
pub use crate::sheet_labels::*;
pub use crate::sheet_layout::*;

#[cfg(test)]
#[path = "sheet_composer_tests.rs"]
mod sheet_composer_tests;
