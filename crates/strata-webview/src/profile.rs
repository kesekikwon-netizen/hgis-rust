//! The data folder of one hidden session. The browser engine keeps its working files there (the session
//! itself is a private one: cookies are not kept after it ends). The folder is made by this crate, marked with
//! a lock file that the session holds open, and removed when the session ends. A session that died without
//! cleaning up is swept at the next start: only a folder that carries the lock file and whose lock nobody
//! holds any more is removed — never a folder of a session that is alive, never a folder this crate did not
//! make (review of 2026-11-14: the earlier engine swept folders of live sessions older than a day).

use std::fs::{self, File, OpenOptions};
use std::io;
use std::os::windows::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const FOLDER_PREFIX: &str = "s-";
const LOCK_FILE: &str = "strata-session.lock";

static SERIAL: AtomicU64 = AtomicU64::new(0);

/// A live session's folder. Dropping it releases the lock; the folder is removed with `remove_session_folder`.
#[derive(Debug)]
pub struct SessionFolder {
    path: PathBuf,
    lock: File,
}

/// Opens the lock file so that nobody else can open it (share mode 0) while the handle lives.
fn hold_lock(path: &Path, create: bool) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).share_mode(0);
    if create {
        options.create_new(true);
    }
    options.open(path)
}

impl SessionFolder {
    /// Makes `<parent>/s-<process id>-<serial>` with its lock file. `parent` is created when missing.
    pub fn create(parent: &Path) -> io::Result<Self> {
        fs::create_dir_all(parent)?;
        let name = format!("{FOLDER_PREFIX}{}-{}", std::process::id(), SERIAL.fetch_add(1, Ordering::Relaxed));
        let path = parent.join(name);
        fs::create_dir(&path)?;
        let lock = hold_lock(&path.join(LOCK_FILE), true)?;
        Ok(Self { path, lock })
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Ends the session's hold and returns the folder for `remove_session_folder`.
    #[must_use]
    pub fn release(self) -> PathBuf {
        drop(self.lock);
        self.path
    }
}

/// Whether `folder` is a session folder of this crate that no live session holds.
fn is_abandoned(folder: &Path) -> bool {
    let named = folder.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with(FOLDER_PREFIX));
    let lock = folder.join(LOCK_FILE);
    named && folder.is_dir() && lock.is_file() && hold_lock(&lock, false).is_ok()
}

/// Removes one released session folder. Returns whether it is gone. The browser's helper processes can hold
/// files for a moment after the views are closed: the caller tries again later instead of waiting here.
pub fn remove_session_folder(folder: &Path) -> bool {
    if !folder.exists() {
        return true;
    }
    if !is_abandoned(folder) {
        return false;
    }
    fs::remove_dir_all(folder).is_ok() || !folder.exists()
}

/// Removes the folders of sessions that ended without cleaning up. Returns how many were removed.
pub fn sweep_abandoned_sessions(parent: &Path) -> usize {
    let Ok(entries) = fs::read_dir(parent) else {
        return 0;
    };
    entries.flatten().map(|e| e.path()).filter(|p| is_abandoned(p) && remove_session_folder(p)).count()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn a_session_folder_is_made_with_its_lock_and_removed_after_release() {
        let parent = tempfile::tempdir().unwrap();
        let root = parent.path().join("webview");
        let session = SessionFolder::create(&root).unwrap();
        let path = session.path().to_path_buf();
        assert!(path.starts_with(&root) && path.join(LOCK_FILE).is_file());
        assert!(path.file_name().unwrap().to_str().unwrap().starts_with(&format!("s-{}-", std::process::id())));
        fs::write(path.join("Cookies"), b"x").unwrap();
        assert!(!remove_session_folder(&path), "held by the live session");
        assert!(path.join("Cookies").is_file());
        let released = session.release();
        assert!(remove_session_folder(&released));
        assert!(!path.exists());
        assert!(remove_session_folder(&released), "already gone counts as removed");
    }

    #[test]
    fn two_sessions_never_share_a_folder() {
        let parent = tempfile::tempdir().unwrap();
        let a = SessionFolder::create(parent.path()).unwrap();
        let b = SessionFolder::create(parent.path()).unwrap();
        assert_ne!(a.path(), b.path());
    }

    /// The sweep takes the folder of a dead session and nothing else: not a live session's, not a folder
    /// without the lock file, not a folder of another name, not a file.
    #[test]
    fn the_sweep_removes_only_abandoned_session_folders() {
        let parent = tempfile::tempdir().unwrap();
        let live = SessionFolder::create(parent.path()).unwrap();
        let dead = SessionFolder::create(parent.path()).unwrap().release();
        fs::write(dead.join("Cache"), b"x").unwrap();
        let foreign = parent.path().join("s-foreign");
        fs::create_dir(&foreign).unwrap();
        fs::write(foreign.join("keep.txt"), b"x").unwrap();
        let other = parent.path().join("other");
        fs::create_dir(&other).unwrap();
        fs::write(other.join(LOCK_FILE), b"").unwrap();
        fs::write(parent.path().join("s-file"), b"x").unwrap();

        assert_eq!(sweep_abandoned_sessions(parent.path()), 1);
        assert!(!dead.exists());
        assert!(live.path().join(LOCK_FILE).is_file());
        assert!(foreign.join("keep.txt").is_file());
        assert!(other.join(LOCK_FILE).is_file());
        assert!(parent.path().join("s-file").is_file());
        assert_eq!(sweep_abandoned_sessions(parent.path()), 0);
        assert_eq!(sweep_abandoned_sessions(&parent.path().join("missing")), 0);
    }
}
