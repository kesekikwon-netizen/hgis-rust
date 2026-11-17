//! Observed DOM acknowledgments (`target/cpp-ref/src/app/KaHeritageBrowser.cpp:925-1147`).

use super::{
    HeritageAutomation, HeritageConfig, HeritageEvent, HeritageStage, PageHost, PageId, PageRef,
};

fn at(stage: HeritageStage) -> HeritageAutomation {
    let mut machine = machine();
    machine.force_stage(stage);
    machine
}

fn machine() -> HeritageAutomation {
    HeritageAutomation::new(HeritageConfig {
        account_id: "fixture-user".into(),
        account_password: "fixture-password".into(),
        sido: "경상북도".into(),
        city: "경주시".into(),
        fixture_base: None,
    })
}

struct Host;
impl PageHost for Host {
    fn pages(&self) -> Vec<PageRef> {
        vec![PageRef {
            id: PageId(1),
            url: "http://127.0.0.1/".into(),
            opener: None,
        }]
    }
    fn navigate(&mut self, _: &str) {}
    fn run_script(&mut self, _: PageId, _: &str) -> Option<u64> {
        None
    }
}

fn answer(machine: &mut HeritageAutomation, value: &str, tick: u64) -> Option<HeritageEvent> {
    let call = machine
        .next_step(&mut Host, tick * super::LOGIN_POLL_MS)
        .expect("next poll");
    machine.script_answered(call.token, value)
}

#[test]
fn a_submitted_login_waits_for_authenticated_dom_without_resubmitting() {
    let mut machine = machine();
    let _ = machine.next_step(&mut Host, 0);
    let _ = answer(&mut machine, "still-login", 1);
    let _ = answer(&mut machine, "submitted", 2);
    let _ = answer(&mut machine, "still-login", 3);
    assert!(!machine.is_over());
    assert_ne!(machine.stage(), &HeritageStage::LoggingIn);
    let _ = answer(&mut machine, "past-login", 4);
    assert_eq!(machine.stage(), &HeritageStage::TutorialProbe);
}

#[test]
fn tutorial_readiness_rechecks_authentication_before_dismissal() {
    let mut machine = at(HeritageStage::LoginProbe);
    let token = machine.current_token();
    let _ = machine.script_answered(token, "past-login");
    let call = machine
        .next_step(&mut Host, super::LOGIN_POLL_MS)
        .expect("authentication readback");
    assert_eq!(call.script, super::scripts::login_probe_script());
}

#[test]
fn a_loading_login_form_keeps_the_original_navigation_deadline() {
    let mut machine = machine();
    let _ = machine.next_step(&mut Host, 0);
    let _ = answer(&mut machine, "still-login", 1);
    let _ = answer(&mut machine, "no-form", 74);
    assert!(machine
        .timers(75 * super::LOGIN_POLL_MS + 1)
        .iter()
        .any(|event| matches!(event, HeritageEvent::Stopped(_))));
}

#[test]
fn the_search_button_can_arrive_after_region_readback() {
    let mut machine = at(HeritageStage::Search);
    let token = machine.current_token();
    assert!(machine.script_answered(token, "not-found").is_none());
    assert_eq!(machine.stage(), &HeritageStage::Search);
}

#[test]
fn tutorial_close_click_waits_until_the_tutorial_is_gone() {
    let mut machine = at(HeritageStage::DismissTutorial);
    let token = machine.current_token();
    assert!(machine.script_answered(token, "closed").is_none());
    assert_eq!(machine.stage(), &HeritageStage::TutorialProbe);
}

#[test]
fn opening_the_panel_waits_for_its_document() {
    let mut machine = at(HeritageStage::DownloadPage);
    let token = machine.current_token();
    assert!(machine.script_answered(token, "opening").is_none());
    assert_ne!(machine.stage(), &HeritageStage::Terms);
}

#[test]
fn submitting_the_pledge_waits_for_the_actual_download_form() {
    let mut machine = at(HeritageStage::Terms);
    let token = machine.current_token();
    assert_eq!(
        machine.script_answered(token, r#"{"status":"agreed","text":"fixture terms"}"#),
        Some(HeritageEvent::PledgeAgreed {
            terms: "fixture terms".into()
        })
    );
    assert_ne!(machine.stage(), &HeritageStage::Kind);
}

#[test]
fn selecting_a_kind_waits_for_a_new_correct_form_and_positive_baseline() {
    let mut machine = at(HeritageStage::Kind);
    let token = machine.current_token();
    assert!(machine.script_answered(token, "selected").is_none());
    assert_ne!(machine.stage(), &HeritageStage::Region);
}

#[test]
fn selecting_the_region_waits_for_values_read_back_from_the_form() {
    let mut machine = at(HeritageStage::Region);
    let token = machine.current_token();
    assert!(machine.script_answered(token, "selected").is_none());
    assert_ne!(machine.stage(), &HeritageStage::Search);
}

#[test]
fn a_search_click_cannot_authorize_a_whole_download() {
    let mut machine = at(HeritageStage::Search);
    let token = machine.current_token();
    assert!(machine.script_answered(token, "searching").is_none());
    assert!(!machine.accepts_files());
    assert_ne!(machine.stage(), &HeritageStage::DownloadAll);
}

#[test]
fn a_page_without_any_answer_stops_at_the_cpp_navigation_bound() {
    let mut machine = at(HeritageStage::DownloadPage);
    let events = machine.timers(75 * super::LOGIN_POLL_MS + 1);
    assert!(events
        .iter()
        .any(|event| matches!(event, HeritageEvent::Stopped(_))));
    assert!(machine.is_over());
}

#[test]
fn a_kind_needs_a_replaced_form_and_a_positive_baseline() {
    let mut machine = at(HeritageStage::Kind);
    let token = machine.current_token();
    let _ = machine.script_answered(token, "selected");
    let _ = answer(&mut machine, "pending", 1);
    assert_ne!(machine.stage(), &HeritageStage::Region);
    let _ = answer(&mut machine, "ready", 2);
    assert_ne!(machine.stage(), &HeritageStage::Region);
    let _ = answer(
        &mut machine,
        r#"{"rows":0,"total":-1,"lastPage":1,"searchReady":false}"#,
        3,
    );
    assert_ne!(machine.stage(), &HeritageStage::Region);
    let _ = answer(
        &mut machine,
        r#"{"rows":2,"total":37,"lastPage":1,"searchReady":false}"#,
        4,
    );
    assert_eq!(machine.stage(), &HeritageStage::Region);
}

fn through_search(machine: &mut HeritageAutomation, tick: &mut u64) {
    machine.force_stage(HeritageStage::Kind);
    let token = machine.current_token();
    let _ = machine.script_answered(token, "selected");
    for value in [
        "ready",
        r#"{"rows":2,"total":37,"lastPage":1,"searchReady":false}"#,
        "selected",
        "ok:47|4713",
        "searching",
    ] {
        *tick += 1;
        let _ = answer(machine, value, *tick);
    }
}

#[test]
fn six_proven_absences_end_with_the_last_absent_before_one_done() {
    let mut machine = machine();
    let mut tick = 0;
    for (index, kind) in [
        "지정유산",
        "현상변경허용기준",
        "매장유산유존지역",
        "문화유적분포지도",
        "지표조사구역",
        "발굴조사구역",
    ]
    .into_iter()
    .enumerate()
    {
        through_search(&mut machine, &mut tick);
        tick += 1;
        let event = answer(
            &mut machine,
            r#"{"rows":0,"total":0,"lastPage":1,"searchReady":true}"#,
            tick,
        );
        assert_eq!(event, Some(HeritageEvent::KindAbsent { kind }));
        let events = machine.timers(tick * super::LOGIN_POLL_MS);
        if index == 5 {
            assert_eq!(events, vec![HeritageEvent::Done]);
            assert_eq!(machine.stage(), &HeritageStage::Done);
        } else {
            assert!(events.is_empty());
            assert_eq!(machine.stage(), &HeritageStage::Kind);
        }
    }
    assert!(machine.timers(u64::MAX).is_empty());
}

#[test]
fn an_unchanged_nationwide_count_never_authorizes_a_download() {
    let mut machine = machine();
    let mut tick = 0;
    through_search(&mut machine, &mut tick);
    for _ in 0..21 {
        tick += 1;
        let event = answer(
            &mut machine,
            r#"{"rows":2,"total":37,"lastPage":1,"searchReady":true}"#,
            tick,
        );
        assert!(!matches!(
            event,
            Some(HeritageEvent::Done | HeritageEvent::KindAbsent { .. })
        ));
        assert!(!machine.accepts_files());
        if machine.is_over() {
            break;
        }
    }
    assert!(matches!(machine.stage(), HeritageStage::Stopped(_)));
}
