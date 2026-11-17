//! Import/retry decisions from `target/cpp-ref/src/app/KaHeritageBrowser.cpp:491-520,1206-1214`.

use strata_net::heritage_auto::{DatasetOutcome, HeritageEvent};

use super::transfer_driver::TransferRun;

#[test]
fn only_the_exact_group_can_be_accepted_once() {
    // Break caught: an incomplete, reordered, or already consumed group advances the kind.
    // Identity is the approved HER-API2 asynchronous equivalent of the C++ generation guard.
    let mut run = TransferRun::waiting();
    let first = run.offer("fixture-first.zip");
    let second = run.offer("fixture-second.zip");
    run.complete(&first);
    run.complete(&second);
    for _ in 0..6 {
        run.tick();
    }
    let files = vec![first.clone(), second.clone()];
    assert_eq!(run.groups(), vec![("지정유산", files.clone())]);
    assert!(!run.session.machine.accepts_files());
    let late = run.offer("fixture-late-after-group.zip");
    run.complete(&late);
    run.drain();
    assert_eq!(run.groups(), vec![("지정유산", files.clone())]);
    for wrong in [vec![], vec![first], vec![second, files[0].clone()]] {
        assert!(!run.loaded(&wrong, DatasetOutcome::Accepted));
        run.drain();
        assert_eq!(run.session.machine.current_kind(), "지정유산");
        run.assert_no_success();
    }
    run.assert_no_script();
    assert!(run.loaded(&files, DatasetOutcome::Accepted));
    assert!(!run.loaded(&files, DatasetOutcome::Accepted));
    run.drain();
    assert_eq!(run.session.machine.current_kind(), "현상변경허용기준");
    assert!(!run.loaded(&files, DatasetOutcome::Accepted));
    run.assert_no_success();
}

#[test]
fn cancelled_group_cannot_be_accepted_or_replace_cancellation() {
    // Break caught: late import completion wins over C++ stop generation invalidation (:479-488).
    let (mut run, files) = TransferRun::one_file_group();
    assert_eq!(run.session.machine.cancel(), Some(HeritageEvent::Cancelled));
    assert!(run.session.machine.cancel().is_none());
    for outcome in [
        DatasetOutcome::Accepted,
        DatasetOutcome::RetryableFailure {
            reason: "손상된 ZIP".into(),
        },
        DatasetOutcome::FatalFailure {
            reason: "저장 공간 부족".into(),
        },
    ] {
        assert!(!run.loaded(&files, outcome));
    }
    run.complete(&files[0]);
    run.tick();
    assert_eq!(run.session.machine.current_kind(), "지정유산");
    assert_eq!(
        run.session.machine.stage(),
        &strata_net::heritage_auto::HeritageStage::Cancelled
    );
    assert!(!run.session.machine.accepts_files());
    run.assert_no_success();
}

#[test]
fn fatal_import_failure_stops_once_without_retry_or_advancement() {
    // Break caught: rejected import becomes Done, gets retried, or loses the original cause.
    // C++ tests/test_heritage_browser.cpp:294-315 and test_heritage_download_retry.cpp:348-355.
    let (mut run, files) = TransferRun::one_file_group();
    assert!(run.loaded(
        &files,
        DatasetOutcome::FatalFailure {
            reason: "저장 공간 부족".into()
        }
    ));
    assert!(!run.loaded(&files, DatasetOutcome::Accepted));
    run.drain();
    assert_eq!(
        run.events
            .iter()
            .filter(|e| matches!(e, HeritageEvent::Stopped(_)))
            .cloned()
            .collect::<Vec<_>>(),
        vec![HeritageEvent::Stopped("저장 공간 부족".into())]
    );
    assert!(run.session.machine.is_over());
    assert_eq!(run.session.machine.current_kind(), "지정유산");
    assert!(!run.session.machine.retry_due(run.session.now_ms + 30_000));
    assert!(!run.session.machine.session_restart_required());
    run.tick();
    assert_eq!(
        run.events
            .iter()
            .filter(|e| matches!(e, HeritageEvent::Stopped(_)))
            .count(),
        1
    );
    run.assert_no_success();
}

#[test]
fn the_last_group_cannot_emit_done_before_import_acceptance() {
    // Break caught: last ZIP completion becomes success before C++ :1209-1214 import can reject it.
    let mut run = TransferRun::waiting();
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
        assert_eq!(run.session.machine.current_kind(), kind);
        let path = run.offer(&format!("fixture-kind-{index}.zip"));
        run.complete(&path);
        for _ in 0..6 {
            run.tick();
        }
        assert_eq!(run.groups().last(), Some(&(kind, vec![path.clone()])));
        run.assert_no_success();
        assert_eq!(run.session.machine.current_kind(), kind);
        run.assert_no_script();
        assert!(run.loaded(std::slice::from_ref(&path), DatasetOutcome::Accepted));
        assert!(!run.loaded(std::slice::from_ref(&path), DatasetOutcome::Accepted));
        run.drain();
        if index < 5 {
            run.assert_no_success();
            run.session
                .reach(strata_net::heritage_auto::HeritageStage::Waiting);
        }
    }
    for _ in 0..12 {
        run.tick();
    }
    assert_eq!(run.groups().len(), 6);
    assert_eq!(
        run.events
            .iter()
            .filter(|e| matches!(e, HeritageEvent::Done))
            .count(),
        1
    );
    assert!(run.session.machine.is_over());
}
