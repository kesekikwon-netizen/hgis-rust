//! The login stages must read the page the way the C++ does
//! (`KaHeritageBrowser.cpp:898-933`).

use super::machine::{HeritageConfig, HeritageEvent, HeritageStage};
use super::{HeritageAutomation, PageHost, PageId, PageRef};

/// A page host that records what it was told and answers nothing.
struct SilentHost {
    navigated: Vec<String>,
}

impl PageHost for SilentHost {
    fn pages(&self) -> Vec<PageRef> {
        vec![PageRef {
            id: PageId(1),
            url: "about:blank".to_owned(),
            opener: None,
        }]
    }

    fn run_script(&mut self, _page: PageId, _script: &str) -> Option<u64> {
        Some(1)
    }

    fn navigate(&mut self, url: &str) {
        self.navigated.push(url.to_owned());
    }
}

fn machine() -> HeritageAutomation {
    HeritageAutomation::new(HeritageConfig {
        account_id: "user1".to_owned(),
        account_password: "pass1".to_owned(),
        sido: "경상북도".to_owned(),
        city: "경주시".to_owned(),
        fixture_base: None,
    })
}

/// The first step opens the login page; nothing is asked of a page before one exists.
#[test]
fn the_run_opens_the_login_page_first() {
    let mut m = machine();
    let mut host = SilentHost { navigated: Vec::new() };
    assert_eq!(m.stage(), &HeritageStage::Idle);
    assert!(m.next_step(&mut host, 0).is_none(), "asked before opening");
    assert_eq!(host.navigated.len(), 1);
    assert!(host.navigated[0].ends_with("/user/checkNet.do"), "{:?}", host.navigated);
    assert_eq!(m.stage(), &HeritageStage::LoginProbe);
}

/// A fixture base is used in place of the site, so a run can be driven locally.
#[test]
fn a_fixture_base_replaces_the_site() {
    let mut m = HeritageAutomation::new(HeritageConfig {
        account_id: "u".to_owned(),
        account_password: "p".to_owned(),
        sido: "경상북도".to_owned(),
        city: "경주시".to_owned(),
        fixture_base: Some("http://127.0.0.1:9999".to_owned()),
    });
    let mut host = SilentHost { navigated: Vec::new() };
    let _ = m.next_step(&mut host, 0);
    assert_eq!(host.navigated[0], "http://127.0.0.1:9999/user/checkNet.do");
}

/// `past-login` means we are in: the run moves on to the download panel.
#[test]
fn a_page_with_logout_means_we_are_in() {
    let mut m = machine();
    let mut host = SilentHost { navigated: Vec::new() };
    let _ = m.next_step(&mut host, 0);
    let call = m.next_step(&mut host, 700).expect("a probe");
    assert!(call.script.contains("로그아웃"), "the probe does not read the logout mark");
    let ev = m.script_answered(call.token, "past-login");
    assert!(matches!(ev, Some(HeritageEvent::Note(_))));
    assert_eq!(m.stage(), &HeritageStage::DownloadPage);
}

/// `still-login` submits once; a second one gives up with the C++ words.
#[test]
fn a_form_that_comes_back_twice_gives_up() {
    let mut m = machine();
    let mut host = SilentHost { navigated: Vec::new() };
    let _ = m.next_step(&mut host, 0);

    let probe = m.next_step(&mut host, 700).expect("a probe");
    assert!(m.script_answered(probe.token, "still-login").is_none(), "stopped too early");
    assert_eq!(m.stage(), &HeritageStage::LoggingIn);

    let login = m.next_step(&mut host, 1400).expect("the login script");
    assert!(login.script.contains("goLogin()"), "does not call the page's own submit");
    assert!(m.script_answered(login.token, "submitted").is_none());
    assert_eq!(m.stage(), &HeritageStage::LoginProbe);

    let probe = m.next_step(&mut host, 2100).expect("a second probe");
    let ev = m.script_answered(probe.token, "still-login");
    let Some(HeritageEvent::Stopped(words)) = ev else {
        panic!("did not give up: {ev:?}");
    };
    assert!(words.contains("로그인"), "{words}");
    assert!(m.is_over());
}

/// `pending` — a blank or still-loading page — is not a login and not a failure.
#[test]
fn a_blank_page_waits() {
    let mut m = machine();
    let mut host = SilentHost { navigated: Vec::new() };
    let _ = m.next_step(&mut host, 0);
    let probe = m.next_step(&mut host, 700).expect("a probe");
    assert!(m.script_answered(probe.token, "pending").is_none());
    assert_eq!(m.stage(), &HeritageStage::LoginProbe, "a blank page moved the run on");
}

/// `no-form` from the login script means the page was not the login page.
#[test]
fn a_login_script_without_a_form_stops() {
    let mut m = machine();
    let mut host = SilentHost { navigated: Vec::new() };
    let _ = m.next_step(&mut host, 0);
    let probe = m.next_step(&mut host, 700).expect("a probe");
    let _ = m.script_answered(probe.token, "still-login");
    let login = m.next_step(&mut host, 1400).expect("the login script");
    let ev = m.script_answered(login.token, "no-form");
    assert!(matches!(ev, Some(HeritageEvent::Stopped(_))), "{ev:?}");
    assert!(m.is_over());
}

/// A late answer to a script we already moved past is ignored.
#[test]
fn an_answer_to_an_old_script_is_ignored() {
    let mut m = machine();
    let mut host = SilentHost { navigated: Vec::new() };
    let _ = m.next_step(&mut host, 0);
    let first = m.next_step(&mut host, 700).expect("a probe");
    let _ = m.script_answered(first.token, "past-login");
    assert!(
        m.script_answered(first.token, "still-login").is_none(),
        "an old token changed the stage"
    );
    assert_eq!(m.stage(), &HeritageStage::DownloadPage);
}

/// The status line names the stage, so the user can see it moving.
#[test]
fn progress_words_follow_the_stage() {
    let mut m = machine();
    assert!(m.progress_words().contains("접속"));
    let mut host = SilentHost { navigated: Vec::new() };
    let _ = m.next_step(&mut host, 0);
    assert!(m.progress_words().contains("로그인"));
}
