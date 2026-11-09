//! `trench_profile3d` 는 시굴 트렌치 3D·단면의 재수출 창구다.
//!
//! 원래 한 파일(984줄)을 300줄 규칙에 맞춰 주제별로 나눴다.
//! 기존 호출부의 `strata_terrain3d::trench_profile3d::<이름>` 경로는 그대로 동작한다.

pub use crate::trench_elevation_profile::*;
pub use crate::trench_hillshade::*;
pub use crate::trench_mesh::*;
pub use crate::trench_survey_points::*;
pub use crate::trench_volume::*;

#[cfg(test)]
#[path = "trench_profile3d_tests.rs"]
mod trench_profile3d_tests;

#[cfg(test)]
#[path = "trench_batch62_tests.rs"]
mod trench_batch62_tests;
