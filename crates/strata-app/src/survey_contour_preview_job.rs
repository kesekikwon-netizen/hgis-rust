//! The dialog's preview read runs off the UI thread.
//!
//! `render_survey_contour` only locks this map and, when a path is new, starts a
//! worker. The worker calls `read_sheet`. A frame never stats or parses the file
//! (`AGENTS.md`: nothing waits on the UI thread). Each path has its own entry so
//! tests can render different files at the same time.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{LazyLock, Mutex};

use super::{preview_of, read_sheet, Sheet};

/// 이 앱의 말. Shown until the worker stores the sheet.
pub(crate) const READING: &str = "좌표 파일을 읽고 있습니다.";

pub(crate) enum PreviewPoll {
    Pending,
    Ready(String),
}

struct Entry {
    started: bool,
    sheet: Option<Sheet>,
}

static CACHE: LazyLock<Mutex<HashMap<String, Entry>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

pub(crate) fn poll(path: &str, interval_m: f64, exclude_suspicious: bool) -> PreviewPoll {
    let ready = {
        let mut cache = CACHE.lock().unwrap_or_else(|poison| poison.into_inner());
        let entry = cache.entry(path.to_owned()).or_insert(Entry {
            started: false,
            sheet: None,
        });
        if let Some(sheet) = entry.sheet.as_ref() {
            return PreviewPoll::Ready(preview_of(sheet, interval_m, exclude_suspicious));
        }
        if entry.started {
            return PreviewPoll::Pending;
        }
        entry.started = true;
        path.to_owned()
    };
    std::thread::spawn(move || {
        let sheet = read_sheet(Path::new(&ready));
        let mut cache = CACHE.lock().unwrap_or_else(|poison| poison.into_inner());
        if let Some(entry) = cache.get_mut(&ready) {
            entry.sheet = Some(sheet);
        }
    });
    PreviewPoll::Pending
}

pub(crate) fn invalidate(path: &str) {
    let mut cache = CACHE.lock().unwrap_or_else(|poison| poison.into_inner());
    cache.remove(path);
}
