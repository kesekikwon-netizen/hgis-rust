//! Dropped files (target/cpp-ref/src/app/MainWindowFiles.cpp:259-289) and close prompt
//! (target/cpp-ref/src/app/KaClosePrompt.cpp).
use eframe::egui::{self, Color32, Vec2};
use crate::app_state::StrataAppState;

impl StrataAppState {
    /// `MainWindowFiles.cpp:252-258` `tryAddDroppedUrls` — 창에 끌어 놓은 파일(user_files.rs).
    pub(crate) fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let dropped: Vec<std::path::PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect());
        if !dropped.is_empty() {
            self.drop_paths(&dropped);
        }
    }
    /// 「저장하지 않고 끝내기」 of the close prompt.
    /// The close it asks for arrives while the edits are still unsaved: it is let through, not asked about
    /// again.
    pub(crate) fn close_without_saving(&mut self, ctx: &egui::Context) {
        (self.close_prompt_open, self.closing_unsaved) = (false, true);
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    /// `KaClosePrompt.cpp` — 저장하지 않은 편집이 있으면 닫기 전에 한 번 묻는다.
    pub(crate) fn render_close_prompt(&mut self, ctx: &egui::Context) {
        // A `--snap` run works on temporary copies and ends by itself: nobody is there to answer.
        if ctx.input(|i| i.viewport().close_requested()) && self.is_dirty && !self.close_prompt_open && !self.closing_unsaved && self.snap.is_none()
        {
            self.close_prompt_open = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
        if !self.close_prompt_open {
            return;
        }
        let full = ctx.screen_rect();
        let w = 420.0_f32.min(full.width() - 32.0);
        let h = 168.0_f32.min(full.height() - 32.0);
        let pos = egui::Pos2::new(
            ((full.width() - w) * 0.5).max(16.0),
            ((full.height() - h) * 0.5).max(16.0),
        );
        egui::Window::new("ka_close_prompt")
            .title_bar(false)
            .resizable(false)
            .fixed_pos(pos)
            .fixed_size([w, h])
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new("저장하지 않은 편집이 있습니다")
                        .size(14.0)
                        .strong(),
                );
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("끝내기 전에 지금까지 그린 조사 내용을 저장할까요?")
                        .size(12.0)
                        .color(Color32::from_rgb(80, 79, 74)),
                );
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    let save_btn = egui::Button::new(
                        egui::RichText::new("저장하고 끝내기")
                            .color(Color32::WHITE)
                            .strong()
                            .size(12.5),
                    )
                    .fill(Color32::from_rgb(181, 87, 58))
                    .corner_radius(6.0)
                    .min_size(Vec2::new(130.0, 30.0));
                    // C++ KaClosePrompt — 실제로 저장하고, 모두 저장됐을 때만 닫는다(실패하면 창은 그대로).
                    if ui.add(save_btn).clicked() && self.save_survey_project() && !self.is_dirty {
                        self.close_prompt_open = false;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("저장하지 않고 끝내기").size(12.0),
                            )
                            .min_size(Vec2::new(150.0, 30.0)),
                        )
                        .clicked()
                    {
                        self.close_without_saving(ctx);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(egui::RichText::new("취소").size(12.0))
                                    .min_size(Vec2::new(70.0, 30.0)),
                            )
                            .clicked()
                        {
                            self.close_prompt_open = false;
                        }
                    });
                });
            });
    }
}

#[cfg(test)]
#[path = "shell_dialogs_tests.rs"]
mod tests;
