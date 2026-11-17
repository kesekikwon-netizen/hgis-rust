//! The download stages: pressing 전체다운로드, a bounded retry, and moving through the six
//! kinds (`KaHeritageBrowser.cpp:1450-1524`, `:1359-1372`).

use super::machine::{HeritageConfig, HeritageEvent, HeritageStage};
use super::{HeritageAutomation, MAX_ATTEMPTS};

fn at(stage: HeritageStage) -> HeritageAutomation {
    let mut m = HeritageAutomation::new(HeritageConfig {
        account_id: "user1".to_owned(),
        account_password: "pass1".to_owned(),
        sido: "경상북도".to_owned(),
        city: "경주시".to_owned(),
        fixture_base: None,
    });
    m.force_stage(stage);
    m
}

/// Pressing download does not produce a file by itself: it moves to the wait stage.
#[test]
fn pressing_download_moves_to_the_wait() {
    let mut m = at(HeritageStage::DownloadAll);
    let ev = m.script_answered(m.current_token(), "all");
    assert!(matches!(ev, Some(HeritageEvent::Note(_))), "{ev:?}");
    assert_eq!(m.stage(), &HeritageStage::Waiting);
    assert!(m.accepts_files(), "a file offered now would be refused");
}

/// A file offered while the run is not fetching is refused (the C++ cancels it,
/// `KaHeritageBrowser.cpp:1295-1302`).
#[test]
fn files_are_refused_outside_the_download_stages() {
    for stage in [
        HeritageStage::LoginProbe,
        HeritageStage::Terms,
        HeritageStage::Kind,
        HeritageStage::Region,
        HeritageStage::Search,
    ] {
        let m = at(stage.clone());
        assert!(!m.accepts_files(), "{stage:?} would accept a file");
    }
    for stage in [HeritageStage::DownloadAll, HeritageStage::Waiting] {
        let m = at(stage.clone());
        assert!(m.accepts_files(), "{stage:?} would refuse a file");
    }
}

/// Two buttons with the same words means the run does not guess which one to press.
#[test]
fn two_download_buttons_stop_instead_of_guessing() {
    let mut m = at(HeritageStage::DownloadAll);
    let ev = m.script_answered(m.current_token(), "ambiguous");
    let Some(HeritageEvent::Stopped(words)) = ev else {
        panic!("did not stop: {ev:?}");
    };
    assert!(words.contains("여러 개"), "{words}");
}

/// A ZIP that arrives is reported with its kind and path.
#[test]
fn a_zip_is_reported_with_its_kind() {
    let mut m = at(HeritageStage::Waiting);
    let path = std::path::PathBuf::from("C:/x/지정유산_261025.zip");
    let ev = m.zip_arrived(path.clone());
    let Some(HeritageEvent::ZipReady { kind, path: got }) = ev else {
        panic!("not a ZipReady: {ev:?}");
    };
    assert_eq!(kind, "지정유산", "the first kind is 지정유산");
    assert_eq!(got, path);
}

/// The retry waits 5 s, then 10, then stops on the third failure — where the C++ retries
/// without limit (`:1359-1372`).
#[test]
fn the_retry_waits_and_then_stops() {
    let mut m = at(HeritageStage::Waiting);

    let first = m.transfer_failed("깨진 파일", 1_000);
    let Some(HeritageEvent::Note(words)) = first else {
        panic!("no note on the first failure: {first:?}");
    };
    assert!(words.contains("5초"), "the first wait is 5 s: {words}");
    assert_eq!(m.stage(), &HeritageStage::Waiting);
    assert!(!m.retry_due(5_999), "retried before 5 s");
    assert!(m.retry_due(6_000), "did not retry after 5 s");

    let second = m.transfer_failed("깨진 파일", 7_000);
    let Some(HeritageEvent::Note(words)) = second else {
        panic!("no note on the second failure: {second:?}");
    };
    assert!(words.contains("10초"), "the second wait is 10 s: {words}");

    let third = m.transfer_failed("깨진 파일", 20_000);
    let Some(HeritageEvent::Stopped(words)) = third else {
        panic!("did not stop on the third failure: {third:?}");
    };
    assert!(words.contains("3번"), "the words do not say how many tries: {words}");
    assert_eq!(MAX_ATTEMPTS, 3);
    assert!(m.is_over());
}

/// A retry that comes due asks for the kind again rather than stopping.
#[test]
fn a_due_retry_asks_again() {
    let mut m = at(HeritageStage::Waiting);
    let _ = m.transfer_failed("깨진 파일", 1_000);
    let events = m.timers(6_000);
    assert!(events.iter().any(|e| matches!(e, HeritageEvent::Note(_))), "{events:?}");
    assert_eq!(m.stage(), &HeritageStage::Kind, "the kind is asked for again");
}

/// A server that never builds the ZIP stops the run instead of waiting forever.
#[test]
fn a_wait_that_never_ends_stops() {
    let mut m = at(HeritageStage::DownloadAll);
    let _ = m.script_answered(m.current_token(), "all");
    assert!(m.timers(super::machine_kinds::WAIT_LIMIT_MS + 1).iter().any(|e| matches!(e, HeritageEvent::Stopped(_))));
    assert!(m.is_over());
}

/// The six kinds run in the C++ tab order, and the run finishes after the last.
#[test]
fn the_six_kinds_run_in_order_and_finish() {
    let mut m = at(HeritageStage::Kind);
    let mut seen = Vec::new();
    for _ in 0..6 {
        seen.push(m.current_kind());
        let _ = m.kind_done();
    }
    assert_eq!(
        seen,
        vec![
            "지정유산",
            "현상변경허용기준",
            "매장유산유존지역",
            "문화유적분포지도",
            "지표조사구역",
            "발굴조사구역"
        ]
    );
    assert_eq!(m.stage(), &HeritageStage::Done);
    assert!(m.is_over());
}
