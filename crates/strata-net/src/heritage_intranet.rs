//! 국가유산 공간정보 인트라넷 창의 단계·주소·폼 파서·HTTP 클라이언트.
//!
//! C++ 정본 그대로 옮겼다.
//! - 단계 이름·주소: `HeritageIntranetFlow.cpp:86-110`, `:203-213`
//! - 요청 헤더: `HeritageHttpClient.cpp:17-19`, `:81-100`
//! - 폼 해석: `HeritageFormParser.cpp:9-212`
//! - 전체다운로드 주소: `HeritageIntranetFlow.cpp:573-619` (요청 기록에서 확인한 모양)
//! - 실패 문구: `HeritageHttpClient.cpp:104-198`, `KaHeritageBrowser.cpp:598-1523`

use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeritageStage {
    Idle,
    Login,
    DismissTutorial,
    OpenDownloadPage,
    AgreeTerms,
    SelectDataset,
    SelectRegion,
    Search,
    CollectResults,
    Download,
    Done,
    Failed,
}

impl HeritageStage {
    pub const fn stage_name_ko(self) -> &'static str {
        match self {
            Self::Idle => "대기",
            Self::Login => "로그인",
            Self::DismissTutorial => "튜토리얼 닫기",
            Self::OpenDownloadPage => "국가유산자료 다운로드 열기",
            Self::AgreeTerms => "서약서 동의",
            Self::SelectDataset => "자료 종류 선택",
            Self::SelectRegion => "시·군 선택",
            Self::Search => "검색",
            Self::CollectResults => "결과 모으기",
            Self::Download => "내려받기",
            Self::Done => "완료",
            Self::Failed => "실패",
        }
    }

    /// 「유산」 창의 단계 번호(`heritage_stage_idx`)로 바꾼다.
    #[must_use]
    pub const fn ui_index(self) -> usize {
        match self {
            Self::Idle => 0,
            Self::Login => 1,
            Self::DismissTutorial => 2,
            Self::OpenDownloadPage => 3,
            Self::AgreeTerms => 4,
            Self::SelectDataset => 5,
            Self::SelectRegion => 6,
            Self::Search => 7,
            Self::CollectResults => 8,
            Self::Download => 9,
            Self::Done => 10,
            Self::Failed => 11,
        }
    }
}

/// 인트라넷이 실패를 알리는 문구. C++ 문장을 그대로 쓴다.
pub mod messages {
    /// `KaHeritageBrowser.cpp:598`
    pub const NO_ACCOUNT: &str = "국가유산 인트라넷 계정이 없습니다. 더보기 → 국가유산 \
                                  인트라넷 아이디·비밀번호에서 넣어 주세요.";
    /// `KaHeritageBrowser.cpp:930`
    pub const LOGIN_FAILED: &str = "로그인이 되지 않았습니다. 아이디·비밀번호를 확인해 주세요.";
    /// `HeritageHttpClient.cpp:104`
    pub const NO_SESSION: &str = "로그인 세션이 없습니다. 먼저 로그인해야 합니다.";
    /// `KaHeritageBrowser.cpp:1523`
    pub const INTRANET_DISCONNECTED: &str = "인트라넷 연결이 끊겼습니다. 다시 눌러 주세요.";
    /// `KaHeritageBrowser.cpp:1017`
    pub const LIST_REQUEST_FAILED: &str = "자료 목록 요청에 실패했습니다.";
    /// `HeritageHttpClient.cpp:159`
    pub const SEARCH_FORM_MISSING: &str = "검색 폼을 아직 읽지 못했습니다.";
    /// `KaHeritageBrowser.cpp:1305`
    pub const NO_DOWNLOAD_ROOT: &str = "저장할 폴더가 없어 내려받기를 취소했습니다.";
    /// `KaHeritageBrowser.cpp:1312`
    pub const CANNOT_CREATE_ROOT: &str = "저장 폴더를 만들지 못했습니다. 폴더 권한을 확인하세요.";
    /// `MainWindow.cpp:3181-3182`
    pub const CREDENTIALS_NEEDED: &str = "받기를 시작하지 않았습니다. 국가유산 인트라넷 \
                                          아이디·비밀번호를 저장한 뒤 다시 눌러 주세요.";

    /// `HeritageHttpClient.cpp:115`
    #[must_use]
    pub fn page_fetch_failed(why: &str) -> String {
        format!("다운로드 화면을 받지 못했습니다: {why}")
    }

    /// `HeritageHttpClient.cpp:132-134`
    #[must_use]
    pub fn list_fetch_failed(http: u16, why: &str) -> String {
        format!("자료 목록을 받지 못했습니다(HTTP {http}): {why}")
    }

    /// `HeritageHttpClient.cpp:144-146`
    #[must_use]
    pub fn form_not_parsed(warnings: &[String], bytes: usize) -> String {
        format!(
            "자료 목록에서 검색 폼을 읽지 못했습니다. {} (받은 크기 {bytes}바이트)",
            warnings.join(" ")
        )
    }

    /// `HeritageHttpClient.cpp:164`
    #[must_use]
    pub fn sido_not_found(sido: &str) -> String {
        format!("시·도 목록에서 「{sido}」을 찾지 못했습니다.")
    }

    /// `HeritageHttpClient.cpp:180`
    #[must_use]
    pub fn search_failed(why: &str) -> String {
        format!("검색 요청이 실패했습니다: {why}")
    }

    /// `HeritageHttpClient.cpp:188-190`
    #[must_use]
    pub fn region_not_applied(count: i32) -> String {
        format!("시·군 조건이 걸리지 않았습니다(검색결과가 {count}건 그대로입니다). \
                 전국 자료를 받지 않기 위해 멈춥니다.")
    }

    /// `HeritageHttpClient.cpp:195-197`
    #[must_use]
    pub fn city_not_found(city: &str) -> String {
        format!("시·군·구 목록에서 「{city}」을 찾지 못했습니다. 시·도 전체를 받지 않기 위해 멈춥니다.")
    }

    /// `KaHeritageBrowser.cpp:1160`
    #[must_use]
    pub fn download_not_started(layer: &str) -> String {
        format!("「{layer}」에서 내려받기를 눌렀지만 파일이 오지 않았습니다.")
    }

    /// `KaHeritageBrowser.cpp:963`
    #[must_use]
    pub fn page_open_failed(why: &str) -> String {
        format!("국가유산 다운로드 화면 열기에 실패했습니다: {why}")
    }
}

/// 다운로드 화면의 검색 폼 (`HeritageFormParser.h:15-26`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HeritageForm {
    pub action: String,
    pub method: String,
    /// hidden 을 포함한 칸 이름 → 현재 값. 서버가 기대하는 값을 그대로 되돌려 보낸다.
    pub fields: Vec<(String, String)>,
    pub sido_field: String,
    pub sigungu_field: String,
    pub sido_options: Vec<(String, String)>,
    pub sigungu_options: Vec<(String, String)>,
    pub warnings: Vec<String>,
}

impl HeritageForm {
    #[must_use]
    pub fn ok(&self) -> bool {
        !self.sido_field.is_empty() && !self.fields.is_empty()
    }

    fn field_value(&self, name: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }
}

fn ci_find(hay: &str, needle_lower: &str, from: usize) -> Option<usize> {
    let h = hay.as_bytes();
    let n = needle_lower.as_bytes();
    if n.is_empty() || h.len() < n.len() {
        return None;
    }
    let mut i = from;
    while i + n.len() <= h.len() {
        if h[i..i + n.len()].eq_ignore_ascii_case(n) {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// `HeritageFormParser.cpp:9-20` — name="..." 또는 name='...' 또는 name=값
fn attr(tag: &str, name: &str) -> Option<String> {
    let mut at = 0;
    while let Some(pos) = ci_find(tag, name, at) {
        at = pos + name.len();
        let before_ok = tag[..pos]
            .chars()
            .next_back()
            .is_none_or(|c| !(c.is_alphanumeric() || c == '_' || c == '-'));
        if !before_ok {
            continue;
        }
        let rest = tag[at..].trim_start();
        let Some(rest) = rest.strip_prefix('=') else {
            continue;
        };
        let rest = rest.trim_start();
        if let Some(body) = rest.strip_prefix('"') {
            return Some(body.split('"').next().unwrap_or("").to_owned());
        }
        if let Some(body) = rest.strip_prefix('\'') {
            return Some(body.split('\'').next().unwrap_or("").to_owned());
        }
        let bare: String = rest
            .chars()
            .take_while(|c| !c.is_whitespace() && *c != '>')
            .collect();
        return Some(bare);
    }
    None
}

/// `HeritageFormParser.cpp:22-28`
fn strip_tags(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut depth = false;
    for c in text.chars() {
        match c {
            '<' => depth = true,
            '>' => depth = false,
            _ if !depth => out.push(c),
            _ => {}
        }
    }
    out.replace("&nbsp;", " ").replace("&amp;", "&").trim().to_owned()
}

/// `HeritageFormParser.cpp:31-35`
fn squash(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect()
}

/// `HeritageFormParser.cpp:39-58` — 「강원도」 ↔ 「강원특별자치도」 표기 차이를 맞춘다.
fn sido_core(raw: &str) -> String {
    let mut s = squash(raw);
    for tail in ["특별자치도", "특별자치시", "특별시", "광역시"] {
        if let Some(head) = s.strip_suffix(tail) {
            s = head.to_owned();
            break;
        }
    }
    if s.chars().count() > 2 && s.ends_with('도') {
        s.pop();
    }
    match s.as_str() {
        "전라북" => "전북".to_owned(),
        "전라남" => "전남".to_owned(),
        "경상북" => "경북".to_owned(),
        "경상남" => "경남".to_owned(),
        "충청북" => "충북".to_owned(),
        "충청남" => "충남".to_owned(),
        _ => s,
    }
}

struct SelectBlock {
    name: String,
    id: String,
    options: Vec<(String, String)>,
    at: usize,
}

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

fn percent_encode(input: &str) -> String {
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

#[derive(Debug, Clone, Default)]
pub struct HeritageIntranetFlow;

impl HeritageIntranetFlow {
    pub const PORTAL_URL: &'static str = "https://intranet.gis-heritage.go.kr/";
    pub const LOGIN_URL: &'static str = "https://intranet.gis-heritage.go.kr/user/checkNet.do";
    /// `HeritageIntranetFlow.cpp:203-205`
    pub const DOWNLOAD_PAGE_PATH: &'static str =
        "/user/data/heritageDownload.do?pageMenuId=DAT010010";
    /// `HeritageIntranetFlow.cpp:207-209`
    pub const DOWNLOAD_LIST_PATH: &'static str = "/user/data/heritageDownloadList.do";
    /// `HeritageIntranetFlow.cpp:211-213`
    pub const DOWNLOAD_FILES_ALL_PATH: &'static str =
        "/user/data/heritageDownload/downloadFilesAll.do";
    /// `HeritageHttpClient.cpp:17-19`
    pub const USER_AGENT: &'static str =
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ka-hgis/0.3";
    /// `HeritageHttpClient.cpp:15` — kTimeoutMs
    pub const TIMEOUT_SECS: u32 = 30;

    #[must_use]
    pub fn js_string(val: &str) -> String {
        serde_json::to_string(val).unwrap_or_else(|_| "\"\"".to_string())
    }

    /// 포털 기준 절대 주소. C++ `QUrl::resolved` 와 같은 결과.
    #[must_use]
    pub fn absolute_url(path: &str) -> String {
        if path.starts_with("https://") || path.starts_with("http://") {
            return path.to_owned();
        }
        let base = Self::PORTAL_URL.trim_end_matches('/');
        if path.starts_with('/') {
            format!("{base}{path}")
        } else {
            format!("{base}/{path}")
        }
    }

    /// `HeritageHttpClient.cpp:109-110` — ① 껍데기 화면
    #[must_use]
    pub fn download_page_url() -> String {
        Self::absolute_url(Self::DOWNLOAD_PAGE_PATH)
    }

    /// `HeritageHttpClient.cpp:119-120` — ② 목록 조각(검색 폼이 여기 있다)
    #[must_use]
    pub fn download_list_url() -> String {
        Self::absolute_url(Self::DOWNLOAD_LIST_PATH)
    }

    /// `HeritageIntranetFlow.cpp:211-213` — 파일 받기 주소
    #[must_use]
    pub fn download_files_all_url() -> String {
        Self::absolute_url(Self::DOWNLOAD_FILES_ALL_PATH)
    }

    /// 사이트 JS 가 늘 함께 보내는 씨앗 값 (`HeritageHttpClient.cpp:121-124`).
    #[must_use]
    pub fn list_seed_body() -> String {
        "pageMenuId=DAT010010&pageIndex=1".to_owned()
    }

    /// `HeritageHttpClient.cpp:81-100` 의 헤더 그대로.
    /// `X-Requested-With` 가 없으면 서버가 404 로 돌려보낸다.
    #[must_use]
    pub fn request_headers() -> [(&'static str, &'static str); 5] {
        [
            (
                "Content-Type",
                "application/x-www-form-urlencoded; charset=UTF-8",
            ),
            ("User-Agent", Self::USER_AGENT),
            ("Accept", "text/html,application/xhtml+xml,*/*"),
            ("X-Requested-With", "XMLHttpRequest"),
            (
                "Referer",
                "https://intranet.gis-heritage.go.kr/ngis/common/main.do?httpYn=Y",
            ),
        ]
    }

    /// `HeritageHttpClient::buildBody` (`HeritageHttpClient.cpp:53-74`)
    /// 폼의 칸을 그대로 되돌려 보내고 시/도·시/군/구만 바꾼다.
    #[must_use]
    pub fn build_search_body(
        form: &HeritageForm,
        sido_value: &str,
        sigungu_value: &str,
    ) -> String {
        let mut parts: Vec<String> = Vec::new();
        for (name, value) in &form.fields {
            let mut v = value.as_str();
            if !form.sido_field.is_empty() && *name == form.sido_field {
                v = sido_value;
            }
            if !form.sigungu_field.is_empty() && *name == form.sigungu_field {
                v = sigungu_value;
            }
            parts.push(format!(
                "{}={}",
                percent_encode(name),
                percent_encode(v)
            ));
        }
        // 폼에 없던 칸이면 새로 넣는다(select 가 form 밖에 있는 화면 대비).
        for (name, value) in [
            (form.sido_field.as_str(), sido_value),
            (form.sigungu_field.as_str(), sigungu_value),
        ] {
            if name.is_empty() || form.field_value(name).is_some() {
                continue;
            }
            parts.push(format!(
                "{}={}",
                percent_encode(name),
                percent_encode(value)
            ));
        }
        // 사이트 JS 와 같은 규칙: codedetaCd 값이 "A,B" 꼴이면 뒤쪽을 codeCd 로 함께 보낸다.
        let code_cd = sido_value.split(',').nth(1).unwrap_or("");
        parts.push(format!("codeCd={}", percent_encode(code_cd)));
        parts.join("&")
    }

    /// 사이트의 전체다운로드 주소를 **우리가 만든다** (`HeritageIntranetFlow.cpp:108-112`,
    /// `:573-619`). 성공한 요청에서 확인한 모양 그대로다:
    /// `/user/data/heritageDownload/downloadFilesAll.do?<searchForm 직렬화>&codeCd=..&tab=..`
    #[must_use]
    pub fn build_download_files_all_url(
        form: &HeritageForm,
        sido_value: &str,
        sigungu_value: &str,
        tab_code: &str,
        region_label: &str,
    ) -> String {
        let mut parts: Vec<String> = Vec::new();
        for (name, value) in &form.fields {
            let mut v = value.as_str();
            if !form.sido_field.is_empty() && *name == form.sido_field {
                v = sido_value;
            }
            if !form.sigungu_field.is_empty() && *name == form.sigungu_field {
                v = sigungu_value;
            }
            parts.push(format!("{}={}", percent_encode(name), percent_encode(v)));
        }
        let code_cd = sido_value.split(',').nth(1).unwrap_or("");
        parts.push(format!("codeCd={}", percent_encode(code_cd)));
        let mode = form.field_value("mode").filter(|m| !m.is_empty());
        let tab = mode.unwrap_or(tab_code);
        parts.push(format!("tab={}", percent_encode(tab)));
        if mode.is_none() {
            parts.push(format!("mode={}", percent_encode(tab)));
        }
        // searchParams 는 화면에 보여 줄 요약. 날것으로 만든다(인코딩하지 않는다).
        let label = if region_label.is_empty() {
            String::new()
        } else {
            format!("지역:{region_label} / ")
        };
        parts.push(format!(
            "searchParams={}",
            if label.is_empty() {
                String::new()
            } else {
                percent_encode(&label)
            }
        ));
        format!("{}?{}", Self::download_files_all_url(), parts.join("&"))
    }

    pub fn login_script(user_id: &str, password: &str) -> String {
        format!(
            "(function(){{var id=document.getElementById('userId');\
             var pw=document.getElementById('pwd');\
             if(!id||!pw)return 'no-form';\
             id.value={};pw.value={};\
             if(typeof goLogin!=='function')return 'no-goLogin';\
             goLogin();return 'submitted';}})()",
            Self::js_string(user_id),
            Self::js_string(password)
        )
    }

    pub fn open_download_page_script() -> &'static str {
        "(function(){if(typeof showAgreePopup==='function'){showAgreePopup();return 'opening';}return 'not-found';})()"
    }

    pub fn search_script() -> String {
        String::from(
            "(function(){var scope=document.getElementById('searchForm');\
             if(!scope)return 'not-found';\
             var btns=scope.querySelectorAll('button,input[type=button],input[type=submit],a');\
             for(var i=0;i<btns.length;i++){\
               var t=(btns[i].innerText||btns[i].value||'').replace(/\\s+/g,'');\
               if(t==='검색'){btns[i].click();return 'searching';}\
             }\
             return 'not-found';})()",
        )
    }

    pub fn redact_request_line(method: &str, url: &str) -> String {
        let path = url.split('?').next().unwrap_or(url);
        let lower = path.to_lowercase();
        if lower.contains("security_check") || lower.contains("login") || lower.contains("checknet")
        {
            return format!("{method} <로그인 요청 — 내용을 남기지 않음>");
        }
        if let Some((_, q)) = url.split_once('?') {
            let short_q: String = q.chars().take(400).collect();
            format!("{method} {path}?{short_q}")
        } else {
            format!("{method} {path}")
        }
    }

    pub const ALLOWED_EHERITAGE_HOSTS: [&'static str; 6] = [
        "heritage.go.kr",
        "gis.heritage.go.kr",
        "gis-heritage.go.kr",
        "e-heritage.go.kr",
        "db.history.go.kr",
        "koreanhistory.or.kr",
    ];

    /// C++ `EHeritageIntranetClient::isAllowedFrameUrl` — 허용 도메인 화이트리스트 대조 및 외부 리다이렉트 차단
    pub fn is_allowed_frame_url(url_str: &str) -> Result<(), String> {
        let trimmed = url_str.trim();
        let after_scheme = trimmed
            .strip_prefix("https://")
            .or_else(|| trimmed.strip_prefix("http://"))
            .ok_or_else(|| format!("허용되지 않은 외부 도메인 접근 차단: [{trimmed}]"))?;
        let authority = after_scheme.split(['/', '?', '#']).next().unwrap_or("");
        let host = authority
            .split(':')
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        if host.is_empty() {
            return Err("허용되지 않은 외부 도메인 접근 차단: []".to_owned());
        }
        let allowed = Self::ALLOWED_EHERITAGE_HOSTS.iter().any(|&allowed_host| {
            host == allowed_host || host.ends_with(&format!(".{allowed_host}"))
        });
        if allowed {
            Ok(())
        } else {
            Err(format!("허용되지 않은 외부 도메인 접근 차단: [{host}]"))
        }
    }
}

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
fn run_curl(url: &str, body: Option<&str>, cookie_jar: Option<&Path>) -> Result<HeritageHttpResponse, String> {
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
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let output = cmd
        .output()
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

/// 자료 종류별 changeTab 코드.
///
/// `HeritageStyle.cpp:27-32` 의 표는 지정유산만 비어 있지만,
/// `HeritageIntranetFlow.cpp:247-248`·`:269` 이 그 자리를 `"S"` 로 채운다(화면에서 확인한 탭).
/// 그래서 지정유산도 실제로는 `"S"` 다.
pub const HERITAGE_TAB_CODES: [(&str, &str); 6] = [
    ("지정유산", "S"),
    ("현상변경허용기준", "P"),
    ("매장유산유존지역", "B"),
    ("문화유적분포지도", "U"),
    ("지표조사구역", "R"),
    ("발굴조사구역", "E"),
];

/// 사이트 탭 이름으로 changeTab 코드를 찾는다. 모르는 이름은 빈 문자열이다.
#[must_use]
pub fn heritage_tab_code(label: &str) -> &'static str {
    HERITAGE_TAB_CODES
        .iter()
        .find(|(name, _)| *name == label)
        .map(|(_, code)| *code)
        .unwrap_or("")
}

/// 인트라넷 요청 한 건의 결과. 받은 것이 ZIP 이면 파일, 아니면 화면(로그인·오류)이다.
#[derive(Debug, Clone)]
pub struct HeritageFileResult {
    pub label: String,
    pub url: String,
    pub saved_path: Option<std::path::PathBuf>,
    pub bytes: usize,
    /// 실패 문구. 성공이면 `None`.
    pub error: Option<String>,
}

/// 「받기」 한 번이 남기는 결과 (`KaHeritageBrowser.cpp:571-1523` 흐름).
#[derive(Debug, Clone, Default)]
pub struct HeritagePipelineOutcome {
    pub matched_sido: String,
    pub matched_city: String,
    pub total_before_search: i32,
    pub search_count: i32,
    pub request_log: Vec<String>,
    pub files: Vec<HeritageFileResult>,
    pub form_found: bool,
    /// ②에서 읽어 낸 검색 폼. ④의 주소를 만드는 데 쓴다.
    pub form: Option<HeritageForm>,
    pub sido_value: String,
    pub city_value: String,
}

impl HeritagePipelineOutcome {
    #[must_use]
    pub fn detail_ko(&self) -> String {
        let saved: usize = self.files.iter().filter(|f| f.saved_path.is_some()).count();
        format!(
            "{} {} — 검색결과 {}건 · 파일 {}개 받음",
            self.matched_sido,
            self.matched_city,
            self.search_count,
            saved
        )
    }
}

/// 「받기」 전에 막는 조건들. C++ 문구를 그대로 쓴다.
///
/// 네트워크를 타지 않으므로 검사에서 그대로 시험한다.
pub fn heritage_preflight(
    have_account: bool,
    survey_dir: Option<&Path>,
    sido: &str,
    city: &str,
) -> Result<(), String> {
    // KaHeritageBrowser.cpp:598
    if !have_account {
        return Err(messages::NO_ACCOUNT.to_owned());
    }
    // KaHeritageBrowser.cpp:585-592
    if sido.trim().is_empty() || city.trim().is_empty() {
        return Err("받을 시·군이 정해지지 않았습니다.".to_owned());
    }
    // MainWindow.cpp:3113-3116 — 받은 자료는 그 조사폴더 안에만 둔다.
    let Some(dir) = survey_dir else {
        return Err("조사를 먼저 열거나 저장하세요.\n받은 자료는 그 조사폴더 안에만 둡니다.".to_owned());
    };
    if dir.as_os_str().is_empty() {
        return Err("조사를 먼저 열거나 저장하세요.\n받은 자료는 그 조사폴더 안에만 둡니다.".to_owned());
    }
    Ok(())
}

/// 받은 ZIP 을 둘 자리. C++ 은 OneDrive 동기화 폴더를 피해 로컬 앱 데이터에 두지만
/// (`MainWindow.cpp:3186-3201`), 여기서는 조사폴더 안에만 둔다.
#[must_use]
pub fn heritage_download_root(survey_dir: &Path, sido: &str, city: &str) -> std::path::PathBuf {
    survey_dir
        .join("주변유적")
        .join(format!("{} {}", sido.trim(), city.trim()))
        .join("원본")
}

/// `KaHeritageBrowser.cpp:1361-1368` — 다시 받아 볼 만한 실패인가.
#[must_use]
pub fn is_retryable_download_error(message: &str) -> bool {
    message.contains("파일을 끝까지 받지 못했습니다")
        || message.contains("저장된 파일이 없거나 비어 있습니다")
        || message.contains("크기가 일치하지 않습니다")
        || message.contains("공식 화면에서 다시 받습니다")
}

fn looks_like_login_page(html: &str) -> bool {
    html.contains("j_spring_security_check")
        || html.contains("checkNet")
        || html.contains("goLogin")
        || html.contains("userId")
}

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

#[cfg(test)]
mod tests {
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

    #[test]
    fn heritage_stage_names_and_ui_indices_match_the_cpp_order() {
        // HeritageIntranetFlow.cpp:94-110 — 단계 이름을 그대로 쓴다.
        let all = [
            (HeritageStage::Idle, "대기"),
            (HeritageStage::Login, "로그인"),
            (HeritageStage::DismissTutorial, "튜토리얼 닫기"),
            (HeritageStage::OpenDownloadPage, "국가유산자료 다운로드 열기"),
            (HeritageStage::AgreeTerms, "서약서 동의"),
            (HeritageStage::SelectDataset, "자료 종류 선택"),
            (HeritageStage::SelectRegion, "시·군 선택"),
            (HeritageStage::Search, "검색"),
            (HeritageStage::CollectResults, "결과 모으기"),
            (HeritageStage::Download, "내려받기"),
            (HeritageStage::Done, "완료"),
            (HeritageStage::Failed, "실패"),
        ];
        for (index, (stage, name)) in all.into_iter().enumerate() {
            assert_eq!(stage.stage_name_ko(), name);
            assert_eq!(stage.ui_index(), index);
        }
    }

    #[test]
    fn heritage_tab_codes_match_heritage_style_and_never_guess_designated() {
        // HeritageStyle.cpp:27-32 의 표 + HeritageIntranetFlow.cpp:247-248 의 지정유산 → "S"
        assert_eq!(heritage_tab_code("지정유산"), "S");
        assert_eq!(heritage_tab_code("현상변경허용기준"), "P");
        assert_eq!(heritage_tab_code("매장유산유존지역"), "B");
        assert_eq!(heritage_tab_code("문화유적분포지도"), "U");
        assert_eq!(heritage_tab_code("지표조사구역"), "R");
        assert_eq!(heritage_tab_code("발굴조사구역"), "E");
        assert_eq!(heritage_tab_code("없는자료"), "");
    }

    #[test]
    fn heritage_preflight_blocks_like_the_cpp_before_any_request() {
        let dir = Path::new("C:/survey/test");
        // KaHeritageBrowser.cpp:598 — 계정 없음
        assert_eq!(
            heritage_preflight(false, Some(dir), "경상북도", "경주시"),
            Err(messages::NO_ACCOUNT.to_owned())
        );
        // KaHeritageBrowser.cpp:585-588 — 시·군 미정
        assert_eq!(
            heritage_preflight(true, Some(dir), "", "경주시"),
            Err("받을 시·군이 정해지지 않았습니다.".to_owned())
        );
        // MainWindow.cpp:3113-3116 — 조사폴더 없음
        assert_eq!(
            heritage_preflight(true, None, "경상북도", "경주시"),
            Err("조사를 먼저 열거나 저장하세요.\n받은 자료는 그 조사폴더 안에만 둡니다.".to_owned())
        );
        assert!(heritage_preflight(true, Some(dir), "경상북도", "경주시").is_ok());
    }

    #[test]
    fn heritage_download_root_keeps_files_inside_the_survey_folder() {
        // MainWindow.cpp:3191-3192 — 주변유적/<시도 시군>/원본
        let root = heritage_download_root(Path::new("C:/survey/경주"), " 경상북도 ", "경주시");
        let text = root.to_string_lossy().replace('\\', "/");
        assert!(text.ends_with("주변유적/경상북도 경주시/원본"), "{text}");
        assert!(text.starts_with("C:/survey/경주"), "{text}");
    }

    #[test]
    fn heritage_retry_classification_matches_the_cpp_download_retry() {
        // KaHeritageBrowser.cpp:1357-1373 — 이 실패들은 다시 받는다.
        assert!(is_retryable_download_error(
            "파일을 끝까지 받지 못했습니다: 네트워크 오류"
        ));
        assert!(is_retryable_download_error(
            "다운로드 완료 신호를 받았지만 저장된 파일이 없거나 비어 있습니다."
        ));
        assert!(is_retryable_download_error("예상·수신·저장 파일 크기가 일치하지 않습니다."));
        // 계정·설정 문제는 다시 받아도 같으므로 곧바로 실패다.
        assert!(!is_retryable_download_error(messages::NO_ACCOUNT));
        assert!(!is_retryable_download_error(messages::NO_SESSION));
    }

    #[test]
    fn heritage_http_search_without_session_stops_with_the_cpp_message() {
        // 네트워크를 타기 전에 막히는 경로만 확인한다(계정 없음).
        let err = heritage_http_search(
            Some(Path::new("C:/survey/test")),
            false,
            "경상북도",
            "경주시",
            None,
        )
        .expect_err("계정이 없으면 요청하지 않고 멈춰야 한다");
        assert_eq!(err, messages::NO_ACCOUNT);
        // 조사폴더가 없으면 역시 요청하지 않는다.
        let err = heritage_http_search(None, true, "경상북도", "경주시", None)
            .expect_err("조사폴더가 없으면 요청하지 않고 멈춰야 한다");
        assert!(err.contains("조사를 먼저 열거나 저장하세요"));
    }

    #[test]
    fn heritage_outcome_detail_reads_like_the_browser_status() {
        let outcome = HeritagePipelineOutcome {
            matched_sido: "경상북도".to_owned(),
            matched_city: "경주시".to_owned(),
            search_count: 90,
            files: vec![HeritageFileResult {
                label: "지정유산".to_owned(),
                url: String::new(),
                saved_path: Some(std::path::PathBuf::from("C:/x/지정유산.zip")),
                bytes: 1024,
                error: None,
            }],
            ..HeritagePipelineOutcome::default()
        };
        assert_eq!(
            outcome.detail_ko(),
            "경상북도 경주시 — 검색결과 90건 · 파일 1개 받음"
        );
    }
}
