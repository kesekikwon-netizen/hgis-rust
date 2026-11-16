//! What each login answer means (`KaHeritageBrowser.cpp:898-933`,
//! `HeritageIntranetFlow.cpp:150-160`).
//!
//! `loginProbeScript` answers one of three words from the page as it stands:
//! `past-login` (the body carries 로그아웃), `still-login` (a visible `#userId` and `#pwd`),
//! `pending` (neither — blank, or still loading).

use super::machine::{HeritageAutomation, HeritageEvent, HeritageStage};

/// The words the C++ shows when the account does not get in (`KaHeritageBrowser.cpp:927-933`).
pub(super) const LOGIN_FAILED: &str =
    "인트라넷에 로그인하지 못했습니다. 「더보기」에서 기관 계정을 확인한 뒤 다시 눌러 주세요.";

impl HeritageAutomation {
    /// One answer to `loginProbeScript`.
    pub(super) fn on_login_probe(&mut self, value: &str) -> Option<HeritageEvent> {
        match value {
            // The body says 로그아웃: we are in. Open the download panel next.
            "past-login" => {
                self.go(HeritageStage::DownloadPage);
                Some(HeritageEvent::Note("인트라넷에 들어갔습니다.".to_owned()))
            }
            // The form is showing. Submit once; a second time means the account is wrong.
            "still-login" => {
                if self.submitted {
                    return self.stop(LOGIN_FAILED.to_owned());
                }
                self.submitted = true;
                self.go(HeritageStage::LoggingIn);
                None
            }
            // Nothing to read yet: a blank page or one still loading. Wait a tick.
            _ => {
                self.quiet_ticks = self.quiet_ticks.saturating_add(1);
                None
            }
        }
    }

    /// The login script answered: `submitted`, `no-form`, `no-goLogin`, or `error:…`.
    pub(super) fn on_login_submitted(&mut self, value: &str) -> Option<HeritageEvent> {
        match value {
            // The page took the form and is posting it; prove we got in on the next probe.
            "submitted" => {
                self.go(HeritageStage::LoginProbe);
                None
            }
            // No form on any frame: the page we are on is not the login page.
            "no-form" => self.stop(LOGIN_FAILED.to_owned()),
            "no-goLogin" => self.stop(LOGIN_FAILED.to_owned()),
            other if other.starts_with("error:") => self.stop(LOGIN_FAILED.to_owned()),
            _ => self.stop(LOGIN_FAILED.to_owned()),
        }
    }
}
