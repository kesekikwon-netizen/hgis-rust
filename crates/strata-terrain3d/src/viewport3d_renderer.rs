//! `viewport3d_renderer` 는 3D 지형 렌더러의 재수출 창구다.
//!
//! 원래 한 파일(728줄)을 300줄 규칙에 맞춰 주제별로 나눴다.
//! `render_terrain3d_rgba` 한 함수가 333줄이라, 하늘 채우기와 메시 래스터화
//! 두 단계를 private helper 로 그대로 옮겨 파일을 300줄 아래로 낮췄다.
//! 기존 호출부의 `strata_terrain3d::viewport3d_renderer::<이름>` 경로는 그대로 동작한다.

pub use crate::terrain3d_camera::*;
pub use crate::terrain3d_dem::*;
pub use crate::terrain3d_render::*;

#[cfg(test)]
#[path = "viewport3d_renderer_tests.rs"]
mod viewport3d_renderer_tests;
