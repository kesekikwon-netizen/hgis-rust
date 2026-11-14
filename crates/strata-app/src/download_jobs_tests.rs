//! Unit tests for the pure download job list, worker channel updates, cancel flag,
//! stop codes, expiry timings, and gauge text formatting (`docs/RUST_PLAN.md` §2.8).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::mpsc;
use std::time::Duration;

use super::*;
use crate::status_bar_text::{
    cut_with_ellipsis, format_gauge_percent, gauge_job_text, gauge_multi_summary,
};

#[test]
fn state_transitions_and_expiry_times_follow_rules() {
    let all_codes = [
        (StopCode::Dir1, "DIR-1", false),
        (StopCode::Dir2, "DIR-2", false),
        (StopCode::Area1, "AREA-1", false),
        (StopCode::Area2, "AREA-2", false),
        (StopCode::Key1, "KEY-1", false),
        (StopCode::Key2, "KEY-2", false),
        (StopCode::Acc1, "ACC-1", false),
        (StopCode::Acc2, "ACC-2", false),
        (StopCode::Site1, "SITE-1", false),
        (StopCode::Data1, "DATA-1", false),
        (StopCode::None1, "NONE-1", true),
        (StopCode::App1, "APP-1", false),
    ];
    assert_eq!(all_codes.len(), StopCode::ALL.len());
    for (code, text, info) in all_codes {
        assert_eq!(code.as_str(), text);
        assert_eq!(code.to_string(), text);
        assert_eq!(code.is_info(), info);
    }

    let mut list = DownloadJobList::new();
    let (done_id, _) = list.add_job("DEM", DownloadJobState::Waiting, 0.0);
    assert_eq!(list.len(), 1);
    assert!(list.get(done_id).unwrap().is_unfinished());

    assert!(list.set_state(
        done_id,
        DownloadJobState::Running {
            fraction: Some(0.62),
            stage: Some("압축 푸는 중".to_owned()),
        },
        2.0,
    ));
    assert!(list.get(done_id).unwrap().is_unfinished());

    // Done leaves 8 seconds after it finished.
    assert!(list.set_state(
        done_id,
        DownloadJobState::Done {
            result: "dem.tif".to_owned(),
        },
        10.0,
    ));
    assert!(!list.get(done_id).unwrap().is_unfinished());
    list.tick(17.99);
    assert!(list.get(done_id).is_some(), "Done job stays before 8 s");
    list.tick(18.0);
    assert!(list.get(done_id).is_none(), "Done job leaves at 8 s");

    // Cancelled leaves 6 seconds after it was cancelled.
    let (cancel_id, _) = list.add_job(
        "지적도",
        DownloadJobState::Running {
            fraction: None,
            stage: None,
        },
        20.0,
    );
    assert!(list.set_state(cancel_id, DownloadJobState::Cancelled, 25.0));
    list.tick(30.99);
    assert!(list.get(cancel_id).is_some(), "Cancelled job stays before 6 s");
    list.tick(31.0);
    assert!(list.get(cancel_id).is_none(), "Cancelled job leaves at 6 s");

    // Stopped stays indefinitely until dismissed by the user.
    let (stop_id, _) = list.add_job(
        "주변유적",
        DownloadJobState::Stopped {
            code: StopCode::Acc2,
            reason: "로그인이 되지 않았습니다. 아이디·비밀번호를 확인해 주세요.".to_owned(),
        },
        30.0,
    );
    list.tick(50_000.0);
    assert!(list.get(stop_id).is_some(), "Stopped job stays until dismissed");
    assert!(list.dismiss(stop_id));
    assert!(list.is_empty());
}

#[test]
fn worker_thread_updates_and_cancel_flag_cross_threads_without_app_state() {
    let mut list = DownloadJobList::new();
    let (id, handle) = list.add_job("주변유적", DownloadJobState::Waiting, 0.0);
    let (step_tx, step_rx) = mpsc::channel();
    let (go_tx, go_rx) = mpsc::channel();

    let worker = std::thread::spawn(move || {
        assert!(!handle.is_cancelled());
        handle.report_running(Some(0.40), Some("목록 읽는 중".to_owned()));
        step_tx.send(1).unwrap();

        go_rx.recv().unwrap();
        handle.report_recovering(3, "파일 대신 웹페이지가 돌아왔습니다.", 20.0);
        step_tx.send(2).unwrap();

        go_rx.recv().unwrap();
        let saw_cancel = handle.is_cancelled();
        if saw_cancel {
            handle.report_cancelled();
        }
        saw_cancel
    });

    assert_eq!(step_rx.recv_timeout(Duration::from_secs(2)).unwrap(), 1);
    list.tick(5.0);
    assert_eq!(
        list.get(id).unwrap().state,
        DownloadJobState::Running {
            fraction: Some(0.40),
            stage: Some("목록 읽는 중".to_owned()),
        }
    );

    go_tx.send(()).unwrap();
    assert_eq!(step_rx.recv_timeout(Duration::from_secs(2)).unwrap(), 2);
    list.tick(12.0);
    assert_eq!(
        list.get(id).unwrap().state,
        DownloadJobState::Recovering {
            attempt: 3,
            reason: "파일 대신 웹페이지가 돌아왔습니다.".to_owned(),
            next_attempt_at: 25.0,
        }
    );

    // Requesting cancel sets only the flag; state changes when the worker reports Cancelled.
    assert!(list.request_cancel(id));
    assert!(list.get(id).unwrap().is_cancel_requested());
    assert!(matches!(
        list.get(id).unwrap().state,
        DownloadJobState::Recovering { attempt: 3, .. }
    ));

    go_tx.send(()).unwrap();
    assert!(worker.join().unwrap(), "worker saw cancel flag");
    list.tick(15.0);
    assert_eq!(list.get(id).unwrap().state, DownloadJobState::Cancelled);
    list.tick(21.0);
    assert!(list.is_empty());
}

#[test]
fn every_state_text_percent_rounding_and_long_text_ellipsis() {
    assert_eq!(format_gauge_percent(None), None);
    assert_eq!(format_gauge_percent(Some(0.0)), Some("0%".to_owned()));
    assert_eq!(format_gauge_percent(Some(0.62)), Some("62%".to_owned()));
    assert_eq!(format_gauge_percent(Some(0.624)), Some("62%".to_owned()));
    assert_eq!(format_gauge_percent(Some(0.625)), Some("63%".to_owned()));
    assert_eq!(format_gauge_percent(Some(1.2)), Some("100%".to_owned()));

    assert_eq!(cut_with_ellipsis("지적도", 10), ("지적도".to_owned(), false));
    let (cut, was_cut) = cut_with_ellipsis("국가유산청주변유적장거리자료받기시험", 10);
    assert!(was_cut);
    assert!(cut.ends_with('…'), "{cut}");
    assert_eq!(cut.chars().count(), 10);

    let mut list = DownloadJobList::new();
    let (id, _) = list.add_job("DEM", DownloadJobState::Waiting, 0.0);
    assert_eq!(gauge_job_text(list.get(id).unwrap(), 0.0, 12).bar_label, "DEM 기다리는 중");

    list.set_state(
        id,
        DownloadJobState::Running {
            fraction: Some(0.62),
            stage: None,
        },
        1.0,
    );
    let t = gauge_job_text(list.get(id).unwrap(), 1.0, 12);
    assert_eq!(t.bar_label, "DEM 받는 중");
    assert_eq!(t.percent_text.as_deref(), Some("62%"));

    list.set_state(
        id,
        DownloadJobState::Running {
            fraction: None,
            stage: Some("타일 합치는 중".to_owned()),
        },
        2.0,
    );
    let t = gauge_job_text(list.get(id).unwrap(), 2.0, 12);
    assert_eq!(t.bar_label, "DEM · 타일 합치는 중");
    assert_eq!(t.percent_text, None);

    list.set_state(
        id,
        DownloadJobState::Recovering {
            attempt: 3,
            reason: "파일 대신 웹페이지가 돌아왔습니다.".to_owned(),
            next_attempt_at: 30.0,
        },
        10.0,
    );
    let t = gauge_job_text(list.get(id).unwrap(), 10.0, 12);
    assert_eq!(t.bar_label, "DEM 다시 받는 중 · 3번째");
    assert_eq!(
        t.tooltip.as_deref(),
        Some("파일 대신 웹페이지가 돌아왔습니다. 20초 뒤 다시 받습니다.")
    );

    list.set_state(
        id,
        DownloadJobState::Done {
            result: "완료".to_owned(),
        },
        12.0,
    );
    assert_eq!(gauge_job_text(list.get(id).unwrap(), 12.0, 12).bar_label, "DEM 받음");

    let (cid, _) = list.add_job("DEM", DownloadJobState::Cancelled, 13.0);
    assert_eq!(
        gauge_job_text(list.get(cid).unwrap(), 13.0, 12).bar_label,
        "DEM 받기를 취소했습니다."
    );

    let full_reason = "로그인이 되지 않았습니다. 아이디·비밀번호를 확인해 주세요.";
    let (acc_id, _) = list.add_job(
        "DEM",
        DownloadJobState::Stopped {
            code: StopCode::Acc2,
            reason: full_reason.to_owned(),
        },
        14.0,
    );
    let t = gauge_job_text(list.get(acc_id).unwrap(), 14.0, 12);
    assert_eq!(t.bar_label, "DEM 받지 못함 · ACC-2");
    assert!(t.warning);
    assert_eq!(t.tooltip.as_deref(), Some(full_reason));

    let (none_id, _) = list.add_job(
        "DEM",
        DownloadJobState::Stopped {
            code: StopCode::None1,
            reason: "이 범위에는 제공되는 도엽이 없습니다.".to_owned(),
        },
        15.0,
    );
    let t_info = gauge_job_text(list.get(none_id).unwrap(), 15.0, 12);
    assert_eq!(t_info.bar_label, "DEM · 이 범위에는 자료가 없습니다 · NONE-1");
    assert!(!t_info.warning, "NONE-1 is shown as information, not a failure");

    // Long title is cut with ellipsis in bar_label and shown in full in tooltip.
    let (long_id, _) = list.add_job(
        "국가유산청주변유적장거리공간자료전체목록",
        DownloadJobState::Running {
            fraction: Some(0.5),
            stage: None,
        },
        16.0,
    );
    let t_long = gauge_job_text(list.get(long_id).unwrap(), 16.0, 10);
    assert!(t_long.bar_label.contains('…'), "{}", t_long.bar_label);
    assert!(
        t_long
            .tooltip
            .as_deref()
            .unwrap_or_default()
            .contains("국가유산청주변유적장거리공간자료전체목록 받는 중"),
        "{:?}",
        t_long.tooltip
    );

    // Multi-job summary text counts unfinished jobs.
    list.apply_snap_preset("two", 20.0);
    assert_eq!(
        gauge_multi_summary(list.jobs()).map(|s| s.full_text()).as_deref(),
        Some("받는 중 2건")
    );
}
