#![allow(
    unknown_lints,
    float_literal_f32_fallback,
    clippy::chunks_exact_to_as_chunks,
    clippy::drain_collect
)]
#![forbid(unsafe_code)]

pub mod borehole_color_ramp;
pub mod borehole_decluster;
pub mod borehole_geotiff;
pub mod borehole_gpkg;
pub mod borehole_gpkg_geom;
pub mod borehole_idw;
pub mod borehole_isopach;
pub mod borehole_log_parse;
pub mod borehole_map_accumulate;
pub mod borehole_map_files;
pub mod borehole_map_grid;
pub mod borehole_map_options;
pub mod borehole_maps;
pub mod borehole_safety;
pub mod borehole_strata;
pub mod borehole_types;
pub mod borehole_xml_parse;
pub mod dem_contour_interp;
pub mod dem_hydrology;
pub mod dem_synthesis_io;
pub mod dem_synthesizer;
pub mod edt_landform;
pub mod grid_transform;
pub mod ngii_dem;
pub mod pipeline;
pub mod pipeline_analysis;
pub mod pipeline_cutline;
pub mod pipeline_stage;
pub mod section_artifacts;
pub mod section_c14_calibration;
pub mod section_homography;
pub mod section_profile_render;
pub mod section_renderer;
pub mod terrain_edt_tpi;
pub mod terrain_hillshade;
pub mod terrain_interp;
pub mod terrain_paleo_classify;
pub mod terrain_viewshed;

#[cfg(test)]
#[path = "borehole_color_ramp_tests.rs"]
mod borehole_color_ramp_tests;

#[cfg(test)]
#[path = "borehole_maps_tests.rs"]
mod borehole_maps_tests;

#[cfg(test)]
#[path = "borehole_strata_tests.rs"]
mod borehole_strata_tests;

#[cfg(test)]
#[path = "dem_synthesizer_tests.rs"]
mod dem_synthesizer_tests;

#[cfg(test)]
#[path = "ngii_dem_tests.rs"]
mod ngii_dem_tests;

#[cfg(test)]
#[path = "pipeline_tests.rs"]
mod pipeline_tests;

#[cfg(test)]
#[path = "section_renderer_tests.rs"]
mod section_renderer_tests;

#[cfg(test)]
#[path = "terrain_edt_tpi_tests.rs"]
mod terrain_edt_tpi_tests;
