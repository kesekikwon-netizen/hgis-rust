//! 국가유산 인트라넷 단계 이름과 실패 문구 (C++ `HeritageIntranetFlow.cpp:86-110`).
//!
//! `heritage_intranet` 본문을 300줄 이하로 유지하기 위해 분리했다. 공개 경로는 그대로다.


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
