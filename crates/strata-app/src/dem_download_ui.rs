//! `dem_download_ui` 는 DEM 내려받기 창·상태·경로 규칙의 재수출 창구다.
//!
//! 원래 459줄 한 파일이던 것을 300줄 규칙(마스터플랜 Chapter 07)에 맞춰 나눴다.
//! 기존 `crate::dem_download_ui::<이름>` 경로는 그대로 동작한다.

#[path = "dem_download_ui_paths.rs"]
mod dem_download_ui_paths;
#[path = "dem_download_ui_render.rs"]
mod dem_download_ui_render;
#[path = "dem_download_ui_state.rs"]
mod dem_download_ui_state;

pub use dem_download_ui_paths::*;
pub use dem_download_ui_render::*;
pub use dem_download_ui_state::*;

#[cfg(test)]
#[path = "dem_download_ui_tests.rs"]
mod tests;
