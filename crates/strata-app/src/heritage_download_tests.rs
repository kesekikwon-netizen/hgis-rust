//! Factual completion and cancellation (`target/cpp-ref/tests/test_heritage_browser.cpp:294-315`).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::download_jobs::{DownloadJobState, StopCode};
use crate::heritage_run::lifecycle_tests::ready;
use strata_net::heritage_auto::HeritageEvent;
use strata_refmap::heritage::DATASETS;

fn start() -> (StrataAppState, tempfile::TempDir) {
    let (mut app, dir) = ready();
    crate::heritage_run_start::start_heritage_fetch(&mut app);
    (app, dir)
}

fn tick(app: &mut StrataAppState) -> DownloadJobState {
    app.drive_heritage_download(&eframe::egui::Context::default());
    app.download_jobs.tick(1.0);
    app.download_jobs.jobs().last().unwrap().state.clone()
}

fn absent(app: &mut StrataAppState, start: usize) {
    for dataset in &DATASETS[start..] {
        app.take_heritage_event(HeritageEvent::KindAbsent { kind: dataset.name });
    }
}

#[test]
fn all_absent_kinds_need_an_engine_done_before_gauge_completion() {
    let (mut app, _dir) = start();
    absent(&mut app, 0);
    assert!(matches!(
        tick(&mut app),
        DownloadJobState::Running { fraction: None, .. }
    ));
    app.take_heritage_event(HeritageEvent::Done);
    assert!(matches!(tick(&mut app), DownloadJobState::Done { .. }));
}

#[test]
fn engine_done_with_a_missing_kind_is_a_data_stop() {
    let (mut app, _dir) = start();
    absent(&mut app, 1);
    app.take_heritage_event(HeritageEvent::Done);
    assert!(matches!(
        tick(&mut app),
        DownloadJobState::Stopped {
            code: StopCode::Data1,
            ..
        }
    ));
}

#[test]
fn engine_done_waits_for_import_and_retains_earlier_kind_rows() {
    use strata_engine::ShapeKind;
    use strata_refmap::heritage_load::{DatasetLoad, Loaded};
    let (mut app, _dir) = start();
    let groups: [Group; 2] = std::array::from_fn(|index| Group {
        job: app.heritage.download.as_ref().unwrap().job.id(),
        dataset: &DATASETS[index],
        files: vec!["fixture.zip".into()],
    });
    for (index, group) in groups.iter().enumerate() {
        app.heritage.download.as_mut().unwrap().kinds[index] = Kind::Publishing(group.clone());
    }
    absent(&mut app, 2);
    app.take_heritage_event(HeritageEvent::Done);
    assert!(matches!(tick(&mut app), DownloadJobState::Running { .. }));
    for index in [0, 1] {
        let load = DatasetLoad {
            layers: vec![Loaded {
                title: format!("fixture-{index}"),
                file: format!("fixture-{index}.gpkg").into(),
                sites: 1,
                kind: ShapeKind::Polygons,
                named: false,
            }],
            messages: vec![],
            empty_in_scope: false,
        };
        app.take_heritage_import(Reply::Published(groups[index].clone(), Ok(load)));
        assert_eq!(
            app.left_layers
                .entries
                .iter()
                .filter(|row| row.layer_key.starts_with(super::super::KEY_PREFIX))
                .count(),
            index + 1
        );
        if index == 0 {
            assert!(matches!(tick(&mut app), DownloadJobState::Running { .. }));
        }
    }
    assert!(matches!(tick(&mut app), DownloadJobState::Done { .. }));
    assert_eq!(app.download_jobs.len(), 1);
}

#[test]
fn rejected_import_cannot_finish_download() {
    let (mut app, _dir) = start();
    let group = Group {
        job: app.heritage.download.as_ref().unwrap().job.id(),
        dataset: &DATASETS[0],
        files: vec!["fixture.zip".into()],
    };
    app.heritage.download.as_mut().unwrap().kinds[0] = Kind::Importing(group.clone());
    absent(&mut app, 1);
    app.take_heritage_event(HeritageEvent::Done);
    app.take_heritage_import_with(
        Reply::Kind(
            group,
            Err(ImportFailure {
                code: StopCode::Data1,
                reason: "fixture import failed".into(),
                download_again: false,
            }),
        ),
        |_, _| true,
    );
    assert!(matches!(
        tick(&mut app),
        DownloadJobState::Stopped {
            code: StopCode::Data1,
            ..
        }
    ));
}

#[test]
fn cancel_then_reopen_rejects_late_events_and_uses_a_new_job() {
    let (mut app, dir) = start();
    let first = app.download_jobs.jobs()[0].id;
    app.download_jobs.request_cancel(first);
    assert!(matches!(tick(&mut app), DownloadJobState::Cancelled));
    app.take_heritage_event(HeritageEvent::PledgeAgreed {
        terms: "late terms".into(),
    });
    app.take_heritage_event(HeritageEvent::Done);
    assert!(!dir.path().join("주변유적/receipts").exists());
    crate::heritage_run_start::start_heritage_fetch(&mut app);
    assert_ne!(app.download_jobs.jobs()[1].id, first);
    assert_eq!(app.download_jobs.len(), 2);
    assert!(app.heritage.is_busy());
}

#[test]
fn request_captures_survey_region_and_account_before_setup_changes() {
    let (mut app, _dir) = start();
    let survey = app.survey_path.clone().unwrap();
    app.modals.heritage_city = "안동시".into();
    app.accounts.heritage_creds.username = "later-user".into();
    let download = app.heritage.download.as_ref().unwrap();
    assert_eq!(download.context.survey, survey);
    assert_eq!(download.context.city, "경주시");
    assert_eq!(download.config.as_ref().unwrap().account_id, "fixture-user");
    app.survey_path = None;
    assert!(matches!(tick(&mut app), DownloadJobState::Cancelled));
    assert!(!app.heritage_run_requested);
}

#[test]
fn engine_stop_stays_small_and_never_becomes_success() {
    let (mut app, _dir) = start();
    app.take_heritage_event(HeritageEvent::Stopped("로그인이 되지 않았습니다.".into()));
    app.take_heritage_event(HeritageEvent::Done);
    assert!(matches!(
        tick(&mut app),
        DownloadJobState::Stopped {
            code: StopCode::Acc2,
            ..
        }
    ));
    assert_eq!(app.modals.active, None);
    assert_eq!(app.status_left_message(), "로그인이 되지 않았습니다.");
}

#[test]
fn pending_hidden_work_requests_a_bounded_repaint_without_gauge_rendering() {
    let (mut app, _dir) = start();
    let ctx = eframe::egui::Context::default();
    let mut delay = std::time::Duration::MAX;
    for _ in 0..3 {
        let out = ctx.run(Default::default(), |ctx| app.drive_heritage_download(ctx));
        delay = out.viewport_output[&eframe::egui::ViewportId::ROOT].repaint_delay;
    }
    assert!(delay <= std::time::Duration::from_millis(60), "{delay:?}");
}

#[test]
fn browser_cleanup_without_an_explicit_outcome_is_not_success() {
    let (mut app, _dir) = start();
    app.heritage_browser_ended();
    assert!(matches!(
        tick(&mut app),
        DownloadJobState::Stopped {
            code: StopCode::App1,
            ..
        }
    ));
}

#[test]
#[cfg(windows)]
fn browser_options_preserve_the_existing_network_gate() {
    let (mut app, dir) = start();
    let options = app
        .heritage
        .download
        .as_mut()
        .unwrap()
        .browser_options(dir.path().join("session"))
        .unwrap();
    let mut allowed = true;
    strata_net::network::probe_dry_run(false, || {
        allowed = (options.network_allowed)();
    });
    assert!(
        !allowed,
        "the app must not replace its network gate with true"
    );
}

#[test]
fn a_work_crs_change_cancels_the_captured_scope_before_import() {
    let (mut app, _dir) = start();
    app.work_crs = strata_types::crs::WorkCrs::Epsg5186;
    assert!(matches!(tick(&mut app), DownloadJobState::Cancelled));
    assert!(!app.heritage_run_requested);
}

#[test]
fn a_new_heritage_job_keeps_another_jobs_recovery_deadline() {
    let (mut app, _dir) = ready();
    let (other_id, other) =
        app.download_jobs
            .add_job("other download", DownloadJobState::Waiting, 42.0);
    crate::heritage_run_start::start_heritage_fetch(&mut app);
    other.report_recovering(1, "fixture retry", 5.0);
    app.download_jobs.tick(42.0);
    let DownloadJobState::Recovering {
        next_attempt_at, ..
    } = &app.download_jobs.get(other_id).unwrap().state
    else {
        panic!("recovery state");
    };
    // These integer seconds are represented exactly in f64, so the deadline is exact.
    assert_eq!(
        *next_attempt_at, 47.0,
        "starting heritage rewound the clock used by another worker"
    );
}

#[test]
fn cancel_replaces_the_last_receiving_note_in_the_bottom_status() {
    for (guide, hint, wanted_guide, wanted_status) in [
        (None, None, "취소했습니다.", "취소했습니다."),
        (Some("newer guide"), None, "newer guide", "newer guide"),
        (None, Some("newer hint"), "취소했습니다.", "newer hint"),
        (
            Some("newer guide"),
            Some("fixture receiving"),
            "newer guide",
            "newer guide",
        ),
        (
            Some("newer guide"),
            Some("newer hint"),
            "newer guide",
            "newer hint",
        ),
    ] {
        let (mut app, _dir) = start();
        app.active_tab = crate::app_state::MainTab::Map;
        app.take_heritage_event(HeritageEvent::Note("fixture receiving".into()));
        if let Some(guide) = guide {
            app.home_map.draw_guide_message = Some(guide.into());
        }
        if let Some(hint) = hint {
            app.status_bar_hint = hint.into();
        }
        app.download_jobs
            .request_cancel(app.download_jobs.jobs()[0].id);
        assert!(matches!(tick(&mut app), DownloadJobState::Cancelled));
        assert_eq!(app.status_left_message(), wanted_status);
        assert_eq!(
            app.home_map.draw_guide_message.as_deref(),
            Some(wanted_guide)
        );
    }
}
