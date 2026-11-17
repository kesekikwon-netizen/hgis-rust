//! Delayed acknowledgments from `target/cpp-ref/src/app/KaHeritageBrowser.cpp:895-1069`.

use strata_net::heritage_auto::{HeritageEvent, HeritageStage};

use super::driver::{Action, Session};

#[test]
fn authenticated_dom_does_not_submit_the_login_form() {
    // target/cpp-ref/src/app/KaHeritageBrowser.cpp:897-904 keeps an authenticated session.
    let mut run = Session::new();
    run.reach(HeritageStage::DownloadPage);
    assert!(!run.requested.contains(&Action::SubmitLogin));
    assert_eq!(*run.machine.stage(), HeritageStage::DownloadPage);
}

#[test]
fn submitted_login_waits_for_authenticated_dom_without_resubmitting() {
    // target/cpp-ref/src/app/KaHeritageBrowser.cpp:918-944 waits through still-login for 8 ticks.
    let mut run = Session::new();
    let _ = run.answer(Action::ProbeLogin, "still-login");
    let _ = run.answer(Action::SubmitLogin, "submitted");
    for _ in 0..3 {
        let _ = run.answer(Action::ProbeLogin, "still-login");
        assert!(
            !run.machine.is_over(),
            "a delayed login acknowledgment must remain recoverable"
        );
    }
    let _ = run.answer(Action::ProbeLogin, "past-login");
    run.reach(HeritageStage::DownloadPage);
    assert_eq!(
        run.requested
            .iter()
            .filter(|a| **a == Action::SubmitLogin)
            .count(),
        1
    );
}

#[test]
fn closing_tutorial_waits_until_the_tutorial_is_observed_absent() {
    // target/cpp-ref/src/app/KaHeritageBrowser.cpp:935-944 advances only on "none".
    let mut run = Session::new();
    run.reach(HeritageStage::DismissTutorial);
    let _ = run.answer(Action::DismissTutorial, "closed");
    assert_eq!(*run.machine.stage(), HeritageStage::TutorialProbe);
    let _ = run.answer(Action::ProbeLogin, "past-login");
    let _ = run.answer(Action::DismissTutorial, "none");
    assert_eq!(*run.machine.stage(), HeritageStage::DownloadPage);
}

#[test]
fn deferred_panel_opens_once_and_waits_for_ready() {
    // target/cpp-ref/tests/test_heritage_browser.cpp:178-200 expects 1 request and no advance.
    // target/cpp-ref/src/app/KaHeritageBrowser.cpp:949-973 requires the "ready" probe.
    let mut run = Session::new();
    run.reach(HeritageStage::DownloadPage);
    let (action, call) = run.request();
    if action == Action::ProbePanel {
        let _ = run.machine.script_answered(call.token, "not-yet");
        let _ = run.answer(Action::OpenPanel, "opening");
    } else {
        assert_eq!(action, Action::OpenPanel);
        let _ = run.machine.script_answered(call.token, "opening");
    }
    for _ in 0..2 {
        assert_eq!(*run.machine.stage(), HeritageStage::DownloadPageReady);
        let _ = run.answer(Action::ProbePanel, "not-yet");
    }
    let _ = run.answer(Action::ProbePanel, "ready");
    assert_eq!(*run.machine.stage(), HeritageStage::Terms);
    assert_eq!(
        run.requested
            .iter()
            .filter(|a| **a == Action::OpenPanel)
            .count(),
        1
    );
}

#[test]
fn submitted_pledge_waits_for_the_download_form() {
    // target/cpp-ref/src/app/KaHeritageBrowser.cpp:975-997 waits for "codedeta" after submission.
    // Exact fixture text: target/cpp-ref/tests/test_heritage_browser.cpp:137.
    let mut run = Session::new();
    run.reach(HeritageStage::Terms);
    let event = run.answer(
        Action::AgreeTerms,
        r#"{"status":"agreed","text":"검증용 서약문"}"#,
    );
    assert_eq!(
        event,
        Some(HeritageEvent::PledgeAgreed {
            terms: "검증용 서약문".to_owned()
        })
    );
    assert_eq!(run.machine.pledge_text(), Some("검증용 서약문"));
    assert_eq!(*run.machine.stage(), HeritageStage::TermsReady);
    let _ = run.answer(Action::FormHint, "other");
    assert_eq!(*run.machine.stage(), HeritageStage::TermsReady);
    let _ = run.answer(Action::FormHint, "codedeta");
    assert_eq!(*run.machine.stage(), HeritageStage::Kind);
}

#[test]
fn selected_kind_waits_for_replacement_form_and_positive_baseline() {
    // target/cpp-ref/tests/test_heritage_browser.cpp:318-385 keeps old/wrong forms pending;
    // the correct replacement returns baseline 37, then advances to SelectRegion.
    let mut run = Session::new();
    run.reach(HeritageStage::Kind);
    let _ = run.answer(Action::SelectKind, "selected");
    for _ in 0..2 {
        assert_eq!(*run.machine.stage(), HeritageStage::KindReady);
        let _ = run.answer(Action::KindReady, "pending");
    }
    let _ = run.answer(Action::KindReady, "ready");
    assert_eq!(*run.machine.stage(), HeritageStage::KindBaseline);
    let _ = run.answer(
        Action::ResultInfo,
        r#"{"rows":10,"lastPage":4,"total":37,"searchReady":false}"#,
    );
    assert_eq!(*run.machine.stage(), HeritageStage::Region);
    assert_eq!(
        run.requested
            .iter()
            .filter(|a| **a == Action::SelectKind)
            .count(),
        1
    );
}

#[test]
fn selected_region_requires_readback_before_search() {
    // target/cpp-ref/tests/test_heritage_browser.cpp:276-279 uses ok:47|47110.
    // target/cpp-ref/src/app/KaHeritageBrowser.cpp:1042-1056 retries after an "empty" readback.
    let mut run = Session::new();
    run.reach(HeritageStage::Region);
    let _ = run.answer(Action::SelectRegion, "selected");
    assert_eq!(*run.machine.stage(), HeritageStage::RegionReady);
    let _ = run.answer(Action::ReadRegion, "empty");
    assert_eq!(*run.machine.stage(), HeritageStage::Region);
    let _ = run.answer(Action::SelectRegion, "selected");
    let _ = run.answer(Action::ReadRegion, "ok:47|47110");
    assert_eq!(*run.machine.stage(), HeritageStage::Search);
    assert!(!run.requested.contains(&Action::Search));
}
