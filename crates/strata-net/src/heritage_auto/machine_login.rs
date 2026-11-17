//! The login stages, as the C++ runs them: what the user sees while a login runs, and when a
//! run is over.

use super::machine::{HeritageAutomation, HeritageStage};

impl HeritageAutomation {
    /// Whether the run is over.
    #[must_use]
    pub fn is_over(&self) -> bool {
        matches!(
            self.stage,
            HeritageStage::Done | HeritageStage::Stopped(_) | HeritageStage::Cancelled
        )
    }

    /// The words to show while the run goes on, for a status line.
    #[must_use]
    pub fn progress_words(&self) -> String {
        match &self.stage {
            HeritageStage::Idle => "인트라넷에 접속하는 중".to_owned(),
            HeritageStage::LoginProbe | HeritageStage::LoggingIn => "인트라넷 로그인 중".to_owned(),
            HeritageStage::DismissTutorial => "안내창을 닫는 중".to_owned(),
            HeritageStage::DownloadPage => "자료 받기 화면을 여는 중".to_owned(),
            HeritageStage::Terms => "서약서에 동의하는 중".to_owned(),
            HeritageStage::Kind => format!("유산 종류 고르는 중 · {}", self.current_kind()),
            HeritageStage::Region => format!("지역 고르는 중 · {}", self.config.city),
            HeritageStage::Search => format!("검색 중 · {}", self.current_kind()),
            HeritageStage::DownloadAll | HeritageStage::Waiting => {
                format!("받는 중 · {}", self.current_kind())
            }
            HeritageStage::Done => "주변유적 자료를 다 받았습니다".to_owned(),
            HeritageStage::Cancelled => "사용자가 중지했습니다.".to_owned(),
            HeritageStage::Stopped(words) => words.clone(),
        }
    }
}

/// The words a run shows when there is no account to log in with, before it starts.
pub const NO_ACCOUNT: &str = "「더보기」에서 기관 계정을 먼저 넣어 주세요.";
