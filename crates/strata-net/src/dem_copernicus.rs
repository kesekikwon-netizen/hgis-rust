//! `dem_copernicus` 는 Copernicus DSM 30m 타일 내려받기 기능의 재수출 창구다.
//!
//! 원래 889줄 한 파일이던 것을 300줄 규칙(마스터플랜 Chapter 07)에 맞춰 나눴다.
//! 기존 `strata_net::dem_copernicus::<이름>` 경로는 그대로 동작한다.

#[path = "dem_consts.rs"]
mod dem_consts;
#[path = "dem_tiles.rs"]
mod dem_tiles;
#[path = "dem_job.rs"]
mod dem_job;
#[path = "dem_fetch.rs"]
mod dem_fetch;
#[path = "dem_pipeline.rs"]
mod dem_pipeline;

pub use dem_consts::*;
#[cfg(test)]
pub(crate) use dem_fetch::{fetch_tile, TileFetch};
pub use dem_job::*;
pub use dem_pipeline::prepare;
pub use dem_tiles::*;

#[cfg(test)]
#[path = "dem_copernicus_tests.rs"]
mod tests;
