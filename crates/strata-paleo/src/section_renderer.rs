//! `section_renderer` 는 단면 렌더링의 재수출 창구다.
//!
//! 원래 한 파일(660줄)을 300줄 규칙에 맞춰 주제별로 나눴다.
//! 기존 호출부의 `strata_paleo::section_renderer::<이름>` 경로는 그대로 동작한다.

pub use crate::section_c14_calibration::*;
pub use crate::section_artifacts::*;
pub use crate::section_homography::*;
pub use crate::section_profile_render::*;
