//! Group import/ACK regressions (target/cpp-ref/tests/test_heritage_download_retry.cpp:167-211).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::download_jobs::{DownloadJobState, StopCode};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use strata_net::heritage_auto::{DatasetOutcome, HeritageEvent};
use strata_refmap::heritage::DATASETS;

#[path = "heritage_download_group_support.rs"]
mod support;
use support::*;

#[test]
fn two_archives_of_one_kind_are_imported_and_recorded_as_one_group() {
    let (mut app, dir) = start();
    let files = archives(dir.path());
    let reply = receive(&mut app, 0, &files);
    let calls = finish(&mut app, reply, &files, DatasetOutcome::Accepted);
    imported(&mut app, 0);
    assert_eq!(
        rows(&app),
        2,
        "every archive must contribute its real layer"
    );
    assert_eq!(calls, [(files.clone(), DatasetOutcome::Accepted)]);
    for row in &app.left_layers.entries {
        if !row.layer_key.starts_with("ref_heritage:") {
            continue;
        }
        let spec = app.ref_vectors.spec_of(&row.layer_key).unwrap();
        assert_eq!(spec.work_epsg, 5187);
        assert_eq!(crate::ref_vector::data::load(spec).unwrap().items.len(), 1);
    }
    let record: serde_json::Value =
        serde_json::from_slice(&std::fs::read(recent(dir.path())).unwrap()).unwrap();
    let actual: Vec<_> = record[DATASETS[0].name]["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| PathBuf::from(v.as_str().unwrap()))
        .collect();
    assert_eq!(actual, files);
    assert_eq!(app.download_jobs.len(), 1);
}

#[test]
fn import_pending_cannot_finish_the_gauge_or_ack_a_partial_group() {
    let (mut app, dir) = start();
    let files = archives(dir.path());
    let reply = receive(&mut app, 0, &files);
    assert!(
        !recent(dir.path()).exists(),
        "recent was published before exact ACK acceptance"
    );
    assert_eq!(rows(&app), 0);
    for dataset in &DATASETS[1..] {
        app.take_heritage_event(HeritageEvent::KindAbsent { kind: dataset.name });
    }
    app.take_heritage_event(HeritageEvent::Done);
    let mut calls = Vec::new();
    app.take_heritage_import_with(reply, |got, outcome| {
        calls.push((got.to_vec(), outcome));
        false
    });
    assert_eq!(calls, [(files, DatasetOutcome::Accepted)]);
    running(&mut app);
    assert_eq!(
        rows(&app),
        0,
        "a refused ACK cannot publish imported layers"
    );
    assert!(!recent(dir.path()).exists());
}

#[test]
fn retryable_import_retries_the_same_kind_without_recent() {
    let (mut app, dir) = start();
    let files = archives(dir.path());
    let first = receive(&mut app, 0, &files[..1]);
    finish(&mut app, first, &files[..1], DatasetOutcome::Accepted);
    imported(&mut app, 0);
    let before = std::fs::read(recent(dir.path())).unwrap();
    let broken = dir.path().join("truncated.zip");
    std::fs::write(&broken, b"PK\x03\x04truncated").unwrap();
    let group = vec![files[1].clone(), broken];
    let reply = receive(&mut app, 1, &group);
    let reason = match &reply {
        Reply::Kind(_, Err(failure)) => {
            assert!(failure.download_again, "full-group retryability was lost");
            assert!(failure.reason.contains("truncated.zip"));
            failure.reason.clone()
        }
        _ => panic!("the second broken archive must reject the full group"),
    };
    let calls = finish(
        &mut app,
        reply,
        &group,
        DatasetOutcome::RetryableFailure {
            reason: reason.clone(),
        },
    );
    assert_eq!(
        calls,
        [(group, DatasetOutcome::RetryableFailure { reason })]
    );
    running(&mut app);
    assert_eq!(rows(&app), 1, "earlier accepted kind must remain alone");
    assert_eq!(std::fs::read(recent(dir.path())).unwrap(), before);
    assert_eq!(app.download_jobs.len(), 1);
    assert!(app.heritage.download.as_ref().unwrap().kinds[1] == Kind::Pending);
}

#[test]
fn fatal_import_failure_preserves_reason_and_stops_small() {
    let (mut app, dir) = start();
    let broken = dir.path().join("broken.shp");
    std::fs::write(&broken, b"invalid shape without sidecars").unwrap();
    let files = vec![broken];
    let reply = receive(&mut app, 0, &files);
    let reason = match &reply {
        Reply::Kind(_, Err(failure)) => {
            assert!(!failure.download_again);
            assert_eq!(
                failure.reason,
                "broken.shp의 필수 파일 .shx가 없습니다. 적재를 중단합니다."
            );
            failure.reason.clone()
        }
        _ => panic!("missing non-archive sidecars must be fatal"),
    };
    let calls = finish(
        &mut app,
        reply,
        &files,
        DatasetOutcome::FatalFailure {
            reason: reason.clone(),
        },
    );
    assert_eq!(
        calls,
        [(
            files,
            DatasetOutcome::FatalFailure {
                reason: reason.clone()
            }
        )]
    );
    app.download_jobs.tick(1.0);
    assert!(
        matches!(&app.download_jobs.jobs()[0].state, DownloadJobState::Stopped { code: StopCode::Data1, reason: got } if got == &reason)
    );
    assert_eq!(app.status_left_message(), reason);
    assert_eq!(app.modals.active, None);
    assert!(!recent(dir.path()).exists());
    assert_eq!(rows(&app), 0);
}

#[test]
fn stale_import_outcomes_after_cancel_restart_are_ignored() {
    for retry in [false, true] {
        let (mut app, dir) = start();
        let files = archives(dir.path());
        let mut reply = receive(&mut app, 0, &files);
        if retry {
            let Reply::Kind(_, result) = &mut reply else {
                panic!("group reply")
            };
            *result = Err(ImportFailure {
                code: StopCode::Data1,
                reason: "old retry".into(),
                download_again: true,
            });
        }
        let old = app.heritage.download.as_ref().unwrap().job.id();
        app.drop_heritage_run();
        crate::heritage_run_start::start_heritage_fetch(&mut app);
        let current = app.heritage.download.as_ref().unwrap().job.id();
        assert_ne!(current, old);
        let mut calls = Vec::new();
        app.take_heritage_import_with(reply, |got, outcome| {
            calls.push((got.to_vec(), outcome));
            true
        });
        assert!(
            calls.is_empty(),
            "stale reply reached the new run's ACK boundary"
        );
        running(&mut app);
        assert_eq!(rows(&app), 0, "stale import changed the new survey view");
        assert!(!recent(dir.path()).exists(), "stale group published recent");
        assert!(app.heritage.download.as_ref().unwrap().kinds[0] == Kind::Pending);
    }
}

#[test]
fn stale_or_consumed_file_group_cannot_publish_layers() {
    let (mut app, dir) = start();
    let files = archives(dir.path());
    let reply = receive(&mut app, 0, &files);
    let Reply::Kind(group, Ok(load)) = reply else {
        panic!("real grouped import")
    };
    let mut wrong = group.clone();
    wrong.files.reverse();
    let mut calls = Vec::new();
    app.take_heritage_import_with(Reply::Kind(wrong, Ok(load.clone())), |got, outcome| {
        calls.push((got.to_vec(), outcome));
        true
    });
    assert!(calls.is_empty());
    assert_eq!(rows(&app), 0, "reordered identity published layers");
    assert!(!recent(dir.path()).exists());
    let calls = finish(
        &mut app,
        Reply::Kind(group.clone(), Ok(load.clone())),
        &files,
        DatasetOutcome::Accepted,
    );
    assert_eq!(calls, [(files, DatasetOutcome::Accepted)]);
    imported(&mut app, 0);
    assert_eq!(rows(&app), 2);
    let before = std::fs::read(recent(dir.path())).unwrap();
    let mut calls = Vec::new();
    app.take_heritage_import_with(Reply::Kind(group, Ok(load)), |got, outcome| {
        calls.push((got.to_vec(), outcome));
        true
    });
    assert!(calls.is_empty(), "consumed group was acknowledged twice");
    assert_eq!(rows(&app), 2);
    assert_eq!(std::fs::read(recent(dir.path())).unwrap(), before);
}

#[test]
fn empty_archive_group_is_not_kind_absence() {
    let (mut app, dir) = start();
    let reply = receive(&mut app, 0, &[]);
    let calls = finish(
        &mut app,
        reply,
        &[],
        DatasetOutcome::FatalFailure {
            reason: "받은 파일이 없습니다.".into(),
        },
    );
    assert_eq!(
        calls,
        [(
            Vec::new(),
            DatasetOutcome::FatalFailure {
                reason: "받은 파일이 없습니다.".into()
            }
        )]
    );
    app.download_jobs.tick(1.0);
    assert!(matches!(
        app.download_jobs.jobs()[0].state,
        DownloadJobState::Stopped {
            code: StopCode::Data1,
            ..
        }
    ));
    assert_eq!(rows(&app), 0);
    assert!(!recent(dir.path()).exists());
}
