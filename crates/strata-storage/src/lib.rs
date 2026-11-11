#![allow(
    unknown_lints,
    float_literal_f32_fallback,
    clippy::chunks_exact_to_as_chunks,
    clippy::drain_collect
)]
#![forbid(unsafe_code)]

pub mod cad_csv_io;
pub mod cad_dxf_input;
pub mod cad_gnss_photo;
pub mod cad_korean_text;
pub mod cad_layer_policy;
pub mod csv_control_points;
pub mod edit_buffer;
pub mod gpkg;
pub mod gpkg_diary;
pub mod gpkg_project;
pub mod gpkg_repository;
pub mod gpkg_repository_write;
pub mod gpkg_schema;
pub mod gpkg_std_sql;
pub mod layer_policy;
pub mod polygon_pieces;
pub mod project_actions;
pub mod project_recent;
pub mod session_save;
pub mod shp_io;
pub mod dxf_lines_reader;
pub mod shp_file_reader;
pub mod shp_file_writer;
pub mod shp_writer;
pub mod survey_doc;
pub mod survey_gpkg_read;
#[cfg(test)]
mod survey_gpkg_tests;
pub mod survey_gpkg_write;
pub mod topo_sheet;
pub mod wkb_codec;
pub mod zip_extract;
pub mod workflow_guide;
