//! 홈 화면의 폴더·시각 글 — C++ `KaHomeText.cpp`(folder :13-28, folderTail :30-36,
//! folderEnd :38-43, when :45-55).

use chrono::{Local, NaiveDate, NaiveDateTime, TimeZone};

const SEP: &str = " › ";

fn cleaned(path: &str) -> String {
    path.replace('\\', "/").trim_end_matches('/').to_owned()
}

fn starts_with_ignore_case(s: &str, prefix: &str) -> bool {
    s.len() >= prefix.len() && s.is_char_boundary(prefix.len()) && s[..prefix.len()].to_lowercase() == prefix.to_lowercase()
}

/// `folder` 의 본문 — 바탕 화면·문서 폴더(`QStandardPaths`)를 받아 시험할 수 있게 나눴다.
pub(crate) fn folder_in(path: &str, desktop: Option<&str>, documents: Option<&str>) -> String {
    let abs = strata_storage::recent_surveys::absolute_file_path(std::path::Path::new(path));
    let dir = abs.rfind('/').map_or(abs.as_str(), |k| &abs[..k]).to_owned();
    for (base, label) in [(desktop, "바탕 화면"), (documents, "문서")] {
        let Some(base) = base.map(cleaned).filter(|b| !b.is_empty()) else { continue };
        if dir.to_lowercase() == base.to_lowercase() {
            return label.to_owned();
        }
        let with_slash = format!("{base}/");
        if starts_with_ignore_case(&dir, &with_slash) {
            let rest: Vec<&str> = dir[with_slash.len()..].split('/').collect();
            return format!("{label}{SEP}{}", rest.join(SEP));
        }
    }
    dir.split('/').filter(|s| !s.is_empty()).collect::<Vec<_>>().join(SEP)
}

/// `KaHomeText::folder` — 조사 파일이 있는 폴더를 「문서 › 경주시」처럼.
pub(crate) fn folder(path: &str) -> String {
    let desktop = dirs::desktop_dir().map(|p| p.to_string_lossy().into_owned());
    let documents = crate::user_settings::documents_path().map(|p| p.to_string_lossy().into_owned());
    folder_in(path, desktop.as_deref(), documents.as_deref())
}

/// `folderTail` — 끝 두 마디와 조사 이름(폴더가 조사 이름과 같으면 되풀이하지 않는다).
pub(crate) fn folder_tail(folder: &str, name: &str) -> String {
    let mut steps: Vec<&str> = folder.split(SEP).filter(|s| !s.is_empty()).collect();
    if !name.is_empty() && steps.last() != Some(&name) {
        steps.push(name);
    }
    if steps.len() <= 2 {
        return steps.join(SEP);
    }
    format!("…{SEP}{}{SEP}{}", steps[steps.len() - 2], steps[steps.len() - 1])
}

/// `folderEnd` — 마디가 셋 이상이면 「… › 마지막」.
pub(crate) fn folder_end(folder: &str) -> String {
    let steps: Vec<&str> = folder.split(SEP).filter(|s| !s.is_empty()).collect();
    match steps.last() {
        Some(last) if steps.len() > 2 => format!("…{SEP}{last}"),
        _ => folder.to_owned(),
    }
}

/// `when` 의 본문 — 현지 시각과 오늘 날짜를 받는다.
pub(crate) fn when_at(at: NaiveDateTime, today: NaiveDate) -> String {
    let time = at.format("%H:%M");
    let date = at.date();
    if date == today {
        format!("오늘 {time}")
    } else if today.pred_opt() == Some(date) {
        format!("어제 {time}")
    } else if date.format("%Y").to_string() == today.format("%Y").to_string() {
        format!("{}월 {}일 {time}", date.format("%-m"), date.format("%-d"))
    } else {
        at.format("%Y-%m-%d %H:%M").to_string()
    }
}

/// `KaHomeText::when` — 저장된 시각(ms)을 「어제 22:26」처럼. 0 이하는 빈 글.
pub(crate) fn when(ms: i64) -> String {
    if ms <= 0 {
        return String::new();
    }
    let Some(at) = Local.timestamp_millis_opt(ms).single() else {
        return String::new();
    };
    when_at(at.naive_local(), Local::now().date_naive())
}

#[cfg(test)]
#[path = "home_text_tests.rs"]
mod tests;
