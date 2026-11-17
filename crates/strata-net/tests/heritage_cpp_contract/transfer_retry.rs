//! Backoff from `target/cpp-ref/src/app/KaHeritageBrowser.cpp:504-520` and import rejection :1209-1214.

use strata_net::heritage_auto::{DatasetOutcome, HeritageEvent, HeritageStage};

use super::transfer_driver::TransferRun;

#[test]
fn retry_backoff_saturates_at_thirty_seconds_without_skipping_the_kind() {
    // Break caught: retry three terminates, a later delay grows past 30s, or a retry skips a kind.
    let mut run = TransferRun::waiting();
    for (attempt, delay) in [
        (1, 5_000),
        (2, 10_000),
        (3, 20_000),
        (4, 30_000),
        (5, 30_000),
        (6, 30_000),
    ] {
        let at = run.session.now_ms;
        run.session.machine.observe_time(at);
        let event = run.session.machine.transfer_failed("손상된 ZIP", at);
        assert!(
            !run.session.machine.is_over(),
            "attempt {attempt} remains recoverable"
        );
        assert!(!matches!(event, Some(HeritageEvent::Stopped(_))));
        assert_eq!(run.session.machine.current_kind(), "지정유산");
        assert!(!run.session.machine.retry_due(at + delay - 1));
        assert!(
            run.session.machine.retry_due(at + delay),
            "attempt {attempt} delay is {delay}ms"
        );
        run.session.now_ms = at + delay;
        run.drain();
        assert!(run.session.machine.session_restart_required());
        // Session's dependency-specific adapter must reopen then replay every real login action.
        run.session.reach(HeritageStage::Waiting);
        assert_eq!(run.session.machine.current_kind(), "지정유산");
        run.assert_no_success();
    }
}

#[test]
fn retryable_import_reopens_only_the_current_kind_and_rejects_old_groups() {
    // Break caught: import retry marks success/absence, reruns completed kinds, or accepts a stale ACK.
    // C++ tests/test_heritage_download_retry.cpp:167-207 preserves datasetIndex on bad ZIP.
    let (mut run, completed) = TransferRun::one_file_group();
    assert!(run.loaded(&completed, DatasetOutcome::Accepted));
    run.drain();
    run.session.reach(HeritageStage::Waiting);
    assert_eq!(run.session.machine.current_kind(), "현상변경허용기준");
    let path = run.offer("fixture-second-kind-first-attempt.zip");
    run.complete(&path);
    for _ in 0..6 {
        run.tick();
    }
    let stale = vec![path];
    assert_eq!(
        run.groups().last(),
        Some(&("현상변경허용기준", stale.clone()))
    );
    let at = run.session.now_ms;
    assert!(run.loaded(
        &stale,
        DatasetOutcome::RetryableFailure {
            reason: "손상된 ZIP".into()
        }
    ));
    assert!(!run.loaded(&stale, DatasetOutcome::Accepted));
    run.drain();
    run.assert_no_success();
    assert!(!run.session.machine.retry_due(at + 4_999));
    assert!(run.session.machine.retry_due(at + 5_000));
    run.session.now_ms = at + 5_000;
    run.drain();
    assert!(run.session.machine.session_restart_required());
    run.session.reach(HeritageStage::Waiting);
    assert_eq!(run.session.machine.current_kind(), "현상변경허용기준");
    let fresh = run.offer("fixture-second-kind-fresh-attempt.zip");
    run.complete(&fresh);
    for _ in 0..6 {
        run.tick();
    }
    assert!(!run.loaded(&completed, DatasetOutcome::Accepted));
    assert!(!run.loaded(&stale, DatasetOutcome::Accepted));
    assert_eq!(run.session.machine.current_kind(), "현상변경허용기준");
    assert_eq!(
        run.groups().last(),
        Some(&("현상변경허용기준", vec![fresh.clone()]))
    );
    assert!(run.loaded(&[fresh], DatasetOutcome::Accepted));
    run.drain();
    assert_eq!(run.session.machine.current_kind(), "매장유산유존지역");
    run.assert_no_success();
}
