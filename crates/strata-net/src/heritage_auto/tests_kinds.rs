//! The download stages: pressing 전체다운로드, a bounded retry, and moving through the six
//! kinds (`KaHeritageBrowser.cpp:1450-1524`, `:1359-1372`).

use super::machine::{HeritageConfig, HeritageEvent, HeritageStage};
use super::HeritageAutomation;

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

/// A completion waits for the whole group's quiet window (`KaHeritageBrowser.cpp:1150-1217`).
#[test]
fn a_completed_group_is_reported_with_its_kind() {
    let mut m = at(HeritageStage::Waiting);
    let path = std::path::PathBuf::from("fixture-first.zip");
    m.file_offered(path.clone());
    assert!(m.zip_arrived(path.clone()).is_none());
    assert_eq!(
        m.timers(4200),
        vec![HeritageEvent::DatasetReady {
            kind: "지정유산",
            files: vec![path]
        }]
    );
}

/// Retry three remains recoverable (`KaHeritageBrowser.cpp:504-562`).
#[test]
fn retries_reopen_the_current_kind_with_capped_delays_without_an_attempt_cap() {
    let mut m = at(HeritageStage::Waiting);
    let mut now = 1000;
    for delay in [5000, 10000, 20000, 30000, 30000, 30000] {
        assert!(matches!(
            m.transfer_failed("fixture broken transfer", now),
            Some(HeritageEvent::Note(_))
        ));
        assert!(!m.is_over());
        assert!(!m.retry_due(now + delay - 1));
        assert!(m.retry_due(now + delay));
        assert!(!m.session_reopened(now + delay - 1));
        now += delay;
        assert!(m.session_reopened(now));
        assert_eq!(m.stage(), &HeritageStage::Idle);
        assert_eq!(m.current_kind(), "지정유산");
        m.force_stage(HeritageStage::Waiting);
    }
}

/// A retry requests a fresh authenticated session (`KaHeritageBrowser.cpp:533-562`).
#[test]
fn a_due_retry_requires_explicit_session_recreation() {
    let mut m = at(HeritageStage::Waiting);
    let _ = m.transfer_failed("fixture broken transfer", 1000);
    assert!(m.session_restart_required());
    let _ = m.timers(6000);
    assert!(m.session_reopened(6000));
    assert_eq!(m.stage(), &HeritageStage::Idle);
}

/// ZIP generation is separate from navigation bounds (`KaHeritageBrowser.cpp:848-869`).
#[test]
fn waiting_for_a_file_has_no_arbitrary_generation_deadline() {
    let mut m = at(HeritageStage::DownloadAll);
    let _ = m.script_answered(m.current_token(), "all");
    assert!(m.timers(150001).is_empty());
    assert!(m.timers(1000000).is_empty());
    assert!(!m.is_over());
}

/// The final kind completes only after actual import acceptance (`KaHeritageBrowser.cpp:1206-1217`).
#[test]
fn six_accepted_groups_finish_in_the_cpp_kind_order() {
    let mut m = at(HeritageStage::Waiting);
    let mut seen = Vec::new();
    let mut now = 0;
    for index in 0..6 {
        seen.push(m.current_kind());
        let files = vec![std::path::PathBuf::from(format!(
            "fixture-kind-{index}.zip"
        ))];
        m.file_offered(files[0].clone());
        assert!(m.zip_arrived(files[0].clone()).is_none());
        now += 4200;
        assert!(matches!(
            m.timers(now).as_slice(),
            [HeritageEvent::DatasetReady { .. }]
        ));
        assert!(!m.is_over());
        assert!(m.dataset_loaded(&files, super::DatasetOutcome::Accepted));
        let events = m.timers(now);
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, HeritageEvent::Done))
                .count(),
            usize::from(index == 5)
        );
        if !m.is_over() {
            m.force_stage(HeritageStage::Waiting);
        }
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
}
