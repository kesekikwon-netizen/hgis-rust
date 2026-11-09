//! `tile_print` 는 타일 인쇄 계획·아틀라스의 재수출 창구다.
//!
//! 원래 한 파일(360줄)을 300줄 규칙에 맞춰 주제별로 나눴다.
//! 기존 호출부의 `strata_layout::tile_print::<이름>` 경로는 그대로 동작한다.

pub use crate::atlas_tiles::*;
pub use crate::tile_print_plan::*;

#[cfg(test)]
#[path = "tile_print_tests.rs"]
mod tile_print_tests;
