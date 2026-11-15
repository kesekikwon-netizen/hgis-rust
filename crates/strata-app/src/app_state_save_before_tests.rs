//! 「새 조사」 and 「열기」 with unsaved work ask first — C++ `MainWindow::newSurvey` (MainWindowSession.cpp:668-676)
//! and `confirmSaveBeforeOpeningSurvey` (:254, :1180-1193); MORE0 M2-1, M2-2, M7-1, M7-2.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{SaveBefore, SaveBeforeAnswer};
use crate::app_state::StrataAppState;
use crate::dialogs_modal::ActiveModalDialog;

fn survey(dir: &std::path::Path, name: &str) -> (StrataAppState, std::path::PathBuf) {
    let path = dir.join(format!("{name}.gpkg"));
    let crs = strata_types::crs::WorkCrs::Epsg5187;
    strata_storage::gpkg::GpkgRepository::create_new_survey(&path, crs).unwrap();
    let mut app = StrataAppState::new_startup();
    app.start_new_survey_project_with_path(name, &path, crs);
    (app, path)
}

/// Unsaved work: 「새 조사」 asks with the C++ words; 취소 keeps everything; 저장 안 함 goes on to the new
/// survey window; 저장 saves and then goes on. Without unsaved work it goes on at once.
#[test]
fn a_new_survey_asks_first_when_work_is_unsaved() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, _) = survey(dir.path(), "가");
    app.request_new_survey();
    assert_eq!(app.modals.active, Some(ActiveModalDialog::NewSurvey), "nothing unsaved: no question");

    app.modals.active = None;
    app.is_dirty = true;
    app.request_new_survey();
    assert_eq!(app.save_before, Some(SaveBefore::NewSurvey));
    assert_eq!(app.save_before_words(), Some("현재 조사에 저장하지 않은 작업이 있습니다. 새 조사를 만들기 전에 저장할까요?"));
    assert_eq!(app.modals.active, None);
    app.answer_save_before(SaveBeforeAnswer::Cancel);
    assert_eq!((app.save_before.clone(), app.modals.active, app.is_dirty), (None, None, true));

    app.request_new_survey();
    app.answer_save_before(SaveBeforeAnswer::Discard);
    assert_eq!(app.modals.active, Some(ActiveModalDialog::NewSurvey));

    app.modals.active = None;
    app.request_new_survey();
    app.answer_save_before(SaveBeforeAnswer::Save);
    assert!(!app.is_dirty, "saved");
    assert_eq!(app.modals.active, Some(ActiveModalDialog::NewSurvey));
}

/// Unsaved work: opening another survey asks with the C++ words; 취소 keeps the open survey; 저장 안 함
/// opens the other one.
#[test]
fn opening_another_survey_asks_first_when_work_is_unsaved() {
    let dir = tempfile::tempdir().unwrap();
    let (_, other) = survey(dir.path(), "나");
    let (mut app, first) = survey(dir.path(), "가");
    app.is_dirty = true;
    app.request_open_survey(other.clone());
    assert_eq!(app.save_before, Some(SaveBefore::Open(other.clone())));
    assert_eq!(app.save_before_words(), Some("현재 조사에 저장하지 않은 작업이 있습니다. 선택한 조사를 열기 전에 저장할까요?"));
    app.answer_save_before(SaveBeforeAnswer::Cancel);
    assert_eq!(app.survey_path.as_deref(), Some(first.as_path()));
    app.request_open_survey(other.clone());
    app.answer_save_before(SaveBeforeAnswer::Discard);
    assert_eq!(app.survey_path.as_deref(), Some(other.as_path()));
    assert!(!app.is_dirty);
}

/// A survey of the recent list asks the same way.
#[test]
fn a_recent_survey_asks_first_when_work_is_unsaved() {
    let dir = tempfile::tempdir().unwrap();
    let (_, other) = survey(dir.path(), "나");
    let (mut app, first) = survey(dir.path(), "가");
    app.is_dirty = true;
    let recent = other.to_string_lossy().into_owned();
    app.request_open_recent(recent.clone());
    assert_eq!(app.save_before, Some(SaveBefore::OpenRecent(recent)));
    assert_eq!(app.survey_path.as_deref(), Some(first.as_path()));
    app.answer_save_before(SaveBeforeAnswer::Discard);
    assert_eq!(app.survey_path.as_deref(), Some(other.as_path()));
}
