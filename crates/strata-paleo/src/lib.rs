#![allow(
    unknown_lints,
    float_literal_f32_fallback,
    clippy::chunks_exact_to_as_chunks,
    clippy::drain_collect
)]
#![forbid(unsafe_code)]

pub mod borehole_color_ramp;
pub mod borehole_geotiff;
pub mod borehole_gpkg;
pub mod borehole_gpkg_geom;
pub mod borehole_idw;
pub mod borehole_isopach;
pub mod borehole_map_accumulate;
pub mod borehole_map_files;
pub mod borehole_map_grid;
pub mod borehole_map_options;
pub mod borehole_maps;
pub mod borehole_strata;
pub mod dem_synthesizer;
pub mod pipeline;
pub mod section_renderer;
pub mod terrain_edt_tpi;

#[cfg(test)]
#[path = "borehole_color_ramp_tests.rs"]
mod borehole_color_ramp_tests;

#[cfg(test)]
#[path = "borehole_maps_tests.rs"]
mod borehole_maps_tests;
