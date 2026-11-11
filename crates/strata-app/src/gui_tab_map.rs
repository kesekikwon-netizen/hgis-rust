//! `gui_tab_map` 은 지도 탭의 재수출 창구다.
//!
//! 원래 한 파일(2,286줄, `render_map_tab_ui` 한 함수가 808줄)이던 것을
//! 300줄 규칙(마스터플랜 Chapter 07)에 맞춰 나눴다.
//! 기존 `crate::gui_tab_map::<이름>` 경로는 그대로 동작한다 —
//! `gui_shell` 이 쓰는 `UX_MAP_*`·`ShellGridConfig`·`GRID_BAND_*`·`scale_bar_*`·`grid_*`,
//! `right_inspector` 가 쓰는 `UX_MAP_RIGHT_WIDTH` 를 모두 여기서 재수출한다.

#[path = "gui_tab_map_canvas_overlay.rs"]
mod gui_tab_map_canvas_overlay;
#[path = "gui_tab_map_canvas.rs"]
mod gui_tab_map_canvas;
#[path = "gui_tab_map_consts.rs"]
mod gui_tab_map_consts;
#[path = "gui_tab_map_context.rs"]
mod gui_tab_map_context;
#[path = "gui_tab_map_format.rs"]
mod gui_tab_map_format;
#[path = "gui_tab_map_geom.rs"]
mod gui_tab_map_geom;
#[path = "gui_tab_map_grid.rs"]
mod gui_tab_map_grid;
#[path = "gui_tab_map_gridctl.rs"]
mod gui_tab_map_gridctl;
#[path = "gui_tab_map_domain.rs"]
mod gui_tab_map_domain;
#[path = "gui_tab_map_labels.rs"]
mod gui_tab_map_labels;
#[path = "gui_tab_map_heritage.rs"]
mod gui_tab_map_heritage;
#[path = "gui_tab_map_hud.rs"]
mod gui_tab_map_hud;
#[path = "gui_tab_map_measure.rs"]
mod gui_tab_map_measure;
#[path = "gui_tab_map_opacity.rs"]
mod gui_tab_map_opacity;
#[path = "gui_tab_map_overlay.rs"]
mod gui_tab_map_overlay;
#[path = "gui_tab_map_reference.rs"]
mod gui_tab_map_reference;
#[path = "gui_tab_map_tab.rs"]
mod gui_tab_map_tab;
#[path = "gui_tab_map_undo.rs"]
mod gui_tab_map_undo;

pub use gui_tab_map_consts::*;
pub use gui_tab_map_format::*;
pub use gui_tab_map_geom::*;
pub use gui_tab_map_grid::paint_projected_grid;

#[cfg(test)]
#[path = "gui_tab_map_tests.rs"]
mod tests_measure_hud;
