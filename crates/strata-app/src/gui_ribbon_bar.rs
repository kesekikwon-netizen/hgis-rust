//! `gui_ribbon_bar` 는 리본(상단 7그룹)·앱바·스튜디오 탭바·우클릭 메뉴·지역 찾기의
//! 재수출 창구다.
//!
//! 원래 한 파일(2,892줄, 가장 큰 `render_cpp_top_bars` 가 710줄)이던 것을
//! 300줄 규칙(마스터플랜 Chapter 07)에 맞춰 나눴다.
//! 기존 `crate::gui_ribbon_bar::<이름>` 경로는 그대로 동작한다 —
//! `app_state`·`app_state_lifecycle` 이 쓰는 `AlignSessionState`,
//! `gui_shell` 이 쓰는 `UX_RIBBON_*`·`ux_ribbon_group_*`·`REGION_*`,
//! `verify_harness_v15_gate` 가 쓰는 `UX_RIBBON_GROUPS` 를 모두 여기서 재수출한다.
//!
//! 본문은 이름 하나 바뀌지 않았다. 300줄을 넘던 함수들은 연속 구간으로 잘라
//! `&mut self` 메서드로 옮겼고, 각 메서드 머리의 alias 줄이 원래 지역변수 이름을
//! 되살린다.

#[path = "gui_ribbon_align.rs"]
mod gui_ribbon_align;
#[path = "gui_ribbon_align_actions.rs"]
mod gui_ribbon_align_actions;
#[path = "gui_ribbon_appbar.rs"]
mod gui_ribbon_appbar;
#[path = "gui_ribbon_click_a.rs"]
mod gui_ribbon_click_a;
#[path = "gui_ribbon_click_b.rs"]
mod gui_ribbon_click_b;
#[path = "gui_ribbon_consts.rs"]
mod gui_ribbon_consts;
#[path = "gui_ribbon_draw.rs"]
mod gui_ribbon_draw;
#[path = "gui_ribbon_menu_actions.rs"]
mod gui_ribbon_menu_actions;
#[path = "gui_ribbon_menus_a.rs"]
mod gui_ribbon_menus_a;
#[path = "gui_ribbon_menus_b.rs"]
mod gui_ribbon_menus_b;
#[path = "gui_ribbon_region.rs"]
mod gui_ribbon_region;
#[path = "gui_ribbon_region_b.rs"]
mod gui_ribbon_region_b;
#[path = "gui_ribbon_search.rs"]
mod gui_ribbon_search;
#[path = "gui_ribbon_subtoolbar.rs"]
mod gui_ribbon_subtoolbar;
#[path = "gui_ribbon_draw_row.rs"]
mod gui_ribbon_draw_row;
#[path = "gui_ribbon_snap_assist.rs"]
mod gui_ribbon_snap_assist;
#[path = "gui_ribbon_tabbar.rs"]
mod gui_ribbon_tabbar;
#[path = "gui_ribbon_top.rs"]
mod gui_ribbon_top;

pub use gui_ribbon_align::*;
pub use gui_ribbon_consts::*;

#[cfg(test)]
#[path = "gui_ribbon_tests_align.rs"]
mod tests_align;
#[cfg(test)]
#[path = "gui_ribbon_tests_overlay.rs"]
mod tests_overlay_toggles;
