//! `shp_io` 는 SHP/DBF 쓰기와 지형도 도엽 번호 해석의 재수출 창구다.
//!
//! 원래 한 파일(350줄)을 300줄 규칙에 맞춰 주제별로 나눴다.
//! 기존 호출부의 `strata_storage::shp_io::<이름>` 경로는 그대로 동작한다.

pub use crate::topo_sheet::*;
pub use crate::shp_writer::*;

#[cfg(test)]
#[path = "shp_io_tests.rs"]
mod shp_io_tests;
