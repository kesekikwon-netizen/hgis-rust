//! Dropped files (target/cpp-ref/src/app/MainWindowFiles.cpp:259-289) and close prompt
//! (target/cpp-ref/src/app/KaClosePrompt.cpp).
use crate::app_state::StrataAppState;
use eframe::egui::{self, Color32, Vec2};

enum CloseAnswer {
    Save,
    Discard,
    Cancel,
}

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

    /// target/cpp-ref/src/app/MainWindow.cpp:333-352: establish close ownership before the workspace handles input.
    pub(crate) fn prepare_close_prompt(&mut self, ctx: &egui::Context) {
        // A `--snap` run works on temporary copies and ends by itself: nobody is there to answer.
        if !ctx.input(|i| i.viewport().close_requested()) || self.closing_unsaved || self.snap.is_some() {
            return;
        }
        // target/cpp-ref/src/app/MainWindow.cpp:334-338: keep an existing modal's ownership of the root close request.
        if !self.close_prompt_open && self.modal_window_open() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            return;
        }
        if !self.close_prompt_open {
            // target/cpp-ref/src/app/KaFeatureCardEditors.cpp:220: native FocusOut commits the old editor before closing.
            self.commit_card_typing();
            if !self.is_dirty {
                return;
            }
            self.close_prompt_open = true;
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
    }

    /// `KaClosePrompt.cpp` — 저장하지 않은 편집이 있으면 닫기 전에 한 번 묻는다.
    pub(crate) fn render_close_prompt(&mut self, ctx: &egui::Context) {
        self.prepare_close_prompt(ctx);
        if !self.close_prompt_open {
            return;
        }
        let mut answer = None;
        // target/cpp-ref/src/app/KaClosePrompt.cpp:52-56: the question blocks the underlying workspace.
        egui::Modal::new(egui::Id::new("ka_close_prompt")).show(ctx, |ui| {
            ui.set_width((ctx.screen_rect().width() - 64.0).min(420.0));
            ui.label(egui::RichText::new("저장하지 않은 편집이 있습니다").size(14.0).strong());
            ui.add_space(4.0);
            ui.label(egui::RichText::new("끝내기 전에 지금까지 그린 조사 내용을 저장할까요?").size(12.0).color(Color32::from_rgb(80, 79, 74)));
            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui| {
                let save = egui::Button::new(egui::RichText::new("저장하고 끝내기").color(Color32::WHITE).strong().size(12.5)).fill(Color32::from_rgb(181, 87, 58)).corner_radius(6.0).min_size(Vec2::new(130.0, 30.0));
                if ui.add(save).clicked() {
                    answer = Some(CloseAnswer::Save);
                }
                if ui.add(egui::Button::new(egui::RichText::new("저장하지 않고 끝내기").size(12.0)).min_size(Vec2::new(150.0, 30.0))).clicked() {
                    answer = Some(CloseAnswer::Discard);
                }
                if ui.add(egui::Button::new(egui::RichText::new("취소").size(12.0)).min_size(Vec2::new(70.0, 30.0))).clicked() {
                    answer = Some(CloseAnswer::Cancel);
                }
            });
        });
        // target/cpp-ref/src/app/KaClosePrompt.cpp:28: Save is default; Escape cancels without reaching the canvas.
        let (escape, enter) = ctx.input_mut(|i| (i.consume_key(egui::Modifiers::NONE, egui::Key::Escape), i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)));
        if escape {
            answer = Some(CloseAnswer::Cancel);
        } else if enter && answer.is_none() {
            answer = Some(CloseAnswer::Save);
        }
        match answer {
            Some(CloseAnswer::Save) => {
                self.close_prompt_open = false;
                if self.save_survey_project() && !self.is_dirty {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                } else {
                    self.close_prompt_open = true;
                }
            }
            Some(CloseAnswer::Discard) => self.close_without_saving(ctx),
            Some(CloseAnswer::Cancel) => self.close_prompt_open = false,
            None => {}
        }
    }
}

#[cfg(test)]
#[path = "shell_dialogs_tests.rs"]
mod tests;
