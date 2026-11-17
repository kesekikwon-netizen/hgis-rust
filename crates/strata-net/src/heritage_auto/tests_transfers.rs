//! Group completion (`target/cpp-ref/src/app/KaHeritageBrowser.cpp:1150-1217`).

use super::{HeritageAutomation, HeritageConfig, HeritageEvent, HeritageStage};
use std::path::PathBuf;

fn waiting() -> HeritageAutomation {
    let mut run = HeritageAutomation::new(HeritageConfig {
        account_id: "fixture-user".into(),
        account_password: "fixture-password".into(),
        sido: "경상북도".into(),
        city: "경주시".into(),
        fixture_base: None,
    });
    run.force_stage(HeritageStage::Waiting);
    run
}

#[test]
fn completing_one_of_two_offers_cannot_publish_a_group() {
    let mut run = waiting();
    run.file_offered("first.zip".into());
    run.file_offered("second.zip".into());
    assert!(
        run.zip_arrived("first.zip".into()).is_none(),
        "one transfer is still pending"
    );
    assert!(run.timers(60_000).is_empty());
    assert_eq!(run.stage(), &HeritageStage::Waiting);
    assert_eq!(run.current_kind(), "지정유산");
}

#[test]
fn a_pending_transfer_survives_the_old_generation_deadline() {
    let mut run = waiting();
    run.file_offered("large.zip".into());
    assert!(
        run.timers(150_001).is_empty(),
        "accepted transfer is still receiving"
    );
    assert_eq!(run.stage(), &HeritageStage::Waiting);
    assert!(run.accepts_files());
}

#[test]
fn six_quiet_polls_publish_one_ordered_group_and_wait_for_import() {
    let mut run = waiting();
    run.file_offered("first.zip".into());
    run.file_offered("second.zip".into());
    // Completion order differs from offer order; the group preserves accepted offers.
    let _ = run.zip_arrived("second.zip".into());
    let _ = run.zip_arrived("first.zip".into());
    for now in [60, 700, 1400, 2100, 2800, 3500, 4199] {
        assert!(
            run.timers(now).is_empty(),
            "quiet window ended early at {now}ms"
        );
    }
    assert_eq!(
        run.timers(4200),
        vec![HeritageEvent::DatasetReady {
            kind: "지정유산",
            files: vec![PathBuf::from("first.zip"), PathBuf::from("second.zip")],
        }]
    );
    assert_eq!(run.stage(), &HeritageStage::AwaitingImport);
    assert!(!run.accepts_files());
    assert!(run.timers(1_000_000).is_empty());
    assert_eq!(run.current_kind(), "지정유산");
}
