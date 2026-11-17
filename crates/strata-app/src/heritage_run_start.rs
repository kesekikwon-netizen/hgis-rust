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
pub(crate) fn write_receipt_after_pledge(app: &StrataAppState, terms: &str) -> String {
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
    use std::sync::Arc;

    let config = strata_heritage_run::HeritageConfig {
        account_id: app.accounts.heritage_creds.username.clone(),
        account_password: app.accounts.heritage_creds.password.clone(),
        sido: app.modals.heritage_sido.clone(),
        city: app.modals.heritage_city.clone(),
        fixture_base: std::env::var("KA_HGIS_HERITAGE_FIXTURE").ok(),
    };
    let root = app.heritage_archive_root();
    let network = Arc::new(|| true);
    let options = strata_heritage_run::RunOptions {
        data_parent: app.accounts.config_dir.join("heritage-session"),
        config,
        network_allowed: network,
        download_root: root,
    };
    match strata_heritage_run::start_run(frame, options) {
        Ok(handle) => {
            app.heritage.browser_run = Some(handle);
        }
        Err(why) => {
            app.status_bar_hint =
                format!("인트라넷 창을 열지 못했습니다: {why}. 잠시 뒤 다시 눌러 주세요.");
        }
    }
}

/// Without a browser engine there is no run; the words say so rather than pretending.
#[cfg(not(windows))]
pub(crate) fn begin_run_in_window(app: &mut StrataAppState, _frame: &eframe::Frame) {
    app.status_bar_hint =
        "이 판에서는 인트라넷 창을 열 수 없습니다(Windows 전용).".to_owned();
}

/// Records a received archive for `kind` and asks the existing loader to show it.
///
/// This is the join HER9 exists for: the run puts a ZIP on the disk, `record_recent` writes the
/// entry the loader reads, and `begin_heritage_run` puts the files on the map — no second
/// unpacking, no second loader.
///
/// Returns the words for the status line. The words never say a layer was added unless the
/// loader was actually asked.
pub(crate) fn show_received(app: &mut StrataAppState, kind: &str, path: &std::path::Path) -> String {
    let Some(region) =
        strata_refmap::heritage_recent::region_folder(&app.heritage_archive_root(), &app.modals.heritage_sido, &app.modals.heritage_city)
    else {
        return format!("{kind}: 받은 파일을 둘 곳을 찾지 못했습니다.");
    };
    let when = chrono::Local::now().naive_local();
    if let Err(why) = crate::heritage_receipt::record_recent(&region, kind, &[path.to_path_buf()], when) {
        return format!("{kind}: 받은 파일을 기록하지 못했습니다({why}).");
    }
    // The loader reads `recent.json` and unpacks; it does the map rows
    // (`heritage_reuse.rs:load_plan`).
    app.heritage.wanted = Some(crate::heritage_run::reuse::Wanted {
        sido: app.modals.heritage_sido.clone(),
        city: app.modals.heritage_city.clone(),
        reuse: true,
    });
    format!("{kind}: 받은 자료를 지도에 올립니다.")
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

    /// The receipt is written, and the words name the file that was written.
    #[test]
    fn the_words_name_the_receipt_that_was_written() {
        let mut app = StrataAppState::new_startup();
        let dir = tempfile::tempdir().expect("temp dir");
        let survey = dir.path().join("현장.gpkg");
        std::fs::write(&survey, b"").expect("survey file");
        app.survey_path = Some(survey);
        app.accounts.heritage_creds.username = "user1".to_owned();
        app.accounts.heritage_creds.password = "pass1".to_owned();

        let words = write_receipt_after_pledge(&app, "서약 본문");
        assert!(words.contains("남겼습니다"), "{words}");
        let path = words.split(": ").nth(1).expect("a path in the words");
        let text = std::fs::read_to_string(path.trim()).expect("the named file exists");
        assert!(text.contains("서약 본문"), "{text}");
        assert!(!text.contains("user1"), "the id reached the receipt: {text}");
        assert!(text.contains("저장된 계정 사용"), "{text}");
    }

    /// No survey: no receipt, and the words say so instead of claiming one.
    #[test]
    fn no_survey_means_no_receipt_and_the_words_say_so() {
        let app = StrataAppState::new_startup();
        let words = write_receipt_after_pledge(&app, "서약 본문");
        assert!(!words.contains("남겼습니다"), "claimed a receipt with no survey: {words}");
        assert!(words.contains("남기지 못했습니다"), "{words}");
    }

    /// The account state is read from what is stored, and none carries the id.
    #[test]
    fn the_account_state_is_read_from_what_is_stored() {
        let mut app = StrataAppState::new_startup();
        app.accounts.heritage_creds.username.clear();
        app.accounts.heritage_creds.password.clear();
        assert_eq!(receipt_account_state(&app).words(), "계정 없음");

        app.accounts.heritage_creds.username = "user1".to_owned();
        assert_eq!(receipt_account_state(&app).words(), "계정 설정 미완료");

        app.accounts.heritage_creds.password = "pass1".to_owned();
        assert_eq!(receipt_account_state(&app).words(), "저장된 계정 사용");
    }

    /// A received ZIP is recorded and the loader is asked to show it — the join HER9 is for.
    #[test]
    fn a_received_zip_is_recorded_and_handed_to_the_loader() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut app = StrataAppState::new_startup();
        app.survey_path = Some(dir.path().join("현장.gpkg"));
        std::fs::write(app.survey_path.as_ref().expect("survey"), b"").expect("survey file");
        app.modals.heritage_sido = "경상북도".to_owned();
        app.modals.heritage_city = "경주시".to_owned();
        app.heritage.archive_root = Some(dir.path().to_path_buf());

        let region = strata_refmap::heritage_recent::region_folder(
            dir.path(),
            "경상북도",
            "경주시",
        )
        .expect("region");
        std::fs::create_dir_all(&region).expect("mkdir");
        let zip = region.join("지정유산_261117100000.zip");
        std::fs::write(&zip, b"PK\x03\x04data").expect("zip");

        let words = show_received(&mut app, "지정유산", &zip);
        assert!(words.contains("지도에 올립니다"), "{words}");

        let dataset = strata_refmap::heritage::DATASETS
            .iter()
            .find(|d| d.name == "지정유산")
            .expect("kind");
        let found = strata_refmap::heritage_recent::find(
            &region,
            dataset,
            chrono::Local::now().naive_local(),
        );
        assert_eq!(found, vec![zip], "the loader cannot find the file just received");

        let wanted = app.heritage.wanted.as_ref().expect("the loader was not asked");
        assert_eq!(wanted.sido, "경상북도");
        assert_eq!(wanted.city, "경주시");
        assert!(wanted.reuse, "the loader was asked without reuse");
    }
}
