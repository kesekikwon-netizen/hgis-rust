//! `vworld_client` 는 VWorld 클라이언트 기능의 재수출 창구다.
//!
//! 원래 1362줄 한 파일이던 것을 300줄 규칙(마스터플랜 Chapter 07)에 맞춰 나눴다.
//! 기존 `strata_net::vworld_client::<이름>` 경로는 그대로 동작한다.

#[path = "vworld_wms.rs"]
mod vworld_wms;
#[path = "vworld_cadastral.rs"]
mod vworld_cadastral;
#[path = "vworld_admin.rs"]
mod vworld_admin;
#[path = "vworld_wkt.rs"]
mod vworld_wkt;
#[path = "vworld_heritage.rs"]
mod vworld_heritage;
#[path = "vworld_tiles.rs"]
mod vworld_tiles;

pub use vworld_admin::*;
pub use vworld_cadastral::*;
pub use vworld_heritage::*;
pub use vworld_tiles::*;
pub use vworld_wms::*;

#[cfg(test)]
#[path = "vworld_client_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "vworld_client_tests_admin.rs"]
mod tests_admin;
