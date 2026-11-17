//! The login stages must read the page the way the C++ does
//! (`KaHeritageBrowser.cpp:898-933`), and must ask one question at a time, waiting for the
//! answer, as the C++ does on its 700 ms poll (`:103`).

use super::machine::{HeritageConfig, HeritageEvent, HeritageStage};
use super::{HeritageAutomation, PageHost, PageId, PageRef, LOGIN_POLL_MS};

/// A page host with one page open, recording what it was told.
struct SilentHost {
    navigated: Vec<String>,
    open: bool,
}

impl SilentHost {
    fn with_page() -> Self {
        Self {
            navigated: Vec::new(),
            open: true,
        }
    }

    fn without_page() -> Self {
        Self {
            navigated: Vec::new(),
            open: false,
        }
    }
}

impl PageHost for SilentHost {
    fn pages(&self) -> Vec<PageRef> {
        if self.open {
            vec![PageRef {
                id: PageId(1),
                url: "about:blank".to_owned(),
                opener: None,
            }]
        } else {
            Vec::new()
        }
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

/// Nothing is asked of a browser that has no page yet, and the portal is opened exactly once.
#[test]
fn the_run_opens_the_login_page_first() {
    let mut m = machine();
    let mut host = SilentHost::without_page();
    assert_eq!(m.stage(), &HeritageStage::Idle);
    assert!(
        m.next_step(&mut host, 0).is_none(),
        "asked before a page existed"
    );
    assert!(host.navigated.is_empty(), "navigated before a page existed");

    let mut host = SilentHost::with_page();
    assert!(
        m.next_step(&mut host, 0).is_none(),
        "the open is not a script"
    );
    assert_eq!(host.navigated.len(), 1);
    assert!(
        host.navigated[0].ends_with("/user/checkNet.do"),
        "{:?}",
        host.navigated
    );
    assert_eq!(m.stage(), &HeritageStage::LoginProbe);

    // A second frame must not open it again, and asks a probe.
    let probe = m
        .next_step(&mut host, LOGIN_POLL_MS)
        .expect("no probe after the open");
    assert_eq!(host.navigated.len(), 1, "the portal was opened twice");
    let _ = probe;
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
    let mut host = SilentHost::with_page();
    let _ = m.next_step(&mut host, 0);
    assert_eq!(host.navigated[0], "http://127.0.0.1:9999/user/checkNet.do");
}

/// One ask at a time: while an answer is outstanding, or before the poll interval, nothing is
/// asked. This is the trap that stalled the first real run — asking every frame made every
/// answer arrive "late" and be dropped.
#[test]
fn nothing_is_asked_while_an_answer_is_outstanding() {
    let mut m = machine();
    let mut host = SilentHost::with_page();
    let _ = m.next_step(&mut host, 0);
    let first = m.next_step(&mut host, LOGIN_POLL_MS).expect("a probe");
    // Same instant, an answer still outstanding: no second ask.
    assert!(
        m.next_step(&mut host, LOGIN_POLL_MS).is_none(),
        "asked again without an answer"
    );
    // After the interval but still unanswered: still no second ask.
    assert!(
        m.next_step(&mut host, LOGIN_POLL_MS * 10).is_none(),
        "asked again without an answer"
    );
    // The answer moves the run on, and the next ask is allowed after the interval.
    let _ = m.script_answered(first.token, "past-login");
    assert!(
        m.next_step(&mut host, LOGIN_POLL_MS * 11).is_some(),
        "the tutorial stage asked nothing"
    );
}

/// The poll interval is respected even with no answer outstanding.
#[test]
fn the_next_ask_waits_for_the_poll_interval() {
    let mut m = machine();
    let mut host = SilentHost::with_page();
    let _ = m.next_step(&mut host, 0);
    let probe = m.next_step(&mut host, 1000).expect("a probe");
    // Answer now; the next stage would ask, but 1000 + interval has not come yet.
    let _ = m.script_answered(probe.token, "still-login");
    assert_eq!(m.stage(), &HeritageStage::LoggingIn);
    assert!(
        m.next_step(&mut host, 1000 + LOGIN_POLL_MS / 2).is_none(),
        "asked before the poll interval"
    );
    assert!(
        m.next_step(&mut host, 1000 + LOGIN_POLL_MS).is_some(),
        "did not ask after the poll interval"
    );
}

/// `past-login` means we are in: the run moves on to the download panel.
#[test]
fn a_page_with_logout_means_we_are_in() {
    let mut m = machine();
    let mut host = SilentHost::with_page();
    let _ = m.next_step(&mut host, 0);
    let call = m.next_step(&mut host, LOGIN_POLL_MS).expect("a probe");
    assert!(
        call.script.contains("로그아웃"),
        "the probe does not read the logout mark"
    );
    assert_eq!(call.page, PageId(1), "the ask did not go to the open page");
    let ev = m.script_answered(call.token, "past-login");
    assert!(matches!(ev, Some(HeritageEvent::Note(_))));
    assert_eq!(
        m.stage(),
        &HeritageStage::TutorialProbe,
        "past-login goes to the tutorial, then the panel"
    );
}

/// A submitted login waits nine polls without a second submission (`KaHeritageBrowser.cpp:925-932`).
#[test]
fn a_persisting_form_stops_after_nine_probes_without_resubmitting() {
    let mut m = machine();
    let mut host = SilentHost::with_page();
    let _ = m.next_step(&mut host, 0);

    let probe = m.next_step(&mut host, LOGIN_POLL_MS).expect("a probe");
    assert!(
        m.script_answered(probe.token, "still-login").is_none(),
        "stopped too early"
    );
    assert_eq!(m.stage(), &HeritageStage::LoggingIn);

    let login = m
        .next_step(&mut host, LOGIN_POLL_MS * 2)
        .expect("the login script");
    assert!(
        login.script.contains("goLogin()"),
        "does not call the page's own submit"
    );
    assert!(m.script_answered(login.token, "submitted").is_none());
    assert_eq!(m.stage(), &HeritageStage::AwaitLogin);
    for tick in 3..11 {
        let probe = m
            .next_step(&mut host, LOGIN_POLL_MS * tick)
            .expect("authentication probe");
        assert!(m.script_answered(probe.token, "still-login").is_none());
        assert!(!m.is_over());
    }
    let probe = m
        .next_step(&mut host, LOGIN_POLL_MS * 11)
        .expect("ninth probe");
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
    let mut host = SilentHost::with_page();
    let _ = m.next_step(&mut host, 0);
    let probe = m.next_step(&mut host, LOGIN_POLL_MS).expect("a probe");
    assert!(m.script_answered(probe.token, "pending").is_none());
    assert_eq!(
        m.stage(),
        &HeritageStage::LoginProbe,
        "a blank page moved the run on"
    );
}

/// A temporarily absent form keeps the login pending (`KaHeritageBrowser.cpp:909-910`).
#[test]
fn a_login_script_waits_when_the_form_is_still_loading() {
    let mut m = machine();
    let mut host = SilentHost::with_page();
    let _ = m.next_step(&mut host, 0);
    let probe = m.next_step(&mut host, LOGIN_POLL_MS).expect("a probe");
    let _ = m.script_answered(probe.token, "still-login");
    let login = m
        .next_step(&mut host, LOGIN_POLL_MS * 2)
        .expect("the login script");
    let ev = m.script_answered(login.token, "no-form");
    assert!(ev.is_none(), "{ev:?}");
    assert_eq!(m.stage(), &HeritageStage::LoginProbe);
    assert!(!m.is_over());
}

/// A late answer to a script we already moved past is ignored.
#[test]
fn an_answer_to_an_old_script_is_ignored() {
    let mut m = machine();
    let mut host = SilentHost::with_page();
    let _ = m.next_step(&mut host, 0);
    let first = m.next_step(&mut host, LOGIN_POLL_MS).expect("a probe");
    let _ = m.script_answered(first.token, "past-login");
    assert!(
        m.script_answered(first.token, "still-login").is_none(),
        "an old token changed the stage"
    );
    assert_eq!(m.stage(), &HeritageStage::TutorialProbe);
}

/// The status line names the stage, so the user can see it moving.
#[test]
fn progress_words_follow_the_stage() {
    let mut m = machine();
    assert!(m.progress_words().contains("접속"));
    let mut host = SilentHost::with_page();
    let _ = m.next_step(&mut host, 0);
    assert!(m.progress_words().contains("로그인"));
}
