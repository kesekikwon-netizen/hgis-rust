//! Where a file the site offers goes, and what the run does with it.
//!
//! The C++ puts each received file in `주변유적/<시도> <시군>/원본/<uuid>/<site name>`
//! (`KaHeritageBrowser.cpp:1308-1315`, `:196-200`) — under the app's own data folder, never the
//! survey folder, because a received reference map is not the user's data
//! (`MainWindow.cpp:3168-3170`).

use std::path::{Path, PathBuf};

/// The folder a received file goes in: `<root>/주변유적/<시도> <시군>/원본/<uuid>`.
///
/// The uuid folder is the C++ one (`:1308-1315`, `QUuid::WithoutBraces`): two runs of the same
/// city do not overwrite each other's files.
#[must_use]
pub fn received_folder(root: &Path, sido: &str, city: &str, serial: u64) -> PathBuf {
    root.join("주변유적")
        .join(format!("{sido} {city}"))
        .join("원본")
        .join(format!("{serial:032x}"))
}

/// A name the site suggested, made safe to write (`KaHeritageBrowser.cpp:196-200 safeName`).
///
/// Windows forbids `\ / : * ? " < > |` and the reserved device names; the C++ replaces the
/// characters and falls back to a fixed name when nothing usable is left.
#[must_use]
pub fn safe_name(suggested: &str) -> String {
    const RESERVED: [&str; 22] = [
        "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7",
        "com8", "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
    ];
    let cleaned: String = suggested
        .chars()
        .map(|c| match c {
            '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    let trimmed = cleaned.trim().trim_matches('.').to_owned();
    if trimmed.is_empty() {
        return "받은자료.zip".to_owned();
    }
    let stem = trimmed
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if RESERVED.contains(&stem.as_str()) {
        return format!("_{trimmed}");
    }
    // A name longer than 120 characters makes `create_dir_all` fail on a deep path.
    if trimmed.chars().count() > 120 {
        return trimmed.chars().take(120).collect();
    }
    trimmed
}

/// Whether a received file is a ZIP the run should keep.
///
/// The site sometimes answers with a page instead of the file; that is not a ZIP and the run
/// asks again (`KaHeritageBrowser.cpp:1370-1382`).
#[must_use]
pub fn looks_like_zip(path: &Path) -> bool {
    let Ok(head) = std::fs::read(path).map(|b| b.into_iter().take(4).collect::<Vec<u8>>()) else {
        return false;
    };
    head == b"PK\x03\x04" || head == b"PK\x05\x06"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forbidden_characters_become_underscores() {
        assert_eq!(safe_name("a/b\\c:d*e?f\"g<h>i|j.zip"), "a_b_c_d_e_f_g_h_i_j.zip");
    }

    #[test]
    fn a_reserved_device_name_is_made_usable() {
        // Writing to `con.zip` fails on Windows.
        assert_eq!(safe_name("con.zip"), "_con.zip");
        assert_eq!(safe_name("LPT1.zip"), "_LPT1.zip");
    }

    #[test]
    fn an_empty_or_dot_only_name_falls_back() {
        assert_eq!(safe_name(""), "받은자료.zip");
        assert_eq!(safe_name("..."), "받은자료.zip");
        assert_eq!(safe_name("   "), "받은자료.zip");
    }

    #[test]
    fn a_long_name_is_shortened() {
        let long = "가".repeat(200);
        assert_eq!(safe_name(&long).chars().count(), 120);
    }

    #[test]
    fn a_zip_is_recognised_and_a_page_is_not() {
        let dir = tempfile::tempdir().expect("tempdir");
        let zip = dir.path().join("a.zip");
        std::fs::write(&zip, b"PK\x03\x04rest").expect("write");
        assert!(looks_like_zip(&zip));
        let page = dir.path().join("b.zip");
        std::fs::write(&page, b"<!doctype html>").expect("write");
        assert!(!looks_like_zip(&page));
        assert!(!looks_like_zip(&dir.path().join("missing.zip")));
    }

    #[test]
    fn the_received_folder_follows_the_cpp_layout() {
        let root = Path::new("C:/data");
        let got = received_folder(root, "경상북도", "경주시", 7);
        assert!(got.ends_with("주변유적/경상북도 경주시/원본/00000000000000000000000000000007"));
    }
}
