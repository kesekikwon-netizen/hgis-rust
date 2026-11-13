#![allow(
    unknown_lints,
    float_literal_f32_fallback,
    clippy::chunks_exact_to_as_chunks,
    clippy::drain_collect
)]
#![forbid(unsafe_code)]

pub mod account_migration;
pub mod account_status;
pub mod accounts;
pub mod dpapi;
#[cfg(windows)]
mod dpapi_win;
pub mod key_stores;
pub mod mask;
pub mod secret_store;
pub mod vworld;
