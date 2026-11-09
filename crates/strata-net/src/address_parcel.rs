//! 지번·도로명 주소 해석과 대조 (C++ `parseParcelAddress` / `sameParcel`).
//!
//! `address_search` 본문을 300줄 이하로 유지하기 위해 분리했다. 공개 경로는 그대로다.

use super::*;

/// C++ `kaIsRoadAddress` (`AddressQuery.h:7-19`) 1:1
#[must_use]
pub fn is_road_address(query: &str) -> bool {
    let words: Vec<&str> = query.split_whitespace().collect();
    if words.len() < 2 {
        return false;
    }
    let last = words[words.len() - 1];
    let last_trimmed = last.strip_suffix('번').unwrap_or(last);
    let valid_bldg = if let Some((main, sub)) = last_trimmed.split_once('-') {
        !main.is_empty()
            && main.chars().all(|c| c.is_ascii_digit())
            && !sub.is_empty()
            && sub.chars().all(|c| c.is_ascii_digit())
    } else {
        !last_trimmed.is_empty() && last_trimmed.chars().all(|c| c.is_ascii_digit())
    };
    if !valid_bldg {
        return false;
    }
    for &word in &words[..words.len() - 1] {
        if word.ends_with("대로") || word.ends_with("길") {
            return true;
        }
        if word.ends_with('로') && word != "세종로" {
            return true;
        }
    }
    false
}

/// C++ `KaAppBar::looksLikeLot` (`KaAppBar.cpp:78-83`) 1:1
#[must_use]
pub fn looks_like_lot(query: &str) -> bool {
    if is_road_address(query) {
        return false;
    }
    let words: Vec<&str> = query.split_whitespace().collect();
    if words.len() < 2 {
        return false;
    }
    let last = words[words.len() - 1];
    let last = last.strip_prefix('산').unwrap_or(last);
    let last = last.strip_suffix("번지").unwrap_or(last);
    if let Some((main, sub)) = last.split_once('-') {
        !main.is_empty()
            && main.chars().all(|c| c.is_ascii_digit())
            && !sub.is_empty()
            && sub.chars().all(|c| c.is_ascii_digit())
    } else {
        !last.is_empty() && last.chars().all(|c| c.is_ascii_digit())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParcelAddress {
    pub(crate) area: Vec<String>,
    pub(crate) mountain: bool,
    pub(crate) main: u32,
    pub(crate) sub: u32,
}

fn is_legal_locality(token: &str) -> bool {
    if token == "세종로" || token.ends_with('동') || token.ends_with('리') {
        return true;
    }
    if let Some(stem) = token.strip_suffix('가') {
        if let Some(idx) = stem.rfind('동') {
            let digits = &stem[idx + '동'.len_utf8()..];
            if !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()) {
                return true;
            }
        }
        let digits: String = stem
            .chars()
            .rev()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if !digits.is_empty() {
            return true;
        }
    }
    false
}

/// C++ `parseParcelAddress` (`LocationSearch.cpp:31-65`) 1:1
pub(super) fn parse_parcel_address(text: &str) -> Option<ParcelAddress> {
    let normalized = text.replace(['\u{2013}', '\u{2011}'], "-");
    let mut tokens: Vec<String> = normalized
        .split_whitespace()
        .map(ToOwned::to_owned)
        .collect();
    if tokens.len() < 2 {
        return None;
    }
    let mut lot_str = tokens.pop()?;
    if lot_str == "번지" {
        lot_str = tokens.pop()?;
    } else if let Some(stripped) = lot_str.strip_suffix("번지") {
        lot_str = stripped.trim().to_owned();
    }
    let mut mountain = false;
    if let Some(stripped) = lot_str.strip_prefix('산') {
        mountain = true;
        lot_str = stripped.trim().to_owned();
    } else if tokens.last().map(String::as_str) == Some("산") {
        mountain = true;
        tokens.pop();
    }
    let (main, sub) = if let Some((m_str, s_str)) = lot_str.split_once('-') {
        let m: u32 = m_str.trim().parse().ok()?;
        let s: u32 = s_str.trim().parse().ok()?;
        if m == 0 || m > 9999 || s == 0 || s > 9999 {
            return None;
        }
        (m, s)
    } else {
        let m: u32 = lot_str.trim().parse().ok()?;
        if m == 0 || m > 9999 {
            return None;
        }
        (m, 0)
    };
    if tokens.is_empty() {
        return None;
    }
    tokens[0] = KoreaRegionCatalog::canonical_sido(&tokens[0]);
    let sejong = tokens[0] == "세종특별자치시";
    if sejong && tokens.get(1).map(String::as_str) == Some("세종시") {
        tokens.remove(1);
    }
    let province_known = KoreaRegionCatalog::all_sido()
        .iter()
        .any(|p| p.name == tokens[0]);
    if !province_known || tokens.len() < if sejong { 2 } else { 3 } {
        return None;
    }
    if !sejong
        && !tokens[1].ends_with('시')
        && !tokens[1].ends_with('군')
        && !tokens[1].ends_with('구')
    {
        return None;
    }
    let last_area = tokens.last()?;
    if !is_legal_locality(last_area) {
        return None;
    }
    Some(ParcelAddress {
        area: tokens,
        mountain,
        main,
        sub,
    })
}

/// C++ `sameParcel` (`LocationSearch.cpp:67-86`) 1:1
pub(super) fn same_parcel(requested: &ParcelAddress, result: &ParcelAddress) -> bool {
    if requested.mountain != result.mountain
        || requested.main != result.main
        || requested.sub != result.sub
    {
        return false;
    }
    if requested.area == result.area {
        return true;
    }
    let mut expanded = result.area.clone();
    if expanded.len() == requested.area.len() + 1
        && expanded.len() > 3
        && expanded[1].ends_with('시')
        && expanded[2].ends_with('구')
    {
        expanded.remove(2);
        return requested.area == expanded;
    }
    false
}

pub(super) fn percent_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 3);
    for b in input.as_bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(char::from(*b));
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}
