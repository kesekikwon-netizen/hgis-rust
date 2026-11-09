//! 다운로드 화면 검색 폼과 태그 도우미 (C++ `HeritageFormParser.cpp:9-212`).
//!
//! `heritage_intranet` 본문을 300줄 이하로 유지하기 위해 분리했다. 공개 경로는 그대로다.


/// 다운로드 화면의 검색 폼 (`HeritageFormParser.h:15-26`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HeritageForm {
    pub action: String,
    pub method: String,
    /// hidden 을 포함한 칸 이름 → 현재 값. 서버가 기대하는 값을 그대로 되돌려 보낸다.
    pub fields: Vec<(String, String)>,
    pub sido_field: String,
    pub sigungu_field: String,
    pub sido_options: Vec<(String, String)>,
    pub sigungu_options: Vec<(String, String)>,
    pub warnings: Vec<String>,
}

impl HeritageForm {
    #[must_use]
    pub fn ok(&self) -> bool {
        !self.sido_field.is_empty() && !self.fields.is_empty()
    }

    pub(super) fn field_value(&self, name: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }
}

pub(super) fn ci_find(hay: &str, needle_lower: &str, from: usize) -> Option<usize> {
    let h = hay.as_bytes();
    let n = needle_lower.as_bytes();
    if n.is_empty() || h.len() < n.len() {
        return None;
    }
    let mut i = from;
    while i + n.len() <= h.len() {
        if h[i..i + n.len()].eq_ignore_ascii_case(n) {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// `HeritageFormParser.cpp:9-20` — name="..." 또는 name='...' 또는 name=값
pub(super) fn attr(tag: &str, name: &str) -> Option<String> {
    let mut at = 0;
    while let Some(pos) = ci_find(tag, name, at) {
        at = pos + name.len();
        let before_ok = tag[..pos]
            .chars()
            .next_back()
            .is_none_or(|c| !(c.is_alphanumeric() || c == '_' || c == '-'));
        if !before_ok {
            continue;
        }
        let rest = tag[at..].trim_start();
        let Some(rest) = rest.strip_prefix('=') else {
            continue;
        };
        let rest = rest.trim_start();
        if let Some(body) = rest.strip_prefix('"') {
            return Some(body.split('"').next().unwrap_or("").to_owned());
        }
        if let Some(body) = rest.strip_prefix('\'') {
            return Some(body.split('\'').next().unwrap_or("").to_owned());
        }
        let bare: String = rest
            .chars()
            .take_while(|c| !c.is_whitespace() && *c != '>')
            .collect();
        return Some(bare);
    }
    None
}

/// `HeritageFormParser.cpp:22-28`
pub(super) fn strip_tags(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut depth = false;
    for c in text.chars() {
        match c {
            '<' => depth = true,
            '>' => depth = false,
            _ if !depth => out.push(c),
            _ => {}
        }
    }
    out.replace("&nbsp;", " ").replace("&amp;", "&").trim().to_owned()
}

/// `HeritageFormParser.cpp:31-35`
pub(super) fn squash(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect()
}

/// `HeritageFormParser.cpp:39-58` — 「강원도」 ↔ 「강원특별자치도」 표기 차이를 맞춘다.
pub(super) fn sido_core(raw: &str) -> String {
    let mut s = squash(raw);
    for tail in ["특별자치도", "특별자치시", "특별시", "광역시"] {
        if let Some(head) = s.strip_suffix(tail) {
            s = head.to_owned();
            break;
        }
    }
    if s.chars().count() > 2 && s.ends_with('도') {
        s.pop();
    }
    match s.as_str() {
        "전라북" => "전북".to_owned(),
        "전라남" => "전남".to_owned(),
        "경상북" => "경북".to_owned(),
        "경상남" => "경남".to_owned(),
        "충청북" => "충북".to_owned(),
        "충청남" => "충남".to_owned(),
        _ => s,
    }
}

pub(super) struct SelectBlock {
    pub(super) name: String,
    pub(super) id: String,
    pub(super) options: Vec<(String, String)>,
    pub(super) at: usize,
}
