//! `checklist_engine` 는 도면 체크리스트 엔진의 재수출 창구다.
//!
//! 원래 한 파일(1290줄)을 300줄 규칙에 맞춰 주제별로 나눴다.
//! 기존 호출부의 `strata_check_export::checklist_engine::<이름>` 경로는 그대로 동작한다.

pub use crate::check_attributes::*;
pub use crate::check_engine::*;
pub use crate::check_harris::*;
pub use crate::check_rules::*;

#[cfg(test)]
#[path = "checklist_engine_tests.rs"]
mod checklist_engine_tests;
