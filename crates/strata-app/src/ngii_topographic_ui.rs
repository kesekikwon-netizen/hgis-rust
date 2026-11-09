//! `ngii_topographic_ui` 는 「수치지형」 도엽 확인 기능의 재수출 창구다.
//!
//! 원래 332줄 한 파일이던 것을 300줄 규칙(마스터플랜 Chapter 07)에 맞춰 나눴다.
//! 기존 `crate::ngii_topographic_ui::<이름>` 경로는 그대로 동작한다.

#[path = "ngii_topographic_geo.rs"]
mod ngii_topographic_geo;
#[path = "ngii_topographic_state.rs"]
mod ngii_topographic_state;

pub use ngii_topographic_geo::*;
pub use ngii_topographic_state::*;

#[cfg(test)]
#[path = "ngii_topographic_ui_tests.rs"]
mod tests;
