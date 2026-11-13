//! 홈 「연결 상태」 칸의 글 — C++ `KaHomeConnectionCard`(displayName :100-108, stateText :110-112,
//! setInputs :123-179, checkInternet :187-207). 그리기는 `gui_tab_home_status`.

use std::sync::{Arc, Mutex};

use strata_secrets::account_status::{Entry, Source};

/// 버튼 「인터넷 확인」의 도움말(:71).
pub(crate) const INTERNET_BUTTON_TIP: &str = "VWorld 서버에 한 번 접속해 봅니다. 누를 때만 확인합니다.";

/// 한 칸 — 이름 위에 상태, 오른쪽에 「설정」.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConnRow {
    pub name: &'static str,
    pub state: &'static str,
    pub ready: bool,
    /// 종이 모양에서 없는 설정 앞의 점(#D97757).
    pub missing_dot: bool,
    pub name_tip: String,
    pub state_tip: Option<String>,
    pub setup_tip: &'static str,
}

/// displayName — 홈은 짧은 이름 셋을 쓴다.
fn display_name(e: &Entry) -> &'static str {
    match e.source {
        Source::VworldKey => "VWorld 키",
        Source::TopographicAccount => "수치지형도 계정",
        Source::HeritageAccount => "문화재 인트라넷",
        _ => e.label,
    }
}

/// KaHomeConnectionCard::stateText.
fn state_text(ready: bool) -> &'static str {
    if ready { "설정됨" } else { "설정 필요" }
}

/// setInputs 의 칸들.
pub(crate) fn conn_rows(entries: &[Entry], plate: bool) -> Vec<ConnRow> {
    entries
        .iter()
        .map(|e| {
            let name = display_name(e);
            ConnRow {
                name,
                state: state_text(e.ready),
                ready: e.ready,
                missing_dot: plate && !e.ready,
                name_tip: format!("{name} — {} · {}", e.used_for, e.menu_path),
                state_tip: (!e.ready).then(|| format!("{}에서 넣습니다.", e.menu_path)),
                setup_tip: e.menu_path,
            }
        })
        .collect()
}

/// 「인터넷 확인」 — 확인 안 함 / 확인 중(뒤 실행이 답을 넣는다) / 답.
#[derive(Debug, Clone, Default)]
pub(crate) enum InternetCheck {
    #[default]
    NotChecked,
    Checking(Arc<Mutex<Option<bool>>>),
    Done(bool),
}

impl InternetCheck {
    /// 한 번 묻기 시작한다(VWorld 서버, 화면 밖 실행).
    pub(crate) fn start() -> Self {
        let slot = Arc::new(Mutex::new(None));
        let answer = slot.clone();
        std::thread::spawn(move || {
            let reached = strata_net::internet_check::vworld_reachable();
            if let Ok(mut g) = answer.lock() {
                *g = Some(reached);
            }
        });
        Self::Checking(slot)
    }

    /// 확인 중이면 답이 왔는지 본다 — 왔으면 true.
    pub(crate) fn poll(&mut self) -> bool {
        let Self::Checking(slot) = self else { return false };
        let Some(reached) = slot.lock().ok().and_then(|g| *g) else { return false };
        *self = Self::Done(reached);
        true
    }
}

/// 인터넷 줄 — 글, 점(닿음 ok / 안 닿음 danger), 도움말, 단추를 누를 수 있는지.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InternetLabel {
    pub text: &'static str,
    pub dot: Option<bool>,
    pub tip: Option<&'static str>,
    pub button_enabled: bool,
}

pub(crate) fn internet_label(check: &InternetCheck) -> InternetLabel {
    let (text, dot, tip, button_enabled) = match check {
        InternetCheck::NotChecked => ("인터넷 · 확인 안 함", None, None, true),
        InternetCheck::Checking(_) => ("인터넷 · 확인 중…", None, None, false),
        InternetCheck::Done(true) => ("인터넷 · 연결됨", Some(true), None, true),
        InternetCheck::Done(false) => (
            "인터넷 · 연결 안 됨",
            Some(false),
            Some("오프라인이어도 조사 열기·그리기·저장은 됩니다. 배경 지도와 자료 받기만 인터넷이 필요합니다."),
            true,
        ),
    };
    InternetLabel { text, dot, tip, button_enabled }
}

#[cfg(test)]
#[path = "home_connection_model_tests.rs"]
mod tests;
