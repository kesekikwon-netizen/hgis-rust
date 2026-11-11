//! 본 앱과 웹 화면 도우미가 주고받는 말(한 줄에 JSON 하나).
//!
//! 본 앱 → 도우미(표준 입력): [`Command`]. 도우미 → 본 앱(표준 출력): [`Event`].
//! 계정 값은 `Eval` 의 `script` 안에만 실려 오고, 도우미는 그것을 화면에도 파일에도 적지 않는다.

use serde::{Deserialize, Serialize};

/// 본 앱이 시키는 일.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Command {
    /// 첫 화면에서 주소를 연다.
    Open { url: String },
    /// 주소의 경로가 `paths` 가운데 하나인 화면에서 스크립트를 돌리고 결과를 돌려준다.
    /// 그런 화면이 없으면 `found: false` 로 답한다.
    Eval { id: u64, paths: Vec<String>, script: String },
    /// 열려 있는 화면들의 주소(경로까지만)를 알려 달라.
    Pages { id: u64 },
    /// 창을 보이거나 감춘다.
    Show { visible: bool },
    /// 끝낸다.
    Quit,
}

/// 도우미가 알리는 일.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum Event {
    /// 준비됐다.
    Ready,
    /// `Eval` 의 답. `result` 는 스크립트가 돌려준 값이다.
    Eval { id: u64, found: bool, url: String, result: serde_json::Value },
    /// `Pages` 의 답.
    Pages { id: u64, urls: Vec<String> },
    /// 파일 받기가 시작됐다(저장할 곳).
    DownloadStarted { path: String },
    /// 파일 받기가 끝났다.
    DownloadDone { path: String, ok: bool },
    /// 사용자가 창을 닫았다.
    Closed,
    /// 도우미 쪽 오류(계정 값은 담지 않는다).
    Error { message: String },
}

/// 믿는 사이트인지: https 이고 호스트가 `allowed` 가운데 하나이거나 그 하위 도메인.
#[must_use]
pub fn is_trusted_url(url: &str, allowed: &[String]) -> bool {
    let Some(rest) = url.strip_prefix("https://") else {
        return false;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    // 사용자 정보(@)나 포트가 붙은 주소는 믿지 않는다.
    if authority.is_empty() || authority.contains('@') || authority.contains(':') {
        return false;
    }
    let host = authority.to_ascii_lowercase();
    allowed.iter().any(|a| host == *a || host.ends_with(&format!(".{a}")))
}

/// 주소의 경로 부분(`/ms/map/NlipMap.do`).
#[must_use]
pub fn url_path(url: &str) -> &str {
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    let from = rest.find('/').map_or("", |i| &rest[i..]);
    from.split(['?', '#']).next().unwrap_or("")
}

/// 받는 파일 이름에서 폴더 구분·위험한 글자를 뺀다. 남는 것이 없으면 `None`.
#[must_use]
pub fn safe_file_name(name: &str) -> Option<String> {
    let last = name.rsplit(['/', '\\']).next().unwrap_or("");
    let cleaned: String = last
        .chars()
        .filter(|c| !c.is_control() && !matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*'))
        .collect();
    let cleaned = cleaned.trim().trim_matches('.').to_owned();
    // Windows 장치 이름(CON, NUL, COM1 …)은 파일 이름으로 쓰지 않는다.
    let stem = cleaned.split('.').next().unwrap_or("").trim().to_ascii_uppercase();
    let device = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.ends_with(|c: char| c.is_ascii_digit()));
    (!cleaned.is_empty() && !device).then_some(cleaned)
}

/// 받는 파일의 출처가 믿는 사이트인지. 화면 안에서 만든 파일(`blob:https://…`)은 그 화면의 주소로 본다.
#[must_use]
pub fn is_trusted_download_url(url: &str, allowed: &[String]) -> bool {
    is_trusted_url(url.strip_prefix("blob:").unwrap_or(url), allowed)
}

/// `dir` 안에서 아직 없는 저장 경로. 같은 이름이 있으면 `이름 (2).zip` 처럼 번호를 붙인다(덮어쓰지 않는다).
#[must_use]
pub fn free_path(dir: &std::path::Path, name: &str) -> Option<std::path::PathBuf> {
    let (stem, ext) = match name.rfind('.') {
        Some(i) if i > 0 => (&name[..i], &name[i..]),
        _ => (name, ""),
    };
    (1..1000)
        .map(|n| if n == 1 { dir.join(name) } else { dir.join(format!("{stem} ({n}){ext}")) })
        .find(|path| !path.exists())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_and_events_are_one_json_line_each() {
        let line = r#"{"cmd":"eval","id":7,"paths":["/ms/map/NlipMap.do"],"script":"1+1"}"#;
        let cmd: Command = serde_json::from_str(line).expect("명령");
        assert_eq!(cmd, Command::Eval { id: 7, paths: vec!["/ms/map/NlipMap.do".to_owned()], script: "1+1".to_owned() });
        let out = serde_json::to_string(&Event::DownloadDone { path: "a.zip".to_owned(), ok: true }).expect("알림");
        assert_eq!(out, r#"{"event":"download_done","path":"a.zip","ok":true}"#);
        assert!(!out.contains('\n'));
    }

    #[test]
    fn only_https_pages_of_the_allowed_sites_are_trusted() {
        let allowed = vec!["ngii.go.kr".to_owned()];
        assert!(is_trusted_url("https://map.ngii.go.kr/ms/map/NlipMap.do", &allowed));
        assert!(is_trusted_url("https://ngii.go.kr/", &allowed));
        for bad in [
            "http://map.ngii.go.kr/",
            "https://ngii.go.kr.evil.example/",
            "https://evilngii.go.kr/",
            "https://map.ngii.go.kr@evil.example/",
            "https://map.ngii.go.kr:8443/",
            "about:blank",
        ] {
            assert!(!is_trusted_url(bad, &allowed), "{bad}");
        }
    }

    #[test]
    fn paths_and_file_names_are_cut_safely() {
        assert_eq!(url_path("https://map.ngii.go.kr/ms/map/NlipMap.do?a=1#x"), "/ms/map/NlipMap.do");
        assert_eq!(url_path("https://map.ngii.go.kr"), "");
        assert_eq!(safe_file_name("..\\..\\(B010)수치지도_36710.zip").as_deref(), Some("(B010)수치지도_36710.zip"));
        assert_eq!(safe_file_name("a/b/c:d?.dxf").as_deref(), Some("cd.dxf"));
        assert_eq!(safe_file_name("../.."), None);
        for device in ["NUL", "con.zip", "COM1.dxf", "lpt9"] {
            assert_eq!(safe_file_name(device), None, "{device}");
        }
        assert_eq!(safe_file_name("COMET.zip").as_deref(), Some("COMET.zip"));
    }

    #[test]
    fn downloads_come_only_from_trusted_sites_and_never_overwrite() {
        let allowed = vec!["ngii.go.kr".to_owned()];
        assert!(is_trusted_download_url("https://nlippd.ngii.go.kr/nlippd/pd/innorix/a.zip", &allowed));
        assert!(is_trusted_download_url("blob:https://nlippd.ngii.go.kr/1b2c", &allowed));
        assert!(!is_trusted_download_url("blob:https://evil.example/1b2c", &allowed));
        assert!(!is_trusted_download_url("data:application/zip;base64,AAAA", &allowed));

        let dir = std::env::temp_dir().join(format!("strata-web-free-path-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("폴더");
        assert_eq!(free_path(&dir, "a.zip"), Some(dir.join("a.zip")));
        std::fs::write(dir.join("a.zip"), b"x").expect("쓰기");
        assert_eq!(free_path(&dir, "a.zip"), Some(dir.join("a (2).zip")));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
