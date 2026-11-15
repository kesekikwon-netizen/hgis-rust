//! 「새 조사」 and 「열기」 with unsaved work ask first — C++ `MainWindow::newSurvey` (MainWindowSession.cpp:668-676)
//! and `confirmSaveBeforeOpeningSurvey` (:254, :1180-1193): 저장 saves and goes on only when everything was
//! saved, 저장 안 함 goes on and leaves the edits, 취소 stays.

use std::path::PathBuf;

use eframe::egui::{self, Color32, Vec2};

use crate::app_state::StrataAppState;
use crate::dialogs_modal::ActiveModalDialog;

/// What waits for the answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SaveBefore {
    NewSurvey,
    Open(PathBuf),
    OpenRecent(String),
}

/// The three buttons of the question (QMessageBox Save / Discard / Cancel).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SaveBeforeAnswer {
    Save,
    Discard,
    Cancel,
}

impl StrataAppState {
    /// 「새 조사」 (the ribbon, the home screen, Ctrl+N).
    pub(crate) fn request_new_survey(&mut self) {
        self.ask_or_go(SaveBefore::NewSurvey);
    }

    /// The survey chosen in 「열기」.
    pub(crate) fn request_open_survey(&mut self, path: PathBuf) {
        self.ask_or_go(SaveBefore::Open(path));
    }

    /// A survey of the recent list.
    pub(crate) fn request_open_recent(&mut self, path: String) {
        self.ask_or_go(SaveBefore::OpenRecent(path));
    }

    fn ask_or_go(&mut self, next: SaveBefore) {
        if self.is_dirty {
            self.save_before = Some(next);
        } else {
            self.go_on(next);
        }
    }

    fn go_on(&mut self, next: SaveBefore) {
        match next {
            SaveBefore::NewSurvey => self.modals.active = Some(ActiveModalDialog::NewSurvey),
            SaveBefore::Open(path) => drop(self.open_survey_gpkg(&path)),
            SaveBefore::OpenRecent(path) => self.open_recent_survey(&path),
        }
    }

    /// The words of the question waiting, if one is.
    pub(crate) fn save_before_words(&self) -> Option<&'static str> {
        self.save_before.as_ref().map(|next| match next {
            SaveBefore::NewSurvey => "현재 조사에 저장하지 않은 작업이 있습니다. 새 조사를 만들기 전에 저장할까요?",
            SaveBefore::Open(_) | SaveBefore::OpenRecent(_) => "현재 조사에 저장하지 않은 작업이 있습니다. 선택한 조사를 열기 전에 저장할까요?",
        })
    }

    /// The answer to the question.
    pub(crate) fn answer_save_before(&mut self, answer: SaveBeforeAnswer) {
        let Some(next) = self.save_before.take() else { return };
        match answer {
            // A failed save keeps the question: nothing is left unsaved behind the user's back.
            SaveBeforeAnswer::Save if !(self.save_survey_project() && !self.is_dirty) => self.save_before = Some(next),
            SaveBeforeAnswer::Save | SaveBeforeAnswer::Discard => self.go_on(next),
            SaveBeforeAnswer::Cancel => {}
        }
    }

    /// The question in the middle of the window, as the close prompt.
    pub(crate) fn render_save_before(&mut self, ctx: &egui::Context) {
        let Some(words) = self.save_before_words() else { return };
        let mut answer = None;
        egui::Window::new("ka_save_before").title_bar(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO).fixed_size([440.0, 120.0]).show(ctx, |ui| {
            ui.label(egui::RichText::new(words).size(13.0));
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                let save = egui::Button::new(egui::RichText::new("저장").color(Color32::WHITE).strong()).fill(Color32::from_rgb(181, 87, 58)).min_size(Vec2::new(88.0, 30.0));
                if ui.add(save).clicked() {
                    answer = Some(SaveBeforeAnswer::Save);
                }
                if ui.add(egui::Button::new("저장 안 함").min_size(Vec2::new(96.0, 30.0))).clicked() {
                    answer = Some(SaveBeforeAnswer::Discard);
                }
                if ui.add(egui::Button::new("취소").min_size(Vec2::new(72.0, 30.0))).clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    answer = Some(SaveBeforeAnswer::Cancel);
                }
            });
        });
        if let Some(answer) = answer {
            self.answer_save_before(answer);
        }
    }
}

#[cfg(test)]
#[path = "app_state_save_before_tests.rs"]
mod tests;
