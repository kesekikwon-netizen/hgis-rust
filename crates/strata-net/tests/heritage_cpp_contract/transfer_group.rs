//! Group readiness from `target/cpp-ref/src/app/KaHeritageBrowser.cpp:1150-1217,1351-1395`.

use super::transfer_driver::TransferRun;

#[test]
fn two_completed_offers_emit_one_group_after_six_quiet_polls() {
    // Break caught: one file triggers readiness, pending transfers are ignored, or six becomes five.
    let mut run = TransferRun::waiting();
    let first = run.offer("fixture-first.zip");
    let second = run.offer("fixture-second.zip");
    run.complete(&first);
    // C++ :1354 consumes a completion once; a duplicate cannot finish the second offer.
    run.complete(&first);
    for _ in 0..6 {
        run.tick();
        assert!(
            run.groups().is_empty(),
            "another accepted transfer is still pending"
        );
        assert_eq!(run.session.machine.current_kind(), "지정유산");
    }
    run.complete(&second);
    for _ in 0..5 {
        run.tick();
        assert!(
            run.groups().is_empty(),
            "five quiet polls cannot finalize a group"
        );
    }
    run.tick();
    assert_eq!(run.groups(), vec![("지정유산", vec![first, second])]);
    for _ in 0..12 {
        run.tick();
    }
    assert_eq!(run.groups().len(), 1, "DatasetReady must be emitted once");
    assert_eq!(run.session.machine.current_kind(), "지정유산");
    run.assert_no_success();
    run.assert_no_script();
}

#[test]
fn a_pending_transfer_outlives_the_no_file_wait_deadline() {
    // Break caught: a 150-second no-file timeout terminates a transfer that is still active.
    // C++ :1151-1154 resets both wait counters while transfers remain. 300*700ms is 210s.
    let mut run = TransferRun::waiting();
    let path = run.offer("fixture-pending.zip");
    for _ in 0..300 {
        run.tick();
        assert!(
            !run.session.machine.is_over(),
            "an active transfer is still receiving"
        );
        assert!(run.groups().is_empty());
    }
    run.complete(&path);
    for _ in 0..5 {
        run.tick();
        assert!(run.groups().is_empty());
    }
    run.tick();
    assert_eq!(run.groups(), vec![("지정유산", vec![path])]);
    run.assert_no_success();
}

#[test]
fn repeated_frames_at_one_time_do_not_replace_six_quiet_polls() {
    // Break caught: repaint count, rather than C++ :103 timer polls, advances the idle budget.
    let mut run = TransferRun::waiting();
    let path = run.offer("fixture-repaint.zip");
    run.complete(&path);
    for _ in 0..100 {
        run.drain();
        assert!(run.groups().is_empty());
    }
    for _ in 0..5 {
        run.tick();
        for _ in 0..10 {
            run.drain();
        }
        assert!(run.groups().is_empty());
    }
    run.tick();
    assert_eq!(run.groups(), vec![("지정유산", vec![path])]);
}
