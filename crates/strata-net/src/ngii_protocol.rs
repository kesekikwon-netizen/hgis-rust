//! 국토정보플랫폼 요청 주소·본문과 안내 문구 (C++ `KaTopographicBrowser.cpp`, `ngii-topographic.js`).
//!
//! `ngii_topographic` 본문을 300줄 이하로 유지하기 위해 분리했다. 공개 경로는 그대로다.

use super::ngii_sheets::SheetScope;

/// JS `String(x)` 와 같은 짧은 숫자 표기
fn fmt_num(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

/// 폼 본문 값 퍼센트 인코딩(C++ `QUrlQuery::FullyEncoded` + `+`→`%2B`)
fn pct(input: &str) -> String {
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

#[derive(Debug, Clone, Copy)]
pub struct NgiiTopographicClient;

impl NgiiTopographicClient {
    pub const PORTAL_URL: &'static str = "https://map.ngii.go.kr/ms/map/NlipMap.do";

    /// `KaTopographicBrowser.cpp:49` `kHome` — 처음 여는 공식 화면
    pub const HOME_URL: &'static str = "https://map.ngii.go.kr/ms/map/NlipMap.do?tabGb=total";

    pub const PORTAL_ORIGIN: &'static str = "https://map.ngii.go.kr";

    pub const CONTOUR_LAYER_CODES: [&'static str; 4] = ["F0010000", "1000", "1011", "1012"];

    /// `MainWindowDownloads.cpp:187` — 조사 주변 5 km
    pub const SCOPE_RADIUS_M: f64 = 5000.0;

    /// `KaTopographicBrowser.cpp:675` — 자동 선택 도엽 상한
    pub const MAX_SHEETS: usize = 100;

    pub const fn portal_url() -> &'static str {
        Self::PORTAL_URL
    }

    pub const fn home_url() -> &'static str {
        Self::HOME_URL
    }

    /// `ngii-topographic.js:43` 도엽 후보 범위 질의(공개, 로그인 불필요)
    #[must_use]
    pub fn combine_buffer_query_url() -> String {
        format!("{}/ms/map/combineBufferQuery.do", Self::PORTAL_ORIGIN)
    }

    /// `ngii-topographic.js:63` 1:25,000 DXF 도엽 목록 질의(공개, 로그인 불필요)
    #[must_use]
    pub fn suchi_query_url() -> String {
        format!("{}/ms/map/suchiQuery.do", Self::PORTAL_ORIGIN)
    }

    /// `ngii-topographic.js:43` `{x, y, buffer}` — `x`/`y` 는 EPSG:5179 미터
    #[must_use]
    pub fn combine_buffer_body(x: f64, y: f64, buffer_m: f64) -> String {
        format!("x={}&y={}&buffer={}", fmt_num(x), fmt_num(y), fmt_num(buffer_m))
    }

    /// `ngii-topographic.js:63-64` `{index, type, scale, wkt, minx, miny, maxx, maxy}`
    #[must_use]
    pub fn suchi_body(area: &SheetScope) -> String {
        format!(
            "index=&type=0&scale=25000&wkt={}&minx={}&miny={}&maxx={}&maxy={}",
            pct(&area.wkt),
            fmt_num(area.minx),
            fmt_num(area.miny),
            fmt_num(area.maxx),
            fmt_num(area.maxy)
        )
    }
}

impl NgiiTopographicClient {
    /// `ngii-topographic.js:120` / `KaTopographicBrowser.cpp:843`
    #[must_use]
    pub const fn scope_error() -> &'static str {
        "도엽·축척·파일 형식 또는 포털 구조를 확인하지 못했습니다. 필요한 도엽을 직접 선택해 주세요."
    }

    /// `ngii-topographic.js:114` — 이 범위에 제공 중인 도엽이 없음
    #[must_use]
    pub const fn unavailable_message() -> &'static str {
        "이 범위에서 제공 중인 1:25,000 DXF 도엽이 없습니다."
    }

    /// `KaTopographicBrowser.cpp:851` — 필요한 지형도를 이미 모두 보관 중
    #[must_use]
    pub const fn cached_message() -> &'static str {
        "필요한 지형도를 모두 보관하고 있습니다. 다시 신청하지 않습니다."
    }

    /// `KaTopographicBrowser.cpp:758-759` — 공식 사이트 응답을 확인하지 못했을 때
    #[must_use]
    pub const fn portal_timeout_message() -> &'static str {
        "공식 사이트의 응답을 확인하지 못했습니다. 계정 설정과 연결 상태를 확인하세요."
    }

    /// `KaTopographicBrowser.cpp:582` — 페이지 자체를 불러오지 못했을 때
    #[must_use]
    pub const fn page_load_failed_message() -> &'static str {
        "공식 사이트 페이지를 불러오지 못했습니다. 인터넷 연결을 확인한 뒤 다시 시도하세요."
    }

    /// `KaTopographicBrowser.cpp:152` — 창의 안내문
    #[must_use]
    pub const fn browser_hint() -> &'static str {
        "현재 조사 주변의 1:25,000 DXF 도엽을 선택합니다. 로그인·인증이 필요하면 공식 화면의 안내를 따라 주세요."
    }

    /// `KaTopographicBrowser.cpp:523` — 공식 화면이 추가 확인을 요구할 때
    #[must_use]
    pub const fn needs_input_message() -> &'static str {
        "자동 받기에 추가 확인이 필요합니다. 로그인 설정과 연결 상태를 확인해 주세요."
    }

    /// `KaTopographicBrowser.cpp:614` — 자동 전송을 계속할 수 없을 때(외부 전송 프로그램 필요)
    #[must_use]
    pub const fn transfer_program_message() -> &'static str {
        "자동 전송을 계속할 수 없습니다. 전송 프로그램 연결이 필요합니다. 지도 작업은 계속할 수 있습니다."
    }

    /// C++ 은 로그인·신청·파일 준비·다운로드를 내장 브라우저(QWebEngine)의 공식 화면에서 한다
    /// (`KaTopographicBrowser.cpp:509-598`). Rust 에는 그 브라우저가 없으므로, C++ 이 같은
    /// 상황에 내는 안내(152·446·214-215)를 그대로 보여 주고 「받은 자료 불러오기」로 넘긴다.
    #[must_use]
    pub const fn login_stage_blocked_message() -> &'static str {
        "로그인·신청·다운로드는 국토정보플랫폼 공식 화면에서 진행합니다. 공식 화면에서 받은 DXF·ZIP 폴더는 「받은 자료 불러오기…」로 지도에 올릴 수 있습니다."
    }
}
