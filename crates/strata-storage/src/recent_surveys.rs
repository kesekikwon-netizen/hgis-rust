//! 최근 조사 목록 — C++ `RecentSurveys`(`RecentSurveys.cpp`, `RecentSurveys.h`)와 같은 ini 줄
//! (`[RecentSurveys] items`)을 같은 규칙으로 읽고 쓴다. 마지막 조사 폴더(`[Survey] LastDir`,
//! `MainWindowSession.cpp:1357-1364` rememberSurveyDir)도 여기서 쓴다.
//!
//! 저장된 목록은 그대로 둔다: USB·네트워크 드라이브가 빠져 지금 없는 조사도 지우지 않는다.
//! C++ 의 USB 드라이브 글자 바뀜 찾기(`onAnotherDrive`)와 네트워크 경로 30초 확인 캐시는 아직 없다.

use std::io;
use std::path::Path;

use crate::qt_ini::{escaped_string, escaped_string_list, read_string, read_string_list, IniFile};

/// `RecentSurveys.h:21` kMaxItems.
pub const MAX_ITEMS: usize = 12;

const SECTION: &str = "RecentSurveys";
const ITEMS_KEY: &str = "items";

/// 목록 한 줄. `available` 은 표시용이고 저장하지 않는다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentSurvey {
    pub name: String,
    pub path: String,
    pub last_opened_ms: i64,
    pub available: bool,
}

/// `QFileInfo::absoluteFilePath` — 절대 경로, 앞으로 빗금, `.`·`..` 정리, 드라이브 글자 대문자.
#[must_use]
pub fn absolute_file_path(path: &Path) -> String {
    let Ok(abs) = std::path::absolute(path) else {
        return String::new();
    };
    let mut s = abs.to_string_lossy().replace('\\', "/");
    if s.len() >= 2 && s.as_bytes()[1] == b':' && s.as_bytes()[0].is_ascii_lowercase() {
        s.replace_range(0..1, &s[0..1].to_ascii_uppercase());
    }
    s
}

/// `normalizedPath` (`RecentSurveys.cpp:34-38`).
#[must_use]
pub fn normalized_path(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    absolute_file_path(Path::new(trimmed))
}

/// `samePath` — Windows 에서는 대소문자·구분자만 다른 경로를 같은 조사로 본다.
#[must_use]
pub fn same_path(a: &str, b: &str) -> bool {
    let x = normalized_path(a);
    !x.is_empty() && x.to_lowercase() == normalized_path(b).to_lowercase()
}

fn has_drive_letter(clean: &str) -> bool {
    let b = clean.as_bytes();
    b.len() >= 3 && b[1] == b':' && b[2] == b'/'
}

/// `sameRestOnOtherDrive` — 드라이브 글자만 다른 같은 경로(USB 가 다른 글자로 돌아옴).
fn same_rest_on_other_drive(a: &str, b: &str) -> bool {
    let (x, y) = (normalized_path(a), normalized_path(b));
    if !has_drive_letter(&x) || !has_drive_letter(&y) {
        return false;
    }
    if x.as_bytes()[0].eq_ignore_ascii_case(&y.as_bytes()[0]) {
        return false;
    }
    x[2..].to_lowercase() == y[2..].to_lowercase()
}

/// `pathAvailable` — 지금 열 수 있는가(파일이 있는가).
fn path_available(path: &str) -> bool {
    let clean = normalized_path(path);
    !clean.is_empty() && Path::new(&clean).exists()
}

/// `storableName` — 탭·줄바꿈은 칸을 밀므로 빈칸으로, 그리고 `QString::simplified`.
fn storable_name(name: &str) -> String {
    name.replace(['\t', '\r', '\n'], " ").split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `QFileInfo::completeBaseName` — 파일 이름에서 마지막 확장자만 뺀다.
#[must_use]
pub fn complete_base_name(path: &str) -> String {
    let name = path.rsplit(['/', '\\']).next().unwrap_or_default();
    name.rfind('.').map_or(name, |k| &name[..k]).to_owned()
}

fn read_ini(ini: &Path) -> IniFile {
    IniFile::read(ini).unwrap_or_default()
}

/// `loadStored` (`RecentSurveys.cpp:143-170`) — 저장된 그대로(겹친 줄만 빼고), 파일은 보지 않는다.
/// 읽지 못한 파일은 빈 목록(표시용). 쓰기 전에는 `stored_in` 으로 같은 읽기 결과를 쓴다.
#[must_use]
pub fn load_stored(ini: &Path) -> Vec<RecentSurvey> {
    stored_in(&read_ini(ini))
}

fn stored_in(file: &IniFile) -> Vec<RecentSurvey> {
    let raw = file.value(SECTION, ITEMS_KEY).map(read_string_list).unwrap_or_default();
    let mut out: Vec<RecentSurvey> = Vec::new();
    for line in raw {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 2 {
            continue;
        }
        let path = parts[0].trim().to_owned();
        if path.is_empty() {
            continue;
        }
        let ms = if parts.len() >= 3 { parts[parts.len() - 1].trim().parse::<i64>().ok() } else { None };
        let (name, last_opened_ms) = match ms {
            Some(ms) => (storable_name(&parts[1..parts.len() - 1].join(" ")), ms),
            None => (storable_name(&parts[1..].join(" ")), 0),
        };
        if out.iter().any(|seen| same_path(&seen.path, &path)) {
            continue;
        }
        out.push(RecentSurvey { name, path, last_opened_ms, available: true });
        if out.len() >= MAX_ITEMS {
            break;
        }
    }
    out
}

/// `store` (`RecentSurveys.cpp:172-180`) — 목록을 읽은 바로 그 글에서 한 줄만 바꿔 쓴다.
fn store(ini: &Path, mut file: IniFile, items: &[RecentSurvey]) -> io::Result<()> {
    let raw: Vec<String> = items
        .iter()
        .map(|it| format!("{}\t{}\t{}", it.path, storable_name(&it.name), it.last_opened_ms))
        .collect();
    file.set_value(SECTION, ITEMS_KEY, &escaped_string_list(&raw));
    file.write(ini)
}

/// `loadAll` — 홈 화면 목록: 모든 항목(새것부터), 없는 조사는 `available = false`.
#[must_use]
pub fn load_all(ini: &Path) -> Vec<RecentSurvey> {
    let mut out: Vec<RecentSurvey> = Vec::new();
    for mut it in load_stored(ini) {
        it.available = path_available(&it.path);
        if it.name.is_empty() {
            it.name = complete_base_name(&it.path);
        }
        if !out.iter().any(|seen| same_path(&seen.path, &it.path)) {
            out.push(it);
        }
    }
    out
}

/// `load` — 지금 열 수 있는 항목만.
#[must_use]
pub fn load(ini: &Path) -> Vec<RecentSurvey> {
    load_all(ini).into_iter().filter(|it| it.available).collect()
}

/// `lastPath` — 열 수 있는 가장 최근 조사, 없으면 빈 글.
#[must_use]
pub fn last_path(ini: &Path) -> String {
    load(ini).into_iter().next().map(|it| it.path).unwrap_or_default()
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

/// `remember` (`RecentSurveys.cpp:256-275`) — 맨 앞에 넣고, 같은 조사는 빼고, 열두 개까지.
/// 파일이 없으면 아무 것도 하지 않는다. 이름이 비면 파일 이름(확장자 뺀 것).
/// ini 를 읽지 못하면(잠김·UTF-8 아님) 아무 것도 쓰지 않는다 — 사용자 목록을 지우지 않게.
///
/// # Errors
/// ini 를 읽거나 쓰지 못함.
pub fn remember(ini: &Path, path: &Path, name: &str) -> io::Result<()> {
    let abs = absolute_file_path(path);
    if abs.is_empty() || !Path::new(&abs).exists() {
        return Ok(());
    }
    let clean = storable_name(name);
    let shown = if clean.is_empty() { complete_base_name(&abs) } else { clean };
    let file = IniFile::read(ini)?;
    let mut next = vec![RecentSurvey { name: shown, path: abs.clone(), last_opened_ms: now_ms(), available: true }];
    for it in stored_in(&file) {
        if same_path(&it.path, &abs) {
            continue;
        }
        if same_rest_on_other_drive(&it.path, &abs) && !path_available(&it.path) {
            continue;
        }
        next.push(it);
        if next.len() >= MAX_ITEMS {
            break;
        }
    }
    store(ini, file, &next)
}

/// `forget` (`RecentSurveys.cpp:276-285`) — 그 조사(와 드라이브 글자만 다른 없는 옛 항목)를 뺀다.
///
/// # Errors
/// ini 를 읽거나 쓰지 못함.
pub fn forget(ini: &Path, path: &str) -> io::Result<()> {
    let file = IniFile::read(ini)?;
    let mut next = Vec::new();
    for it in stored_in(&file) {
        if same_path(&it.path, path) {
            continue;
        }
        // 홈 목록은 옮긴 USB 조사를 새 글자로 보인다 — 저장된 옛 글자 항목도 함께 뺀다.
        if same_rest_on_other_drive(&it.path, path) && !path_available(&it.path) {
            continue;
        }
        next.push(it);
    }
    store(ini, file, &next)
}

/// `Survey/LastDir` — 마지막으로 조사를 저장·연 폴더(없으면 빈 글).
#[must_use]
pub fn last_dir(ini: &Path) -> String {
    read_ini(ini).value("Survey", "LastDir").map(read_string).unwrap_or_default()
}

/// `topographic/folder` — start folder of the topographic folder picker
/// (MainWindowTopographic.cpp:105); `None` when the key is absent.
#[must_use]
pub fn topographic_folder(ini: &Path) -> Option<String> {
    read_ini(ini).value("topographic", "folder").map(read_string)
}

/// `rememberSurveyDir` 의 쓰기 — 그 한 줄만 바꾼다.
///
/// # Errors
/// ini 를 읽거나 쓰지 못함.
pub fn set_last_dir(ini: &Path, dir: &str) -> io::Result<()> {
    let mut file = IniFile::read(ini)?;
    file.set_value("Survey", "LastDir", &escaped_string(dir));
    file.write(ini)
}

#[cfg(test)]
#[path = "recent_surveys_tests.rs"]
mod tests;
