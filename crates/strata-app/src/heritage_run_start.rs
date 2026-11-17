//! The 「받기」 button of 주변유적 받기, and the run it starts.
//!
//! The C++ runs this flow inside an embedded browser with the user's login session
//! (`KaHeritageBrowser.cpp:571-629`); the run is `strata-heritage-run`.
//!
//! A screen change needs a `--snap` PNG, which this file cannot make on its own; the run's own
//! checks are `cargo run -p strata-heritage-run --example heritage_run` against the fixture.

use crate::app_state::StrataAppState;

/// Starts a heritage run for the city chosen in the modal.
///
/// The browser session needs the app's window, which is only available inside `eframe`'s
/// update; the handle is kept on the state and driven from there
/// (`app_state_heritage_run.rs`).
pub(crate) fn start_heritage_fetch(app: &mut StrataAppState) {
    if app.heritage.is_busy() {
        return;
    }
    let (ready, why) = crate::dlg_heritage_browser::dlg_heritage_browser_footer::fetch_state(app);
    if !ready {
        if let Some(why) = why {
            app.status_bar_hint = why.to_owned();
        }
        return;
    }
    let scope = match app.heritage_scope() {
        Ok(scope) => scope,
        Err((code, reason)) => {
            app.show_status_message(reason.clone());
            app.heritage.refused = Some((code, reason));
            return;
        }
    };
    let stage = format!(
        "{} {} — 인트라넷에서 자료를 받습니다.",
        app.modals.heritage_sido, app.modals.heritage_city
    );
    let initial = crate::download_jobs::DownloadJobState::Running {
        fraction: None,
        stage: Some(stage.clone()),
    };
    let now = app.download_jobs.current_time();
    let (_, job) = app
        .download_jobs
        .add_job(crate::heritage_run::HERITAGE_JOB_TITLE, initial, now);
    app.heritage.download = Some(crate::heritage_run::download::Download::new(
        app,
        scope,
        job,
        stage.clone(),
    ));
    app.remove_heritage_rows();
    app.show_status_message(stage.clone());
    app.status_bar_hint = stage;
    app.modals.active = None;
    app.heritage_run_requested = true;
}

/// The account state for the receipt, from what this PC has stored.
///
/// The id and the password never reach the receipt; only which of the three states it is in
/// (`HeritageIntranetSettings.cpp:64-70`).
#[must_use]
pub(crate) fn receipt_account_state(app: &StrataAppState) -> crate::heritage_receipt::AccountState {
    if app.accounts.heritage_creds.username.trim().is_empty() {
        crate::heritage_receipt::AccountState::None
    } else if app.accounts.heritage_creds.password.is_empty() {
        crate::heritage_receipt::AccountState::Incomplete
    } else {
        crate::heritage_receipt::AccountState::Stored
    }
}

/// Writes the pledge receipt once the run has agreed to the pledge on the site.
///
/// Called when the run reports the pledge was ticked. Returns the words for the status line:
/// where the receipt went, or why it could not be written. It never says "남겼습니다" without
/// having written the file (`MainWindow.cpp:3228-3244`).
#[cfg(test)]
fn write_receipt_after_pledge(app: &StrataAppState, terms: &str) -> String {
    let Some(survey) = app.survey_path.as_deref() else {
        return "서약서에 동의했습니다. 조사 파일이 없어 영수증은 남기지 못했습니다.".to_owned();
    };
    let at = chrono::Local::now().naive_local();
    match crate::heritage_receipt::write_pledge_receipt(
        survey,
        receipt_account_state(app),
        terms,
        at,
    ) {
        Ok(path) => format!("서약서 영수증을 남겼습니다: {}", path.display()),
        Err(why) => format!("서약서 영수증을 남기지 못했습니다: {why}"),
    }
}

/// Starts the hidden-browser run now that the window exists.
///
/// Called from the update loop. When the browser session cannot be made (WebView2 missing, or
/// the data folder unwritable) the words say so instead of a run that never starts — the C++
/// does the same (`KaHeritageBrowser.cpp:596-600`).
#[cfg(windows)]
pub(crate) fn begin_run_in_window(app: &mut StrataAppState, frame: &eframe::Frame) {
    let Some(download) = app.heritage.download.as_mut() else {
        return;
    };
    let Some(options) = download.browser_options(app.accounts.config_dir.join("heritage-session"))
    else {
        return;
    };
    match strata_heritage_run::start_run(frame, options) {
        Ok(handle) => {
            app.heritage.browser_run = Some(handle);
        }
        Err(why) => {
            app.stop_heritage_download(
                crate::download_jobs::StopCode::App1,
                format!("인트라넷 창을 열지 못했습니다: {why}. 잠시 뒤 다시 눌러 주세요."),
            );
        }
    }
}

/// Without a browser engine there is no run; the words say so rather than pretending.
#[cfg(not(windows))]
pub(crate) fn begin_run_in_window(app: &mut StrataAppState, _frame: &eframe::Frame) {
    app.stop_heritage_download(
        crate::download_jobs::StopCode::App1,
        "이 판에서는 인트라넷 창을 열 수 없습니다(Windows 전용).".to_owned(),
    );
}

#[cfg(test)]
#[path = "heritage_run_start_tests.rs"]
mod tests;
