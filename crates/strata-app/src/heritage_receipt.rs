//! The pledge receipt and `recent.json`, written where the C++ writes them.
//!
//! `MainWindow.cpp:3228-3244` puts the receipt in the survey folder's `주변유적/receipts`, and
//! `HeritageRecentDownloads.cpp:47-49` keeps `recent.json` beside the received archives.
//!
//! Two windows used to say a receipt was written while nothing wrote one. This module writes
//! it, and the words in those windows now match what happens.

use std::path::{Path, PathBuf};

/// What the receipt says about the account, without the id or the password
/// (`HeritageIntranetSettings.cpp:64-70`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountState {
    /// No account was ever set.
    None,
    /// An id, but no password.
    Incomplete,
    /// An id and a password are stored.
    Stored,
}

impl AccountState {
    /// The words the C++ writes (`describeForLog`).
    #[must_use]
    pub fn words(self) -> &'static str {
        match self {
            Self::None => "계정 없음",
            Self::Incomplete => "계정 설정 미완료",
            Self::Stored => "저장된 계정 사용",
        }
    }
}

/// The folder the receipt goes in: `<survey folder>/주변유적/receipts`.
///
/// `None` when there is no survey file, which is the C++ rule: no survey, no receipt
/// (`MainWindow.cpp:3228-3244`).
fn receipt_folder(survey_path: &Path) -> Option<PathBuf> {
    crate::survey_folder::survey_folder(Some(survey_path))
        .map(|folder| folder.join("주변유적").join("receipts"))
}

/// Writes the pledge receipt for `survey_path`. Returns the file it wrote.
///
/// The C++ writes this only after the pledge was actually agreed on the site
/// (`MainWindow.cpp:3228-3244`), and only when there is a survey path. `terms` is the pledge
/// text the page showed, which the run keeps when the box is ticked.
///
/// # Errors
///
/// When there is no survey file, or the folder cannot be made, or the file cannot be written.
pub fn write_pledge_receipt(
    survey_path: &Path,
    account: AccountState,
    terms: &str,
    at: chrono::NaiveDateTime,
) -> std::io::Result<PathBuf> {
    let folder = receipt_folder(survey_path).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "조사 파일이 없어 영수증을 남길 곳이 없습니다",
        )
    })?;
    std::fs::create_dir_all(&folder)?;
    let path = folder.join(format!("서약서-{}.txt", at.format("%Y%m%d-%H%M%S")));
    let text = format!(
        "국가유산 공간정보 원본자료 사용 서약서 동의 기록\n동의 시각: {}\n기관 계정: {}\n----\n{}\n",
        at.format("%Y-%m-%dT%H:%M:%S"),
        account.words(),
        terms
    );
    std::fs::write(&path, text)?;
    Ok(path)
}

/// Writes one kind's entry into `recent.json` beside the received archives.
///
/// The C++ keeps a **map from the kind's Korean name to its entry**
/// (`HeritageRecentDownloads.cpp:record`): `{"지정유산": {"when": "…", "files": ["…"]}}`.
/// The reader both sides use (`heritage_recent.rs:usable`) reads exactly that, so an array of
/// records would be invisible to it and the received files would never be offered again.
///
/// `region_folder` is the **`원본` folder itself** — what `heritage_recent::region_folder`
/// returns. Appending another `원본` puts the record where no reader looks.
///
/// # Errors
///
/// When the folder cannot be made or the file cannot be written.
pub fn record_recent(
    region_folder: &Path,
    kind: &str,
    files: &[PathBuf],
    when: chrono::NaiveDateTime,
) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(region_folder)?;
    let path = region_folder.join("recent.json");
    // Read what is there so kinds received earlier are kept (`record` merges into the object).
    let mut manifest: serde_json::Map<String, serde_json::Value> = std::fs::read(&path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    let list: Vec<String> = files
        .iter()
        .map(|f| f.to_string_lossy().into_owned())
        .collect();
    manifest.insert(
        kind.to_owned(),
        serde_json::json!({
            "when": when.format("%Y-%m-%dT%H:%M:%S").to_string(),
            "files": list,
        }),
    );
    std::fs::write(
        &path,
        serde_json::to_string_pretty(&serde_json::Value::Object(manifest)).unwrap_or_default(),
    )?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn when() -> chrono::NaiveDateTime {
        chrono::NaiveDate::from_ymd_opt(2026, 11, 17)
            .and_then(|d| d.and_hms_opt(9, 30, 15))
            .expect("a date")
    }

    /// The receipt has the C++ lines, in the C++ order, and never the account id.
    #[test]
    fn the_receipt_has_the_cpp_lines_and_no_account_id() {
        let dir = tempfile::tempdir().expect("temp dir");
        let survey = dir.path().join("현장.gpkg");
        std::fs::write(&survey, b"").expect("survey file");

        let path = write_pledge_receipt(&survey, AccountState::Stored, "서약 본문", when())
            .expect("written");
        let text = std::fs::read_to_string(&path).expect("read");

        assert!(
            text.starts_with("국가유산 공간정보 원본자료 사용 서약서 동의 기록"),
            "{text}"
        );
        assert!(text.contains("동의 시각: 2026-11-17T09:30:15"), "{text}");
        assert!(text.contains("기관 계정: 저장된 계정 사용"), "{text}");
        assert!(text.contains("\n----\n"), "{text}");
        assert!(text.ends_with("서약 본문\n"), "{text}");
        assert!(!text.contains("user1"), "an account id reached the receipt: {text}");
        assert_eq!(
            path.file_name().and_then(|n| n.to_str()),
            Some("서약서-20261117-093015.txt")
        );
    }

    /// The receipt goes in the survey folder, not the app's data folder.
    #[test]
    fn the_receipt_lives_in_the_survey_folder() {
        let dir = tempfile::tempdir().expect("temp dir");
        let survey = dir.path().join("조사").join("현장.gpkg");
        std::fs::create_dir_all(survey.parent().expect("parent")).expect("mkdir");
        std::fs::write(&survey, b"").expect("survey file");
        let path = write_pledge_receipt(&survey, AccountState::Stored, "t", when()).expect("written");
        assert!(
            path.starts_with(dir.path().join("조사").join("주변유적").join("receipts")),
            "{}",
            path.display()
        );
    }

    /// The three account words are the C++ ones, and none of them is the id.
    #[test]
    fn the_account_words_are_the_cpp_ones() {
        assert_eq!(AccountState::None.words(), "계정 없음");
        assert_eq!(AccountState::Incomplete.words(), "계정 설정 미완료");
        assert_eq!(AccountState::Stored.words(), "저장된 계정 사용");
    }

    /// **The point of this file**: what the run writes must be what the existing loader reads.
    ///
    /// `heritage_recent::find` is the function the app calls to put received files back on the
    /// map. Writing a shape it cannot read means the files are on the disk and never shown —
    /// which is exactly the symptom HER9 exists to remove.
    #[test]
    fn what_the_run_writes_the_existing_loader_can_read() {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = dir.path();
        let region = strata_refmap::heritage_recent::region_folder(root, "경상북도", "경주시")
            .expect("a region folder");
        std::fs::create_dir_all(&region).expect("mkdir");

        // A received archive that exists and is not empty, as the reader requires.
        let archive = region.join("지정유산_261117100000.zip");
        std::fs::write(&archive, b"PK\x03\x04data").expect("archive");

        let dataset = strata_refmap::heritage::DATASETS
            .iter()
            .find(|d| d.name == "지정유산")
            .expect("the kind");

        // Before the record: the loader finds nothing.
        let now = when();
        assert!(
            strata_refmap::heritage_recent::find(&region, dataset, now).is_empty(),
            "found a file with no record"
        );

        record_recent(&region, "지정유산", std::slice::from_ref(&archive), now).expect("recorded");

        // After: the loader finds the archive, which is what puts it on the map.
        let found = strata_refmap::heritage_recent::find(&region, dataset, now + chrono::Duration::seconds(30));
        assert_eq!(found, vec![archive], "the loader cannot read what the run wrote");
    }

    /// A second kind is added without losing the first (`record` merges into the object).
    #[test]
    fn a_second_kind_keeps_the_first() {
        let dir = tempfile::tempdir().expect("temp dir");
        let region = dir.path().join("주변유적").join("경상북도 경주시").join("원본");
        std::fs::create_dir_all(&region).expect("mkdir");
        let a = region.join("지정유산_1.zip");
        let b = region.join("지표조사구역_1.zip");
        std::fs::write(&a, b"PK\x03\x04a").expect("a");
        std::fs::write(&b, b"PK\x03\x04b").expect("b");

        record_recent(&region, "지정유산", std::slice::from_ref(&a), when()).expect("first");
        let path = record_recent(&region, "지표조사구역", std::slice::from_ref(&b), when()).expect("second");
        let text = std::fs::read_to_string(&path).expect("read");
        let parsed: serde_json::Value = serde_json::from_str(&text).expect("json");

        assert!(parsed["지정유산"]["files"].is_array(), "the first kind was lost: {text}");
        assert!(parsed["지표조사구역"]["files"].is_array(), "{text}");
        assert_eq!(parsed["지정유산"]["files"][0].as_str(), a.to_str(), "{text}");
    }
}
