#![allow(
    unknown_lints,
    float_literal_f32_fallback,
    clippy::chunks_exact_to_as_chunks,
    clippy::drain_collect
)]
#![forbid(unsafe_code)]

pub mod crs;
pub mod error;
pub mod process;
pub mod schema;
pub mod schema_layer;
pub mod theme_contrast;
pub mod theme_display;
pub mod theme_era;
pub mod theme_palette_light;
pub mod theme_palette_themed;
pub mod theme_splash;
pub mod theme_tokens;
pub mod theme_tokens_base;
