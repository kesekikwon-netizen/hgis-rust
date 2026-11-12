//! `real_survey` 는 실제 조사 자료(GeoPackage) 읽기·레이어 상태·도형 조작의 재수출 창구다.
//!
//! 원래 668줄 한 파일이던 것을 300줄 규칙(마스터플랜 Chapter 07)에 맞춰 나눴다.
//! 기존 `crate::real_survey::<이름>` 경로는 그대로 동작한다.
//!
//! `RealSurveyData` 의 `impl` 블록은 주제별 파일로 나뉘어 있다(Rust 는 같은 타입에 여러 파일의
//! `impl` 을 허용한다). 항목을 실제로 내보내는 것은 `real_survey_types` 하나이고, 나머지
//! 세 파일은 `impl` 만 담는다. 그래서 `pub use` 는 그 하나만 하면 된다.

#[path = "real_survey_actions.rs"]
mod real_survey_actions;
#[path = "real_survey_layers.rs"]
mod real_survey_layers;
#[path = "real_survey_load.rs"]
mod real_survey_load;
#[path = "real_survey_types.rs"]
mod real_survey_types;

pub use real_survey_types::*;
