//! curl 로 인트라넷 화면을 받는 HTTP 클라이언트 (C++ `HeritageHttpClient.cpp`).
//!
//! `heritage_intranet` 본문을 300줄 이하로 유지하기 위해 분리했다. 공개 경로는 그대로다.
use std::path::Path;
use std::process::Command;

use super::*;

/// HTTP 응답 한 건. 상태 코드와 몸통만 남긴다.
#[derive(Debug, Clone)]
pub struct HeritageHttpResponse {
    pub status: u16,
    pub body: Vec<u8>,
}

impl HeritageHttpResponse {
    #[must_use]
    pub fn body_text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }

    /// ZIP 파일인가. 받은 것이 웹페이지(로그인 화면)면 파일이 아니다.
    #[must_use]
    pub fn is_zip(&self) -> bool {
        self.body.starts_with(b"PK\x03\x04")
    }
}

fn curl_binary() -> &'static str {
    if cfg!(target_os = "windows") {
        "curl.exe"
    } else {
        "curl"
    }
}

/// `address_search.rs:758` 의 `http_get_bytes` 와 같은 방식(curl.exe)을 쓰되,
/// 상태 코드와 쿠키 항아리를 함께 다룬다. 새 크레이트를 넣지 않는다.
pub(crate) fn run_curl(url: &str, body: Option<&str>, cookie_jar: Option<&Path>) -> Result<HeritageHttpResponse, String> {
    if !crate::network::is_allowed() {
        return Err("연결 실패(상태 0)".to_owned());
    }
    let method = if body.is_some() { "POST" } else { "GET" };
    let mut cmd = Command::new(curl_binary());
    cmd.args([
        "-sS",
        "--max-time",
        &HeritageIntranetFlow::TIMEOUT_SECS.to_string(),
        "-X",
        method,
        "-A",
        HeritageIntranetFlow::USER_AGENT,
        "-w",
        "\n__KA_HTTP_STATUS__:%{http_code}",
    ]);
    for (name, value) in HeritageIntranetFlow::request_headers() {
        cmd.args(["-H", &format!("{name}: {value}")]);
    }
    if let Some(jar) = cookie_jar {
        let jar = jar.to_string_lossy().into_owned();
        cmd.args(["-b", &jar, "-c", &jar]);
    }
    if let Some(b) = body {
        cmd.args(["--data-binary", b]);
    }
    cmd.arg(url);
    strata_types::process::no_window(&mut cmd);
    let output = crate::network::run_output(&mut cmd)
        .map_err(|e| format!("curl 실행 실패: {e}"))?;
    let raw = String::from_utf8_lossy(&output.stdout).into_owned();
    let (payload, status) = raw
        .rsplit_once("__KA_HTTP_STATUS__:")
        .ok_or_else(|| "curl 응답에서 상태 코드를 읽지 못했습니다.".to_owned())?;
    let status: u16 = status.trim().parse().unwrap_or(0);
    if !output.status.success() || status == 0 {
        let why = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(if why.is_empty() {
            format!("연결 실패(상태 {status})")
        } else {
            why
        });
    }
    Ok(HeritageHttpResponse {
        status,
        body: payload.trim_end_matches('\n').as_bytes().to_vec(),
    })
}

/// ① 다운로드 화면(껍데기)과 목록 조각을 받아 검색 폼을 읽는다
/// (`HeritageHttpClient.cpp:102-155`).
pub fn fetch_download_form(cookie_jar: Option<&Path>) -> Result<(HeritageHttpResponse, HeritageForm), String> {
    let page = run_curl(&HeritageIntranetFlow::download_page_url(), None, cookie_jar)
        .map_err(|why| messages::page_fetch_failed(&why))?;
    if page.status >= 400 {
        return Err(messages::page_fetch_failed(&format!(
            "HTTP {}",
            page.status
        )));
    }
    let list_url = HeritageIntranetFlow::download_list_url();
    let seed = HeritageIntranetFlow::list_seed_body();
    let list = run_curl(&list_url, Some(&seed), cookie_jar).map_err(|why| {
        messages::list_fetch_failed(page.status, &why)
    })?;
    if list.status >= 400 {
        return Err(messages::list_fetch_failed(
            list.status,
            if list.body.is_empty() {
                "응답 본문 없음"
            } else {
                "서버가 오류 화면을 보냈습니다"
            },
        ));
    }
    let html = list.body_text();
    let form = parse_form(&html, "codedeta");
    if !form.ok() {
        return Err(messages::form_not_parsed(&form.warnings, list.body.len()));
    }
    Ok((list, form))
}

/// ② 시·군으로 좁혀 검색한다. 건수가 줄지 않으면 실패로 본다(전국을 받지 않는다)
/// (`HeritageHttpClient.cpp:157-201`).
pub fn search_region(
    page_url: &str,
    form: &HeritageForm,
    sido: &str,
    city: &str,
    total_before_search: i32,
    cookie_jar: Option<&Path>,
) -> Result<(i32, String), String> {
    if !form.ok() {
        return Err(messages::SEARCH_FORM_MISSING.to_owned());
    }
    let sido_value = match_option(&form.sido_options, sido);
    if sido_value.is_empty() {
        return Err(messages::sido_not_found(sido));
    }
    let city_value = match_option(&form.sigungu_options, city);
    let body = HeritageIntranetFlow::build_search_body(form, &sido_value, &city_value);
    let target = if form.action.trim().is_empty() {
        page_url.to_owned()
    } else {
        HeritageIntranetFlow::absolute_url(&form.action)
    };
    let reply = run_curl(&target, Some(&body), cookie_jar)
        .map_err(|why| messages::search_failed(&why))?;
    if reply.status >= 400 {
        return Err(messages::search_failed(&format!("HTTP {}", reply.status)));
    }
    let html = reply.body_text();
    let count = result_count(&html);
    // 시/군 조건이 걸리지 않았는데 받으면 전국 자료를 내려받는다. 그건 하지 않는다.
    if count > 0 && total_before_search > 0 && count == total_before_search {
        return Err(messages::region_not_applied(count));
    }
    if city_value.is_empty() {
        // 시/군을 못 골랐으면 시/도 전체다. 조용히 넘어가지 않는다.
        return Err(messages::city_not_found(city));
    }
    Ok((count, html))
}
