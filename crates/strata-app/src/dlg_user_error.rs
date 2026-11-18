//! The field error window — `KaUserError::dress` / `warn` / `critical`
//! (`target/cpp-ref/src/app/KaUserError.cpp:65-120`). Three headings, 「닫기」, and
//! 「로그 폴더 열기」 when a log folder is set. The log button does not close the window.

use eframe::egui::{self, RichText};

use super::user_error::{self, format_body, FieldError};
use crate::app_state::StrataAppState;

const INK: egui::Color32 = egui::Color32::from_rgb(30, 30, 28);

impl StrataAppState {
    /// `KaUserError::warn`.
    pub(crate) fn warn_field(&mut self, spec: FieldError) {
        self.field_error = Some(spec);
    }

    /// `reportSaveFailure` (`MainWindowSession.cpp:1006-1016`): the three-part window, and the status
    /// sentence the map already shows. The window is the field error; the status line is not a substitute.
    pub(crate) fn open_save_failure(&mut self, reason: &str) {
        self.field_error = Some(FieldError::warn(
            "저장 실패",
            "저장 실패",
            reason,
            "지도 화면 위 알림 줄에 자세한 내용이 있습니다.",
        ));
        self.home_map.draw_guide_message = Some(format!(
            "저장 실패 — {reason} 미저장 편집은 그대로 있습니다. 다시 저장하세요."
        ));
    }

    /// The plain three-part body, for tests and the window.
    #[must_use]
    pub(crate) fn field_error_text(&self) -> String {
        self.field_error
            .as_ref()
            .map(format_body)
            .unwrap_or_default()
    }

    pub(crate) fn render_field_error(&mut self, ctx: &egui::Context) {
        let Some(mut spec) = self.field_error.clone() else {
            return;
        };
        let folder = self.log_folder.clone();
        let escape = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        let enter = ctx.input(|i| i.key_pressed(egui::Key::Enter));
        let mut close = escape;
        let mut action = false;
        let mut open_log = false;
        let mut open = true;
        let title = spec.title.clone();
        egui::Window::new(title)
            .id(egui::Id::new("kaUserError"))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                // `formatBody` (`KaUserError.cpp:34-41`): the three headings, why and how only when set.
                ui.label(RichText::new("무엇이 안 됐나").strong().color(INK));
                ui.label(RichText::new(spec.what.trim()).color(INK));
                if !spec.why.trim().is_empty() {
                    ui.label(RichText::new("왜 그런가").strong().color(INK));
                    ui.label(RichText::new(spec.why.trim()).color(INK));
                }
                if !spec.how.trim().is_empty() {
                    ui.label(RichText::new("어떻게 하나").strong().color(INK));
                    ui.label(RichText::new(spec.how.trim()).color(INK));
                }
                if !spec.details.trim().is_empty() && ui.button("자세히").clicked() {
                    spec.details_open = !spec.details_open;
                }
                if spec.details_open {
                    ui.label(RichText::new(spec.details.trim()).color(INK));
                }
                ui.horizontal(|ui| {
                    if !spec.action_label.trim().is_empty()
                        && ui.button(spec.action_label.trim()).clicked()
                    {
                        action = true;
                    }
                    if !folder.is_empty() {
                        let tip = user_error::log_button_tooltip(&folder);
                        if ui.button("로그 폴더 열기").on_hover_text(tip).clicked() {
                            open_log = true;
                        }
                    }
                    if ui.button("닫기").clicked() {
                        close = true;
                    }
                });
            });
        // Enter runs the offered fix when there is one; otherwise 닫기 (`KaUserError.cpp:98-100`).
        if enter {
            if spec.action_label.trim().is_empty() {
                close = true;
            } else {
                action = true;
            }
        }
        if !open {
            close = true;
        }
        if open_log {
            self.open_recorded_or_system_log();
        }
        if action {
            self.field_action_chosen = true;
            self.field_error = None;
        } else if close {
            self.field_error = None;
        } else {
            self.field_error = Some(spec);
        }
    }

    fn open_recorded_or_system_log(&mut self) {
        let folder = self.log_folder.clone();
        if let Some(record) = self.log_opener_records.as_mut() {
            record.push(folder);
        } else {
            let _ = user_error::open_log_folder(std::path::Path::new(&folder));
        }
    }
}

#[cfg(test)]
#[path = "dlg_user_error_tests.rs"]
mod tests;
