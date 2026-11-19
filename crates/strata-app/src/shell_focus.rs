//! Map focus and the left panel, target/cpp-ref/src/app/KaShellFocus.cpp:99-146.
//! The keys are `F9` and `Ctrl+F11` (KaShellFocus.h:27-28). The menu rows call the same toggles.

use crate::app_state::StrataAppState;

impl StrataAppState {
    /// `KaShellFocus::toggleLeftPanel` (KaShellFocus.cpp:108, messages :103-105).
    pub(crate) fn toggle_left_panel(&mut self) {
        self.right_inspector.left_panel_collapsed = !self.right_inspector.left_panel_collapsed;
        self.home_map.draw_guide_message = Some(
            if self.right_inspector.left_panel_collapsed {
                "왼쪽 패널을 접었습니다. F9 키를 누르면 다시 펼칩니다."
            } else {
                "왼쪽 패널을 펼쳤습니다."
            }
            .to_owned(),
        );
    }

    /// `KaShellFocus::toggleMapFocus` (KaShellFocus.cpp:146, messages :133-143).
    pub(crate) fn toggle_map_focus(&mut self) {
        self.right_inspector.map_focused = !self.right_inspector.map_focused;
        self.home_map.draw_guide_message = Some(
            if self.right_inspector.map_focused {
                "지도 넓게 보기 — Ctrl+F11 키를 다시 누르면 리본과 왼쪽 패널이 돌아옵니다."
            } else {
                "지도 넓게 보기를 끝냈습니다."
            }
            .to_owned(),
        );
    }
}
