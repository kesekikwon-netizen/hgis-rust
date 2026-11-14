//! Tests for verbatim fixture loaders, runtime builders, and per-test presets
//! (`target/cpp-ref/tests/test_topographic_browser.cpp:194-1415`).

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;
use strata_ngii_fixture::*;

fn http_get_body(site: &NgiiFixtureSite, target: &str) -> Vec<u8> {
    let mut s = TcpStream::connect_timeout(&site.addr(), Duration::from_secs(2)).expect("connect");
    s.set_read_timeout(Some(Duration::from_secs(2))).expect("rto");
    s.set_write_timeout(Some(Duration::from_secs(2))).expect("wto");
    write!(s, "GET {target} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
        .expect("write");
    let mut raw = Vec::new();
    s.read_to_end(&mut raw).expect("read");
    let sep = raw.windows(4).position(|w| w == b"\r\n\r\n").expect("hdr");
    raw[sep + 4..].to_vec()
}

#[test]
fn batch_sdk_and_portal_helpers_match_cpp_verbatim_fixtures() {
    assert!(batch_sdk_fixture().starts_with("<html><body><div id=\"fileControl\"></div><script>\n"));
    assert!(batch_sdk_fixture().ends_with("  </script></body></html>"));
    assert!(batch_portal_fixture().contains("for(const num of ['378044','378043'])"));
}

#[test]
fn automatic_selection_search_builder_matches_qt_compact_json() {
    let single = build_automatic_selection_search_fixture(false);
    assert_eq!(
        single,
        r#"{"items":[{"fileExt":"dxf","historyNo":0,"makeYer":"2024","maxx":1132145.6,"maxy":1987191.4,"minx":1120928.8,"miny":1973152.5,"name":"강릉","num":"378044","projection":"GRS80","scale":"25000"}]}"#
    );

    let pair = build_automatic_selection_search_fixture(true);
    assert_eq!(
        pair,
        r#"{"items":[{"fileExt":"dxf","historyNo":0,"makeYer":"2024","maxx":1132145.6,"maxy":1987191.4,"minx":1120928.8,"miny":1973152.5,"name":"강릉","num":"378044","projection":"GRS80","scale":"25000"},{"fileExt":"dxf","historyNo":0,"makeYer":"2024","maxx":1132145.6,"maxy":1987191.4,"minx":1120928.8,"miny":1973152.5,"name":"강릉","num":"378043","projection":"GRS80","scale":"25000"}]}"#
    );
}

#[test]
fn automatic_selection_preset_covers_all_eight_cpp_data_rows() {
    // Row 1: completed-official-order
    let r1 = automatic_selection_preserves_product_and_submits_once(
        true, false, false, false, false, false, false,
    );
    let p1 = std::str::from_utf8(&r1.portal_fixture).expect("utf8");
    assert!(p1.contains(
        r#"<script>window.fixtureSheets=[{"fileExt":"dxf","historyNo":0,"makeYer":"2024","maxx":1132145.6,"maxy":1987191.4,"minx":1120928.8,"miny":1973152.5,"name":"강릉","num":"378044","projection":"GRS80","scale":"25000"}];window.fixtureRetainedModal=false;window.fixtureApplicationSucceeds=true;"#
    ));
    assert!(r1.logged_in && !r1.accept_login && !r1.stream_fixture_archives && !r1.redirect_transfer);
    assert!(std::str::from_utf8(&r1.download_fixture)
        .expect("utf8")
        .contains("<title>download fixture</title>"));

    // Row 2: server-rejected-order-stays-before-download
    let r2 = automatic_selection_preserves_product_and_submits_once(
        false, false, false, false, false, false, false,
    );
    assert!(std::str::from_utf8(&r2.portal_fixture)
        .expect("utf8")
        .contains("window.fixtureApplicationSucceeds=false;"));

    // Row 3: two-sheets-after-official-close-retains-hidden-dom-and-clears-selection
    let r3 = automatic_selection_preserves_product_and_submits_once(
        true, true, false, false, false, false, false,
    );
    assert!(r3.redirect_transfer);
    assert_eq!(r3.download_fixture, batch_sdk_fixture().as_bytes());

    // Row 4: document-reload-after-form-before-submit
    let r4 = automatic_selection_preserves_product_and_submits_once(
        true, false, true, false, false, false, false,
    );
    assert!(std::str::from_utf8(&r4.portal_fixture)
        .expect("utf8")
        .contains("sessionStorage.fixtureReloaded='1';location.reload();"));

    // Row 5 & 6: compact + loginRequired
    let r6 = automatic_selection_preserves_product_and_submits_once(
        true, false, false, true, true, false, false,
    );
    assert!(!r6.logged_in && r6.accept_login && r6.stream_fixture_archives);
    assert!(std::str::from_utf8(&r6.portal_fixture)
        .expect("utf8")
        .contains("setTimeout(()=>window.open('/pd/purchs/download.do','downForm'),1200);"));
    assert!(std::str::from_utf8(&r6.download_fixture)
        .expect("utf8")
        .contains("id=\"xmlName\""));

    // Row 7 & 8: unsupportedTransfer and redirectTransfer
    let r7 = automatic_selection_preserves_product_and_submits_once(
        true, false, false, true, false, true, false,
    );
    assert!(!r7.stream_fixture_archives);
    assert!(std::str::from_utf8(&r7.download_fixture)
        .expect("utf8")
        .contains("<title>proprietary transfer fixture</title>"));

    let r8 = automatic_selection_preserves_product_and_submits_once(
        true, false, false, true, false, false, true,
    );
    assert!(r8.redirect_transfer && r8.stream_fixture_archives);
}

#[test]
fn login_and_any_id_builders_and_presets_match_cpp_mutations() {
    let k0 = known_official_login_submits_only_once_and_stop_prevents_more_work(false);
    let k1 = known_official_login_submits_only_once_and_stop_prevents_more_work(true);
    assert!(!k0.logged_in && !k0.hold_login_response);
    assert!(!k1.logged_in && k1.hold_login_response);
    assert!(std::str::from_utf8(&k0.login_fixture)
        .expect("utf8")
        .contains("window.testClock=0;"));

    let paused = automatic_login_waits_until_the_form_can_submit(true);
    let paused_html = std::str::from_utf8(&paused.login_fixture).expect("utf8");
    assert!(!paused.logged_in && paused.accept_login);
    assert!(paused_html.contains("<script src='/held-login-script.js'></script>"));
    assert!(paused_html.contains("document.addEventListener('DOMContentLoaded',finishReady);"));
    assert!(!paused_html.contains("PAUSE") && !paused_html.contains("READY"));

    let unpaused = automatic_login_waits_until_the_form_can_submit(false);
    let unpaused_html = std::str::from_utf8(&unpaused.login_fixture).expect("utf8");
    assert!(!unpaused_html.contains("held-login-script.js"));
    assert!(!unpaused_html.contains("PAUSE") && !unpaused_html.contains("READY"));

    let fail = genuine_page_connection_failure_stops_automatic_download();
    assert!(fail.fail_portal_connection);

    let unrec = completed_unrecognized_login_form_never_receives_credentials();
    assert!(std::str::from_utf8(&unrec.login_fixture)
        .expect("utf8")
        .contains("action=\"unexpected.do\""));

    let any_ok = hidden_any_id_chooser_opens_id_method_once(true);
    assert!(std::str::from_utf8(&any_ok.any_id_fixture)
        .expect("utf8")
        .contains("href=\"javascript:testFnLogin();\""));
    let any_bad = hidden_any_id_chooser_opens_id_method_once(false);
    assert!(std::str::from_utf8(&any_bad.any_id_fixture)
        .expect("utf8")
        .contains("href=\"javascript:unrecognizedLogin();\""));
    assert!(std::str::from_utf8(&any_bad.login_fixture)
        .expect("utf8")
        .contains("<title>public ID form</title>"));
}

#[test]
fn remaining_presets_and_official_controller_builder_serve_over_http() {
    let exact = final_list_requires_exact_sheet_number();
    assert!(std::str::from_utf8(&exact.download_fixture)
        .expect("utf8")
        .contains("13780449.zip"));

    let cached = cached_actual_sheet_skips_login_and_application();
    assert_eq!(cached.portal_fixture, b"<html><title>cached fixture</title></html>");

    let wrong = unexpected_product_stops_before_consent_or_application();
    assert!(std::str::from_utf8(&wrong.search_fixture)
        .expect("utf8")
        .contains("\"fileExt\":\"shp\""));

    let empty = empty_availability_is_reported_without_application();
    assert_eq!(empty.search_fixture, br#"{"items":[]}"#);

    let ready = file_selection_waits_until_download_page_is_ready();
    assert!(std::str::from_utf8(&ready.download_fixture)
        .expect("utf8")
        .contains("terrain_378044.zip"));

    let no_agent = official_controller_starts_without_an_external_agent();
    assert!(std::str::from_utf8(&no_agent.download_fixture)
        .expect("utf8")
        .contains("transferMode:'download'"));

    let c1 = official_controller_transfers_only_the_requested_sheet(true, "fixture-data");
    let c1_html = std::str::from_utf8(&c1.download_fixture).expect("utf8");
    assert!(c1_html.contains("id:'fixture-data'") && c1_html.contains("terrain_378044.zip"));
    let c2 = official_controller_transfers_only_the_requested_sheet(false, "");
    let c2_html = std::str::from_utf8(&c2.download_fixture).expect("utf8");
    assert!(c2_html.contains("id:''") && c2_html.contains("terrain_378043.zip"));

    let batch = batch_application_serializes_actual_http_completion_and_waits_for_each_xml();
    assert!(batch.redirect_transfer && batch.hold_sdk_transfers);
    let site = NgiiFixtureSite::start_with(batch).expect("start batch");
    assert_eq!(
        http_get_body(&site, "/ms/map/NlipMap.do"),
        batch_portal_fixture().as_bytes()
    );
    assert_eq!(
        http_get_body(&site, "/nlippd/pd/innorix/innorixdownLoad.do"),
        batch_sdk_fixture().as_bytes()
    );

    // Default-configured C++ test presets
    for preset in [
        compact_download_keeps_progress_and_offers_explicit_details(),
        compact_popup_and_external_transfer_never_expose_site(),
        compact_transfer_uses_received_bytes_and_preparation_never_completes_early(),
        compact_unknown_length_stays_indeterminate_and_cancel_keeps_snapshot(),
        compact_interrupted_http_transfer_preserves_failure_and_received_bytes(),
        changed_account_replaces_session_and_same_account_keeps_it(),
        login_status_checks_are_serialized_and_expire_without_reopening(),
        popup_shares_ephemeral_login_session(),
        completed_attachment_has_unique_path_and_cancel_does_not_import(),
    ] {
        assert!(preset.logged_in && !preset.fail_portal_connection);
    }
}
