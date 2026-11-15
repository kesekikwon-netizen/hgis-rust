//! Reference maps fetched from public map services and kept in the survey folder, as the C++ does in
//! `GeologyMapService`, `SoilMapService`, `RiverMapService`, `ReferenceTiledFetch` and `PreparedReferenceMap`
//! (`target/cpp-ref/src/core/`). The
//! rules, limits and words are the C++ ones; the code is this crate's own. Nothing here sends a request:
//! the caller hands in the function that fetches.

#![forbid(unsafe_code)]

pub mod collection;
pub mod geojson;
pub mod geology_colour;
pub mod geology_era;
pub mod geology_prepare;
pub mod geology_request;
pub mod heritage;
pub mod river_level;
pub mod river_prepare;
pub mod river_request;
pub mod soil_prepare;
pub mod soil_request;
pub mod soil_terrain;
pub mod tiled_fetch;
pub mod transfer;
