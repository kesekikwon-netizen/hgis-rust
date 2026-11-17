//! Background directory and dedicated pin IO, target/cpp-ref/src/app/KaFileBrowserPanel.cpp:289-357.
use super::{Entry, Location, Sort};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub(super) fn read(location: &Location, sort: Sort, descending: bool) -> std::io::Result<Vec<Entry>> {
    let paths = match location {
        Location::Recent => return Ok(Vec::new()),
        Location::Computer => {
            #[cfg(windows)]
            let paths = ('A'..='Z').map(|letter| PathBuf::from(format!("{letter}:\\"))).filter(|p| p.is_dir()).collect::<Vec<_>>();
            #[cfg(not(windows))]
            let paths = vec![PathBuf::from("/")];
            paths
        }
        Location::Folder(path) => std::fs::read_dir(path)?.map(|entry| entry.map(|e| e.path())).collect::<std::io::Result<Vec<_>>>()?,
    };
    let mut entries = Vec::with_capacity(paths.len());
    for path in paths {
        let name = path.file_name().map_or_else(|| path.display().to_string(), |n| n.to_string_lossy().into_owned());
        if name.eq_ignore_ascii_case("$Recycle.Bin") || name.eq_ignore_ascii_case("System Volume Information") {
            continue;
        }
        let metadata = match std::fs::symlink_metadata(&path) {
            Ok(m) if !m.is_symlink() => m,
            _ => continue,
        };
        let directory = metadata.is_dir();
        entries.push(Entry { path, name, directory, bytes: (!directory).then_some(metadata.len()), modified: metadata.modified().ok() });
    }
    entries.sort_by(|a, b| {
        let names = a.name.to_lowercase().cmp(&b.name.to_lowercase()).then_with(|| a.name.cmp(&b.name));
        let order = match sort {
            Sort::Name => names,
            Sort::Size => a.bytes.cmp(&b.bytes),
            Sort::Modified => a.modified.cmp(&b.modified),
            Sort::Kind => a.path.extension().cmp(&b.path.extension()),
        };
        b.directory.cmp(&a.directory).then(if descending { order.reverse() } else { order }).then(names)
    });
    Ok(entries)
}

#[derive(Serialize, Deserialize)]
struct Pins {
    version: u8,
    folders: Vec<PathBuf>,
}

pub(super) fn same_path(a: &Path, b: &Path) -> bool {
    let key = |p: &Path| {
        let s = p.to_string_lossy().replace('\\', "/");
        let s = s.trim_end_matches('/');
        if cfg!(windows) {
            s.to_lowercase()
        } else {
            s.to_owned()
        }
    };
    key(a) == key(b)
}
pub(super) fn load_pins(file: &Path) -> std::io::Result<Vec<PathBuf>> {
    let bytes = match std::fs::read(file) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let saved: Pins = serde_json::from_slice(&bytes).map_err(std::io::Error::other)?;
    if saved.version != 1 {
        return Err(std::io::Error::other("unknown explorer preference version"));
    }
    let mut pins: Vec<PathBuf> = Vec::new();
    for folder in saved.folders {
        if !pins.iter().any(|p| same_path(p, &folder)) {
            pins.push(folder);
        }
    }
    Ok(pins)
}
pub(super) fn save_pins(file: &Path, folders: &[PathBuf]) -> std::io::Result<()> {
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(&Pins { version: 1, folders: folders.to_vec() }).map_err(std::io::Error::other)?;
    let temporary = file.with_extension(format!("{}.tmp", std::process::id()));
    let result = std::fs::write(&temporary, bytes).and_then(|()| std::fs::rename(&temporary, file));
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}
