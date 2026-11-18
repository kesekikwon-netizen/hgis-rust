//! Submit package from the open survey. `KaSubmitFlow.cpp:81-171` asks, then
//! `ExportService::exportSubmissionPackage` (`ExportService.cpp:73-241`) writes the folder.
//! The placeholder in `submit_export.rs` is not on this path. DXF is not a submit file
//! (`CLAUDE.md` product rule; only `SUBMIT_KEYS` are written).

use std::path::PathBuf;

use strata_check_export::checklist_engine::ChecklistEngine;
use strata_engine::SubmitEncoding;

use crate::app_state::StrataAppState;

/// The question in front of the folder (`KaSubmitFlow.cpp:84-122`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SubmitPhase {
    Errors {
        errors: usize,
        summary: String,
    },
    Unsaved {
        summary: String,
        errors_remain: bool,
    },
    Encoding {
        summary: String,
        errors_remain: bool,
        save_first: bool,
    },
}

const SAVE_FAILED: &str =
    "저장하지 못해 제출 꾸러미를 만들지 않았습니다. 저장 문제를 해결한 뒤 다시 만드세요.";

/// `SubmitReadme.cpp:106-110`.
pub(crate) fn folder_name_part(text: &str) -> String {
    let mut safe: String = text
        .trim()
        .chars()
        .map(|c| {
            if c < ' ' || "\\/:*?\"<>|".contains(c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    while safe.ends_with('.') || safe.ends_with(' ') {
        safe.pop();
    }
    safe.chars().take(40).collect()
}

/// `KaSubmitFlow.cpp:124-130` — `제출_<name>_<stamp>`, then `_2` … `_99` when that folder exists.
pub(crate) fn package_folder_name(
    survey_name: &str,
    stamp: &str,
    exists: impl Fn(&str) -> bool,
) -> String {
    let part = folder_name_part(survey_name);
    let base = if part.is_empty() {
        format!("제출_{stamp}")
    } else {
        format!("제출_{part}_{stamp}")
    };
    let mut name = base.clone();
    let mut n = 2;
    while exists(&name) && n < 100 {
        name = format!("{base}_{n}");
        n += 1;
    }
    name
}

fn checklist(app: &StrataAppState) -> (usize, String) {
    let engine = ChecklistEngine::with_default_rules();
    let results = engine.evaluate(&app.checklist_state());
    (
        ChecklistEngine::failed_count(&results, "error"),
        ChecklistEngine::summary_text(&results),
    )
}

impl StrataAppState {
    /// `KaSubmitFlow.cpp:81` — open the review and ask before any folder dialog.
    pub(crate) fn begin_submit_package(&mut self) {
        if self.submit_phase.is_some() {
            return;
        }
        self.modals.active = Some(crate::dialogs_modal::ActiveModalDialog::ExportSubmitPackage);
        let (errors, summary) = checklist(self);
        self.submit_phase = Some(if errors > 0 {
            SubmitPhase::Errors { errors, summary }
        } else if self.is_dirty {
            SubmitPhase::Unsaved {
                summary,
                errors_remain: false,
            }
        } else {
            SubmitPhase::Encoding {
                summary,
                errors_remain: false,
                save_first: false,
            }
        });
    }

    pub(crate) fn cancel_submit_package(&mut self) {
        self.submit_phase = None;
    }

    /// 「그래도 만들기」 (`KaSubmitFlow.cpp:90-94`).
    pub(crate) fn accept_submit_errors(&mut self) {
        let Some(SubmitPhase::Errors { summary, .. }) = self.submit_phase.clone() else {
            return;
        };
        self.submit_phase = Some(if self.is_dirty {
            SubmitPhase::Unsaved {
                summary,
                errors_remain: true,
            }
        } else {
            SubmitPhase::Encoding {
                summary,
                errors_remain: true,
                save_first: false,
            }
        });
    }

    /// `KaSubmitFlow.cpp:107-120`. `save_first` is 「저장하고 만들기」.
    pub(crate) fn answer_submit_unsaved(&mut self, save_first: bool) {
        let Some(SubmitPhase::Unsaved {
            summary,
            errors_remain,
        }) = self.submit_phase.clone()
        else {
            return;
        };
        if save_first && (!self.save_survey_project() || self.is_dirty) {
            self.submit_phase = None;
            self.show_status_message(SAVE_FAILED);
            return;
        }
        self.submit_phase = Some(SubmitPhase::Encoding {
            summary,
            errors_remain,
            save_first,
        });
    }

    /// `KaSubmitFlow.cpp:121-171`. An empty folder choice cancels, as `getExistingDirectory` does.
    pub(crate) fn choose_submit_encoding(
        &mut self,
        encoding: SubmitEncoding,
    ) -> Result<PathBuf, String> {
        let Some(SubmitPhase::Encoding {
            summary,
            errors_remain,
            save_first,
        }) = self.submit_phase.clone()
        else {
            return Err(String::new());
        };
        self.submit_phase = None;
        let Some(parent) = self.pick_submit_parent() else {
            return Err(String::new());
        };
        self.write_submit_package(&parent, encoding, &summary, errors_remain, save_first)
    }

    fn pick_submit_parent(&self) -> Option<PathBuf> {
        if let Some(dir) = &self.submit_parent_override {
            return Some(dir.clone());
        }
        if cfg!(test) {
            return None;
        }
        rfd::FileDialog::new()
            .set_title("제출 꾸러미를 만들 위치")
            .set_directory(self.preferred_survey_dir())
            .pick_folder()
    }
}

#[path = "submit_package_write.rs"]
mod write;

#[path = "submit_package_ask.rs"]
mod ask;
pub(crate) use ask::paint_submit_phase;

#[cfg(test)]
#[path = "submit_package_run_tests.rs"]
mod tests;
