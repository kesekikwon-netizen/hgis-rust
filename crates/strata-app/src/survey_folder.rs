//! The survey folder, derived from the survey path at each use (never stored: a stored copy goes stale on
//! 다른 이름으로 저장). ≡ cpp `ReferenceStorage::surveyFolder` (target/cpp-ref/src/core/ReferenceStorage.cpp:39-43).

use std::path::{Path, PathBuf};

/// `ReferenceStorage::surveyFolder` — `None` for no path or a path that is not an existing file;
/// otherwise the absolute, cleaned parent folder.
pub(crate) fn survey_folder(survey_path: Option<&Path>) -> Option<PathBuf> {
    let survey = survey_path?;
    if !survey.is_file() {
        return None;
    }
    std::path::absolute(survey).ok()?.parent().map(Path::to_path_buf)
}

#[cfg(test)]
#[path = "survey_folder_tests.rs"]
mod tests;
