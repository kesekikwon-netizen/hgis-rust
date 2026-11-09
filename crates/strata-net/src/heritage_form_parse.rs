//! 검색 폼 해석·시도 매칭·결과 건수 (C++ `HeritageFormParser.cpp`).
//!
//! `heritage_intranet` 본문을 300줄 이하로 유지하기 위해 분리했다. 공개 경로는 그대로다.
use super::heritage_form::ci_find;
use super::heritage_form::attr;
use super::heritage_form::strip_tags;
use super::heritage_form::squash;
use super::heritage_form::sido_core;
use super::heritage_form::SelectBlock;

use super::*;

/// `HeritageFormParser.cpp:67-94`
fn selects_in(html: &str) -> Vec<SelectBlock> {
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(open) = ci_find(html, "<select", at) {
        let Some(head_end_rel) = html[open..].find('>') else {
            break;
        };
        let head_end = open + head_end_rel;
        let Some(close_rel) = ci_find(html, "</select", head_end) else {
            break;
        };
        let head = &html[open + "<select".len()..head_end];
        let body = &html[head_end + 1..close_rel];
        let mut options = Vec::new();
        let mut opt_at = 0;
        while let Some(o) = ci_find(body, "<option", opt_at) {
            let Some(o_end_rel) = body[o..].find('>') else {
                break;
            };
            let o_end = o + o_end_rel;
            let Some(oc_rel) = ci_find(body, "</option", o_end) else {
                break;
            };
            let label = strip_tags(&body[o_end + 1..oc_rel]);
            let value = attr(&body[o + "<option".len()..o_end], "value").unwrap_or_else(|| label.clone());
            if !label.is_empty() {
                options.push((label, value));
            }
            opt_at = oc_rel + "</option".len();
        }
        out.push(SelectBlock {
            name: attr(head, "name").unwrap_or_default(),
            id: attr(head, "id").unwrap_or_default(),
            options,
            at: open,
        });
        at = close_rel + "</select".len();
    }
    out
}

/// `HeritageFormParser::parse` (`HeritageFormParser.cpp:98-174`)
#[must_use]
pub fn parse_form(html: &str, anchor: &str) -> HeritageForm {
    let mut out = HeritageForm {
        method: "POST".to_owned(),
        ..HeritageForm::default()
    };
    let want = anchor.to_lowercase();

    let mut head = String::new();
    let mut body = String::new();
    let mut at = 0;
    while let Some(open) = ci_find(html, "<form", at) {
        let Some(head_end_rel) = html[open..].find('>') else {
            break;
        };
        let head_end = open + head_end_rel;
        let Some(close_rel) = ci_find(html, "</form", head_end) else {
            break;
        };
        let candidate = &html[head_end + 1..close_rel];
        if candidate.to_lowercase().contains(&want) {
            head = html[open + "<form".len()..head_end].to_owned();
            body = candidate.to_owned();
            break;
        }
        at = close_rel + "</form".len();
    }
    if body.is_empty() {
        out.warnings
            .push(format!("'{anchor}' 칸을 품은 폼을 찾지 못했습니다."));
        return out;
    }

    out.action = attr(&head, "action").unwrap_or_default();
    let method = attr(&head, "method").unwrap_or_default().to_uppercase();
    if !method.is_empty() {
        out.method = method;
    }

    let mut at = 0;
    while let Some(open) = ci_find(&body, "<input", at) {
        let Some(end_rel) = body[open..].find('>') else {
            break;
        };
        let end = open + end_rel;
        let tag = &body[open + "<input".len()..end];
        at = end + 1;
        let Some(name) = attr(tag, "name") else {
            continue;
        };
        if name.is_empty() {
            continue;
        }
        let kind = attr(tag, "type").unwrap_or_default().to_lowercase();
        if kind == "submit" || kind == "button" {
            continue;
        }
        // 체크되지 않은 것은 보내지 않는다. 브라우저와 같은 규칙이다.
        if (kind == "checkbox" || kind == "radio")
            && ci_find(tag, "checked", 0).is_none()
        {
            continue;
        }
        let value = attr(tag, "value").unwrap_or_default();
        if let Some(slot) = out.fields.iter_mut().find(|(k, _)| *k == name) {
            slot.1 = value;
        } else {
            out.fields.push((name, value));
        }
    }

    let sels = selects_in(&body);
    let mut anchor_at = None;
    for s in &sels {
        let key = format!("{} {}", s.id, s.name).to_lowercase();
        if key.contains(&want) {
            out.sido_field = s.name.clone();
            out.sido_options = s.options.clone();
            anchor_at = Some(s.at);
            break;
        }
    }
    let Some(anchor_at) = anchor_at else {
        out.warnings
            .push(format!("'{anchor}' select 를 찾지 못했습니다."));
        return out;
    };

    // 시/군/구는 같은 폼에서 시/도 **바로 다음**에 오는 select 다.
    for s in &sels {
        if s.at <= anchor_at || s.name.is_empty() {
            continue;
        }
        out.sigungu_field = s.name.clone();
        out.sigungu_options = s.options.clone();
        break;
    }
    if out.sigungu_field.is_empty() {
        out.warnings
            .push("시·군·구 select 를 찾지 못했습니다.".to_owned());
    }

    for s in &sels {
        if !s.name.is_empty() && out.field_value(&s.name).is_none() {
            out.fields.push((s.name.clone(), String::new()));
        }
    }
    out
}

/// `HeritageFormParser::matchOption` (`HeritageFormParser.cpp:176-200`)
#[must_use]
pub fn match_option(options: &[(String, String)], wanted: &str) -> String {
    let w = squash(wanted);
    if options.is_empty() || w.is_empty() {
        return String::new();
    }
    for (label, value) in options {
        if squash(label) == w {
            return value.clone();
        }
    }
    for (label, value) in options {
        let squashed = squash(label);
        if squashed.contains(&w) || w.contains(&squashed) {
            return value.clone();
        }
    }
    let core = sido_core(wanted);
    for (label, value) in options {
        if sido_core(label) == core {
            return value.clone();
        }
    }
    // 전라남도·광주광역시는 사이트에서 「전남광주통합특별시」로 합쳐져 있다.
    if core == "전남" || core == "광주" {
        for (label, value) in options {
            if squash(label).contains("전남광주") {
                return value.clone();
            }
        }
    }
    String::new()
}

/// `HeritageFormParser::resultCount` (`HeritageFormParser.cpp:202-212`) — 없으면 -1
#[must_use]
pub fn result_count(html: &str) -> i32 {
    let text = strip_tags(html);
    let Some(pos) = text.find("검색결과") else {
        return -1;
    };
    let tail = &text[pos + "검색결과".len()..];
    let digits: String = tail
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit() || *c == ',')
        .collect();
    let cleaned = digits.replace(',', "");
    cleaned.parse::<i32>().unwrap_or(-1)
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
