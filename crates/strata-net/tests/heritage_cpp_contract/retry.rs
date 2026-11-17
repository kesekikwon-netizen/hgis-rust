//! Retry boundary from `target/cpp-ref/tests/test_heritage_download_retry.cpp:305-325`.

use strata_net::heritage_auto::{HeritageEvent, HeritageStage};

use super::driver::Session;

#[test]
fn third_transfer_failure_remains_recoverable() {
    // target/cpp-ref/src/app/KaHeritageBrowser.cpp:504-520 schedules 5/10/20/30 seconds;
    // the third failure changes the notice, not the terminal state. No clock tolerance is needed.
    let mut run = Session::new();
    run.reach(HeritageStage::Waiting);
    let kind = run.machine.current_kind();
    for (attempt, delay_ms) in [(1, 5_000), (2, 10_000), (3, 20_000)] {
        let at = run.now_ms;
        let event = run.machine.transfer_failed("손상된 ZIP", at);
        assert!(
            !run.machine.is_over(),
            "transfer failure {attempt} must schedule recovery"
        );
        assert!(!matches!(event, Some(HeritageEvent::Stopped(_))));
        assert_eq!(run.machine.current_kind(), kind);
        let _ = run.machine.timers(at + delay_ms - 1);
        assert!(!run.machine.retry_due(at + delay_ms - 1));
        assert!(
            run.machine.retry_due(at + delay_ms),
            "retry {attempt} must become due after {delay_ms}ms"
        );
        let _ = run.machine.timers(at + delay_ms);
        run.now_ms = at + delay_ms;
        run.reach(HeritageStage::Waiting);
        assert_eq!(run.machine.current_kind(), kind);
    }
}
