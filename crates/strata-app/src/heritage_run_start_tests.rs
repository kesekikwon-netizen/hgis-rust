//! Fetch and pledge regressions (`target/cpp-ref/src/app/MainWindow.cpp:3146-3148`).

use super::*;

fn test_app() -> StrataAppState {
    let mut app = StrataAppState::new_startup();
    app.accounts.heritage_configured = false;
    app.accounts.heritage_creds.username = "user1".into();
    app.accounts.heritage_creds.password = "pass1".into();
    app
}

#[test]
fn the_button_waits_for_an_account() {
    let mut app = test_app();
    // A fresh state may already know an account from this PC's saved settings; this case
    // is about the state with none.
    app.accounts.heritage_configured = false;
    app.modals.heritage_sido = "경상북도".to_owned();
    app.modals.heritage_city = "경주시".to_owned();
    let (ready, why) = crate::dlg_heritage_browser::dlg_heritage_browser_footer::fetch_state(&app);
    assert!(!ready, "started without an account");
    assert!(why.unwrap_or_default().contains("아이디"), "{why:?}");

    start_heritage_fetch(&mut app);
    assert!(
        !app.heritage_run_requested,
        "started a run without an account"
    );
    assert!(
        app.status_bar_hint.contains("아이디"),
        "{}",
        app.status_bar_hint
    );
}

#[test]
fn the_button_waits_for_a_survey() {
    let mut app = test_app();
    app.accounts.heritage_configured = true;
    app.modals.heritage_sido = "경상북도".to_owned();
    app.modals.heritage_city = "경주시".to_owned();
    let (ready, why) = crate::dlg_heritage_browser::dlg_heritage_browser_footer::fetch_state(&app);
    assert!(!ready, "started without a survey");
    assert!(why.unwrap_or_default().contains("조사"), "{why:?}");
}

#[test]
fn the_button_waits_for_a_city() {
    let mut app = test_app();
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
    let mut app = test_app();
    app.accounts.heritage_configured = true;
    let dir = tempfile::tempdir().expect("temp dir");
    let survey = dir.path().join("현장.gpkg");
    std::fs::write(&survey, b"").expect("survey file");
    app.survey_path = Some(survey);
    crate::ref_vector::map::tests::draw_survey_area(
        &mut app,
        crate::ref_vector::map::tests::CENTRE,
        5187,
    );
    app.modals.heritage_sido = "경상북도".to_owned();
    app.modals.heritage_city = "경주시".to_owned();

    assert_eq!(
        crate::dlg_heritage_browser::dlg_heritage_browser_footer::fetch_state(&app),
        (true, None)
    );
    assert!(!app.heritage_run_requested);
    start_heritage_fetch(&mut app);
    assert!(
        app.heritage_run_requested,
        "the press did not ask for a run"
    );
    assert!(
        app.status_bar_hint.contains("경주시"),
        "{}",
        app.status_bar_hint
    );
}

/// The receipt is written, and the words name the file that was written.
#[test]
fn the_words_name_the_receipt_that_was_written() {
    let mut app = test_app();
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
    assert!(
        !text.contains("user1"),
        "the id reached the receipt: {text}"
    );
    assert!(text.contains("저장된 계정 사용"), "{text}");
}

/// No survey: no receipt, and the words say so instead of claiming one.
#[test]
fn no_survey_means_no_receipt_and_the_words_say_so() {
    let app = test_app();
    let words = write_receipt_after_pledge(&app, "서약 본문");
    assert!(
        !words.contains("남겼습니다"),
        "claimed a receipt with no survey: {words}"
    );
    assert!(words.contains("남기지 못했습니다"), "{words}");
}

/// **The button the user actually presses**: 「이 지역 받기」 without 「최근 받은 자료 다시 쓰기」
/// must go to the intranet. It used to call only the reuse path, so pressing it did nothing
/// when there was nothing on disk — the run existed and was never asked for.
#[test]
fn the_region_dialog_asks_for_an_intranet_run_not_only_a_reuse() {
    use eframe::egui;

    let dir = tempfile::tempdir().expect("temp dir");
    let survey = dir.path().join("현장.gpkg");
    std::fs::write(&survey, b"").expect("survey file");
    let mut app = test_app();
    app.survey_path = Some(survey);
    crate::ref_vector::map::tests::draw_survey_area(
        &mut app,
        crate::ref_vector::map::tests::CENTRE,
        5187,
    );
    app.accounts.heritage_configured = true;
    app.accounts.heritage_creds.username = "user1".to_owned();
    app.accounts.heritage_creds.password = "pass1".to_owned();
    app.heritage.archive_root = Some(dir.path().join("자료"));
    app.modals.active = Some(crate::dialogs_modal::ActiveModalDialog::HeritageRegionSelect);
    app.modals.heritage_sido = "경상북도".to_owned();
    app.modals.heritage_city = "경주시".to_owned();
    // Nothing received before: the reuse checkbox is not even offered, so the press must go
    // to the intranet. It used to call only the reuse path, so the press did nothing.
    app.modals.heritage_reuse_recent = false;

    // The dialog draws; nothing here sends synthetic input (the repo forbids it), so the
    // wiring is asserted on the path the press takes with reuse off.
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut open = true;
    for _ in 0..3 {
        let _ = ctx.run(Default::default(), |ctx| {
            crate::dlg_heritage_region_select::render_heritage_region_select(
                &mut app, ctx, &mut open,
            );
        });
    }
    assert!(
        !app.modals.heritage_reuse_recent,
        "the case is about reuse being off"
    );

    start_heritage_fetch(&mut app);
    assert!(
        !app.heritage.wanted.as_ref().is_some_and(|w| w.reuse),
        "the press only asked to reuse archives"
    );
    assert!(
        app.heritage_run_requested,
        "the press did not ask for an intranet run"
    );
}

/// The account state is read from what is stored, and none carries the id.
#[test]
fn the_account_state_is_read_from_what_is_stored() {
    let mut app = test_app();
    app.accounts.heritage_creds.username.clear();
    app.accounts.heritage_creds.password.clear();
    assert_eq!(receipt_account_state(&app).words(), "계정 없음");

    app.accounts.heritage_creds.username = "user1".to_owned();
    assert_eq!(receipt_account_state(&app).words(), "계정 설정 미완료");

    app.accounts.heritage_creds.password = "pass1".to_owned();
    assert_eq!(receipt_account_state(&app).words(), "저장된 계정 사용");
}
