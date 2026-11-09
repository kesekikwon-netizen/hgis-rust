//! `tabs_home_map` 는 홈 지도 탭 상태와 실행취소 스택의 재수출 창구다.
//!
//! 원래 359줄 한 파일이던 것을 300줄 규칙(마스터플랜 Chapter 07)에 맞춰 나눴다.
//! 기존 `crate::tabs_home_map::<이름>` 경로는 그대로 동작한다.

#[path = "tabs_home_map_state.rs"]
mod tabs_home_map_state;
#[path = "tabs_home_map_undo.rs"]
mod tabs_home_map_undo;

pub use tabs_home_map_state::*;
pub use tabs_home_map_undo::*;

#[cfg(test)]
#[path = "tabs_home_map_tests.rs"]
mod tests;
