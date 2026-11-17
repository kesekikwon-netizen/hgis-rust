//! Search acceptance from `target/cpp-ref/src/app/KaHeritageBrowser.cpp:1085-1147,1261-1268`.

use strata_net::heritage_auto::{HeritageEvent, HeritageStage};

use super::driver::{Action, Session, FILTERED};

#[test]
fn search_waits_for_refreshed_dom_before_whole_download() {
    // target/cpp-ref/src/app/KaHeritageBrowser.cpp:1105 returns until searchReady.
    // The independently recorded count transition is 8302 -> 90 at :1124-1126.
    let mut run = Session::new();
    run.reach(HeritageStage::Search);
    let _ = run.answer(Action::Search, "searching");
    assert!(
        !run.machine.accepts_files(),
        "clicking search does not acknowledge its result"
    );
    let _ = run.answer(
        Action::ResultInfo,
        r#"{"rows":10,"lastPage":9,"total":90,"searchReady":false}"#,
    );
    assert!(
        !run.machine.accepts_files(),
        "old DOM must not authorize a whole download"
    );
    let _ = run.answer(Action::ResultInfo, FILTERED);
    let _ = run.answer(Action::Download, "all");
    assert_eq!(*run.machine.stage(), HeritageStage::Waiting);
    assert_eq!(
        run.requested
            .iter()
            .filter(|a| **a == Action::Download)
            .count(),
        1
    );
}

fn rejected_count_never_downloads(reply: &str) {
    let mut run = Session::new();
    run.reach(HeritageStage::Search);
    let _ = run.answer(Action::Search, "searching");
    // target/cpp-ref/src/app/KaHeritageBrowser.cpp:1115-1133 allows >20 polling ticks,
    // then stops instead of downloading. Twenty-four exact 700ms ticks cover that boundary.
    for _ in 0..24 {
        if run.machine.is_over() {
            break;
        }
        let _ = run.answer(Action::ResultInfo, reply);
        assert!(
            !run.machine.accepts_files(),
            "unproven city filter must not authorize download"
        );
    }
    assert!(matches!(run.machine.stage(), HeritageStage::Stopped(_)));
    assert!(!run.requested.contains(&Action::Download));
}

#[test]
fn unchanged_nationwide_count_cannot_trigger_whole_download() {
    // target/cpp-ref/src/app/KaHeritageBrowser.cpp:1124-1133 rejects unchanged positive totals.
    rejected_count_never_downloads(r#"{"rows":10,"lastPage":831,"total":8302,"searchReady":true}"#);
}

#[test]
fn unreadable_result_count_cannot_trigger_whole_download() {
    // target/cpp-ref/src/app/KaHeritageBrowser.cpp:1115-1122 treats total=-1 as unreadable.
    rejected_count_never_downloads(r#"{"rows":10,"lastPage":1,"total":-1,"searchReady":true}"#);
}

#[test]
fn proven_zero_results_advance_without_a_download() {
    // target/cpp-ref/src/app/KaHeritageBrowser.cpp:1108-1112 advances only with a positive baseline.
    let mut run = Session::new();
    run.reach(HeritageStage::Search);
    let first_kind = run.machine.current_kind();
    let _ = run.answer(Action::Search, "searching");
    let event = run.answer(
        Action::ResultInfo,
        r#"{"rows":0,"lastPage":1,"total":0,"searchReady":true}"#,
    );
    assert_ne!(run.machine.current_kind(), first_kind);
    assert_eq!(event, Some(HeritageEvent::KindAbsent { kind: first_kind }));
    assert_eq!(*run.machine.stage(), HeritageStage::Kind);
    assert!(!run.requested.contains(&Action::Download));
}

#[test]
fn nonpositive_kind_baseline_cannot_start_region_search() {
    // target/cpp-ref/src/app/KaHeritageBrowser.cpp:1261-1268 requires total>0 before SelectRegion.
    let mut run = Session::new();
    run.reach(HeritageStage::Kind);
    let _ = run.answer(Action::SelectKind, "selected");
    let _ = run.answer(Action::KindReady, "ready");
    for total in [-1, 0] {
        let reply = format!(r#"{{"rows":0,"lastPage":1,"total":{total},"searchReady":false}}"#);
        let _ = run.answer(Action::ResultInfo, &reply);
        assert_eq!(*run.machine.stage(), HeritageStage::KindBaseline);
        assert!(!run.requested.contains(&Action::SelectRegion));
    }
    let _ = run.answer(
        Action::ResultInfo,
        r#"{"rows":10,"lastPage":4,"total":37,"searchReady":false}"#,
    );
    assert_eq!(*run.machine.stage(), HeritageStage::Region);
}
