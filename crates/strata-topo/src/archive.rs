//! Received sheet files, made ready to read: a ZIP is unpacked into the survey's sheet library, a plain map
//! file is used where it lies (C++ `TopographicArchive`, `target/cpp-ref/src/core/TopographicArchive.cpp`).
//!
//! The source file is never changed. A ZIP is published only after every entry has been copied and checked,
//! under a folder named by the ZIP's SHA-256; no half-unpacked folder is ever returned. A copy that was
//! unpacked before is used again only if every file still has its recorded size and SHA-256.

use std::collections::{BTreeMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::hash::{BuildHasher, Hasher};
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

#[path = "archive_extract.rs"]
mod extract;
#[cfg(test)]
#[path = "archive_test_zip.rs"]
mod test_zip;
#[cfg(test)]
#[path = "archive_tests.rs"]
mod tests;

use extract::{extract as unpack, file_hash, no_links, safe_relative, ManifestEntry, Work};

/// How much an archive may hold (`Limits`, TopographicArchive.h:9-14).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Limits {
    pub max_total_bytes: u64,
    pub max_file_bytes: u64,
    pub max_entries: usize,
    pub max_archive_depth: u32,
}

impl Default for Limits {
    fn default() -> Self {
        Self { max_total_bytes: 4 * 1024 * 1024 * 1024, max_file_bytes: 1024 * 1024 * 1024, max_entries: 10000, max_archive_depth: 4 }
    }
}

/// What `prepare` came to (`Result`, TopographicArchive.h:15-22). `error` is empty when it went well.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Prepared {
    /// Folder that holds the readable files.
    pub directory: Option<PathBuf>,
    /// The readable files, sorted.
    pub files: Vec<PathBuf>,
    pub error: String,
    /// The source ZIP itself is incomplete or damaged (fetching it again may help); storage and rule
    /// failures are not.
    pub invalid_archive: bool,
    pub canceled: bool,
    /// An earlier unpacked copy was checked and used.
    pub reused: bool,
}

/// A name nobody else has: 32 random hex digits in the usual 8-4-4-4-12 groups (the C++ uses `QUuid`).
pub(crate) fn random_name() -> String {
    let word = || std::collections::hash_map::RandomState::new().build_hasher().finish();
    let hex = format!("{:016x}{:016x}", word(), word());
    format!("{}-{}-{}-{}-{}", &hex[..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..])
}

/// Holds `<destination>.lock` so that two programs never unpack the same archive at once (`QLockFile`, :228-229).
fn lock(path: &Path) -> Option<File> {
    let mut options = OpenOptions::new();
    options.create(true).truncate(false).write(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(0);
    }
    options.open(path).ok()
}

/// Whether `directory` holds a complete earlier copy of the archive with this hash (`cached`, :169-212).
fn cached(directory: &Path, source_hash: &str, work: &mut Work<'_>) -> bool {
    let manifest_path = directory.join("manifest.json");
    if !no_links(&manifest_path) || fs::metadata(&manifest_path).map_or(true, |m| m.len() > 8 * 1024 * 1024) {
        return false;
    }
    let Some(object) = fs::read(&manifest_path).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok()) else {
        return false;
    };
    if object["version"].as_i64() != Some(1) || object["sourceSha256"].as_str() != Some(source_hash) {
        return false;
    }
    let files = object["files"].as_array().cloned().unwrap_or_default();
    if files.is_empty() || files.len() > work.limits.max_entries {
        return false;
    }
    let payload = directory.join("payload");
    let (mut expected, mut paths, mut bytes) = (HashSet::new(), Vec::new(), 0_u64);
    for item in &files {
        if work.stopped() {
            return false;
        }
        let relative = item["path"].as_str().unwrap_or_default();
        let Some(size) = item["size"].as_str().and_then(|s| s.parse::<u64>().ok()) else {
            return false;
        };
        if !safe_relative(relative) || size > work.limits.max_file_bytes || size > work.limits.max_total_bytes - bytes || expected.contains(&relative.to_lowercase()) {
            return false;
        }
        bytes += size;
        let path = payload.join(relative);
        if !no_links(&path) || fs::metadata(&path).map_or(true, |m| !m.is_file() || m.len() != size) {
            return false;
        }
        let digest = file_hash(&path, work);
        if work.stopped() || digest.as_deref() != item["sha256"].as_str() {
            return false;
        }
        expected.insert(relative.to_lowercase());
        paths.push(path);
    }
    // Nothing but the recorded files may lie in the folder, and no link (:197-207).
    let mut pending = vec![payload.clone()];
    let mut seen = 0_usize;
    while let Some(folder) = pending.pop() {
        let Ok(entries) = fs::read_dir(&folder) else {
            return false;
        };
        for entry in entries.flatten() {
            seen += 1;
            if work.stopped() || seen > work.limits.max_entries + 64 * files.len() {
                return false;
            }
            let path = entry.path();
            let Ok(kind) = entry.file_type() else {
                return false;
            };
            let relative = path.strip_prefix(&payload).ok().map(|p| p.to_string_lossy().replace('\\', "/").to_lowercase());
            if kind.is_symlink() || (kind.is_file() && !relative.is_some_and(|r| expected.contains(&r))) {
                return false;
            }
            if kind.is_dir() {
                pending.push(path);
            }
        }
    }
    paths.sort();
    work.result.directory = Some(payload);
    work.result.files = paths;
    work.result.reused = true;
    true
}

/// The completion record of an unpacked archive, in the form the C++ writes (`QJsonDocument::Compact`, keys
/// in alphabetical order; sizes as text).
fn manifest_json(digest: &str, entries: &[ManifestEntry]) -> String {
    let files: Vec<Value> = entries
        .iter()
        .map(|e| Value::Object(BTreeMap::from([("path".to_owned(), json!(e.path)), ("sha256".to_owned(), json!(e.sha256)), ("size".to_owned(), json!(e.size.to_string()))]).into_iter().collect()))
        .collect();
    let top = BTreeMap::from([("files".to_owned(), Value::Array(files)), ("sourceSha256".to_owned(), json!(digest)), ("version".to_owned(), json!(1))]);
    Value::Object(top.into_iter().collect()).to_string()
}

/// Makes a received file ready to read (`TopographicArchive::prepare`, :215-288).
pub fn prepare(source: &Path, library_root: &Path, limits: &Limits, is_canceled: &dyn Fn() -> bool) -> Prepared {
    let mut work = Work::new(limits, is_canceled);
    if work.stopped() {
        return work.result;
    }
    let source = std::path::absolute(source).unwrap_or_else(|_| source.to_path_buf());
    if !source.is_file() || !no_links(&source) {
        work.fail("완료된 원본 자료 파일을 찾지 못했거나 링크 파일입니다.", false);
        return work.result;
    }
    let extension = source.extension().and_then(|e| e.to_str()).unwrap_or_default().to_ascii_lowercase();
    if extension != "zip" {
        if ["dxf", "shp", "gpkg", "ngi"].contains(&extension.as_str()) {
            work.result.directory = source.parent().map(Path::to_path_buf);
            work.result.files = vec![source];
        } else {
            work.fail("수치지형도 ZIP·SHP·DXF·GPKG·NGI 자료를 선택하세요.", false);
        }
        return work.result;
    }
    let limits_ok = limits.max_total_bytes > 0 && limits.max_file_bytes > 0 && limits.max_entries > 0 && limits.max_archive_depth > 0;
    if !limits_ok || library_root.as_os_str().is_empty() || !no_links(library_root) || fs::create_dir_all(library_root).is_err() {
        work.fail("자료 보관 경로 또는 압축 해제 제한이 올바르지 않습니다.", false);
        return work.result;
    }
    let Some(digest) = file_hash(&source, &mut work) else {
        return work.result;
    };
    let root = std::path::absolute(library_root).unwrap_or_else(|_| library_root.to_path_buf());
    let destination = root.join(&digest);
    let lock_path = root.join(format!("{digest}.lock"));
    let Some(held) = lock(&lock_path) else {
        work.fail("같은 자료를 준비 중입니다. 잠시 후 다시 시도하세요.", false);
        return work.result;
    };
    let result = publish(&source, &root, &destination, &digest, &mut work);
    drop(held);
    let _ = fs::remove_file(&lock_path);
    result
}

/// Uses the kept copy, or unpacks into a staging folder and moves it into place (:230-287).
fn publish(source: &Path, root: &Path, destination: &Path, digest: &str, work: &mut Work<'_>) -> Prepared {
    if cached(destination, digest, work) || work.stopped() {
        return work.result.clone();
    }
    if !no_links(destination) {
        work.fail("자료 보관 폴더에 링크가 있어 사용할 수 없습니다.", false);
        return work.result.clone();
    }
    let staging = root.join(format!(".preparing-{}", &random_name()[..8]));
    let (payload, scratch) = (staging.join("payload"), staging.join("archives"));
    if fs::create_dir_all(&payload).is_err() || fs::create_dir_all(&scratch).is_err() {
        work.fail("자료 준비 폴더를 만들지 못했습니다.", false);
        return work.result.clone();
    }
    let done = finish(source, &staging, destination, digest, work);
    if !done {
        // The staging folder is this call's own; what was unpacked so far is not kept.
        let _ = fs::remove_dir_all(&staging);
        work.result.directory = None;
        work.result.files.clear();
    }
    work.result.clone()
}

fn finish(source: &Path, staging: &Path, destination: &Path, digest: &str, work: &mut Work<'_>) -> bool {
    let (payload, scratch) = (staging.join("payload"), staging.join("archives"));
    if !unpack(source, &payload, &scratch, "", 1, work) {
        return false;
    }
    if file_hash(source, work).as_deref() != Some(digest) && !work.stopped() {
        work.fail("준비 중 원본 자료가 변경되었습니다. 다시 시도하세요.", true);
    }
    if work.stopped() {
        return false;
    }
    if work.manifest.is_empty() {
        return work.fail("ZIP 자료에 적재할 파일이 없습니다.", true);
    }
    // ZIPs found inside lie outside the payload and are never seen by the catalog (:252-256).
    if !no_links(&scratch) || fs::remove_dir_all(&scratch).is_err() {
        return work.fail("중간 압축 자료를 정리하지 못했습니다.", false);
    }
    if fs::write(staging.join("manifest.json"), manifest_json(digest, &work.manifest)).is_err() {
        return work.fail("자료 완료 정보를 저장하지 못했습니다.", false);
    }
    if work.stopped() {
        return false;
    }
    let backup = destination.with_file_name(format!("{digest}.previous-{}", random_name()));
    let had_previous = destination.exists();
    if had_previous && fs::rename(destination, &backup).is_err() {
        return work.fail("이전 자료가 사용 중이어서 갱신하지 못했습니다.", false);
    }
    if fs::rename(staging, destination).is_err() {
        let restored = !had_previous || fs::rename(&backup, destination).is_ok();
        let message = if restored { "자료 준비를 완료하지 못했습니다. 이전 자료는 유지됩니다.".to_owned() } else { format!("자료 준비를 완료하지 못했습니다. 이전 자료 보관 경로: {}", backup.display()) };
        return work.fail(message, false);
    }
    // Earlier content that failed its check is kept beside for recovery: a file that an open reader may
    // still use is never deleted (:278-279).
    let directory = destination.join("payload");
    let mut files: Vec<PathBuf> = work.manifest.iter().map(|e| directory.join(&e.path)).collect();
    files.sort();
    work.result.directory = Some(directory);
    work.result.files = files;
    true
}
