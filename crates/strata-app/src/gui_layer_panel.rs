//! `gui_layer_panel` 는 왼쪽 레이어 카드·파일함·레이어 우클릭 메뉴의 재수출 창구다.
//!
//! 원래 한 파일(1,187줄)이던 것을 300줄 규칙(마스터플랜 Chapter 07)에 맞춰 나눴다.
//! 기존 `crate::gui_layer_panel::<이름>` 경로는 그대로 동작한다.

#[path = "gui_layer_panel_card.rs"]
mod gui_layer_panel_card;
#[path = "gui_layer_panel_consts.rs"]
mod gui_layer_panel_consts;
#[path = "gui_layer_panel_context_menu.rs"]
mod gui_layer_panel_context_menu;
#[path = "gui_layer_panel_paint.rs"]
mod gui_layer_panel_paint;
#[path = "gui_layer_panel_row_spec.rs"]
pub(crate) mod gui_layer_panel_row_spec;
#[path = "gui_layer_panel_rows.rs"]
mod gui_layer_panel_rows;
#[path = "gui_layer_panel_tree.rs"]
mod gui_layer_panel_tree;

pub use gui_layer_panel_consts::*;
