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
