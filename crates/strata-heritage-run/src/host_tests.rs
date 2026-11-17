//! Document dispatch regressions (`target/cpp-ref/src/app/KaHeritageBrowser.cpp:639-778`).
#![allow(clippy::expect_used)]

use super::*;

fn page(id: u32) -> PageRef {
    PageRef {
        id: PageId(id),
        url: "https://a.example/".into(),
        opener: None,
    }
}

fn batch(stage: HeritageStage) -> QueuedHost {
    let pages = vec![page(1), page(2)];
    QueuedHost {
        pending: Some(Pending {
            ticket: Ticket {
                epoch: SessionEpoch(0),
                token: 7,
            },
            request: EngineCall {
                page: PageId(1),
                script: "guarded action".into(),
                token: 7,
            },
            scope: Scope::for_stage(&stage),
            phase: Phase::Probing(vec![
                Probe {
                    page: PageId(1),
                    call: Some(BrowserCall(1)),
                    facts: None,
                },
                Probe {
                    page: PageId(2),
                    call: Some(BrowserCall(2)),
                    facts: None,
                },
            ]),
        }),
        pages,
        ..QueuedHost::default()
    }
}

const FORM: &str =
    r#"{"form":true,"agreement":false,"login":false,"menu":false,"authenticated":false}"#;
const MENU: &str =
    r#"{"form":false,"agreement":false,"login":false,"menu":true,"authenticated":true}"#;

#[test]
fn form_owner_wins_after_every_probe_regardless_of_callback_order() {
    let mut host = batch(HeritageStage::Kind);
    assert!(host.record_probe(SessionEpoch(0), BrowserCall(2), FORM));
    assert!(
        host.take_ready(true).is_none(),
        "one probe is still pending"
    );
    assert!(host.record_probe(SessionEpoch(0), BrowserCall(1), MENU));
    assert_eq!(host.take_ready(true).expect("resolved").1, Some(PageId(2)));
    assert!(host.take_ready(true).is_none(), "dispatch is consumed once");
}

#[test]
fn tied_forms_use_inventory_order_not_reply_order() {
    let mut host = batch(HeritageStage::Search);
    host.record_probe(SessionEpoch(0), BrowserCall(2), FORM);
    host.record_probe(SessionEpoch(0), BrowserCall(1), FORM);
    assert_eq!(host.take_ready(true).expect("resolved").1, Some(PageId(1)));
}

#[test]
fn duplicate_probe_cannot_replace_a_fact_or_complete_another_probe() {
    let mut host = batch(HeritageStage::Kind);
    assert!(host.record_probe(SessionEpoch(0), BrowserCall(1), MENU));
    assert!(!host.record_probe(SessionEpoch(0), BrowserCall(1), FORM));
    assert!(host.take_ready(true).is_none());
    host.record_probe(SessionEpoch(0), BrowserCall(2), MENU);
    assert_eq!(host.take_ready(true).expect("finishEmpty").1, None);
}

#[test]
fn missing_or_removed_form_finishes_discovery_without_acting() {
    let mut host = batch(HeritageStage::Region);
    host.record_probe(SessionEpoch(0), BrowserCall(1), MENU);
    host.record_probe(SessionEpoch(0), BrowserCall(2), FORM);
    host.pages.pop();
    assert_eq!(host.take_ready(true).expect("finishEmpty").1, None);
}

#[test]
fn removed_unanswered_view_does_not_leave_discovery_pending() {
    let mut host = batch(HeritageStage::Kind);
    host.record_probe(SessionEpoch(0), BrowserCall(1), FORM);
    host.pages.pop();
    assert_eq!(
        host.take_ready(true).expect("remaining owner").1,
        Some(PageId(1))
    );
}

#[test]
fn phase_invalid_token_and_reset_epoch_cannot_dispatch() {
    let mut host = batch(HeritageStage::Kind);
    host.record_probe(SessionEpoch(0), BrowserCall(1), FORM);
    host.record_probe(SessionEpoch(0), BrowserCall(2), FORM);
    assert!(host.take_ready(false).is_none());
    assert!(host.take_ready(true).is_none());
    host = batch(HeritageStage::Kind);
    host.reset_documents();
    assert_eq!(host.epoch(), SessionEpoch(1));
    assert!(!host.record_probe(SessionEpoch(0), BrowserCall(1), FORM));
    assert!(host.take_ready(true).is_none());
}

#[test]
fn old_epoch_does_not_answer_a_new_batch_with_reused_browser_call_ids() {
    let mut host = batch(HeritageStage::Kind);
    host.epoch = SessionEpoch(1);
    host.pending.as_mut().expect("batch").ticket.epoch = SessionEpoch(1);
    assert!(!host.record_probe(SessionEpoch(0), BrowserCall(1), FORM));
    assert!(host.take_ready(true).is_none());
}

fn acting() -> (QueuedHost, HeritageAutomation) {
    let mut host = QueuedHost {
        pages: vec![page(1)],
        ..QueuedHost::default()
    };
    let mut machine = HeritageAutomation::new(strata_net::heritage_auto::HeritageConfig {
        account_id: "fixture".into(),
        account_password: "fixture".into(),
        sido: "fixture".into(),
        city: "fixture".into(),
        fixture_base: None,
    });
    assert!(machine.next_step(&mut host, 0).is_none());
    let request = machine.next_step(&mut host, 700).expect("login probe");
    host.pending = Some(Pending {
        ticket: Ticket {
            epoch: host.epoch,
            token: request.token,
        },
        request,
        scope: Scope::General,
        phase: Phase::Acting {
            page: PageId(1),
            call: BrowserCall(8),
        },
    });
    (host, machine)
}

#[test]
fn cancelled_action_reply_never_reaches_the_machine() {
    let (mut host, mut machine) = acting();
    machine.cancel();
    assert!(host
        .action_answer(SessionEpoch(0), BrowserCall(8), "past-login", &mut machine)
        .is_none());
    assert_eq!(*machine.stage(), HeritageStage::Cancelled);
    assert!(host.pending.is_none());
}

#[test]
fn removed_action_view_cannot_report_authentication() {
    let (mut host, mut machine) = acting();
    host.pages.clear();
    assert!(host
        .action_answer(SessionEpoch(0), BrowserCall(8), "past-login", &mut machine)
        .is_none());
    assert_eq!(*machine.stage(), HeritageStage::LoginProbe);
    assert!(host.pending.is_none());
}

#[test]
fn action_answer_is_forwarded_once_with_its_original_token() {
    let (mut host, mut machine) = acting();
    assert!(matches!(
        host.action_answer(SessionEpoch(0), BrowserCall(8), "past-login", &mut machine),
        Some(HeritageEvent::Note(_))
    ));
    assert!(host
        .action_answer(SessionEpoch(0), BrowserCall(8), "past-login", &mut machine)
        .is_none());
    assert_eq!(*machine.stage(), HeritageStage::TutorialProbe);
}

#[test]
fn queued_host_keeps_original_navigation_and_tokens() {
    let mut host = QueuedHost {
        pages: vec![page(1)],
        ..QueuedHost::default()
    };
    assert_eq!(host.run_script(PageId(1), "1"), Some(1));
    assert_eq!(host.run_script(PageId(7), "2"), None);
    assert_eq!(host.run_script(PageId(1), "3"), Some(2));
    host.navigate("https://b.example/");
    assert_eq!(host.wanted.len(), 3);
    assert_eq!(
        host.wanted.last(),
        Some(&Wanted::Navigate("https://b.example/".into()))
    );
    host.wanted.clear();
    assert_eq!(host.run_script(PageId(1), "4"), Some(3));
}
