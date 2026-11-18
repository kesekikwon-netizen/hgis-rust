//! What a CAD drawing says its CRS is (`CadCrsHints.cpp:27-94`, `CadCrsGuess.cpp:52-86`).

use std::path::Path;

/// `CadCrsGuess.cpp:52-59` `candidateAuthIds`, in that order.
pub(crate) const CANDIDATE_AUTH_IDS: [&str; 21] = [
    "EPSG:5186",
    "EPSG:5187",
    "EPSG:5185",
    "EPSG:5188",
    "EPSG:5174",
    "EPSG:5176",
    "EPSG:5173",
    "EPSG:5177",
    "EPSG:5175",
    "EPSG:5181",
    "EPSG:5183",
    "EPSG:5180",
    "EPSG:5184",
    "EPSG:5182",
    "EPSG:2097",
    "EPSG:2096",
    "EPSG:2098",
    "EPSG:5179",
    "EPSG:32652",
    "EPSG:32651",
    "EPSG:4326",
];

/// `CadCrsHints.cpp:49-56`. GRS80 codes, then the corrected Bessel code.
const ORIGINS: [(&str, &[&str], &str); 5] = [
    ("중부", &["EPSG:5186", "EPSG:5181"], "EPSG:5174"),
    ("동부", &["EPSG:5187", "EPSG:5183"], "EPSG:5176"),
    ("서부", &["EPSG:5185", "EPSG:5180"], "EPSG:5173"),
    ("동해", &["EPSG:5188", "EPSG:5184"], "EPSG:5177"),
    ("제주", &["EPSG:5182"], "EPSG:5175"),
];

/// `CadCrsGuess.cpp:62-86` `label`. An id outside the table is returned as itself (`names.value`).
#[must_use]
pub(crate) fn crs_label(auth_id: &str) -> &str {
    const LABELS: &[(&str, &str)] = &[
        ("EPSG:5186", "GRS80 중부원점(2010)"),
        ("EPSG:5187", "GRS80 동부원점(2010)"),
        ("EPSG:5185", "GRS80 서부원점(2010)"),
        ("EPSG:5188", "GRS80 동해원점(2010)"),
        ("EPSG:5174", "베셀 중부원점 보정"),
        ("EPSG:5176", "베셀 동부원점 보정"),
        ("EPSG:5173", "베셀 서부원점 보정"),
        ("EPSG:5177", "베셀 동해원점 보정"),
        ("EPSG:5175", "베셀 제주원점 보정"),
        ("EPSG:5181", "GRS80 중부원점(2002)"),
        ("EPSG:5183", "GRS80 동부원점(2002)"),
        ("EPSG:5180", "GRS80 서부원점(2002)"),
        ("EPSG:5184", "GRS80 동해원점(2002)"),
        ("EPSG:5182", "GRS80 제주원점(2002)"),
        ("EPSG:2097", "베셀 중부원점"),
        ("EPSG:2096", "베셀 동부원점"),
        ("EPSG:2098", "베셀 서부원점"),
        ("EPSG:5179", "UTM-K"),
        ("EPSG:32652", "UTM 52N"),
        ("EPSG:32651", "UTM 51N"),
        ("EPSG:4326", "경위도(WGS84)"),
    ];
    LABELS
        .iter()
        .find(|(id, _)| *id == auth_id)
        .map(|(_, label)| *label)
        .unwrap_or(auth_id)
}

/// `CadCrsHints.cpp:82-94` `stated`.
#[must_use]
pub(crate) fn stated_auth_ids(path: &Path, texts: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(id) = prj_auth(path) {
        add_auth(&mut out, &id);
    }
    if let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) {
        stated_from_stem(stem, &mut out);
    }
    for text in texts {
        from_text(text, &mut out);
    }
    out
}
fn add_auth(out: &mut Vec<String>, auth_id: &str) {
    // `CadCrsHints.cpp:27-28`.
    if !auth_id.is_empty()
        && !out.iter().any(|id| id == auth_id)
        && CANDIDATE_AUTH_IDS.contains(&auth_id)
    {
        out.push(auth_id.to_owned());
    }
}

fn prj_auth(path: &Path) -> Option<String> {
    // `CadCrsHints.cpp:32-44` — the `.prj` beside the drawing.
    let stem = path.file_stem()?.to_str()?;
    let folder = path.parent()?;
    let mut found = None;
    for entry in std::fs::read_dir(folder).ok()?.flatten() {
        let candidate = entry.path();
        if !candidate.is_file() {
            continue;
        }
        let same_stem = candidate
            .file_stem()
            .and_then(|s| s.to_str())
            .is_some_and(|s| s.eq_ignore_ascii_case(stem));
        let prj = candidate
            .extension()
            .and_then(|s| s.to_str())
            .is_some_and(|s| s.eq_ignore_ascii_case("prj"));
        if same_stem && prj {
            found = Some(candidate);
        }
    }
    let bytes = std::fs::read(found?).ok()?;
    if bytes.len() > 65_536 {
        return None;
    }
    strata_net::topographic_catalog::crs_auth_id_from_wkt(&String::from_utf8_lossy(&bytes))
}

fn stated_from_stem(stem: &str, out: &mut Vec<String>) {
    let chars: Vec<char> = stem.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if let Some(end) = epsg_at(&chars, i, out) {
            i = end;
            continue;
        }
        if let Some(end) = bare_at(&chars, i, out) {
            i = end;
            continue;
        }
        i += 1;
    }
}

fn epsg_at(chars: &[char], i: usize, out: &mut Vec<String>) -> Option<usize> {
    const EPSG: [char; 4] = ['E', 'P', 'S', 'G'];
    if !starts_ci(&chars[i..], &EPSG) {
        return None;
    }
    let mut j = i + 4;
    while j < chars.len() && is_sep(chars[j]) {
        j += 1;
    }
    let start = j;
    while j < chars.len() && chars[j].is_ascii_digit() {
        j += 1;
    }
    let len = j - start;
    if (4..=5).contains(&len) {
        let digits: String = chars[start..j].iter().collect();
        add_auth(out, &format!("EPSG:{digits}"));
        return Some(j);
    }
    Some(i + 4)
}

fn bare_at(chars: &[char], i: usize, out: &mut Vec<String>) -> Option<usize> {
    if !chars[i].is_ascii_digit() {
        return None;
    }
    let blocked = i > 0 && (chars[i - 1].is_ascii_digit() || chars[i - 1].is_alphabetic());
    if blocked {
        return None;
    }
    let mut j = i;
    while j < chars.len() && chars[j].is_ascii_digit() {
        j += 1;
    }
    let len = j - i;
    let hyphen_digit = j + 1 < chars.len() && chars[j] == '-' && chars[j + 1].is_ascii_digit();
    let next_letter = j < chars.len() && chars[j].is_alphabetic();
    if (4..=5).contains(&len) && !hyphen_digit && !next_letter {
        let digits: String = chars[i..j].iter().collect();
        add_auth(out, &format!("EPSG:{digits}"));
    }
    Some(j)
}

fn from_text(text: &str, out: &mut Vec<String>) {
    text::scan_epsg_words(text, out);
    if text::has_utmk(text) {
        add_auth(out, "EPSG:5179");
    }
    let Some(origin) = text::find_origin(text) else {
        return;
    };
    let (_, grs, bessel) = ORIGINS[origin];
    let grs80 = text::is_grs80(text);
    let bessel_datum = text::is_bessel(text);
    if grs80 || !bessel_datum {
        for id in grs {
            add_auth(out, id);
        }
    }
    if bessel_datum || !grs80 {
        add_auth(out, bessel);
    }
}

#[path = "cad_crs_text.rs"]
mod text;

fn starts_ci(hay: &[char], needle: &[char]) -> bool {
    hay.len() >= needle.len()
        && hay[..needle.len()]
            .iter()
            .zip(needle)
            .all(|(a, b)| a.eq_ignore_ascii_case(b))
}

fn is_sep(c: char) -> bool {
    c.is_ascii_whitespace() || matches!(c, '_' | ':' | '-')
}
