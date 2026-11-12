//! `gui_tab_home` 는 「홈」 탭 렌더링과 조사 열기 동작의 재수출 창구다.
//!
//! 원래 942줄 한 파일이던 것을 300줄 규칙(마스터플랜 Chapter 07)에 맞춰 나눴다.
//! 기존 `crate::gui_tab_home::...` 경로와 동작은 그대로다.
//!
//! `render_home_tab_ui` 의 네 블록(히어로 → 최근 조사·작업 순서 → 연결 상태) 순서는
//! 원문 그대로이고, 각 블록은 자기 파일로 옮겨졌다. 순서를 바꾸면 화면 배치가 바뀐다.

use eframe::egui;

use crate::app_state::StrataAppState;

#[path = "gui_tab_home_hero.rs"]
mod gui_tab_home_hero;
#[path = "gui_tab_home_recent.rs"]
mod gui_tab_home_recent;
#[path = "gui_tab_home_recent_box.rs"]
mod gui_tab_home_recent_box;
#[path = "gui_tab_home_status.rs"]
mod gui_tab_home_status;
#[path = "gui_tab_home_open.rs"]
mod gui_tab_home_open;
#[path = "gui_tab_home_survey.rs"]
mod gui_tab_home_survey;

impl StrataAppState {
    pub(crate) fn render_home_tab_ui(&mut self, ui: &mut egui::Ui) {
        let total_w = ui.available_width().clamp(800.0, 3600.0);
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_max_width(total_w);
                egui::Frame::NONE
                    .inner_margin(egui::Margin::symmetric(20, 14))
                    .show(ui, |ui| {
                        let inner_w = (total_w - 40.0).max(760.0);
                        ui.set_max_width(inner_w);

                        // (1) 히어로: 새 조사·조사 열기 + 이어서 작업 표
                        self.render_home_hero(ui, inner_w);

                        ui.add_space(22.0);

                        // (2) 최근 조사 목록 + 작업 순서
                        self.render_home_recent_and_guide(ui, inner_w);

                        ui.add_space(18.0);
                        ui.separator();
                        ui.add_space(6.0);

                        // (3) 연결 상태
                        self.render_home_connection_status(ui);
                    });
            });
    }
}

#[cfg(test)]
#[path = "gui_tab_home_tests.rs"]
mod tests_new_survey;
