//! 다운로드 파이프라인 결과·사전 점검·재시도 분류.
//!
//! `heritage_intranet` 본문을 300줄 이하로 유지하기 위해 분리했다. 공개 경로는 그대로다.
use std::path::Path;

use super::*;

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

pub(super) fn looks_like_login_page(html: &str) -> bool {
    html.contains("j_spring_security_check")
        || html.contains("checkNet")
        || html.contains("goLogin")
        || html.contains("userId")
}
