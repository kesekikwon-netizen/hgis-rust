//! 인트라넷 주소·헤더·본문 (C++ `HeritageHttpClient.cpp:17-19`, `:81-100`).
//!
//! `heritage_intranet` 본문을 300줄 이하로 유지하기 위해 분리했다. 공개 경로는 그대로다.


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
}
