//! F1c-4 — the survey folder follows `survey_path` at each use, like cpp `ReferenceStorage::surveyFolder`
//! (ReferenceStorage.cpp:39-43). Consumers: heritage (MainWindow.cpp:3095-3099), topographic picker
//! (MainWindowTopographic.cpp:103-106). Tempdir surveys only.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use super::survey_folder;
use crate::app_state::StrataAppState;

fn survey_in(dir: &Path) -> PathBuf {
    let p = dir.join("조사.gpkg");
    std::fs::write(&p, b"").expect("survey file");
    p
}

// test_reference_status.cpp:137-138 (generatedFolderNamesAreRecognisedInEveryLayerForm),
// :168 (onlyReplacedFoldersOfThisSessionAreRemovedAtSave)
#[test]
fn survey_folder_matches_cpp_reference_storage() {
    assert_eq!(survey_folder(None), None);
    assert_eq!(survey_folder(Some(Path::new("Z:/없는/조사.gpkg"))), None);
    let dir = tempfile::tempdir().expect("temp dir");
    assert_eq!(survey_folder(Some(dir.path())), None, "a folder is not a survey file");
    let survey = survey_in(dir.path());
    assert_eq!(survey_folder(Some(&survey)), Some(dir.path().to_path_buf()));
    let dotted = dir.path().join(".").join("조사.gpkg");
    assert_eq!(survey_folder(Some(&dotted)), Some(dir.path().to_path_buf()), "cleaned like QDir::cleanPath");
}

/// MainWindow.cpp:3095-3099 — no survey (or a missing file) stops before any request with the cpp text.
#[test]
fn heritage_without_survey_file_stops_with_cpp_text() {
    for missing in [None, Some(PathBuf::from("Z:/없는/조사.gpkg"))] {
        let mut app = StrataAppState::new_startup();
        app.survey_path = missing;
        app.accounts.heritage_configured = true;
        app.modals.heritage_sido = "경상북도".to_owned();
        app.modals.heritage_city = "경주시".to_owned();
        crate::dialogs_download_print::heritage_start_fetch(&mut app);
        assert!(!crate::dialogs_download_print::heritage_fetch_in_progress(), "no request thread may start");
        assert_eq!(
            app.modals.heritage_status_detail,
            "조사를 먼저 열거나 저장하세요.\n받은 자료는 그 조사폴더 안에만 둡니다."
        );
    }
}

/// MainWindowTopographic.cpp:103-106 — no survey: ini `topographic/folder`, else Documents;
/// with a survey: <survey folder>/지형도, and it follows 다른 이름으로 저장.
#[test]
fn topographic_picker_starts_like_cpp() {
    let dir = tempfile::tempdir().expect("temp dir");
    let ini = dir.path().join("ka-hgis.ini");
    let mut app = StrataAppState::new_startup();
    app.settings_ini = Some(ini.clone());
    assert_eq!(app.topographic_picker_start(), crate::user_settings::documents_path());
    std::fs::write(&ini, "[topographic]\nfolder=C:/지형 자료\n").expect("ini");
    assert_eq!(app.topographic_picker_start(), Some(PathBuf::from("C:/지형 자료")));
    let a = tempfile::tempdir().expect("temp dir");
    app.survey_path = Some(survey_in(a.path()));
    assert_eq!(app.topographic_picker_start(), Some(a.path().join("지형도")));
    let b = tempfile::tempdir().expect("temp dir");
    app.survey_path = Some(survey_in(b.path()));
    assert_eq!(app.topographic_picker_start(), Some(b.path().join("지형도")));
}
