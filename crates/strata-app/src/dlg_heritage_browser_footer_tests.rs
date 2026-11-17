//! Setup handler checks without synthetic input (`target/cpp-ref/src/app/MainWindow.cpp:3146-3148`).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::app_state::StrataAppState;

#[test]
fn fetch_click_writes_no_pledge_receipt() {
    let (mut app, dir) = crate::heritage_run::lifecycle_tests::ready();
    crate::heritage_run_start::start_heritage_fetch(&mut app);
    assert!(!dir.path().join("주변유적/receipts").exists());
    app.drop_heritage_run();
}

#[test]
fn heritage_start_button_is_live_and_asks_for_a_run() {
    let (mut app, _dir) = crate::heritage_run::lifecycle_tests::ready();
    assert_eq!(super::fetch_state(&app), (true, None));
    crate::heritage_run_start::start_heritage_fetch(&mut app);
    assert!(app.heritage_run_requested);
    assert_eq!(app.modals.active, None);
    assert!(!super::fetch_state(&app).0);
    app.drop_heritage_run();
}

#[test]
fn without_an_account_the_button_names_what_is_missing() {
    let mut app = StrataAppState::new_startup();
    app.accounts.heritage_configured = false;
    let (enabled, why) = super::fetch_state(&app);
    assert!(!enabled);
    assert!(why.unwrap().contains("아이디"));
}
