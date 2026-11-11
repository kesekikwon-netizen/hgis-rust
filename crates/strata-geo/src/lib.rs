#![allow(
    unknown_lints,
    float_literal_f32_fallback,
    clippy::chunks_exact_to_as_chunks,
    clippy::drain_collect
)]
#![forbid(unsafe_code)]

pub mod boolean_split;
pub mod buffer_round;
pub mod cadastral_parcel;
pub mod cluster_traverse;
pub mod contour_tin;
pub mod crs_datum_shift;
pub mod crs_transform;
pub mod gcp_solve;
pub mod georef_quality;
pub mod helmert_solve;
pub mod mbr_catalog;
pub mod polygon_ops;
pub mod sliver_hole;
pub mod snap_index;
pub mod snap_vertex_ops;
pub mod trench_edit;
pub mod trench_generate;
pub mod trench_grid;
pub mod trench_lshape;
pub mod trench_ratio_fill;
pub mod trench_stats;
pub mod trench_types;

#[cfg(test)]
#[path = "polygon_ops_tests.rs"]
mod polygon_ops_tests;

#[cfg(test)]
#[path = "georef_tests.rs"]
mod georef_tests;

#[cfg(test)]
#[path = "trench_grid_tests.rs"]
mod trench_grid_tests;
