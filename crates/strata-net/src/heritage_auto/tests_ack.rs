//! Asynchronous import outcomes (`target/cpp-ref/src/app/KaHeritageBrowser.cpp:491-562,1206-1217`).

use super::{DatasetOutcome, HeritageAutomation, HeritageEvent, HeritageStage};
use std::path::PathBuf;

fn pending_group() -> (HeritageAutomation, Vec<PathBuf>) {
    let mut run = super::tests_transfers::waiting();
    let files = vec![PathBuf::from("first.zip"), PathBuf::from("second.zip")];
    for file in &files {
        run.file_offered(file.clone());
        assert!(run.zip_arrived(file.clone()).is_none());
    }
    assert_eq!(
        run.timers(4200),
        vec![HeritageEvent::DatasetReady {
            kind: "지정유산",
            files: files.clone()
        }]
    );
    (run, files)
}

#[test]
fn wrong_ack_identity_does_not_consume_the_pending_group() {
    let (mut run, files) = pending_group();
    for wrong in [
        vec![],
        vec![files[0].clone()],
        vec![files[1].clone(), files[0].clone()],
    ] {
        assert!(!run.dataset_loaded(&wrong, DatasetOutcome::Accepted));
    }
    assert!(run.dataset_loaded(&files, DatasetOutcome::Accepted));
    assert!(!run.dataset_loaded(&files, DatasetOutcome::Accepted));
    assert_eq!(run.current_kind(), "현상변경허용기준");
    assert!(run
        .timers(4200)
        .iter()
        .all(|event| !matches!(event, HeritageEvent::Done)));
}

#[test]
fn fatal_ack_queues_one_original_reason_and_never_retries() {
    let (mut run, files) = pending_group();
    assert!(run.dataset_loaded(
        &files,
        DatasetOutcome::FatalFailure {
            reason: "fixture original I/O cause".into()
        }
    ));
    assert_eq!(
        run.timers(4200),
        vec![HeritageEvent::Stopped("fixture original I/O cause".into())]
    );
    assert!(run.timers(u64::MAX).is_empty());
    assert!(!run.dataset_loaded(&files, DatasetOutcome::Accepted));
    assert!(!run.retry_due(u64::MAX));
    assert!(!run.session_restart_required());
}

#[test]
fn retry_ack_preserves_the_kind_and_reopens_only_after_backoff() {
    let (mut run, files) = pending_group();
    assert!(run.dataset_loaded(
        &files,
        DatasetOutcome::RetryableFailure {
            reason: "fixture truncated ZIP".into()
        }
    ));
    assert_eq!(run.stage(), &HeritageStage::RetryWait);
    assert_eq!(run.current_kind(), "지정유산");
    assert!(run.session_restart_required());
    assert!(!run.dataset_loaded(&files, DatasetOutcome::Accepted));
    assert!(!run.session_reopened(9199));
    assert!(run.session_reopened(9200));
    assert_eq!(run.stage(), &HeritageStage::Idle);
    assert!(!run.session_reopened(9200));
    assert!(!run.dataset_loaded(&files, DatasetOutcome::Accepted));
}

#[test]
fn cancel_during_retry_prevents_reopening_and_late_ack() {
    let (mut run, files) = pending_group();
    assert!(run.dataset_loaded(
        &files,
        DatasetOutcome::RetryableFailure {
            reason: "fixture truncated ZIP".into()
        }
    ));
    assert_eq!(run.cancel(), Some(HeritageEvent::Cancelled));
    assert!(!run.session_restart_required());
    assert!(!run.session_reopened(u64::MAX));
    assert!(!run.dataset_loaded(&files, DatasetOutcome::Accepted));
    assert!(run.timers(u64::MAX).is_empty());
}
