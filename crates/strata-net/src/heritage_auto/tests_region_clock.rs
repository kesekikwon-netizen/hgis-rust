//! Timer polls, including an outstanding script (`KaHeritageBrowser.cpp:871-885,1036-1079`).

use super::tests_region_wait::Host;
use super::{HeritageAutomation, HeritageEvent, HeritageStage, LOGIN_POLL_MS};

fn region() -> HeritageAutomation {
    let mut machine = super::tests_region_wait::Run::new().machine;
    // Enter through the real baseline callback, then ask one real Region script.
    machine.force_stage(HeritageStage::KindBaseline);
    machine.script_answered(
        machine.current_token(),
        r#"{"rows":2,"lastPage":1,"total":37,"searchReady":false}"#,
    );
    assert_eq!(machine.stage(), &HeritageStage::Region);
    machine
}

fn delayed_answer(poll: u64, value: &str) -> Option<HeritageEvent> {
    let mut machine = region();
    let call = machine
        .next_step(&mut Host, LOGIN_POLL_MS)
        .expect("first Region ask");
    for tick in 1..poll {
        assert!(machine.timers(tick * LOGIN_POLL_MS).is_empty());
        assert!(machine.next_step(&mut Host, tick * LOGIN_POLL_MS).is_none());
    }
    // The browser can deliver the answer before timers() in its current UI frame.
    machine.observe_time(poll * LOGIN_POLL_MS);
    let event = machine.script_answered(call.token, value);
    assert!(machine.script_answered(call.token, value).is_none());
    if event.is_some() {
        assert!(machine.is_over());
        assert!(!machine.accepts_files());
    }
    event
}

#[test]
fn outstanding_region_token_uses_poll_21_for_missing_sido() {
    assert_eq!(
        delayed_answer(21, "not-found"),
        Some(HeritageEvent::Stopped(
            "다운로드 폼에서 시·도 칸을 찾지 못했습니다.".into()
        ))
    );
}

#[test]
fn outstanding_region_token_uses_poll_21_for_missing_sigungu() {
    assert_eq!(
        delayed_answer(21, "sigungu-pending"),
        Some(HeritageEvent::Stopped(
            "다운로드 폼에서 시·군·구 칸을 찾지 못했습니다.".into()
        ))
    );
}

#[test]
fn outstanding_region_token_uses_poll_13_for_positive_city_count() {
    assert!(matches!(
        delayed_answer(13, "sigungu:22"),
        Some(HeritageEvent::Stopped(_))
    ));
    assert!(delayed_answer(12, "sigungu:22").is_none());
    assert!(delayed_answer(21, "sigungu:0").is_none());
}

#[test]
fn repaint_and_duplicate_observations_do_not_count_as_polls() {
    let mut machine = region();
    let call = machine
        .next_step(&mut Host, LOGIN_POLL_MS)
        .expect("first ask");
    for now in (LOGIN_POLL_MS..2 * LOGIN_POLL_MS).step_by(60) {
        machine.observe_time(now);
        machine.observe_time(now);
        assert!(machine.timers(now).is_empty());
        assert!(machine.next_step(&mut Host, now).is_none());
    }
    assert_eq!(machine.quiet_ticks, 1);
    assert!(machine.script_answered(call.token, "sigungu:22").is_none());
    assert_eq!(machine.stage(), &HeritageStage::Region);
}

#[test]
fn entering_a_new_region_phase_resets_the_poll_budget() {
    let mut machine = region();
    machine.observe_time(20 * LOGIN_POLL_MS);
    machine.force_stage(HeritageStage::KindBaseline);
    machine.script_answered(
        machine.current_token(),
        r#"{"rows":2,"lastPage":1,"total":37,"searchReady":false}"#,
    );
    let now = 21 * LOGIN_POLL_MS;
    let call = machine.next_step(&mut Host, now).expect("new Region ask");
    machine.observe_time(now);
    assert!(machine.script_answered(call.token, "not-found").is_none());
    assert_eq!(machine.quiet_ticks, 1);
}
