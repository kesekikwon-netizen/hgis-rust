//! 홈의 조사 사실(구역·유구 수, 검수, 작업 좌표계) — C++ `SurveyFacts`(core/SurveyFacts.cpp)와 같은 사용자 ini 줄
//! `[RecentSurveys] facts\<키>`·`factsOrder` 를 같은 키(대소문자를 접은 깨끗한 경로의 SHA-1 앞 16자)와 같은 글
//! (크기 \t 수정 ms \t 구역 \t 유구 \t 오류 \t 경고 \t 검수 ms \t 센 ms \t 꾸러미 ms \t 좌표계)로 읽고 쓴다.
//! 파일 크기·수정 시각이 기록과 같을 때만 그 사실을 믿는다(SurveyFileFingerprint).

use std::io;
use std::path::Path;

use sha1::{Digest, Sha1};

use crate::qt_ini::{escaped_string, escaped_string_list, read_string, read_string_list, IniFile};
use crate::recent_surveys::normalized_path;

const SECTION: &str = "RecentSurveys";
const ORDER: &str = "factsOrder";
/// SurveyFacts.h:34-35.
pub const MAX_ENTRIES: usize = 24;
pub const MAX_PROBES: usize = 12;

/// `SurveyFacts::Facts` — 모르면 -1·0.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Facts {
    pub known: bool,
    pub areas: i32,
    pub features: i32,
    pub errors: i32,
    pub warnings: i32,
    pub checked_at_ms: i64,
    pub counted_at_ms: i64,
    pub packaged_at_ms: i64,
    pub work_crs: String,
}

impl Default for Facts {
    fn default() -> Self {
        Self { known: false, areas: -1, features: -1, errors: -1, warnings: -1, checked_at_ms: 0, counted_at_ms: 0, packaged_at_ms: 0, work_crs: String::new() }
    }
}

impl Facts {
    #[must_use]
    pub fn has_counts(&self) -> bool {
        self.known && self.areas >= 0 && self.features >= 0
    }
    #[must_use]
    pub fn has_check(&self) -> bool {
        self.known && self.checked_at_ms > 0
    }
}

/// keyFor — Windows 에서는 대소문자·구분자만 다른 경로를 같은 조사로 본다.
#[must_use]
pub fn key_for(path: &str) -> String {
    let clean = normalized_path(path);
    if clean.is_empty() {
        return String::new();
    }
    let folded = if cfg!(windows) { clean.to_lowercase() } else { clean };
    let digest = Sha1::digest(folded.as_bytes());
    digest.iter().take(8).map(|b| format!("{b:02x}")).collect()
}

/// SurveyFileFingerprint::capture — (크기, 수정 시각 ms). 없으면 None.
#[must_use]
pub fn fingerprint(path: &str) -> Option<(i64, i64)> {
    let meta = std::fs::metadata(path).ok().filter(std::fs::Metadata::is_file)?;
    let ms = meta.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_millis();
    Some((i64::try_from(meta.len()).ok()?, i64::try_from(ms).ok()?))
}

fn entry_key(key: &str) -> String {
    format!("facts\\{key}")
}

/// read — 9칸보다 적으면 없는 것, 10번째(좌표계)는 옛 기록에 없을 수 있다.
fn read_record(file: &IniFile, key: &str) -> Option<(i64, i64, Facts)> {
    if key.is_empty() {
        return None;
    }
    let text = read_string(file.value(SECTION, &entry_key(key))?);
    let f: Vec<&str> = text.split('\t').collect();
    if f.len() < 9 {
        return None;
    }
    let int = |i: usize| f[i].trim().parse::<i32>().unwrap_or(0);
    let long = |i: usize| f[i].trim().parse::<i64>().unwrap_or(0);
    let facts = Facts {
        known: false,
        areas: int(2),
        features: int(3),
        errors: int(4),
        warnings: int(5),
        checked_at_ms: long(6),
        counted_at_ms: long(7),
        packaged_at_ms: long(8),
        work_crs: f.get(9).map(|s| (*s).to_owned()).unwrap_or_default(),
    };
    Some((long(0), long(1), facts))
}

/// write — 지금 파일의 지문과 함께 쓰고, 순서 목록 맨 앞으로, 24개가 넘으면 오래된 기록을 지운다.
fn write_record(file: &mut IniFile, key: &str, path: &str, facts: &Facts) {
    if key.is_empty() {
        return;
    }
    let (size, mtime) = fingerprint(path).unwrap_or((-1, 0));
    let fields = [
        size.to_string(),
        mtime.to_string(),
        facts.areas.to_string(),
        facts.features.to_string(),
        facts.errors.to_string(),
        facts.warnings.to_string(),
        facts.checked_at_ms.to_string(),
        facts.counted_at_ms.to_string(),
        facts.packaged_at_ms.to_string(),
        facts.work_crs.clone(),
    ];
    file.set_value(SECTION, &entry_key(key), &escaped_string(&fields.join("\t")));
    let mut order: Vec<String> = file.value(SECTION, ORDER).map(read_string_list).unwrap_or_default();
    order.retain(|k| k != key);
    order.insert(0, key.to_owned());
    while order.len() > MAX_ENTRIES {
        if let Some(old) = order.pop() {
            file.remove(SECTION, &entry_key(&old));
        }
    }
    file.set_value(SECTION, ORDER, &escaped_string_list(&order));
}

pub(crate) fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(0))
}

/// lookup — 기록이 있고 파일이 그대로면 그 사실(known), 아니면 모름.
#[must_use]
pub fn lookup(ini: &Path, path: &str) -> Facts {
    let Ok(file) = IniFile::read(ini) else { return Facts::default() };
    lookup_in(&file, path)
}

/// 여러 조사의 사실을 ini 를 한 번 읽어(홈 목록).
#[must_use]
pub fn lookup_all(ini: &Path, paths: &[String]) -> Vec<Facts> {
    let file = IniFile::read(ini).ok();
    paths.iter().map(|p| file.as_ref().map_or_else(Facts::default, |f| lookup_in(f, p))).collect()
}

pub(crate) fn lookup_in(file: &IniFile, path: &str) -> Facts {
    match read_record(file, &key_for(path)) {
        Some((size, mtime, mut facts)) if fingerprint(path) == Some((size, mtime)) => {
            facts.known = true;
            facts
        }
        _ => Facts::default(),
    }
}

/// rememberCounts — 저장 뒤: 검수·꾸러미 칸은 두고 구역·유구 수와 센 시각, 좌표계(있으면).
///
/// # Errors
/// ini 를 읽거나 쓰지 못함.
pub fn remember_counts(ini: &Path, path: &str, areas: i32, features: i32, work_crs: &str) -> io::Result<()> {
    let mut file = IniFile::read(ini)?;
    let key = key_for(path);
    let mut facts = read_record(&file, &key).map(|r| r.2).unwrap_or_default();
    facts.areas = areas.max(0);
    facts.features = features.max(0);
    facts.counted_at_ms = now_ms();
    if !work_crs.is_empty() {
        facts.work_crs = work_crs.to_owned();
    }
    write_record(&mut file, &key, path, &facts);
    file.write(ini)
}

pub(crate) fn store_counts(file: &mut IniFile, path: &str, facts: &Facts) {
    write_record(file, &key_for(path), path, facts);
}

/// KaHomeTitleBlock::crsText.
#[must_use]
pub fn crs_text(auth_id: &str) -> String {
    if auth_id.ends_with("5186") {
        "5186 중부원점".to_owned()
    } else if auth_id.ends_with("5187") {
        "5187 동부원점".to_owned()
    } else if auth_id.is_empty() {
        "—".to_owned()
    } else {
        auth_id.rsplit(':').next().unwrap_or(auth_id).to_owned()
    }
}

/// 검수 칸 — 「검수 전」, 「통과」, 「오류 %1 · 주의 %2」.
#[must_use]
pub fn check_text(facts: &Facts) -> String {
    if !facts.has_check() {
        "검수 전".to_owned()
    } else if facts.errors + facts.warnings == 0 {
        "통과".to_owned()
    } else {
        format!("오류 {} · 주의 {}", facts.errors, facts.warnings)
    }
}

#[cfg(test)]
#[path = "survey_facts_tests.rs"]
mod tests;
