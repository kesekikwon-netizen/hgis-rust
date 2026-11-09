//! `canvas_painter` 는 화면 그리기·선택·적중 검사의 재수출 창구다.
//!
//! 원래 한 파일(538줄)을 300줄 규칙에 맞춰 주제별로 나눴다.
//! 기존 호출부의 `strata_canvas2d::canvas_painter::<이름>` 경로는 그대로 동작한다.

pub use crate::painter_hatch::*;
pub use crate::painter_marker::*;
pub use crate::painter_pick::*;
pub use crate::painter_scene::*;

#[cfg(test)]
#[path = "canvas_painter_tests.rs"]
mod canvas_painter_tests;
