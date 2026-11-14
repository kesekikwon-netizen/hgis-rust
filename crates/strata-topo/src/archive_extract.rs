//! Unpacking a ZIP into a fresh folder under the rules of the C++ `TopographicArchive.cpp:20-168`: every
//! path is checked, nothing but plain files is written, sizes are counted as they are read, and a ZIP inside
//! a ZIP is unpacked beside its siblings.

use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;

use sha2::{Digest, Sha256};

use super::{random_name, Limits, Prepared};

/// Bytes read at a time (`ChunkSize`, :22).
pub(super) const CHUNK: usize = 256 * 1024;

/// One file of the unpacked archive as the completion record lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ManifestEntry {
    pub(super) path: String,
    pub(super) size: u64,
    pub(super) sha256: String,
}

/// The state of one `prepare` call (`Work`, :23-41).
pub(super) struct Work<'a> {
    pub(super) limits: &'a Limits,
    pub(super) cancel: &'a dyn Fn() -> bool,
    pub(super) result: Prepared,
    pub(super) total: u64,
    pub(super) entries: usize,
    files: HashSet<String>,
    directories: HashSet<String>,
    pub(super) manifest: Vec<ManifestEntry>,
}

impl<'a> Work<'a> {
    pub(super) fn new(limits: &'a Limits, cancel: &'a dyn Fn() -> bool) -> Self {
        Self { limits, cancel, result: Prepared::default(), total: 0, entries: 0, files: HashSet::new(), directories: HashSet::new(), manifest: Vec::new() }
    }

    pub(super) fn stopped(&mut self) -> bool {
        if (self.cancel)() {
            self.result.canceled = true;
        }
        self.result.canceled || !self.result.error.is_empty()
    }

    /// Records why the work stops. `invalid_archive` says the download itself is bad.
    pub(super) fn fail(&mut self, message: impl Into<String>, invalid_archive: bool) -> bool {
        self.result.error = message.into();
        self.result.invalid_archive = invalid_archive;
        false
    }
}

/// A relative path that can be created safely on this PC (`safeRelative`, :42-58).
pub(super) fn safe_relative(name: &str) -> bool {
    if name.is_empty() || name.encode_utf16().count() > 2048 || name.contains('\u{FFFD}') || name.contains('\\') || absolute(name) {
        return false;
    }
    let parts: Vec<&str> = name.split('/').collect();
    parts.len() <= 64
        && parts.iter().all(|part| {
            !part.is_empty()
                && *part != "."
                && *part != ".."
                && !part.ends_with('.')
                && !part.ends_with(' ')
                && !device_name(part)
                && !part.chars().any(|c| (c as u32) < 32 || ":*?\"<>|".contains(c))
        })
}

/// `QDir::isAbsolutePath` for the names that can occur here: a leading slash or a drive.
fn absolute(name: &str) -> bool {
    let bytes = name.as_bytes();
    name.starts_with('/') || (bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':')
}

/// `^(CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])(?:\.|$)`, any case (:47-48).
fn device_name(part: &str) -> bool {
    let stem = part.split('.').next().unwrap_or_default().to_ascii_uppercase();
    matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL") || (stem.len() == 4 && (stem.starts_with("COM") || stem.starts_with("LPT")) && matches!(stem.as_bytes()[3], b'1'..=b'9'))
}

/// No part of the path is a link or a junction (`noLinks`, :59-69).
pub(super) fn no_links(path: &Path) -> bool {
    let Ok(full) = std::path::absolute(path) else {
        return false;
    };
    full.ancestors().all(|p| fs::symlink_metadata(p).map_or(true, |m| !m.file_type().is_symlink()))
}

/// SHA-256 of a file as lower-case hex; `None` when the work stopped (`fileHash`, :70-83).
pub(super) fn file_hash(path: &Path, work: &mut Work<'_>) -> Option<String> {
    if work.stopped() {
        return None;
    }
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(e) => {
            work.fail(format!("자료 파일을 읽지 못했습니다: {e}"), false);
            return None;
        }
    };
    let mut hash = Sha256::new();
    let mut buffer = vec![0_u8; CHUNK];
    loop {
        if work.stopped() {
            return None;
        }
        match file.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => hash.update(&buffer[..n]),
            Err(_) => {
                work.fail("자료 파일을 끝까지 읽지 못했습니다.", false);
                return None;
            }
        }
    }
    Some(hex(&hash.finalize()))
}

pub(super) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Notes a path of the archive and refuses one that collides with another, ignoring case (`registerPath`, :84-99).
fn register_path(relative: &str, directory: bool, work: &mut Work<'_>) -> bool {
    let key = relative.to_lowercase();
    let mut parent = relative.rsplit_once('/').map(|(p, _)| p);
    while let Some(p) = parent {
        let parent_key = p.to_lowercase();
        if work.files.contains(&parent_key) {
            return work.fail("압축 자료의 파일 경로가 서로 충돌합니다.", false);
        }
        work.directories.insert(parent_key);
        parent = p.rsplit_once('/').map(|(up, _)| up);
    }
    if work.files.contains(&key) || (!directory && work.directories.contains(&key)) {
        return work.fail("압축 자료에 중복된 파일 경로가 있습니다.", false);
    }
    if directory {
        work.directories.insert(key);
    } else {
        work.files.insert(key);
    }
    true
}

/// What GDAL's ZIP reader, which the C++ reads archives with, never shows: entries that point above the
/// archive or to the root (`test_topographic_archive.cpp:69-73`). They are passed over the same way here.
fn left_out_by_the_reader(name: &str) -> bool {
    let unified = name.replace('\\', "/");
    unified.starts_with('/') || unified.split('/').any(|part| part == "..")
}

/// Unpacks `archive` into `payload` (`extract`, :100-168). `scratch` takes ZIPs found inside.
pub(super) fn extract(archive: &Path, payload: &Path, scratch: &Path, prefix: &str, depth: u32, work: &mut Work<'_>) -> bool {
    if work.stopped() {
        return false;
    }
    if depth > work.limits.max_archive_depth {
        return work.fail("압축 자료의 중첩 깊이가 허용 범위를 넘었습니다.", false);
    }
    let Some(mut zip) = File::open(archive).ok().and_then(|f| zip::ZipArchive::new(f).ok()) else {
        return work.fail("ZIP 자료를 열지 못했습니다. 다운로드 완료 여부를 확인하세요.", true);
    };
    let mut found = false;
    for index in 0..zip.len() {
        if work.stopped() {
            return false;
        }
        let Ok(mut entry) = zip.by_index(index) else {
            return work.fail("ZIP 안의 파일을 읽지 못했습니다.", true);
        };
        if left_out_by_the_reader(entry.name()) {
            continue;
        }
        found = true;
        work.entries += 1;
        if work.entries > work.limits.max_entries {
            return work.fail("압축 자료의 파일 수가 허용 범위를 넘었습니다.", false);
        }
        let is_directory = entry.is_dir();
        let name = entry.name().strip_suffix('/').filter(|_| is_directory).unwrap_or(entry.name()).to_owned();
        if !safe_relative(&name) {
            return work.fail("압축 자료에 안전하지 않은 파일 경로가 있습니다.", false);
        }
        let relative = format!("{prefix}{name}");
        if !safe_relative(&relative) {
            return work.fail("압축 자료의 경로가 너무 길거나 안전하지 않습니다.", false);
        }
        if !register_path(&relative, is_directory, work) {
            return false;
        }
        if is_directory {
            continue;
        }
        // Never restore archive permissions or links. All entry contents are copied into newly created
        // regular files; link targets are never followed (:122-125).
        if entry.unix_mode().is_some_and(|mode| !matches!(mode & 0o170_000, 0 | 0o100_000)) {
            return work.fail("압축 자료에 지원하지 않는 링크 또는 특수 파일이 있습니다.", false);
        }
        let expected = entry.size();
        if expected > work.limits.max_file_bytes || expected > work.limits.max_total_bytes - work.total {
            return work.fail("압축 해제 크기가 허용 범위를 넘었습니다.", false);
        }
        let nested = name.to_ascii_lowercase().ends_with(".zip");
        let target = if nested { scratch.join(format!("{}.zip", random_name())) } else { payload.join(&relative) };
        if !no_links(&target) || target.parent().is_none_or(|p| fs::create_dir_all(p).is_err()) {
            return work.fail("자료를 보관할 안전한 폴더를 만들지 못했습니다.", false);
        }
        let mut output = match File::create(&target) {
            Ok(file) => file,
            Err(e) => return work.fail(format!("압축 자료를 저장하지 못했습니다: {e}"), false),
        };
        let mut hash = Sha256::new();
        let mut buffer = vec![0_u8; CHUNK];
        let mut copied = 0_u64;
        loop {
            if work.stopped() {
                return false;
            }
            let count = match entry.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => n as u64,
                // The reader checks each entry's checksum and stream as it goes.
                Err(_) => return work.fail("압축 자료가 손상되었거나 다운로드가 끝나지 않았습니다.", true),
            };
            if count > expected - copied {
                return work.fail("ZIP 안의 파일 크기가 기록된 크기와 다릅니다.", true);
            }
            if count > work.limits.max_total_bytes - work.total {
                return work.fail("압축 자료의 실제 크기가 허용 범위를 넘었습니다.", false);
            }
            copied += count;
            work.total += count;
            if let Err(e) = output.write_all(&buffer[..count as usize]) {
                return work.fail(format!("압축 자료 저장 중 오류가 발생했습니다: {e}"), false);
            }
            hash.update(&buffer[..count as usize]);
        }
        if copied != expected {
            return work.fail("압축 자료가 손상되었거나 다운로드가 끝나지 않았습니다.", true);
        }
        if let Err(e) = output.sync_all() {
            return work.fail(format!("압축 자료 저장을 완료하지 못했습니다: {e}"), false);
        }
        drop(output);
        drop(entry);
        if nested {
            if !extract(&target, payload, scratch, &format!("{relative}.contents/"), depth + 1, work) {
                return false;
            }
        } else {
            work.manifest.push(ManifestEntry { path: relative, size: copied, sha256: hex(&hash.finalize()) });
        }
    }
    found || work.fail("ZIP 자료에 읽을 수 있는 파일이 없습니다.", true)
}
