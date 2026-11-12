//! `gpkg` 는 GeoPackage 저장소의 재수출 창구다.
//!
//! 원래 한 파일(951줄)을 300줄 규칙에 맞춰 주제별로 나눴다.
//! 기존 호출부의 `strata_storage::gpkg::<이름>` 경로는 그대로 동작한다.

pub use crate::gpkg_diary::*;
pub use crate::gpkg_project::*;
pub use crate::gpkg_repository::*;
pub use crate::gpkg_schema::*;

#[cfg(test)]
#[path = "gpkg_core_tests.rs"]
mod gpkg_core_tests;

#[cfg(test)]
#[path = "gpkg_project_tests.rs"]
mod gpkg_project_tests;

#[cfg(test)]
#[path = "survey_gpkg_create_tests.rs"]
mod survey_gpkg_create_tests;
