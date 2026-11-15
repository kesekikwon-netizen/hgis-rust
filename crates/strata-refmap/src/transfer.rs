//! One request to a map service, as this crate asks its caller to send it, and the folder a fetched map is
//! built in (C++ `ReferenceMapPreparation`, `target/cpp-ref/src/core/PreparedReferenceMap.h:39-52`,
//! `PreparedReferenceMap.cpp:17-36`, `:80-93`).

use std::path::{Path, PathBuf};

#[cfg(test)]
#[path = "transfer_tests.rs"]
mod tests;

/// `PreparedReferenceMap.cpp:33`.
pub const CANCELLED: &str = "지도 내려받기를 취소했습니다. 기존 지도는 유지됩니다.";
/// `PreparedReferenceMap.cpp:86`.
pub const TOO_SLOW: &str = "지도 서버의 응답이 늦어 요청을 중단했습니다. 인터넷 연결을 확인한 뒤 다시 내려받으세요.";
/// `PreparedReferenceMap.cpp:87`.
pub const NOT_REACHED: &str = "지도 서버에 연결하지 못했습니다. 인터넷 연결을 확인한 뒤 다시 내려받으세요.";
/// `PreparedReferenceMap.cpp:23`.
pub const NO_FOLDER: &str = "지도를 저장할 임시 폴더를 만들지 못했습니다. 저장 폴더의 쓰기 권한과 남은 공간을 확인하세요.";
/// `PreparedReferenceMap.cpp:141`.
pub const NOT_SAVED: &str = "새 지도 파일을 저장하지 못했습니다. 저장 공간과 폴더 권한을 확인하세요. 기존 지도는 유지됩니다.";
/// `PreparedReferenceMap.cpp:147`.
pub const NOT_VERIFIED: &str = "저장한 지도를 다시 확인하지 못했습니다. 기존 지도는 유지됩니다. 다른 저장 폴더에서 다시 내려받으세요.";

/// A GET request: given up when no byte arrives for `idle_secs`, and in any case after `total_secs`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub url: String,
    pub user_agent: &'static str,
    pub idle_secs: u64,
    pub total_secs: u64,
}

impl Request {
    /// A request with the limits of a large transfer (`kLargeTransfer`, `PreparedReferenceMap.h:52`): 15 s
    /// without a byte, 180 s in all.
    #[must_use]
    pub fn large(url: String, user_agent: &'static str) -> Self {
        Self { url, user_agent, idle_secs: 15, total_secs: 180 }
    }

    /// A request for a small answer (`ReferenceTransferLimits`, `PreparedReferenceMap.h:41-44`): 15 s
    /// without a byte and 15 s in all.
    #[must_use]
    pub fn plain(url: String, user_agent: &'static str) -> Self {
        Self { url, user_agent, idle_secs: 15, total_secs: 15 }
    }
}

/// Why a request brought no answer (`ReferenceTransferFailure`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferFailure {
    Cancelled,
    Timeout,
    Network,
}

/// The function that sends a request and returns the body of a successful answer. The app hands in the
/// real one; tests hand in recorded answers.
pub type Download<'a> = &'a dyn Fn(&Request) -> Result<Vec<u8>, TransferFailure>;

/// A stored map: its own folder in the survey folder, the file in it, how many rows it holds, and what the
/// user should know about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prepared {
    pub folder: PathBuf,
    pub gpkg: PathBuf,
    pub rows: u64,
    pub warnings: Vec<String>,
}

/// What kind of thing stood in the way (the app shows one fixed code for each).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Why {
    /// The survey folder cannot take the map.
    Storage,
    /// The area asked for is wider than the service is asked for.
    TooWide,
    /// The service has nothing for this area.
    NoData,
    /// The service was not reached or did not answer with map data.
    Service,
    /// The answer cannot be read or placed.
    Data,
}

/// Why a map was not prepared. The text is what the user is told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stopped {
    Cancelled(String),
    Failed(Why, String),
}

impl Stopped {
    #[must_use]
    pub fn text(&self) -> &str {
        match self {
            Self::Cancelled(text) | Self::Failed(_, text) => text,
        }
    }
}

/// A new, empty folder `ka-hgis-reference-XXXXXX` beside the requested file, and the path of the file of
/// that name inside it (`initializeStorage`, `PreparedReferenceMap.cpp:17-28`). The requested path itself
/// is never written: a map that is there stays until the new one is complete.
///
/// # Errors
/// `NO_FOLDER` when the folder cannot be made.
pub fn new_storage(requested: &Path) -> Result<(PathBuf, PathBuf), Stopped> {
    let failed = || Stopped::Failed(Why::Storage, NO_FOLDER.to_owned());
    let (parent, name) = (requested.parent().ok_or_else(failed)?, requested.file_name().ok_or_else(failed)?);
    const LETTERS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    for _ in 0..64 {
        // Six letters from the random keys the standard library seeds its hash maps with.
        let mut seed = {
            use std::hash::{BuildHasher, Hasher};
            std::collections::hash_map::RandomState::new().build_hasher().finish()
        };
        let mut tail = String::new();
        for _ in 0..6 {
            tail.push(char::from(LETTERS[usize::try_from(seed % 62).unwrap_or(0)]));
            seed /= 62;
        }
        let folder = parent.join(format!("ka-hgis-reference-{tail}"));
        match std::fs::create_dir(&folder) {
            Ok(()) => return Ok((folder.clone(), folder.join(name))),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(_) => return Err(failed()),
        }
    }
    Err(failed())
}
