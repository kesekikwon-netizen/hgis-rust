//! `tile_fetcher` 는 타일 캐시·배경지도·웹뷰 자동화 기능의 재수출 창구다.
//!
//! 원래 431줄 한 파일이던 것을 300줄 규칙(마스터플랜 Chapter 07)에 맞춰 나눴다.
//! 기존 `strata_net::tile_fetcher::<이름>` 경로는 그대로 동작한다.

#[path = "tile_cache_consts.rs"]
mod tile_cache_consts;
#[path = "tile_dispatcher.rs"]
mod tile_dispatcher;
#[path = "tile_layers.rs"]
mod tile_layers;

pub use tile_cache_consts::*;
pub use tile_dispatcher::*;
pub use tile_layers::*;

#[cfg(test)]
#[path = "tile_fetcher_tests.rs"]
mod tests;
