//! Qt 6 `QSettings` IniFormat 값 읽기·쓰기 — C++ 앱이 쓰는 사용자 ini 를 함께 쓴다.
//!
//! 글 규칙은 Qt 6 `qsettings.cpp` 의 `iniEscapedString`·`iniEscapedStringList`·
//! `iniUnescapedStringList` 와 `variantToString`/`stringToVariant` 의 문자열 부분이다. 실제
//! Qt 6.11.1(OSGeo4W PyQt6)이 쓴 바이트로 맞췄다(`qt_ini_tests.rs`). Qt 는 저장할 때 파일 전체를
//! 다시 쓰지만, 여기서는 바꾸는 한 줄만 고치고 나머지 바이트(BOM·줄바꿈·다른 절)는 그대로 둔다.

use std::fmt::Write as _;

/// `variantToString`(QString) — NUL 이 있으면 `@String(…)`, `@` 로 시작하면 `@@`.
fn variant_to_string(s: &str) -> String {
    if s.contains('\0') {
        format!("@String({s})")
    } else if s.starts_with('@') {
        format!("@{s}")
    } else {
        s.to_owned()
    }
}

/// `iniEscapedString` — 제어 글자는 `\t`·`\x1` 꼴, `\x` 뒤에 오는 16진 글자도 `\x62` 로 쓴다.
fn ini_escaped(s: &str, out: &mut String) {
    let start = out.len();
    let mut needs_quotes = false;
    let mut escape_next_if_digit = false;
    for ch in s.chars() {
        if matches!(ch, ';' | ',' | '=') {
            needs_quotes = true;
        }
        if escape_next_if_digit && ch.is_ascii_hexdigit() {
            let _ = write!(out, "\\x{:x}", u32::from(ch));
            continue;
        }
        escape_next_if_digit = false;
        match ch {
            '\0' => {
                out.push_str("\\0");
                escape_next_if_digit = true;
            }
            '\u{7}' => out.push_str("\\a"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{b}' => out.push_str("\\v"),
            '"' | '\\' => {
                out.push('\\');
                out.push(ch);
            }
            c if u32::from(c) <= 0x1f => {
                let _ = write!(out, "\\x{:x}", u32::from(c));
                escape_next_if_digit = true;
            }
            c => out.push(c),
        }
    }
    let part = &out[start..];
    if needs_quotes || part.starts_with(' ') || part.ends_with(' ') {
        out.insert(start, '"');
        out.push('"');
    }
}

/// 글자 값 하나(`setValue(key, QString)`)의 ini 글.
#[must_use]
pub fn escaped_string(s: &str) -> String {
    let mut out = String::new();
    ini_escaped(&variant_to_string(s), &mut out);
    out
}

/// 글 목록(`setValue(key, QStringList)`)의 ini 글 — 「, 」로 잇고, 빈 목록은 `@Invalid()`.
#[must_use]
pub fn escaped_string_list(items: &[String]) -> String {
    if items.is_empty() {
        return "@Invalid()".to_owned();
    }
    let mut out = String::new();
    for (i, s) in items.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        ini_escaped(&variant_to_string(s), &mut out);
    }
    out
}

/// `iniUnescapedStringList` 의 결과 — 따옴표 밖 쉼표가 있으면 목록이다.
enum Unescaped {
    Text(String),
    List(Vec<String>),
}

fn simple_escape(ch: u8) -> Option<u16> {
    let v = match ch {
        b'a' => 0x07,
        b'b' => 0x08,
        b'f' => 0x0c,
        b'n' => 0x0a,
        b'r' => 0x0d,
        b't' => 0x09,
        b'v' => 0x0b,
        b'"' | b'?' | b'\'' | b'\\' => ch,
        _ => return None,
    };
    Some(u16::from(v))
}

fn chop_trailing_spaces(cur: &mut Vec<u16>, limit: usize) {
    while cur.len() > limit && matches!(cur.last(), Some(&0x20 | &0x09)) {
        cur.pop();
    }
}

/// Qt 의 상태 기계(StSkipSpaces·StNormal·StHexEscape·StOctEscape)를 그대로 옮겼다. 글은 UTF-16
/// 단위로 모은다 — Qt 5 가 쓴 `\xac00` 같은 값도 같은 글자가 된다.
fn ini_unescaped(raw: &[u8]) -> Unescaped {
    let n = raw.len();
    let mut cur: Vec<u16> = Vec::new();
    let mut list: Option<Vec<String>> = None;
    let (mut in_quotes, mut cur_quoted) = (false, false);
    let mut i = 0usize;
    let mut skip_spaces = true;
    'normal: loop {
        if skip_spaces {
            while i < n && matches!(raw[i], b' ' | b'\t') {
                i += 1;
            }
            skip_spaces = false;
        }
        let mut chop_limit = cur.len();
        while i < n {
            match raw[i] {
                b'\\' => {
                    i += 1;
                    if i >= n {
                        break 'normal;
                    }
                    let ch = raw[i];
                    i += 1;
                    if let Some(v) = simple_escape(ch) {
                        cur.push(v);
                        continue 'normal;
                    }
                    let radix_digits: Option<(u32, u16)> = if ch == b'x' {
                        if i >= n {
                            break 'normal;
                        }
                        raw[i].is_ascii_hexdigit().then_some((16, 0))
                    } else if (b'0'..=b'7').contains(&ch) {
                        Some((8, u16::from(ch - b'0')))
                    } else {
                        if (ch == b'\n' || ch == b'\r') && i < n && matches!(raw[i], b'\n' | b'\r') && raw[i] != ch {
                            i += 1;
                        }
                        None
                    };
                    if let Some((radix, mut val)) = radix_digits {
                        while i < n {
                            let Some(d) = char::from(raw[i]).to_digit(radix) else { break };
                            val = val.wrapping_mul(radix as u16).wrapping_add(d as u16);
                            i += 1;
                        }
                        cur.push(val);
                        if i >= n {
                            break 'normal;
                        }
                        continue 'normal;
                    }
                    chop_limit = cur.len();
                }
                b'"' => {
                    i += 1;
                    cur_quoted = true;
                    in_quotes = !in_quotes;
                    if !in_quotes {
                        skip_spaces = true;
                        continue 'normal;
                    }
                }
                b',' if !in_quotes => {
                    if !cur_quoted {
                        chop_trailing_spaces(&mut cur, chop_limit);
                    }
                    list.get_or_insert_with(Vec::new).push(String::from_utf16_lossy(&cur));
                    cur.clear();
                    cur_quoted = false;
                    i += 1;
                    skip_spaces = true;
                    continue 'normal;
                }
                _ => {
                    let j = (i + 1..n).find(|&j| matches!(raw[j], b'\\' | b'"' | b',')).unwrap_or(n);
                    cur.extend(String::from_utf8_lossy(&raw[i..j]).encode_utf16());
                    i = j;
                }
            }
        }
        if !cur_quoted {
            chop_trailing_spaces(&mut cur, chop_limit);
        }
        break;
    }
    let last = String::from_utf16_lossy(&cur);
    match list {
        Some(mut items) => {
            items.push(last);
            Unescaped::List(items)
        }
        None => Unescaped::Text(last),
    }
}

/// `stringToVariant` 의 글자 부분 — `@@` 는 `@`, `@String(…)`·`@ByteArray(…)` 는 안의 글,
/// `@Invalid()`·다른 꼴(`@Variant(…)` 등)은 글이 아니다(None).
fn variant_string(s: String) -> Option<String> {
    if let Some(rest) = s.strip_prefix("@@") {
        return Some(format!("@{rest}"));
    }
    if s.starts_with('@') && s.ends_with(')') {
        for prefix in ["@String(", "@ByteArray("] {
            if let Some(inner) = s.strip_prefix(prefix).and_then(|r| r.strip_suffix(')')) {
                return Some(inner.to_owned());
            }
        }
        if s.contains('(') {
            return None;
        }
    }
    Some(s)
}

/// `value(key).toStringList()`.
#[must_use]
pub fn read_string_list(raw: &str) -> Vec<String> {
    match ini_unescaped(raw.as_bytes()) {
        Unescaped::List(items) => items.into_iter().map(|e| variant_string(e).unwrap_or_default()).collect(),
        Unescaped::Text(s) => variant_string(s).into_iter().collect(),
    }
}

/// `value(key).toString()` — 목록은 한 항목일 때만 그 글.
#[must_use]
pub fn read_string(raw: &str) -> String {
    match ini_unescaped(raw.as_bytes()) {
        Unescaped::List(_) => String::new(),
        Unescaped::Text(s) => variant_string(s).unwrap_or_default(),
    }
}

#[path = "qt_ini_file.rs"]
mod qt_ini_file;
pub use qt_ini_file::IniFile;

#[cfg(test)]
#[path = "qt_ini_tests.rs"]
mod tests;
