//! File names offered by a site, made safe for this PC — `safeFileName` of the C++ browser
//! (`target/cpp-ref/src/app/KaTopographicBrowser.cpp:103-114`), without its default name: an empty result
//! means the site offered nothing usable and the caller decides.

/// Characters Windows does not take in a file name, and control characters (`:106`).
fn forbidden(c: char) -> bool {
    matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') || (c as u32) < 0x20
}

/// Device names of Windows (`:110-112`), matched on the part before the first dot, any case.
fn reserved(stem: &str) -> bool {
    let s = stem.to_ascii_uppercase();
    matches!(s.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (s.len() == 4 && (s.starts_with("COM") || s.starts_with("LPT")) && matches!(s.as_bytes()[3], b'1'..=b'9'))
}

/// The last path component with forbidden characters replaced by `_`, at most 180 UTF-16 units, without
/// trailing dots and spaces, and never a device name.
#[must_use]
pub fn safe_file_name(offered: &str) -> String {
    // :104-105 — both separators count; only the file name is kept.
    let unified = offered.replace('\\', "/");
    let last = unified.rsplit('/').next().unwrap_or("");
    // :106
    let replaced: String = last.chars().map(|c| if forbidden(c) { '_' } else { c }).collect();
    // :107 — QString::left(180) counts UTF-16 units; a character that would straddle the limit is left out.
    let mut cut = String::new();
    let mut units = 0;
    for c in replaced.chars() {
        units += c.len_utf16();
        if units > 180 {
            break;
        }
        cut.push(c);
    }
    let mut name = cut.trim().to_owned();
    // :108
    while name.ends_with('.') || name.ends_with(' ') {
        name.pop();
    }
    // :109-112
    let stem = name.split('.').next().unwrap_or("");
    if reserved(stem) {
        name.insert(0, '_');
    }
    name
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_ordinary_name_is_kept() {
        assert_eq!(safe_file_name("378044.zip"), "378044.zip");
        assert_eq!(safe_file_name("(B090)수치지도_378044_2023_00000123456789.zip"), "(B090)수치지도_378044_2023_00000123456789.zip");
    }

    /// :104-105 — a path offered by the site never leaves the target folder.
    #[test]
    fn only_the_last_path_component_is_kept() {
        assert_eq!(safe_file_name("..\\..\\Windows\\system32\\evil.dll"), "evil.dll");
        assert_eq!(safe_file_name("/etc/passwd"), "passwd");
        assert_eq!(safe_file_name("C:\\Users\\x\\a.zip"), "a.zip");
        assert_eq!(safe_file_name("folder/"), "");
        assert_eq!(safe_file_name(".."), "");
    }

    /// :106
    #[test]
    fn forbidden_and_control_characters_become_underscores() {
        assert_eq!(safe_file_name("a<b>c:d\"e|f?g*h.zip"), "a_b_c_d_e_f_g_h.zip");
        assert_eq!(safe_file_name("a\u{0001}b\tc.zip"), "a_b_c.zip");
    }

    /// :107-108
    #[test]
    fn long_names_are_cut_and_trailing_dots_and_spaces_go() {
        let long = "가".repeat(200) + ".zip";
        let cut = safe_file_name(&long);
        assert_eq!(cut.chars().count(), 180);
        assert!(cut.chars().all(|c| c == '가'));
        assert_eq!(safe_file_name("  name.zip . . "), "name.zip");
        assert_eq!(safe_file_name("...."), "");
        // A character outside the basic plane takes two UTF-16 units and is not split.
        let astral = "a".repeat(179) + "𝒳z";
        assert_eq!(safe_file_name(&astral), "a".repeat(179));
    }

    /// :109-112
    #[test]
    fn device_names_get_a_leading_underscore() {
        for (offered, want) in [("CON", "_CON"), ("con.zip", "_con.zip"), ("Lpt1.txt", "_Lpt1.txt"), ("COM9", "_COM9"), ("nul.tar.gz", "_nul.tar.gz")] {
            assert_eq!(safe_file_name(offered), want);
        }
        for kept in ["COM0", "COM10.zip", "CONSOLE.zip", "LPT.zip", "xCON.zip"] {
            assert_eq!(safe_file_name(kept), kept);
        }
    }
}
