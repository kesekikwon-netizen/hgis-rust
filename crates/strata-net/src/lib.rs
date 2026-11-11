#![allow(
    unknown_lints,
    float_literal_f32_fallback,
    clippy::chunks_exact_to_as_chunks,
    clippy::drain_collect
)]
#![forbid(unsafe_code)]

pub mod address_search;
pub mod dem_copernicus;
pub mod geoinfo_borehole;
pub mod heritage_intranet;
pub mod kigam_soil_river;
pub mod ngii_topographic;
pub mod tile_fetcher;
pub mod topographic_catalog;
pub mod kigam_geology;
pub mod portal_session;
pub mod reference_wfs;
pub mod vworld_client;
pub mod vworld_login;

#[cfg(test)]
#[path = "geoinfo_borehole_tests.rs"]
mod geoinfo_borehole_tests;
