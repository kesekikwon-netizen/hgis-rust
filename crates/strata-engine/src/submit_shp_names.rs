//! SHP field names ≡ cpp `SubmitShp::leftUtf8` / `fieldNameFor`
//! (target/cpp-ref/src/core/SubmitShp.cpp:50-80). DBF names hold at most 10 bytes.

use std::collections::HashSet;

/// `leftUtf8` (SubmitShp.cpp:50-63) — the longest prefix of whole characters within `max_bytes` UTF-8 bytes.
pub(crate) fn left_utf8(text: &str, max_bytes: usize) -> &str {
    let mut end = 0;
    for (i, c) in text.char_indices() {
        if i + c.len_utf8() > max_bytes {
            break;
        }
        end = i + c.len_utf8();
    }
    &text[..end]
}

/// `fieldNameFor` (SubmitShp.cpp:65-80) — the two legacy aliases, else a 10-byte cut; a serial
/// suffix (2..=99) keeps names unique within `used`.
pub(crate) fn field_name_for(name: &str, used: &mut HashSet<String>) -> String {
    let mapped = if name.len() > 10 {
        match name {
            "survey_name" => "surv_name",
            "artifact_no" => "artif_no",
            _ => left_utf8(name, 10),
        }
    } else {
        name
    };
    let mut candidate = mapped.to_owned();
    let mut serial = 2;
    while (used.contains(&candidate) || candidate.is_empty()) && serial <= 99 {
        let suffix = serial.to_string();
        serial += 1;
        candidate = format!("{}{suffix}", left_utf8(mapped, 10 - suffix.len()));
    }
    used.insert(candidate.clone());
    candidate
}
