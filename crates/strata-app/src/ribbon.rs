//! `ribbon` 은 리본 상태·분류·지역 타일·상단 검색창의 재수출 창구다.
//!
//! 원래 422줄 한 파일이던 것을 300줄 규칙(마스터플랜 Chapter 07)에 맞춰 나눴다.
//! 기존 `crate::ribbon::<이름>` 경로는 그대로 동작한다.

#[path = "ribbon_appbar.rs"]
mod ribbon_appbar;
#[path = "ribbon_region_tiles.rs"]
mod ribbon_region_tiles;
#[path = "ribbon_state.rs"]
mod ribbon_state;
#[path = "ribbon_types.rs"]
mod ribbon_types;

pub use ribbon_appbar::*;
pub use ribbon_region_tiles::*;
pub use ribbon_state::*;
pub use ribbon_types::*;

#[cfg(test)]
#[path = "ribbon_tests.rs"]
mod tests;
