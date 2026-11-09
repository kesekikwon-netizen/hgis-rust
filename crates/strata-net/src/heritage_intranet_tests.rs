//! `heritage_intranet` 시험 1/2 — 주소·헤더·폼 해석.
//!
//! 원래 `heritage_intranet.rs` 안 `mod tests` 를 주제별로 나눴다.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

#[test]
fn heritage_intranet_scripts_never_navigate_page_and_redact_login_urls() {
    let search_js = HeritageIntranetFlow::search_script();
    assert!(search_js.contains("searchForm"));
    assert!(!search_js.contains("location.href="));
    let redacted = HeritageIntranetFlow::redact_request_line(
        "POST",
        "https://intranet.gis-heritage.go.kr/j_spring_security_check?pwd=secret",
    );
    assert_eq!(redacted, "POST <로그인 요청 — 내용을 남기지 않음>");
}

#[test]
fn test_eheritage_intranet_whitelist_rejection_blocks_external_hosts() {
    assert!(
        HeritageIntranetFlow::is_allowed_frame_url("https://e-heritage.go.kr/main.do").is_ok()
    );
    assert!(HeritageIntranetFlow::is_allowed_frame_url(
        "https://intranet.gis-heritage.go.kr/user/checkNet.do"
    )
    .is_ok());

    let blocked =
        HeritageIntranetFlow::is_allowed_frame_url("https://malicious.example.com/phish");
    assert!(blocked.is_err());
    assert_eq!(
        blocked.expect_err("external host must be blocked"),
        "허용되지 않은 외부 도메인 접근 차단: [malicious.example.com]"
    );
}

#[test]
fn heritage_download_urls_match_the_cpp_flow_paths() {
    // HeritageIntranetFlow.cpp:86-92, 203-213
    assert_eq!(
        HeritageIntranetFlow::PORTAL_URL,
        "https://intranet.gis-heritage.go.kr/"
    );
    assert_eq!(
        HeritageIntranetFlow::LOGIN_URL,
        "https://intranet.gis-heritage.go.kr/user/checkNet.do"
    );
    assert_eq!(
        HeritageIntranetFlow::download_page_url(),
        "https://intranet.gis-heritage.go.kr/user/data/heritageDownload.do?pageMenuId=DAT010010"
    );
    assert_eq!(
        HeritageIntranetFlow::download_list_url(),
        "https://intranet.gis-heritage.go.kr/user/data/heritageDownloadList.do"
    );
    assert_eq!(
        HeritageIntranetFlow::download_files_all_url(),
        "https://intranet.gis-heritage.go.kr/user/data/heritageDownload/downloadFilesAll.do"
    );
    // 「/」 두 개가 겹치지 않고, 이미 절대 주소면 그대로 둔다.
    assert_eq!(
        HeritageIntranetFlow::absolute_url("user/x.do"),
        "https://intranet.gis-heritage.go.kr/user/x.do"
    );
    assert_eq!(
        HeritageIntranetFlow::absolute_url("https://e-heritage.go.kr/a"),
        "https://e-heritage.go.kr/a"
    );
}

#[test]
fn heritage_request_headers_match_the_cpp_http_client() {
    // HeritageHttpClient.cpp:81-95 — X-Requested-With 가 없으면 서버가 404 로 돌려보낸다.
    let headers = HeritageIntranetFlow::request_headers();
    assert_eq!(headers.len(), 5);
    assert!(headers.contains(&(
        "Content-Type",
        "application/x-www-form-urlencoded; charset=UTF-8"
    )));
    assert!(headers.contains(&("X-Requested-With", "XMLHttpRequest")));
    assert!(headers.contains(&("User-Agent", HeritageIntranetFlow::USER_AGENT)));
    let referer = headers
        .iter()
        .find(|(name, _)| *name == "Referer")
        .map(|(_, value)| *value)
        .unwrap_or("");
    assert_eq!(
        referer,
        "https://intranet.gis-heritage.go.kr/ngis/common/main.do?httpYn=Y"
    );
    assert_eq!(HeritageIntranetFlow::list_seed_body(), "pageMenuId=DAT010010&pageIndex=1");
}

const LIST_HTML: &str = r#"<html><body>
  <form name="mapForm"><select name="bjdCd"><option value="1">리</option></select></form>
  <form id="searchForm" action="/user/data/heritageDownloadList.do" method="post">
    <input type="hidden" name="pageMenuId" value="DAT010010">
    <input type="hidden" name="pageIndex" value="1">
    <input type="submit" name="goSearch" value="검색">
    <input type="checkbox" name="onlyOwned" value="Y">
    <input type="checkbox" name="includeAll" value="Y" checked>
    <select id="codedetaCd0" name="codedetaCd">
      <option value="">시/도 선택</option>
      <option value="47,47">경상북도</option>
      <option value="42">강원특별자치도</option>
      <option value="46">전남광주통합특별시</option>
    </select>
    <select id="codeCdSg0" name="codeCdSg">
      <option value="">시/군/구 선택</option>
      <option value="47130">경주시</option>
    </select>
    <select id="mode" name="mode"><option value="S">S</option></select>
    <div>검색결과 <b>4,569</b>건</div>
  </form>
</body></html>"#;

#[test]
fn heritage_form_parser_reads_the_search_form_like_cpp() {
    // HeritageFormParser.cpp:98-174 — codedeta 를 품은 폼만 고른다(지도 패널 bjdCd 폼 제외)
    let form = parse_form(LIST_HTML, "codedeta");
    assert!(form.ok());
    assert_eq!(form.action, "/user/data/heritageDownloadList.do");
    assert_eq!(form.method, "POST");
    assert_eq!(form.sido_field, "codedetaCd");
    assert_eq!(form.sigungu_field, "codeCdSg");
    // submit 은 빠지고, 체크하지 않은 checkbox 도 빠진다(브라우저와 같은 규칙).
    assert!(form.field_value("goSearch").is_none());
    assert!(form.field_value("onlyOwned").is_none());
    assert_eq!(form.field_value("includeAll"), Some("Y"));
    assert_eq!(form.field_value("pageMenuId"), Some("DAT010010"));
    assert_eq!(form.sido_options.len(), 4);
    assert_eq!(
        match_option(&form.sido_options, "경상북도"),
        "47,47".to_owned()
    );
    assert_eq!(
        match_option(&form.sido_options, "경주시"),
        String::new(),
        "시/도 목록에 없는 이름은 비어야 한다"
    );
    assert_eq!(match_option(&form.sigungu_options, "경주시"), "47130");
    // 표기가 달라도 맞춘다(HeritageFormParser.cpp:39-58).
    assert_eq!(match_option(&form.sido_options, "강원도"), "42");
    assert_eq!(match_option(&form.sido_options, "전라남도"), "46");
    assert_eq!(match_option(&form.sido_options, "광주광역시"), "46");
    assert_eq!(result_count(LIST_HTML), 4569);
    assert_eq!(result_count("<p>결과 없음</p>"), -1);
    // 검색 폼이 없는 화면은 조용히 성공하지 않는다.
    let empty = parse_form("<html><body>로그인</body></html>", "codedeta");
    assert!(!empty.ok());
    assert!(!empty.warnings.is_empty());
}

#[test]
fn heritage_download_files_all_url_serializes_the_search_form() {
    // HeritageIntranetFlow.cpp:108-112 — 확인한 모양 그대로 우리가 주소를 만든다.
    let form = parse_form(LIST_HTML, "codedeta");
    let url = HeritageIntranetFlow::build_download_files_all_url(
        &form,
        "47,47",
        "47130",
        "S",
        "경상북도 경주시",
    );
    let (path, query) = url.split_once('?').unwrap_or(("", ""));
    assert_eq!(
        path,
        "https://intranet.gis-heritage.go.kr/user/data/heritageDownload/downloadFilesAll.do"
    );
    assert!(query.contains("pageMenuId=DAT010010"));
    assert!(query.contains("codedetaCd=47%2C47"), "{query}");
    assert!(query.contains("codeCdSg=47130"), "{query}");
    assert!(query.contains("codeCd=47"), "{query}");
    assert!(query.contains("tab=S"), "{query}");
    assert!(query.contains("searchParams="), "{query}");
    assert!(query.contains("includeAll=Y"), "{query}");
    // 「빈 값」으로도 주소가 깨지지 않는다.
    let bare = HeritageIntranetFlow::build_download_files_all_url(
        &HeritageForm::default(),
        "",
        "",
        "B",
        "",
    );
    assert!(bare.ends_with("downloadFilesAll.do?codeCd=&tab=B&mode=B&searchParams="));
    // 검색 몸통은 폼 칸을 그대로 되돌려 보낸다(HeritageHttpClient.cpp:53-74).
    let body = HeritageIntranetFlow::build_search_body(&form, "47,47", "47130");
    assert!(body.contains("codedetaCd=47%2C47"));
    assert!(body.contains("codeCdSg=47130"));
    assert!(body.contains("codeCd=47"));
}

#[test]
fn heritage_failure_messages_match_the_cpp_wording() {
    // HeritageHttpClient.cpp:104-198, KaHeritageBrowser.cpp:598-1523
    assert_eq!(
        messages::NO_ACCOUNT,
        "국가유산 인트라넷 계정이 없습니다. 더보기 → 국가유산 인트라넷 아이디·비밀번호에서 넣어 주세요."
    );
    assert_eq!(
        messages::LOGIN_FAILED,
        "로그인이 되지 않았습니다. 아이디·비밀번호를 확인해 주세요."
    );
    assert_eq!(messages::NO_SESSION, "로그인 세션이 없습니다. 먼저 로그인해야 합니다.");
    assert_eq!(
        messages::INTRANET_DISCONNECTED,
        "인트라넷 연결이 끊겼습니다. 다시 눌러 주세요."
    );
    assert_eq!(
        messages::CREDENTIALS_NEEDED,
        "받기를 시작하지 않았습니다. 국가유산 인트라넷 아이디·비밀번호를 저장한 뒤 다시 눌러 주세요."
    );
    assert_eq!(
        messages::region_not_applied(8302),
        "시·군 조건이 걸리지 않았습니다(검색결과가 8302건 그대로입니다). 전국 자료를 받지 않기 위해 멈춥니다."
    );
    assert_eq!(
        messages::city_not_found("경주시"),
        "시·군·구 목록에서 「경주시」을 찾지 못했습니다. 시·도 전체를 받지 않기 위해 멈춥니다."
    );
    assert_eq!(
        messages::sido_not_found("경상북도"),
        "시·도 목록에서 「경상북도」을 찾지 못했습니다."
    );
    assert_eq!(
        messages::download_not_started("지정유산"),
        "「지정유산」에서 내려받기를 눌렀지만 파일이 오지 않았습니다."
    );
    assert_eq!(
        messages::list_fetch_failed(404, "Not Found"),
        "자료 목록을 받지 못했습니다(HTTP 404): Not Found"
    );
    assert_eq!(
        messages::form_not_parsed(&["'codedeta' select 를 찾지 못했습니다.".to_owned()], 900),
        "자료 목록에서 검색 폼을 읽지 못했습니다. 'codedeta' select 를 찾지 못했습니다. (받은 크기 900바이트)"
    );
}
