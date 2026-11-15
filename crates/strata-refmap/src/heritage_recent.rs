//! The record of recently received heritage archives of one city (`target/cpp-ref/src/core/
//! HeritageRecentDownloads.cpp`). The intranet builds each archive on request (40 to 120 s a kind); a kind
//! received, checked and put on the map within the last 30 days can be loaded again from the local copy,
//! when the user chooses 「최근 받은 자료 다시 쓰기」. The record lies beside the archives, in the app's own
//! data folder, never in the survey folder.

use std::path::{Path, PathBuf};

use chrono::{Duration, NaiveDateTime};
use serde_json::{Map, Value};

use crate::heritage::{HeritageDataset, DATASETS};

#[cfg(test)]
#[path = "heritage_recent_tests.rs"]
mod tests;

/// `kMaxAgeDays` (`HeritageRecentDownloads.h:21`).
pub const MAX_AGE_DAYS: i64 = 30;

/// Where the archives of a city are kept: `<app data>/주변유적/<시도> <시군>/원본`.
#[must_use]
pub fn region_folder(app_data: &Path, sido: &str, city: &str) -> Option<PathBuf> {
    let (sido, city) = (sido.trim(), city.trim());
    (!sido.is_empty() && !city.is_empty()).then(|| app_data.join("주변유적").join(format!("{sido} {city}")).join("원본"))
}

/// The record of a folder: the kinds by their Korean names. Unreadable: empty.
fn manifest(folder: &Path) -> Map<String, Value> {
    let read = std::fs::read(folder.join("recent.json")).ok().and_then(|bytes| serde_json::from_slice(&bytes).ok());
    if let Some(Value::Object(kinds)) = read { kinds } else { Map::new() }
}

/// `fresh` (`:34-37`): received not more than a minute after `now`, and at most 30 calendar days before.
fn fresh(when: NaiveDateTime, now: NaiveDateTime) -> bool {
    when <= now + Duration::seconds(60) && (now.date() - when.date()).num_days() <= MAX_AGE_DAYS
}

/// When a kind was received and its archives, if the record can be used again at `now` (`usableFiles`,
/// `:22-32`: every file there and not empty, at least one).
fn usable(kinds: &Map<String, Value>, dataset: &HeritageDataset, now: NaiveDateTime) -> Option<(NaiveDateTime, Vec<PathBuf>)> {
    let entry = kinds.get(dataset.name)?;
    let when = NaiveDateTime::parse_from_str(entry.get("when")?.as_str()?, "%Y-%m-%dT%H:%M:%S").ok()?;
    let files: Vec<PathBuf> = entry.get("files")?.as_array()?.iter().map(|file| file.as_str().map(PathBuf::from)).collect::<Option<_>>()?;
    let there = |file: &PathBuf| std::fs::metadata(file).is_ok_and(|facts| facts.is_file() && facts.len() > 0);
    (fresh(when, now) && !files.is_empty() && files.iter().all(there)).then_some((when, files))
}

/// The archives of a kind that can be used again at `now` (local time); empty when there are none.
#[must_use]
pub fn find(folder: &Path, dataset: &HeritageDataset, now: NaiveDateTime) -> Vec<PathBuf> {
    usable(&manifest(folder), dataset, now).map(|(_, files)| files).unwrap_or_default()
}

/// When the newest kind that can be used again was received.
#[must_use]
pub fn latest(folder: &Path, now: NaiveDateTime) -> Option<NaiveDateTime> {
    let kinds = manifest(folder);
    DATASETS.iter().filter_map(|dataset| usable(&kinds, dataset, now)).map(|(when, _)| when).max()
}

/// What the dialog writes beside 「최근 받은 자료 다시 쓰기」 (`KaHeritageSetupDialogs.cpp:230-254`).
#[must_use]
pub fn reuse_note(received: NaiveDateTime) -> String {
    format!("{} 받음 · 없는 자료만 서버에서 받음", received.format("%Y-%m-%d %H:%M"))
}
