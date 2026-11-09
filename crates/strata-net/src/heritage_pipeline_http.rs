//! 인트라넷 검색·내려받기 실제 단계 (C++ `KaHeritageBrowser.cpp`).
//!
//! `heritage_intranet` 본문을 300줄 이하로 유지하기 위해 분리했다. 공개 경로는 그대로다.
use std::path::Path;
use super::heritage_http::run_curl;
use super::heritage_pipeline::looks_like_login_page;

use super::*;

/// ①~③ 다운로드 화면 → 목록 → 폼 → 시·군 검색. 실제 HTTP 로 되는 데까지 한다.
///
/// 로그인 세션 쿠키(`cookie_jar`)가 없거나 만료면 사이트가 로그인 화면을 돌려주고,
/// 그때는 C++ 과 같은 문구로 멈춘다(`HeritageHttpClient.cpp:102-155`).
pub fn heritage_http_search(
    survey_dir: Option<&Path>,
    have_account: bool,
    sido: &str,
    city: &str,
    cookie_jar: Option<&Path>,
) -> Result<HeritagePipelineOutcome, String> {
    heritage_preflight(have_account, survey_dir, sido, city)?;
    let mut out = HeritagePipelineOutcome::default();

    // 껍데기 화면 + 목록 조각: 폼(#searchForm)은 목록 조각 안에 있다.
    let page = run_curl(&HeritageIntranetFlow::download_page_url(), None, cookie_jar)
        .map_err(|_| messages::INTRANET_DISCONNECTED.to_owned())?;
    out.request_log.push(HeritageIntranetFlow::redact_request_line(
        "GET",
        &HeritageIntranetFlow::download_page_url(),
    ));
    if page.status >= 400 {
        return Err(messages::page_fetch_failed(&format!("HTTP {}", page.status)));
    }
    if looks_like_login_page(&page.body_text()) {
        return Err(messages::NO_SESSION.to_owned());
    }

    let list_url = HeritageIntranetFlow::download_list_url();
    let seed = HeritageIntranetFlow::list_seed_body();
    let list = run_curl(&list_url, Some(&seed), cookie_jar).map_err(|why| {
        if why.is_empty() {
            messages::INTRANET_DISCONNECTED.to_owned()
        } else {
            messages::list_fetch_failed(page.status, &why)
        }
    })?;
    out.request_log
        .push(HeritageIntranetFlow::redact_request_line("POST", &list_url));
    if list.status >= 400 {
        return Err(messages::list_fetch_failed(
            list.status,
            "서버가 오류 화면을 보냈습니다",
        ));
    }
    let html = list.body_text();
    if looks_like_login_page(&html) {
        return Err(messages::NO_SESSION.to_owned());
    }
    let form = parse_form(&html, "codedeta");
    if !form.ok() {
        return Err(messages::form_not_parsed(&form.warnings, list.body.len()));
    }
    out.total_before_search = result_count(&html);

    // ② 시·도·시·군·구 값을 폼에서 찾는다. 지어내지 않는다.
    let sido_value = match_option(&form.sido_options, sido);
    if sido_value.is_empty() {
        return Err(messages::sido_not_found(sido));
    }
    let city_value = match_option(&form.sigungu_options, city);
    if city_value.is_empty() {
        return Err(messages::city_not_found(city));
    }
    let body = HeritageIntranetFlow::build_search_body(&form, &sido_value, &city_value);
    let target = if form.action.trim().is_empty() {
        list_url.clone()
    } else {
        HeritageIntranetFlow::absolute_url(form.action.trim())
    };
    out.request_log.push(HeritageIntranetFlow::redact_request_line(
        if form.method == "GET" { "GET" } else { "POST" },
        &target,
    ));
    let reply = run_curl(&target, Some(&body), cookie_jar)
        .map_err(|why| messages::search_failed(&why))?;
    if reply.status >= 400 {
        return Err(messages::search_failed(&format!("HTTP {}", reply.status)));
    }
    let reply_html = reply.body_text();
    if looks_like_login_page(&reply_html) {
        return Err(messages::NO_SESSION.to_owned());
    }
    let count = result_count(&reply_html);
    // 시/군 조건이 걸리지 않았으면 전국 자료를 받는다. 그건 하지 않는다.
    if count > 0 && out.total_before_search > 0 && count == out.total_before_search {
        return Err(messages::region_not_applied(count));
    }
    out.matched_sido = sido.trim().to_owned();
    out.matched_city = city.trim().to_owned();
    out.search_count = count;
    out.form_found = true;
    Ok(out.for_form(form, sido_value, city_value))
}

impl HeritagePipelineOutcome {
    fn for_form(
        mut self,
        form: HeritageForm,
        sido_value: String,
        city_value: String,
    ) -> Self {
        self.form = Some(form);
        self.sido_value = sido_value;
        self.city_value = city_value;
        self
    }
}

/// ④ 고른 자료 종류마다 받기 파일 주소를 만들어 ZIP 을 저장한다
/// (`KaHeritageBrowser.cpp:1219-1253`, `:1293-1398`).
pub fn heritage_http_download(
    outcome: &mut HeritagePipelineOutcome,
    survey_dir: &Path,
    ciudades: &[(String, String)],
    cookie_jar: Option<&Path>,
) -> Result<(), String> {
    let Some(form) = outcome.form.clone() else {
        return Err(messages::SEARCH_FORM_MISSING.to_owned());
    };
    let root = heritage_download_root(survey_dir, &outcome.matched_sido, &outcome.matched_city);
    std::fs::create_dir_all(&root).map_err(|_| messages::CANNOT_CREATE_ROOT.to_owned())?;
    for (label, tab) in ciudades {
        let url = HeritageIntranetFlow::build_download_files_all_url(
            &form,
            &outcome.sido_value,
            &outcome.city_value,
            tab,
            &format!("{} {}", outcome.matched_sido, outcome.matched_city),
        );
        outcome.request_log.push(HeritageIntranetFlow::redact_request_line("GET", &url));
        match run_curl(&url, None, cookie_jar) {
            Ok(response) if response.is_zip() && !response.body.is_empty() => {
                let name = format!("{label}.zip");
                let path = root.join(&name);
                match std::fs::write(&path, &response.body) {
                    Ok(()) => outcome.files.push(HeritageFileResult {
                        label: label.clone(),
                        url,
                        bytes: response.body.len(),
                        saved_path: Some(path),
                        error: None,
                    }),
                    Err(err) => outcome.files.push(HeritageFileResult {
                        label: label.clone(),
                        url,
                        bytes: 0,
                        saved_path: None,
                        error: Some(format!("저장 폴더를 만들지 못했습니다. 폴더 권한을 확인하세요. ({err})")),
                    }),
                }
            }
            Ok(response) => outcome.files.push(HeritageFileResult {
                label: label.clone(),
                url,
                bytes: response.body.len(),
                saved_path: None,
                // 파일 대신 화면이 왔다 = 로그인/권한 문제이거나 그 자료가 없다.
                error: Some(messages::download_not_started(label)),
            }),
            Err(why) => outcome.files.push(HeritageFileResult {
                label: label.clone(),
                url,
                bytes: 0,
                saved_path: None,
                error: Some(messages::search_failed(&why)),
            }),
        }
    }
    Ok(())
}
