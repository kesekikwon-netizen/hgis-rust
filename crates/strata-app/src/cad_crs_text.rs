//! Text patterns in `CadCrsHints::fromText` (`CadCrsHints.cpp:59-78`).

pub(super) fn scan_epsg_words(text: &str, out: &mut Vec<String>) {
    // `EPSG\s*[:_-]?\s*(\d{4,5})(?!\d)` (`CadCrsHints.cpp:60-61`).
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        const EPSG: [char; 4] = ['E', 'P', 'S', 'G'];
        if !super::starts_ci(&chars[i..], &EPSG) {
            i += 1;
            continue;
        }
        let mut j = i + 4;
        while j < chars.len() && chars[j].is_ascii_whitespace() {
            j += 1;
        }
        if j < chars.len() && matches!(chars[j], ':' | '_' | '-') {
            j += 1;
            while j < chars.len() && chars[j].is_ascii_whitespace() {
                j += 1;
            }
        }
        let start = j;
        while j < chars.len() && chars[j].is_ascii_digit() {
            j += 1;
        }
        let len = j - start;
        if (4..=5).contains(&len) {
            let digits: String = chars[start..j].iter().collect();
            super::add_auth(out, &format!("EPSG:{digits}"));
        }
        i = j.max(i + 1);
    }
}

pub(super) fn find_origin(text: &str) -> Option<usize> {
    let chars: Vec<char> = text.chars().collect();
    for i in 0..chars.len() {
        for (idx, (name, _, _)) in super::ORIGINS.iter().enumerate() {
            let name_chars: Vec<char> = name.chars().collect();
            if !chars[i..].starts_with(name_chars.as_slice()) {
                continue;
            }
            let mut j = i + name_chars.len();
            while j < chars.len() && chars[j].is_ascii_whitespace() {
                j += 1;
            }
            if chars[j..].starts_with(&['원', '점']) {
                return Some(idx);
            }
        }
    }
    None
}

pub(super) fn is_grs80(text: &str) -> bool {
    has_gap(text, "GRS", "80") || has_gap(text, "세계", "측지계") || has_token(text, "ITRF")
}

pub(super) fn is_bessel(text: &str) -> bool {
    has_token(text, "베셀")
        || has_token(text, "Bessel")
        || has_gap(text, "동경", "측지계")
        || has_gap(text, "지역", "측지계")
}

pub(super) fn has_utmk(text: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    for i in 0..chars.len() {
        if !super::starts_ci(&chars[i..], &['U', 'T', 'M']) {
            continue;
        }
        let mut j = i + 3;
        while j < chars.len() && chars[j].is_ascii_whitespace() {
            j += 1;
        }
        if j < chars.len() && chars[j] == '-' {
            j += 1;
        }
        while j < chars.len() && chars[j].is_ascii_whitespace() {
            j += 1;
        }
        if j < chars.len() && chars[j].eq_ignore_ascii_case(&'K') {
            return true;
        }
    }
    false
}

fn has_token(text: &str, token: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    let needle: Vec<char> = token.chars().collect();
    if needle.is_empty() || chars.len() < needle.len() {
        return false;
    }
    chars
        .windows(needle.len())
        .any(|window| super::starts_ci(window, &needle))
}

fn has_gap(text: &str, left: &str, right: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    let left_chars: Vec<char> = left.chars().collect();
    let right_chars: Vec<char> = right.chars().collect();
    for i in 0..chars.len() {
        if !super::starts_ci(&chars[i..], &left_chars) {
            continue;
        }
        let mut j = i + left_chars.len();
        while j < chars.len() && chars[j].is_ascii_whitespace() {
            j += 1;
        }
        if super::starts_ci(&chars[j..], &right_chars) {
            return true;
        }
    }
    false
}
