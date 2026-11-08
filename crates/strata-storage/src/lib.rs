#![allow(
    unknown_lints,
    float_literal_f32_fallback,
    clippy::chunks_exact_to_as_chunks,
    clippy::drain_collect
)]
#![forbid(unsafe_code)]

pub mod cad_csv_io;
pub mod edit_buffer;
pub mod gpkg;
pub mod polygon_pieces;
pub mod project_actions;
pub mod session_save;
pub mod shp_io;
pub mod wkb_codec;
