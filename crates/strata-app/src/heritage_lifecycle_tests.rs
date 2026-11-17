//! Bottom-gauge lifecycle regressions for HER-CODEX-UI1 and HER-API1.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::app_state::StrataAppState;
use crate::dialogs_modal::ActiveModalDialog;
use crate::download_jobs::DownloadJobState;
use crate::ref_vector::map::tests::{draw_survey_area, CENTRE};

pub(crate) fn ready() -> (StrataAppState, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let mut app = StrataAppState::new_startup();
    app.survey_path = Some(dir.path().join("fixture.gpkg"));
    std::fs::write(app.survey_path.as_ref().unwrap(), b"fixture").unwrap();
    // Synthetic survey area in the work CRS EPSG:5187.
    draw_survey_area(&mut app, CENTRE, 5187);
    app.accounts.heritage_configured = true;
    app.accounts.heritage_creds.username = "fixture-user".into();
    app.accounts.heritage_creds.password = "fixture-password".into();
    app.heritage.archive_root = Some(dir.path().to_owned());
    app.modals.heritage_sido = "경상북도".into();
    app.modals.heritage_city = "경주시".into();
    app.modals.active = Some(ActiveModalDialog::HeritageBrowser);
    (app, dir)
}

#[test]
fn heritage_press_closes_setup_and_creates_one_indeterminate_gauge() {
    let (mut app, _dir) = ready();
    crate::heritage_run_start::start_heritage_fetch(&mut app);
    assert_eq!(
        app.modals.active, None,
        "setup must close once receiving starts"
    );
    assert!(
        app.heritage.is_busy(),
        "pending browser work must stay busy"
    );
    assert_eq!(
        app.download_jobs.len(),
        1,
        "browser work needs the existing gauge"
    );
    assert!(matches!(
        app.download_jobs.jobs()[0].state,
        DownloadJobState::Running { fraction: None, .. }
    ));
    crate::heritage_run_start::start_heritage_fetch(&mut app);
    app.press_heritage();
    assert_eq!(
        app.download_jobs.len(),
        1,
        "repeat press must retain one job"
    );
    assert_eq!(
        app.modals.active, None,
        "repeat press must not reopen setup"
    );
    assert!(app.heritage.resolving.is_none());
}

#[test]
fn heritage_survey_change_discards_pending_start_and_pledge() {
    let (mut app, _dir) = ready();
    crate::heritage_run_start::start_heritage_fetch(&mut app);
    app.drop_heritage_run();
    assert!(
        !app.heritage_run_requested,
        "the former survey must never start a browser"
    );
    app.take_heritage_event(strata_net::heritage_auto::HeritageEvent::PledgeAgreed {
        terms: "stale terms".into(),
    });
    assert!(
        !_dir.path().join("주변유적/receipts").exists(),
        "no receipt in the next survey"
    );
    app.download_jobs.tick(1.0);
    assert!(matches!(
        app.download_jobs.jobs()[0].state,
        DownloadJobState::Cancelled
    ));
}

#[test]
fn heritage_pending_hidden_work_remains_busy_before_the_window_frame() {
    let (mut app, _dir) = ready();
    crate::heritage_run_start::start_heritage_fetch(&mut app);
    assert!(
        app.heritage.is_busy(),
        "snapshot waits must include the pending browser"
    );
}

#[test]
fn heritage_legacy_setup_has_no_progress_card_or_unsupported_selection() {
    let (mut app, _dir) = ready();
    let ctx = eframe::egui::Context::default();
    let mut texts = Vec::new();
    fn collect(shape: &eframe::egui::epaint::Shape, texts: &mut Vec<String>) {
        match shape {
            eframe::egui::epaint::Shape::Text(text) => texts.push(text.galley.text().into()),
            eframe::egui::epaint::Shape::Vec(shapes) => {
                for shape in shapes {
                    collect(shape, texts);
                }
            }
            _ => {}
        }
    }
    for _ in 0..3 {
        let out = ctx.run(Default::default(), |ctx| {
            crate::dlg_heritage_browser::render_heritage_browser(&mut app, ctx, &mut true);
        });
        texts.clear();
        for shape in out.shapes {
            collect(&shape.shape, &mut texts);
        }
    }
    let text = texts.join("\n");
    assert!(
        !text.contains("단계:"),
        "legacy progress card remains: {text}"
    );
    assert!(
        !text.contains("받을 항목:"),
        "selection has no runner contract: {text}"
    );
}
