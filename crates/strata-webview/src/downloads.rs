//! Where a download is written. The engine writes into a part file that it makes itself, inside the folder the
//! host chose, and gives the file its real name only when the transfer is complete. It never overwrites and
//! never deletes a file it did not create (the earlier engine could delete an existing file of the same name
//! when a transfer failed: review of 2026-11-14, finding b7).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::names::safe_file_name;

/// Why a download was not started.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadRefusal {
    /// The host answered no.
    Declined,
    /// The name is empty or is not what `safe_file_name` makes of it.
    UnsafeName,
    /// The folder is missing.
    NoFolder,
    /// A file of that name (or its part file) is already there.
    Exists,
}

/// The two paths of one transfer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedFile {
    pub final_path: PathBuf,
    pub part_path: PathBuf,
}

/// Plans a transfer of `file_name` into `directory`. `serial` makes the part name unique within a session.
pub fn plan_download(directory: &Path, file_name: &str, serial: u64) -> Result<PlannedFile, DownloadRefusal> {
    if file_name.is_empty() || safe_file_name(file_name) != file_name {
        return Err(DownloadRefusal::UnsafeName);
    }
    if !directory.is_dir() {
        return Err(DownloadRefusal::NoFolder);
    }
    let final_path = directory.join(file_name);
    let part_path = directory.join(format!("{file_name}.{serial}.part"));
    if final_path.exists() || part_path.exists() {
        return Err(DownloadRefusal::Exists);
    }
    Ok(PlannedFile { final_path, part_path })
}

/// Bytes received so far: the part file, or the browser's own temporary beside it while it still writes there.
#[must_use]
pub fn received_bytes(plan: &PlannedFile) -> u64 {
    let own = fs::metadata(&plan.part_path).map_or(0, |m| m.len());
    let browser_temp = browser_temp_path(plan).and_then(|p| fs::metadata(p).ok()).map_or(0, |m| m.len());
    own.max(browser_temp)
}

fn browser_temp_path(plan: &PlannedFile) -> Option<PathBuf> {
    let mut name = plan.part_path.file_name()?.to_os_string();
    name.push(".crdownload");
    Some(plan.part_path.with_file_name(name))
}

/// The transfer is complete: the part file gets the real name. Fails when the part file is missing or empty,
/// or when a file of the real name appeared in the meantime (it is left alone).
pub fn finish_download(plan: &PlannedFile) -> io::Result<u64> {
    let bytes = fs::metadata(&plan.part_path)?.len();
    if bytes == 0 {
        return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "the received file is empty"));
    }
    if plan.final_path.exists() {
        return Err(io::Error::new(io::ErrorKind::AlreadyExists, "a file of that name is already there"));
    }
    fs::rename(&plan.part_path, &plan.final_path)?;
    Ok(bytes)
}

/// The transfer failed or was stopped: only the part file and the browser's temporary of it are removed.
pub fn discard_download(plan: &PlannedFile) {
    let _ = fs::remove_file(&plan.part_path);
    if let Some(temp) = browser_temp_path(plan) {
        let _ = fs::remove_file(temp);
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn a_plan_names_a_part_file_beside_the_real_name() {
        let dir = tempfile::tempdir().unwrap();
        let plan = plan_download(dir.path(), "378044.zip", 7).unwrap();
        assert_eq!(plan.final_path, dir.path().join("378044.zip"));
        assert_eq!(plan.part_path, dir.path().join("378044.zip.7.part"));
        assert!(!plan.final_path.exists() && !plan.part_path.exists(), "planning creates nothing");
    }

    #[test]
    fn unsafe_names_missing_folders_and_existing_files_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["", "..\\x.zip", "a/b.zip", "CON", "x.zip ", "a:b.zip"] {
            assert_eq!(plan_download(dir.path(), name, 1), Err(DownloadRefusal::UnsafeName), "{name:?}");
        }
        assert_eq!(plan_download(&dir.path().join("missing"), "a.zip", 1), Err(DownloadRefusal::NoFolder));
        fs::write(dir.path().join("a.zip"), b"mine").unwrap();
        assert_eq!(plan_download(dir.path(), "a.zip", 1), Err(DownloadRefusal::Exists));
        fs::write(dir.path().join("b.zip.2.part"), b"other").unwrap();
        assert_eq!(plan_download(dir.path(), "b.zip", 2), Err(DownloadRefusal::Exists));
        assert_eq!(fs::read(dir.path().join("a.zip")).unwrap(), b"mine");
    }

    #[test]
    fn finishing_renames_the_part_file_and_reports_its_size() {
        let dir = tempfile::tempdir().unwrap();
        let plan = plan_download(dir.path(), "a.zip", 1).unwrap();
        fs::write(&plan.part_path, b"PK12345").unwrap();
        assert_eq!(received_bytes(&plan), 7);
        assert_eq!(finish_download(&plan).unwrap(), 7);
        assert!(!plan.part_path.exists());
        assert_eq!(fs::read(&plan.final_path).unwrap(), b"PK12345");
    }

    /// A file that someone else put there under the real name while the transfer ran is never replaced.
    #[test]
    fn finishing_never_overwrites_and_never_accepts_an_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        let plan = plan_download(dir.path(), "a.zip", 1).unwrap();
        assert!(finish_download(&plan).is_err(), "no part file");
        fs::write(&plan.part_path, b"").unwrap();
        assert_eq!(finish_download(&plan).unwrap_err().kind(), io::ErrorKind::UnexpectedEof);
        fs::write(&plan.part_path, b"new").unwrap();
        fs::write(&plan.final_path, b"theirs").unwrap();
        assert_eq!(finish_download(&plan).unwrap_err().kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read(&plan.final_path).unwrap(), b"theirs");
        assert!(plan.part_path.exists(), "the part file stays for discard_download");
    }

    #[test]
    fn discarding_removes_only_what_the_engine_made() {
        let dir = tempfile::tempdir().unwrap();
        let plan = plan_download(dir.path(), "a.zip", 3).unwrap();
        fs::write(&plan.part_path, b"half").unwrap();
        fs::write(dir.path().join("a.zip.3.part.crdownload"), b"halfhalf").unwrap();
        assert_eq!(received_bytes(&plan), 8, "the browser's temporary is the larger one");
        fs::write(&plan.final_path, b"theirs").unwrap();
        fs::write(dir.path().join("other.txt"), b"x").unwrap();
        discard_download(&plan);
        let mut left: Vec<String> = fs::read_dir(dir.path()).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
        left.sort();
        assert_eq!(left, vec!["a.zip", "other.txt"]);
    }
}
