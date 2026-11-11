//! VWorld 누리집 로그인 — 연속지적도 ZIP 은 로그인한 세션에서만 받을 수 있다.
//!
//! C++ `CadastralPortal.cpp:264-292`:
//! 1. `GET` 자료 화면(쿠키를 받는다. 본문은 쓰지 않는다)
//! 2. `POST /v4po_usrlogin_a004.do` — 폼 `usrIdeE`·`usrPwdE` 는 아이디·비밀번호(UTF-8)의 Base64,
//!    `nextUrl` 은 자료 화면 경로
//! 3. 응답 JSON 의 `resultMap.result` 가 `"success"` 면 성공
//!
//! 아이디·비밀번호는 VWorld 누리집 계정이다(API 키가 아니다). 값은 어디에도 기록하지 않는다.

use base64::Engine;

use crate::portal_session::PortalSession;

/// 연속지적도 자료 화면(Referer 이자 쿠키를 받는 주소).
pub const CADASTRAL_DATASET_URL: &str = "https://www.vworld.kr/dtmk/dtmk_ntads_s002.do?dsId=30563";
const LOGIN_URL: &str = "https://www.vworld.kr/v4po_usrlogin_a004.do";
const NEXT_URL: &str = "/dtmk/dtmk_ntads_s002.do?dsId=30563";

/// 로그인이 안 된 까닭. `account_rejected` 면 아이디·비밀번호를 다시 넣게 한다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginFailure {
    pub message: String,
    pub account_rejected: bool,
}

/// `<br>` 는 공백으로, 나머지 태그는 지운다(C++ `:279-290`).
fn strip_tags(text: &str) -> String {
    let spaced = text.replace("<br>", " ").replace("<br/>", " ").replace("<br />", " ");
    let mut out = String::new();
    let mut in_tag = false;
    for ch in spaced.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// 로그인 응답을 판정한다. C++ `CadastralPortal.cpp:276-292` 의 세 갈래와 같은 문구.
pub fn classify_login_response(body: &str) -> Result<(), LoginFailure> {
    let json: serde_json::Value = serde_json::from_str(body).unwrap_or(serde_json::Value::Null);
    let map = json.get("resultMap");
    let result = map
        .and_then(|m| m.get("result"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    if result == "success" {
        return Ok(());
    }
    let (mut message, account_rejected) = match result {
        "" => (
            "VWorld 로그인 응답을 읽지 못했습니다. 누리집의 로그인 방식이 바뀌었을 수 있습니다. 잠시 뒤 다시 시도하고, 계속되면 개발자에게 알려 주세요.".to_owned(),
            false,
        ),
        "error" => (
            "VWorld 로그인에 실패했습니다. 아이디·비밀번호는 VWorld 누리집(vworld.kr) 로그인 계정입니다(API 키가 아님).".to_owned(),
            true,
        ),
        _ => (
            "VWorld 누리집이 로그인 전에 추가 절차를 요구합니다. 인터넷 브라우저로 vworld.kr 에 로그인해 안내를 마친 뒤 다시 시도하세요.".to_owned(),
            true,
        ),
    };
    let notice = map
        .and_then(|m| m.get("msg"))
        .and_then(serde_json::Value::as_str)
        .map(strip_tags)
        .unwrap_or_default();
    if !notice.is_empty() {
        message.push_str("\n\nVWorld 안내: ");
        message.push_str(&notice);
    }
    Err(LoginFailure {
        message,
        account_rejected,
    })
}

/// 세션을 VWorld 누리집에 로그인시킨다.
pub fn vworld_portal_login(session: &PortalSession, username: &str, password: &str) -> Result<(), LoginFailure> {
    let network = || LoginFailure {
        message: "VWorld 통신에 실패했습니다. 인터넷 연결과 계정 설정을 확인하세요.".to_owned(),
        account_rejected: false,
    };
    // 1) 쿠키 받기 — 본문은 쓰지 않는다.
    session.get_text(CADASTRAL_DATASET_URL).map_err(|_| network())?;
    // 2) 로그인
    let b64 = base64::engine::general_purpose::STANDARD;
    let id = b64.encode(username.as_bytes());
    let pw = b64.encode(password.as_bytes());
    let body = session
        .post_form(LOGIN_URL, &[("usrIdeE", &id), ("usrPwdE", &pw), ("nextUrl", NEXT_URL)])
        .map_err(|_| network())?;
    classify_login_response(&body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_error_and_extra_step_are_told_apart_like_cpp() {
        assert_eq!(classify_login_response(r#"{"resultMap":{"result":"success"}}"#), Ok(()));

        let wrong = classify_login_response(r#"{"resultMap":{"result":"error","msg":"비밀번호가<br>틀렸습니다. <b>5회</b> 남음"}}"#)
            .expect_err("실패");
        assert!(wrong.account_rejected);
        assert!(wrong.message.starts_with("VWorld 로그인에 실패했습니다."));
        assert!(wrong.message.ends_with("VWorld 안내: 비밀번호가 틀렸습니다. 5회 남음"));

        let extra = classify_login_response(r#"{"resultMap":{"result":"pwdChange"}}"#).expect_err("추가 절차");
        assert!(extra.account_rejected && extra.message.contains("추가 절차를 요구합니다"));

        // JSON 이 아닌 응답(누리집이 바뀌었을 때)은 계정 탓으로 돌리지 않는다.
        let html = classify_login_response("<html>로그인</html>").expect_err("읽지 못함");
        assert!(!html.account_rejected && html.message.starts_with("VWorld 로그인 응답을 읽지 못했습니다."));
    }
}
