//! Unit tests for `DownloadJobList` invariants (order DL1b): `APP-1` on dropped or panicked
//! workers, final terminal states, retry countdown anchored at worker report time without
//! `0초 뒤`, and safe detached cloning.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::mpsc;
use std::time::Duration;

use crate::download_jobs::{
    DownloadJobList, DownloadJobState, DownloadJobUpdate, StopCode, APP1_STOP_REASON,
};
use crate::status_bar_text::gauge_job_text;

#[test]
fn dropped_or_panicked_worker_turns_unfinished_job_into_stopped_app1() {
    let app1 = StopCode::from_code("APP-1").expect("APP-1 must be in StopCode::ALL");
    assert_eq!(app1.as_str(), "APP-1");
    assert_eq!(app1.to_string(), "APP-1");
    assert!(!app1.is_info(), "APP-1 needs user attention (warning)");

    // 1) Dropped handle without reporting Done, Cancelled, or Stopped -> APP-1 on tick.
    let mut list = DownloadJobList::new();
    let (drop_id, handle) = list.add_job("DEM", DownloadJobState::Waiting, 0.0);
    handle.report_running(Some(0.35), None);
    drop(handle);
    list.tick(1.0);
    assert_eq!(
        list.get(drop_id).unwrap().state,
        DownloadJobState::Stopped {
            code: StopCode::App1,
            reason: APP1_STOP_REASON.to_owned(),
        }
    );

    // 2) Worker thread that panics mid-flight -> APP-1 on tick after thread unwinds.
    let (panic_id, panic_handle) = list.add_job("주변유적", DownloadJobState::Waiting, 1.0);
    let worker = std::thread::spawn(move || {
        panic_handle.report_running(Some(0.10), Some("목록 확인 중".to_owned()));
        panic!("simulated worker panic");
    });
    assert!(worker.join().is_err(), "worker thread panicked as expected");
    list.tick(2.0);
    assert_eq!(
        list.get(panic_id).unwrap().state,
        DownloadJobState::Stopped {
            code: StopCode::App1,
            reason: APP1_STOP_REASON.to_owned(),
        }
    );

    // 3) Worker that sends Done and then drops its handle stays Done (not APP-1).
    let (ok_id, ok_handle) = list.add_job("지적도", DownloadJobState::Waiting, 2.0);
    ok_handle.report_done("cadastral.gpkg");
    drop(ok_handle);
    list.tick(3.0);
    assert_eq!(
        list.get(ok_id).unwrap().state,
        DownloadJobState::Done {
            result: "cadastral.gpkg".to_owned(),
        }
    );
}

#[test]
fn settled_jobs_ignore_late_worker_updates() {
    let mut list = DownloadJobList::new();

    let (done_id, done_h) = list.add_job("DEM", DownloadJobState::Waiting, 0.0);
    done_h.report_done("dem.tif");
    let (cancel_id, cancel_h) = list.add_job("지적도", DownloadJobState::Waiting, 0.0);
    cancel_h.report_cancelled();
    let (stop_id, stop_h) = list.add_job("수치지형도", DownloadJobState::Waiting, 0.0);
    stop_h.report_stopped(
        StopCode::Acc2,
        "로그인이 되지 않았습니다. 아이디·비밀번호를 확인해 주세요.",
    );
    list.tick(5.0);

    // Late updates from stray worker threads or set_state must not revive settled jobs.
    done_h.report_running(Some(0.50), Some("뒤늦은 진행".to_owned()));
    done_h.report_recovering(1, "뒤늦은 재시도", 10.0);
    cancel_h.report_running(Some(0.80), None);
    cancel_h.send(DownloadJobUpdate::Done {
        result: "late.gpkg".to_owned(),
    });
    stop_h.report_running(Some(0.20), None);
    stop_h.send(DownloadJobUpdate::Waiting);
    list.tick(6.0);

    assert!(!list.set_state(
        done_id,
        DownloadJobState::Running {
            fraction: Some(0.1),
            stage: None,
        },
        6.0,
    ));
    assert!(!list.set_state(cancel_id, DownloadJobState::Waiting, 6.0));
    assert!(!list.set_state(stop_id, DownloadJobState::Waiting, 6.0));

    assert_eq!(
        list.get(done_id).unwrap().state,
        DownloadJobState::Done {
            result: "dem.tif".to_owned(),
        }
    );
    assert_eq!(list.get(cancel_id).unwrap().state, DownloadJobState::Cancelled);
    assert_eq!(
        list.get(stop_id).unwrap().state,
        DownloadJobState::Stopped {
            code: StopCode::Acc2,
            reason: "로그인이 되지 않았습니다. 아이디·비밀번호를 확인해 주세요.".to_owned(),
        }
    );

    // Explicit restart_job starts a fresh run with a new handle; old stop_h updates stay ignored.
    let restarted = list
        .restart_job(stop_id, DownloadJobState::Waiting, 7.0)
        .expect("explicit restart allowed");
    stop_h.report_done("stale_from_previous_run.dxf");
    restarted.report_running(Some(0.45), None);
    list.tick(13.0);
    assert!(list.get(done_id).is_none());
    assert_eq!(
        list.get(stop_id).unwrap().state,
        DownloadJobState::Running {
            fraction: Some(0.45),
            stage: None,
        }
    );
}

#[test]
fn retry_wait_counts_from_worker_report_time_and_tooltip_never_shows_zero_seconds() {
    let mut list = DownloadJobList::new();
    let (id, handle) = list.add_job("주변유적", DownloadJobState::Waiting, 0.0);
    list.tick(5.0);

    let (reported_tx, reported_rx) = mpsc::channel();
    let (exit_tx, exit_rx) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        // Reported while fake time is 5.0 -> deadline must be 5.0 + 20.0 = 25.0.
        handle.report_recovering(2, "파일 대신 웹페이지가 돌아왔습니다.", 20.0);
        reported_tx.send(()).unwrap();
        let _ = exit_rx.recv();
    });

    reported_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    // UI drains 7 seconds later at t = 12.0.
    list.tick(12.0);
    assert_eq!(
        list.get(id).unwrap().state,
        DownloadJobState::Recovering {
            attempt: 2,
            reason: "파일 대신 웹페이지가 돌아왔습니다.".to_owned(),
            next_attempt_at: 25.0,
        }
    );
    let t12 = gauge_job_text(list.get(id).unwrap(), 12.0, 12);
    assert_eq!(
        t12.tooltip.as_deref(),
        Some("파일 대신 웹페이지가 돌아왔습니다. 13초 뒤 다시 받습니다.")
    );

    // Just before deadline -> 1초 뒤 (never 0초 뒤).
    let t_almost = gauge_job_text(list.get(id).unwrap(), 24.9, 12);
    assert_eq!(
        t_almost.tooltip.as_deref(),
        Some("파일 대신 웹페이지가 돌아왔습니다. 1초 뒤 다시 받습니다.")
    );

    // At and past deadline -> recovering text without any countdown or 「0초 뒤」.
    for now in [25.0, 25.5, 40.0] {
        let jt = gauge_job_text(list.get(id).unwrap(), now, 12);
        let tip = jt.tooltip.as_deref().unwrap_or_default();
        assert!(!tip.contains("0초 뒤"), "unexpected 0s countdown at {now}: {tip}");
        assert!(!tip.contains("초 뒤"), "unexpected countdown at/past deadline {now}: {tip}");
        assert_eq!(tip, "파일 대신 웹페이지가 돌아왔습니다.");
    }

    let _ = exit_tx.send(());
    worker.join().unwrap();
}

#[test]
fn cloned_job_list_does_not_swallow_original_worker_updates() {
    let mut original = DownloadJobList::new();
    let (id, handle) = original.add_job("DEM", DownloadJobState::Waiting, 0.0);
    let mut cloned = original.clone();

    handle.report_running(Some(0.62), Some("타일 합치는 중".to_owned()));

    // Ticking and cancelling on the clone must not drain or mutate the original list's worker.
    cloned.tick(1.0);
    let _ = cloned.request_cancel(id);

    original.tick(1.0);
    assert_eq!(
        original.get(id).unwrap().state,
        DownloadJobState::Running {
            fraction: Some(0.62),
            stage: Some("타일 합치는 중".to_owned()),
        }
    );
    assert!(!original.get(id).unwrap().is_cancel_requested());
}
