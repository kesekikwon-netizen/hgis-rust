//! Terminal events and actual agreements (`KaHeritageBrowser.cpp:985-1001,1259-1280`).

use super::{HeritageAutomation, HeritageConfig, HeritageEvent, HeritageStage};

fn machine(stage: HeritageStage) -> HeritageAutomation {
    let mut machine = HeritageAutomation::new(HeritageConfig {
        account_id: "fixture-user".to_owned(),
        account_password: "fixture-password".to_owned(),
        sido: "경상북도".to_owned(),
        city: "경주시".to_owned(),
        fixture_base: None,
    });
    machine.force_stage(stage);
    machine
}

#[test]
fn an_actual_agreement_emits_the_exact_terms_once() {
    let mut machine = machine(HeritageStage::Terms);
    let answer = r#"{"status":"agreed","text":"원본자료 사용 서약서\n확인"}"#;
    let token = machine.current_token();
    assert_eq!(
        machine.script_answered(token, answer),
        Some(HeritageEvent::PledgeAgreed {
            terms: "원본자료 사용 서약서\n확인".to_owned()
        })
    );
    assert!(machine.script_answered(token, answer).is_none());
}

#[test]
fn a_browser_json_string_delivers_the_exact_pledge_terms() {
    let mut machine = machine(HeritageStage::Terms);
    let answer =
        r#""{\"status\":\"agreed\",\"text\":\"원본자료 \\\"사용\\\"\\n서약서 \\\\자료\"}""#;
    let token = machine.current_token();
    assert_eq!(
        machine.script_answered(token, answer),
        Some(HeritageEvent::PledgeAgreed {
            terms: "원본자료 \"사용\"\n서약서 \\자료".to_owned(),
        })
    );
}

#[test]
fn malformed_browser_pledges_never_invent_agreement() {
    for answer in ["", "null", r#""{broken}""#, r#""{\"status\":\"agreed\"}""#] {
        let mut machine = machine(HeritageStage::Terms);
        let token = machine.current_token();
        assert!(!matches!(
            machine.script_answered(token, answer),
            Some(HeritageEvent::PledgeAgreed { .. })
        ));
        assert!(machine.pledge_text().is_none());
    }
}

#[test]
fn an_already_open_panel_does_not_invent_an_agreement() {
    let mut machine = machine(HeritageStage::Terms);
    let token = machine.current_token();
    assert!(machine.script_answered(token, "already-open").is_none());
    assert!(machine.pledge_text().is_none());
}

#[test]
fn the_last_zip_precedes_one_done_event() {
    let mut machine = machine(HeritageStage::Waiting);
    for _ in 0..5 {
        let _ = machine.zip_arrived("fixture.zip".into());
        let _ = machine.kind_done();
        machine.force_stage(HeritageStage::Waiting);
    }
    assert!(matches!(
        machine.zip_arrived("last.zip".into()),
        Some(HeritageEvent::ZipReady {
            kind: "발굴조사구역",
            ..
        })
    ));
    assert_eq!(machine.kind_done(), Some(HeritageEvent::Done));
    assert!(machine.kind_done().is_none());
}

#[test]
fn a_stopped_run_cannot_finish_or_report_a_late_zip() {
    let mut machine = machine(HeritageStage::Stopped("fixture stop".to_owned()));
    assert!(machine.zip_arrived("late.zip".into()).is_none());
    assert!(machine.kind_done().is_none());
    assert!(matches!(machine.stage(), HeritageStage::Stopped(_)));
}

#[test]
fn a_kind_without_a_received_file_cannot_be_completed() {
    let mut machine = machine(HeritageStage::Waiting);
    assert!(machine.kind_done().is_none());
    assert_eq!(machine.current_kind(), "지정유산");
    assert_eq!(machine.stage(), &HeritageStage::Waiting);
}

#[test]
fn cancellation_happens_once_and_discards_late_answers_and_files() {
    let mut machine = machine(HeritageStage::Terms);
    let token = machine.current_token();
    assert_eq!(machine.cancel(), Some(HeritageEvent::Cancelled));
    assert_eq!(machine.stage(), &HeritageStage::Cancelled);
    assert!(machine.is_over());
    assert!(!machine.accepts_files());
    assert!(machine.cancel().is_none());
    assert!(machine
        .script_answered(token, r#"{"status":"agreed","text":"late"}"#)
        .is_none());
    assert!(machine.zip_arrived("late.zip".into()).is_none());
    assert!(machine.kind_done().is_none());
    assert!(machine.transfer_failed("late failure", 0).is_none());
    assert!(machine.timers(u64::MAX).is_empty());
}
