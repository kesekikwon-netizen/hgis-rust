//! `cadastral_download_ui` 는 「지적」 내려받기 창의 재수출 창구다.
//!
//! 원래 639줄 한 파일이던 것을 300줄 규칙(마스터플랜 Chapter 07)에 맞춰 나눴다.
//! 기존 `crate::cadastral_download_ui::<이름>` 경로는 그대로 동작한다.

#[path = "cadastral_download_ui_paths.rs"]
mod cadastral_download_ui_paths;
#[path = "cadastral_download_ui_render.rs"]
mod cadastral_download_ui_render;
#[path = "cadastral_download_ui_state.rs"]
mod cadastral_download_ui_state;
#[path = "cadastral_download_ui_worker.rs"]
pub(crate) mod cadastral_download_ui_worker;

pub use cadastral_download_ui_paths::*;
pub use cadastral_download_ui_render::*;

#[cfg(test)]
#[path = "cadastral_download_ui_tests.rs"]
mod tests;
