//! `address_search` 는 주소·지명 검색 기능의 재수출 창구다.
//!
//! 원래 1372줄 한 파일이던 것을 300줄 규칙(마스터플랜 Chapter 07)에 맞춰 나눴다.
//! 기존 `strata_net::address_search::<이름>` 경로는 그대로 동작한다.

#[path = "address_types.rs"]
mod address_types;
#[path = "address_sido_data.rs"]
mod address_sido_data;
#[path = "address_catalog.rs"]
mod address_catalog;
#[path = "address_parcel.rs"]
mod address_parcel;
#[path = "address_geocode.rs"]
mod address_geocode;
#[path = "address_vworld_search.rs"]
mod address_vworld_search;
#[path = "address_query.rs"]
mod address_query;

pub use address_catalog::*;
pub use address_geocode::*;
pub use address_parcel::*;
pub use address_query::*;
pub use address_sido_data::*;
pub use address_types::*;

#[cfg(test)]
#[path = "address_search_tests.rs"]
mod tests;
