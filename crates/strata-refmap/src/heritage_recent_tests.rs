//! The record of recently received heritage archives (`HeritageRecentDownloads.cpp`), on made-up files.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use chrono::{NaiveDate, NaiveDateTime};

use super::*;
use crate::heritage::DATASETS;

fn at(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(year, month, day).unwrap().and_hms_opt(hour, minute, 0).unwrap()
}

/// A record as the C++ writes it: the kinds by their Korean names, `when` as local ISO time, the files as
/// absolute paths.
fn record(folder: &Path, entries: &[(&str, &str, &[&Path])]) {
    let mut manifest = serde_json::Map::new();
    for (kind, when, files) in entries {
        let files: Vec<String> = files.iter().map(|f| f.to_string_lossy().into_owned()).collect();
        manifest.insert((*kind).to_owned(), serde_json::json!({ "when": when, "files": files }));
    }
    std::fs::write(folder.join("recent.json"), serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
}

fn archive(folder: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = folder.join(name);
    std::fs::write(&path, bytes).unwrap();
    path
}

/// `defaultRoot` (`:41-46`): `<app data>/주변유적/<시도> <시군>/원본`, the names trimmed; no folder without
/// both names.
#[test]
fn the_folder_of_a_city() {
    let root = Path::new("data");
    assert_eq!(region_folder(root, " 경상북도 ", "경주시 "), Some(root.join("주변유적").join("경상북도 경주시").join("원본")));
    assert_eq!(region_folder(root, "경상북도", " "), None);
    assert_eq!(region_folder(root, "", "경주시"), None);
}

/// `find` (`:69-78`) and `fresh` (`:34-37`): the files of a kind received within 30 days (counted in
/// calendar days), not more than a minute in the future, every file there and not empty.
#[test]
fn a_kind_received_lately_is_found() {
    let dir = tempfile::tempdir().unwrap();
    let (kind, other) = (&DATASETS[0], &DATASETS[2]);
    let zip = archive(dir.path(), "지정유산_1.zip", b"zip");
    record(dir.path(), &[(kind.name, "2026-10-25T23:59:00", &[&zip])]);

    assert_eq!(find(dir.path(), kind, at(2026, 10, 26, 9, 0)), std::slice::from_ref(&zip));
    assert_eq!(find(dir.path(), kind, at(2026, 11, 24, 0, 1)), std::slice::from_ref(&zip), "30 calendar days later");
    assert!(find(dir.path(), kind, at(2026, 11, 25, 0, 1)).is_empty(), "31 calendar days later");
    assert_eq!(find(dir.path(), kind, at(2026, 10, 25, 23, 58)), std::slice::from_ref(&zip), "a clock a minute behind");
    assert!(find(dir.path(), kind, at(2026, 10, 25, 23, 57)).is_empty(), "received in the future");
    assert!(find(dir.path(), other, at(2026, 10, 26, 9, 0)).is_empty(), "no record of that kind");
    assert_eq!(MAX_AGE_DAYS, 30);
}

/// `usableFiles` (`:22-32`): one file gone or empty spoils the record of its kind; a record without files,
/// without a readable time, or no readable record at all gives nothing.
#[test]
fn a_record_that_cannot_be_used_gives_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let now = at(2026, 10, 26, 9, 0);
    let (zip, empty) = (archive(dir.path(), "a.zip", b"zip"), archive(dir.path(), "empty.zip", b""));
    let gone = dir.path().join("gone.zip");
    let names: Vec<&str> = DATASETS.iter().map(|d| d.name).collect();
    record(
        dir.path(),
        &[
            (names[0], "2026-10-25T08:00:00", &[&zip, &gone]),
            (names[1], "2026-10-25T08:00:00", &[&zip, &empty]),
            (names[2], "2026-10-25T08:00:00", &[]),
            (names[3], "어제", &[&zip]),
            (names[4], "2026-10-25T08:00:00", &[&zip]),
        ],
    );
    for kind in &DATASETS[..4] {
        assert!(find(dir.path(), kind, now).is_empty(), "{}", kind.name);
    }
    assert_eq!(find(dir.path(), &DATASETS[4], now), [zip]);

    std::fs::write(dir.path().join("recent.json"), "not json").unwrap();
    assert!(find(dir.path(), &DATASETS[4], now).is_empty());
    assert!(find(&dir.path().join("없음"), &DATASETS[4], now).is_empty());
}

/// `latest` (`:80-91`): the newest time among the kinds that can be used again; none: nothing.
#[test]
fn the_newest_usable_record() {
    let dir = tempfile::tempdir().unwrap();
    let zip = archive(dir.path(), "a.zip", b"zip");
    let gone = dir.path().join("gone.zip");
    record(
        dir.path(),
        &[
            (DATASETS[0].name, "2026-10-25T08:49:03", &[&zip]),
            (DATASETS[1].name, "2026-10-25T08:50:02", &[&zip]),
            (DATASETS[2].name, "2026-10-25T09:30:00", &[&gone]),
            (DATASETS[3].name, "2026-08-01T08:00:00", &[&zip]),
        ],
    );
    assert_eq!(latest(dir.path(), at(2026, 10, 26, 9, 0)), Some(at(2026, 10, 25, 8, 50).with_second(2).unwrap()));
    assert_eq!(latest(dir.path(), at(2027, 1, 1, 0, 0)), None);
    // `KaHeritageSetupDialogs.cpp:230-254`: what the dialog writes beside the check box.
    assert_eq!(reuse_note(at(2026, 10, 25, 8, 50)), "2026-10-25 08:50 받음 · 없는 자료만 서버에서 받음");
}

use chrono::Timelike;
