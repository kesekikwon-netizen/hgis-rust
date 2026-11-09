//! `submit_package` 는 제출 꾸러미 생성의 재수출 창구다.
//!
//! 원래 한 파일(355줄)을 300줄 규칙에 맞춰 주제별로 나눴다.
//! 기존 호출부의 `strata_check_export::submit_package::<이름>` 경로는 그대로 동작한다.

pub use crate::submit_audit::*;
pub use crate::submit_export::*;
pub use crate::submit_manifest::*;

#[cfg(test)]
#[path = "submit_package_tests.rs"]
mod submit_package_tests;
