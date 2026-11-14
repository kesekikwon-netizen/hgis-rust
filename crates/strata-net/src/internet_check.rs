//! 「인터넷 확인」 — C++ `KaHomeConnectionCard::checkInternet`(KaHomeConnectionCard.cpp:187-207): VWorld 서버
//! (`https://api.vworld.kr/`, 키 없음)에 HEAD 를 한 번, 6초 안에. 어떤 HTTP 대답이든 오면 닿은 것이다.

use std::process::{Command, Stdio};

/// checkInternet 의 주소.
pub const CHECK_URL: &str = "https://api.vworld.kr/";

/// 지금 VWorld 서버에 닿는지(한 번 묻는다, 최대 6초 — 화면 밖 실행에서 부른다).
#[must_use]
pub fn vworld_reachable() -> bool {
    if !crate::network::is_allowed() {
        return false;
    }
    let (curl, null) = if cfg!(target_os = "windows") { ("curl.exe", "NUL") } else { ("curl", "/dev/null") };
    let mut cmd = Command::new(curl);
    cmd.args(["-s", "-I", "-o", null, "-w", "%{http_code}", "--max-time", "6", CHECK_URL])
        .stdin(Stdio::null())
        .stderr(Stdio::null());
    crate::network::run_output(&mut cmd)
        .is_ok_and(|o| reached_from_http_code(&String::from_utf8_lossy(&o.stdout)))
}

/// curl `%{http_code}` — 대답이 없으면 000.
#[must_use]
pub fn reached_from_http_code(code: &str) -> bool {
    code.trim().parse::<u16>().is_ok_and(|c| c > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// QNetworkReply 의 HttpStatusCodeAttribute > 0 과 같다 — 오류 상태(404·500)도 서버에 닿은 것.
    #[test]
    fn any_http_status_means_reached() {
        for (code, reached) in [("200", true), ("404", true), ("500", true), ("000", false), ("", false)] {
            assert_eq!(reached_from_http_code(code), reached, "{code:?}");
        }
    }
}
