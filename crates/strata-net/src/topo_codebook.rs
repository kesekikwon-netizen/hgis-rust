//! 파일 이름·PRJ 로 도엽 코드를 추정하는 규칙 (C++ `TopographicCatalog.cpp:64-103`).
//!
//! `topographic_catalog` 본문을 300줄 이하로 유지하기 위해 분리했다. 공개 경로는 그대로다.
use std::path::Path;
use std::path::PathBuf;
use std::collections::BTreeMap;
use std::sync::OnceLock;
use serde_json::Value;
use super::topo_types::LAYER_CODES_JSON;

use super::*;

#[derive(Debug, Clone)]
struct CodeInfo {
    category: Category,
    name: String,
}

/// C++ `TopographicCatalog.cpp:64-88` `codeBook()` — `data/topographic/layer-codes.json`.
fn code_book() -> &'static BTreeMap<String, CodeInfo> {
    static BOOK: OnceLock<BTreeMap<String, CodeInfo>> = OnceLock::new();
    BOOK.get_or_init(|| {
        let mut entries = BTreeMap::new();
        let document: Value = serde_json::from_str(LAYER_CODES_JSON).unwrap_or(Value::Null);
        let Some(codes) = document.get("codes").and_then(Value::as_array) else {
            return entries;
        };
        for value in codes {
            let code = value.get("code").and_then(Value::as_str).unwrap_or_default();
            let name = value.get("name").and_then(Value::as_str).unwrap_or_default();
            if code.is_empty() || name.is_empty() {
                continue;
            }
            let category = value
                .get("category")
                .and_then(Value::as_str)
                .map_or(Category::Unknown, Category::from_key);
            entries.insert(
                code.to_owned(),
                CodeInfo {
                    category,
                    name: name.to_owned(),
                },
            );
        }
        entries
    })
}

/// C++ `TopographicCatalog.cpp:94` `(?<![\p{L}\p{N}])([A-H][0-9]{7})(?![\p{L}\p{N}])`.
fn code_tokens(name: &str) -> Vec<&str> {
    let chars: Vec<(usize, char)> = name.char_indices().collect();
    let mut tokens = Vec::new();
    let mut index = 0;
    while index + 8 <= chars.len() {
        let window = &chars[index..index + 8];
        let shape_ok =
            matches!(window[0].1, 'A'..='H') && window[1..].iter().all(|(_, c)| c.is_ascii_digit());
        let before_ok = index == 0 || !chars[index - 1].1.is_alphanumeric();
        let after_ok = index + 8 == chars.len() || !chars[index + 8].1.is_alphanumeric();
        if shape_ok && before_ok && after_ok {
            tokens.push(&name[window[0].0..=window[7].0]);
            index += 8;
        } else {
            index += 1;
        }
    }
    tokens
}

/// C++ `TopographicCatalog.cpp:89-103` `inferCode` — 카테고리 한글명, 이어서 코드북 대조.
/// 코드북에 없거나 카테고리가 서로 다른 코드가 섞이면 미분류다.
#[must_use]
pub fn infer_code(name: &str) -> (Category, String) {
    let trimmed = name.trim();
    for category in Category::ALL {
        if trimmed == category.name_ko() {
            return (category, trimmed.to_owned());
        }
    }
    let mut found: Option<&CodeInfo> = None;
    for token in code_tokens(name) {
        let Some(info) = code_book().get(token) else {
            return (Category::Unknown, String::new());
        };
        if found.is_some_and(|known| known.category != info.category) {
            return (Category::Unknown, String::new());
        }
        found = Some(info);
    }
    found.map_or((Category::Unknown, String::new()), |info| {
        (info.category, info.name.clone())
    })
}

/// `TopographicCatalog.cpp:41-45` — 확장자(대소문자 무관)가 네 가지 중 하나인가.
#[must_use]
pub fn is_supported(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            SUPPORTED_EXTENSIONS
                .iter()
                .any(|known| extension.eq_ignore_ascii_case(known))
        })
}

pub(super) fn suffix_of(path: &Path) -> String {
    path.extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}

/// C++ `TopographicCatalog.cpp:50-52` — 같은 이름의 곁 파일(`.prj`, 대소문자 무관).
pub(super) fn companion(path: &Path, wanted: &str) -> Option<PathBuf> {
    let stem = path.file_stem()?.to_str()?;
    let folder = path.parent()?;
    let mut found: Option<PathBuf> = None;
    for entry in std::fs::read_dir(folder).ok()?.flatten() {
        let candidate = entry.path();
        if !candidate.is_file() {
            continue;
        }
        let Some(candidate_stem) = candidate.file_stem().and_then(|value| value.to_str()) else {
            continue;
        };
        let matches_extension = candidate
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case(wanted));
        if matches_extension && candidate_stem.eq_ignore_ascii_case(stem) {
            found = Some(candidate);
        }
    }
    found
}

/// `.prj` 글 안의 원점 이름 — `CadCrsHints.cpp:48-57` 이 후보 정의와 맞춰 보던 자리.
/// ESRI 글에는 권위 번호가 없는 일이 흔해 이름으로 읽는다.
/// (원점 이름, GRS80 계열, 보정 베셀 계열) — `CadCrsHints.cpp:50-54`.
const PRJ_NAME_CODES: [(&str, u32, u32); 7] = [
    ("JEJU", 5182, 5175),
    ("EAST_SEA", 5188, 5177),
    ("WEST", 5185, 5173),
    ("EAST", 5187, 5176),
    ("CENTRAL", 5186, 5174),
    ("UNIFIED", 5179, 5179),
    ("UTM", 5179, 5179),
];

/// `.prj` WKT 에서 좌표계를 읽는다. 못 읽으면 `None`.
#[must_use]
pub fn crs_auth_id_from_wkt(wkt: &str) -> Option<String> {
    let upper = wkt.to_ascii_uppercase();
    if let Some(position) = upper.find("AUTHORITY[\"EPSG\",\"") {
        let rest = &upper[position + "AUTHORITY[\"EPSG\",\"".len()..];
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        return (!digits.is_empty()).then(|| format!("EPSG:{digits}"));
    }
    let bessel = upper.contains("BESSEL");
    for (needle, code_grs80, code_bessel) in PRJ_NAME_CODES {
        if upper.contains(needle) {
            let code = if bessel { code_bessel } else { code_grs80 };
            return Some(format!("EPSG:{code}"));
        }
    }
    None
}

/// 곁 파일 `.prj` 를 읽어 좌표계를 얻는다. 없거나 못 읽으면 `None`.
pub(super) fn prj_crs_auth_id(path: &Path) -> Option<String> {
    let prj = companion(path, "prj")?;
    let bytes = std::fs::read(prj).ok()?;
    if bytes.len() > 65_536 {
        return None;
    }
    crs_auth_id_from_wkt(&String::from_utf8_lossy(&bytes))
}
