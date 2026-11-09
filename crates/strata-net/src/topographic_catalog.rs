//! `topographic_catalog` 는 「지도에 올리기」 폴더 스캔 기능의 재수출 창구다.
//!
//! 원래 1028줄 한 파일이던 것을 300줄 규칙(마스터플랜 Chapter 07)에 맞춰 나눴다.
//! 기존 `strata_net::topographic_catalog::<이름>` 경로는 그대로 동작한다.
//!
//! C++ 은 GDAL/OGR 로 파일을 열어 레이어·좌표계·도형 범위를 읽는다. 이 모듈은 그중
//! 이 창이 실제로 보여 주는 만큼만 읽는다.

#[path = "topo_types.rs"]
mod topo_types;
#[path = "topo_codebook.rs"]
mod topo_codebook;
#[path = "topo_scan.rs"]
mod topo_scan;

pub use topo_codebook::*;
pub use topo_scan::*;
pub use topo_types::*;

#[cfg(test)]
#[path = "topographic_catalog_support.rs"]
mod test_support;

#[cfg(test)]
#[path = "topographic_catalog_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "topographic_catalog_tests_codebook.rs"]
mod tests_codebook;
