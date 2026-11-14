//! 홈에 올 때 아직 모르는 조사의 구역·유구 수를 센다 — C++ `SurveyFacts::probeCountsIfUnknown`·`probeBlockFor`·
//! `countLayers`(core/SurveyFacts.cpp:166-230). 로컬 고정 드라이브의 GPKG 만, 앱에 열린 파일은 건드리지 않고, 읽기만 한다.

use std::collections::HashMap;
use std::io;
use std::path::Path;

use crate::qt_ini::IniFile;
use crate::recent_surveys::normalized_path;
use crate::sqlite_read::open_survey_read_only;
use crate::survey_facts::{lookup_in, now_ms, store_counts};

/// 세지 않는 까닭(ProbeBlock).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeBlock {
    None,
    Disabled,
    EmptyPath,
    Remote,
    NotFixedDrive,
    OpenInApp,
    MissingOrEmpty,
}

/// probeEnabled — KA_HGIS_HOME_PROBE=0 이면 끈다.
#[must_use]
pub fn probe_enabled() -> bool {
    std::env::var("KA_HGIS_HOME_PROBE").map_or(true, |v| v != "0")
}

/// 드라이브 글자의 종류(GetDriveTypeW 를 .NET DriveInfo 로 — unsafe 없이). 이 PC 의 드라이브를 한 번에 읽는다.
#[must_use]
pub fn drive_kinds() -> HashMap<char, String> {
    let script = "[IO.DriveInfo]::GetDrives() | ForEach-Object { $_.Name.Substring(0,1) + '=' + $_.DriveType }";
    let mut cmd = std::process::Command::new("powershell.exe");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", script]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let Ok(out) = cmd.output() else { return HashMap::new() };
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.trim().split_once('=').and_then(|(d, k)| Some((d.chars().next()?.to_ascii_uppercase(), k.trim().to_owned()))))
        .collect()
}

/// probeBlockFor — `drive_kind` 는 드라이브 글자 → "Fixed"·"Removable"·"Network"…, `open` 은 앱에 열린 조사들.
pub fn probe_block_for(path: &str, open: &[String], drive_kind: &dyn Fn(char) -> Option<String>) -> ProbeBlock {
    if !probe_enabled() {
        return ProbeBlock::Disabled;
    }
    let clean = normalized_path(path);
    if clean.is_empty() {
        return ProbeBlock::EmptyPath;
    }
    if clean.starts_with("//") {
        return ProbeBlock::Remote;
    }
    if cfg!(windows) {
        let letter = clean.chars().next().filter(|_| clean.as_bytes().get(1) == Some(&b':'));
        match letter.and_then(|d| drive_kind(d.to_ascii_uppercase())).as_deref() {
            Some("Network") => return ProbeBlock::Remote,
            Some("Fixed") => {}
            _ => return ProbeBlock::NotFixedDrive,
        }
    }
    if open.iter().any(|o| crate::recent_surveys::same_path(o, &clean)) {
        return ProbeBlock::OpenInApp;
    }
    match std::fs::metadata(&clean) {
        Ok(m) if m.is_file() && m.len() > 0 => ProbeBlock::None,
        _ => ProbeBlock::MissingOrEmpty,
    }
}

/// countLayers — GPKG 의 survey_area, feature_poly + feature_line 도형 수(읽기만). GPKG 가 아니면 None.
#[must_use]
pub fn count_layers(path: &str) -> Option<(i32, i32)> {
    let conn = open_survey_read_only(Path::new(path)).ok()?;
    let has = |t: &str| conn.query_row("SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = ?1", [t], |r| r.get::<_, i64>(0)).unwrap_or(0) > 0;
    if !has("gpkg_contents") {
        return None;
    }
    let count = |t: &str| if has(t) { conn.query_row(&format!("SELECT count(*) FROM \"{t}\""), [], |r| r.get::<_, i64>(0)).unwrap_or(0) } else { 0 };
    let as_i32 = |n: i64| i32::try_from(n).unwrap_or(i32::MAX);
    Some((as_i32(count("survey_area")), as_i32(count("feature_poly") + count("feature_line"))))
}

/// probeCountsIfUnknown — 아직 개수를 모르는 조사를 `max` 개까지 세어 기록한다. 기록한 수.
///
/// # Errors
/// ini 를 읽거나 쓰지 못함.
pub fn probe_counts_if_unknown(ini: &Path, paths: &[String], open: &[String], max: usize, drive_kind: &dyn Fn(char) -> Option<String>) -> io::Result<usize> {
    if !probe_enabled() {
        return Ok(0);
    }
    let mut file = IniFile::read(ini)?;
    let (mut stored, mut tried) = (0, 0);
    for path in paths {
        if tried >= max {
            break;
        }
        let mut facts = lookup_in(&file, path); // 기록이 맞으면 검수·꾸러미 칸은 그대로 둔다.
        if facts.has_counts() || probe_block_for(path, open, drive_kind) != ProbeBlock::None {
            continue;
        }
        tried += 1;
        let Some((areas, features)) = count_layers(path) else { continue };
        (facts.areas, facts.features, facts.counted_at_ms) = (areas, features, now_ms());
        store_counts(&mut file, path, &facts);
        stored += 1;
    }
    if stored > 0 {
        file.write(ini)?;
    }
    Ok(stored)
}

#[cfg(test)]
#[path = "survey_facts_probe_tests.rs"]
mod tests;
