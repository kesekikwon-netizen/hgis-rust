//! `pipeline` 는 고고 지형 분석 파이프라인의 재수출 창구다.
//!
//! 원래 한 파일(568줄)을 300줄 규칙에 맞춰 주제별로 나눴다.
//! 기존 호출부의 `strata_paleo::pipeline::<이름>` 경로는 그대로 동작한다.

pub use crate::pipeline_analysis::*;
pub use crate::pipeline_cutline::*;
pub use crate::pipeline_stage::*;
