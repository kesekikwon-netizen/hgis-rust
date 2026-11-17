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
    let (ready, why) = crate::dlg_heritage_browser::dlg_heritage_browser_footer::fetch_state(app);
    if !ready {
        if let Some(why) = why {
            app.status_bar_hint = why.to_owned();
        }
        return;
    }
    app.status_bar_hint = format!(
        "{} {} — 인트라넷에서 자료를 받습니다.",
        app.modals.heritage_sido, app.modals.heritage_city
    );
    // The run itself starts on the next frame, where the window handle exists.
    app.heritage_run_requested = true;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_button_waits_for_an_account() {
        let mut app = StrataAppState::new_startup();
        // A fresh state may already know an account from this PC's saved settings; this case
        // is about the state with none.
        app.accounts.heritage_configured = false;
        app.modals.heritage_sido = "경상북도".to_owned();
        app.modals.heritage_city = "경주시".to_owned();
        let (ready, why) = crate::dlg_heritage_browser::dlg_heritage_browser_footer::fetch_state(&app);
        assert!(!ready, "started without an account");
        assert!(why.unwrap_or_default().contains("아이디"), "{why:?}");

        start_heritage_fetch(&mut app);
        assert!(!app.heritage_run_requested, "started a run without an account");
        assert!(app.status_bar_hint.contains("아이디"), "{}", app.status_bar_hint);
    }

    #[test]
    fn the_button_waits_for_a_survey() {
        let mut app = StrataAppState::new_startup();
        app.accounts.heritage_configured = true;
        app.modals.heritage_sido = "경상북도".to_owned();
        app.modals.heritage_city = "경주시".to_owned();
        let (ready, why) = crate::dlg_heritage_browser::dlg_heritage_browser_footer::fetch_state(&app);
        assert!(!ready, "started without a survey");
        assert!(why.unwrap_or_default().contains("조사"), "{why:?}");
    }

    #[test]
    fn the_button_waits_for_a_city() {
        let mut app = StrataAppState::new_startup();
        app.accounts.heritage_configured = true;
        let dir = tempfile::tempdir().expect("temp dir");
        let survey = dir.path().join("현장.gpkg");
        std::fs::write(&survey, b"").expect("survey file");
        app.survey_path = Some(survey);
        let (ready, why) = crate::dlg_heritage_browser::dlg_heritage_browser_footer::fetch_state(&app);
        assert!(!ready, "started without a city");
        assert!(why.unwrap_or_default().contains("시"), "{why:?}");
    }

    /// With an account, a survey and a city the button is live and a press asks for a run.
    #[test]
    fn a_press_with_everything_in_place_asks_for_a_run() {
        let mut app = StrataAppState::new_startup();
        app.accounts.heritage_configured = true;
        let dir = tempfile::tempdir().expect("temp dir");
        let survey = dir.path().join("현장.gpkg");
        std::fs::write(&survey, b"").expect("survey file");
        app.survey_path = Some(survey);
        app.modals.heritage_sido = "경상북도".to_owned();
        app.modals.heritage_city = "경주시".to_owned();

        assert_eq!(crate::dlg_heritage_browser::dlg_heritage_browser_footer::fetch_state(&app), (true, None));
        assert!(!app.heritage_run_requested);
        start_heritage_fetch(&mut app);
        assert!(app.heritage_run_requested, "the press did not ask for a run");
        assert!(app.status_bar_hint.contains("경주시"), "{}", app.status_bar_hint);
    }
}
