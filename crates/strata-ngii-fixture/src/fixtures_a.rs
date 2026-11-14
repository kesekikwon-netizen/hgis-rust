//! Verbatim fixtures (`include_str!`), runtime builders, and test presets part A
//! (`target/cpp-ref/tests/test_topographic_browser.cpp:133-683`).

use crate::NgiiFixtureState;
use std::collections::BTreeMap;

/// Verbatim JSON for `/ms/map/combineBufferQuery.do` (`test_topographic_browser.cpp:133`).
pub const COMBINE_BUFFER_QUERY_JSON: &str =
    include_str!("../fixtures/local_download_server__combine_buffer_query.json");

const BATCH_SDK_HTML: &str = include_str!("../fixtures/batch_sdk_fixture__download_fixture.html");
const BATCH_PORTAL_HTML: &str =
    include_str!("../fixtures/batch_portal_fixture__portal_fixture.html");
const FINAL_LIST_DOWNLOAD_HTML: &str =
    include_str!("../fixtures/final_list_requires_exact_sheet_number__download_fixture.html");
const AUTO_SELECT_SEARCH_JSON: &str = include_str!(
    "../fixtures/automatic_selection_preserves_product_and_submits_once__search_fixture.json"
);
const AUTO_SELECT_PORTAL_HTML: &str = include_str!(
    "../fixtures/automatic_selection_preserves_product_and_submits_once__portal_fixture.html"
);
const AUTO_SELECT_LOGIN_HTML: &str = include_str!(
    "../fixtures/automatic_selection_preserves_product_and_submits_once__login_fixture.html"
);
const AUTO_SELECT_DOWNLOAD_1_HTML: &str = include_str!(
    "../fixtures/automatic_selection_preserves_product_and_submits_once__download_fixture__1.html"
);
const AUTO_SELECT_DOWNLOAD_2_HTML: &str = include_str!(
    "../fixtures/automatic_selection_preserves_product_and_submits_once__download_fixture__2.html"
);
const AUTO_SELECT_DOWNLOAD_3_HTML: &str = include_str!(
    "../fixtures/automatic_selection_preserves_product_and_submits_once__download_fixture__3.html"
);
const CACHED_SHEET_PORTAL_HTML: &str = include_str!(
    "../fixtures/cached_actual_sheet_skips_login_and_application__portal_fixture.html"
);
const CACHED_SHEET_SEARCH_JSON: &str = include_str!(
    "../fixtures/cached_actual_sheet_skips_login_and_application__search_fixture.json"
);
const UNEXPECTED_PRODUCT_PORTAL_HTML: &str = include_str!(
    "../fixtures/unexpected_product_stops_before_consent_or_application__portal_fixture.html"
);
const UNEXPECTED_PRODUCT_SEARCH_JSON: &str = include_str!(
    "../fixtures/unexpected_product_stops_before_consent_or_application__search_fixture.json"
);
const EMPTY_AVAILABILITY_PORTAL_HTML: &str =
    include_str!("../fixtures/empty_availability_is_reported_without_application__portal_fixture.html");
const EMPTY_AVAILABILITY_SEARCH_JSON: &str =
    include_str!("../fixtures/empty_availability_is_reported_without_application__search_fixture.json");

/// `batchSdkFixture()` (`target/cpp-ref/tests/test_topographic_browser.cpp:194-205`).
pub fn batch_sdk_fixture() -> &'static str {
    BATCH_SDK_HTML
}

/// `batchPortalFixture()` (`target/cpp-ref/tests/test_topographic_browser.cpp:206-218`).
pub fn batch_portal_fixture() -> &'static str {
    BATCH_PORTAL_HTML
}

fn fixture_sheets_compact_array(retained_modal: bool) -> String {
    let start = AUTO_SELECT_SEARCH_JSON
        .find("[{")
        .map(|i| i + 2)
        .unwrap_or(0);
    let end = AUTO_SELECT_SEARCH_JSON.rfind("}]").unwrap_or(start);
    let mut first: BTreeMap<String, String> = BTreeMap::new();
    for field in AUTO_SELECT_SEARCH_JSON[start..end].split(',') {
        if let Some((k, v)) = field.split_once(':') {
            first.insert(k.trim().trim_matches('"').to_owned(), v.trim().to_owned());
        }
    }
    let format_obj = |map: &BTreeMap<String, String>| -> String {
        let entries: Vec<String> = map.iter().map(|(k, v)| format!("\"{k}\":{v}")).collect();
        format!("{{{}}}", entries.join(","))
    };
    if retained_modal {
        let mut second = first.clone();
        second.insert("num".to_owned(), "\"378043\"".to_owned());
        format!("[{},{}]", format_obj(&first), format_obj(&second))
    } else {
        format!("[{}]", format_obj(&first))
    }
}

/// Builds `searchFixture` via sorted-key `QJsonDocument::Compact` (`test_topographic_browser.cpp:483-486`).
pub fn build_automatic_selection_search_fixture(retained_modal: bool) -> String {
    format!("{{\"items\":{}}}", fixture_sheets_compact_array(retained_modal))
}

/// Builds `portalFixture` with `.replace(...)` steps (`test_topographic_browser.cpp:487-503`).
pub fn build_automatic_selection_portal_fixture(
    application_succeeds: bool,
    retained_modal: bool,
    reload_before_submit: bool,
    compact: bool,
) -> String {
    let mut html = AUTO_SELECT_PORTAL_HTML.replace(
        "<script>",
        if application_succeeds {
            "<script>window.fixtureApplicationSucceeds=true;"
        } else {
            "<script>window.fixtureApplicationSucceeds=false;"
        },
    );
    let sheets = fixture_sheets_compact_array(retained_modal);
    let retained_str = if retained_modal { "true;" } else { "false;" };
    html = html.replace(
        "<script>",
        &format!("<script>window.fixtureSheets={sheets};window.fixtureRetainedModal={retained_str}"),
    );
    if reload_before_submit {
        html = html.replace(
            "document.body.appendChild(f);",
            "document.body.appendChild(f);f.querySelector('[name=onBrthdy]').oninput=()=>{if(!sessionStorage.fixtureReloaded){sessionStorage.fixtureReloaded='1';location.reload();}};",
        );
    }
    if compact {
        html = html.replace(
            "if(window.fixtureApplicationSucceeds)window.open('/pd/purchs/download.do','downForm');",
            "if(window.fixtureApplicationSucceeds)setTimeout(()=>window.open('/pd/purchs/download.do','downForm'),1200);",
        );
    }
    html
}

/// `compactDownloadKeepsProgressAndOffersExplicitDetails` (`test_topographic_browser.cpp:236-237`).
pub fn compact_download_keeps_progress_and_offers_explicit_details() -> NgiiFixtureState {
    NgiiFixtureState::default()
}

/// `compactPopupAndExternalTransferNeverExposeSite` (`test_topographic_browser.cpp:284-285`).
pub fn compact_popup_and_external_transfer_never_expose_site() -> NgiiFixtureState {
    NgiiFixtureState::default()
}

/// `compactTransferUsesReceivedBytesAndPreparationNeverCompletesEarly` (`test_topographic_browser.cpp:310-311`).
pub fn compact_transfer_uses_received_bytes_and_preparation_never_completes_early() -> NgiiFixtureState {
    NgiiFixtureState::default()
}

/// `compactUnknownLengthStaysIndeterminateAndCancelKeepsSnapshot` (`test_topographic_browser.cpp:348-349`).
pub fn compact_unknown_length_stays_indeterminate_and_cancel_keeps_snapshot() -> NgiiFixtureState {
    NgiiFixtureState::default()
}

/// `compactInterruptedHttpTransferPreservesFailureAndReceivedBytes` (`test_topographic_browser.cpp:376-377`).
pub fn compact_interrupted_http_transfer_preserves_failure_and_received_bytes() -> NgiiFixtureState {
    NgiiFixtureState::default()
}

/// `changedAccountReplacesSessionAndSameAccountKeepsIt` (`test_topographic_browser.cpp:403-404`).
pub fn changed_account_replaces_session_and_same_account_keeps_it() -> NgiiFixtureState {
    NgiiFixtureState::default()
}

/// `finalListRequiresExactSheetNumber` (`test_topographic_browser.cpp:438-440`).
pub fn final_list_requires_exact_sheet_number() -> NgiiFixtureState {
    NgiiFixtureState {
        download_fixture: FINAL_LIST_DOWNLOAD_HTML.as_bytes().to_vec(),
        ..NgiiFixtureState::default()
    }
}

/// `automaticSelectionPreservesProductAndSubmitsOnce` (`test_topographic_browser.cpp:471-511`).
pub fn automatic_selection_preserves_product_and_submits_once(
    application_succeeds: bool,
    retained_modal: bool,
    reload_before_submit: bool,
    compact: bool,
    login_required: bool,
    unsupported_transfer: bool,
    redirect_transfer: bool,
) -> NgiiFixtureState {
    let mut state = NgiiFixtureState {
        redirect_transfer,
        logged_in: !login_required,
        accept_login: login_required,
        stream_fixture_archives: compact && !unsupported_transfer,
        search_fixture: build_automatic_selection_search_fixture(retained_modal).into_bytes(),
        portal_fixture: build_automatic_selection_portal_fixture(
            application_succeeds,
            retained_modal,
            reload_before_submit,
            compact,
        )
        .into_bytes(),
        login_fixture: AUTO_SELECT_LOGIN_HTML.as_bytes().to_vec(),
        download_fixture: AUTO_SELECT_DOWNLOAD_1_HTML.as_bytes().to_vec(),
        ..NgiiFixtureState::default()
    };
    if compact {
        state.download_fixture = AUTO_SELECT_DOWNLOAD_2_HTML.as_bytes().to_vec();
    }
    if retained_modal {
        state.redirect_transfer = true;
        state.download_fixture = BATCH_SDK_HTML.as_bytes().to_vec();
    }
    if unsupported_transfer {
        state.download_fixture = AUTO_SELECT_DOWNLOAD_3_HTML.as_bytes().to_vec();
    }
    state
}

/// `cachedActualSheetSkipsLoginAndApplication` (`test_topographic_browser.cpp:639-642`).
pub fn cached_actual_sheet_skips_login_and_application() -> NgiiFixtureState {
    NgiiFixtureState {
        portal_fixture: CACHED_SHEET_PORTAL_HTML.as_bytes().to_vec(),
        search_fixture: CACHED_SHEET_SEARCH_JSON.as_bytes().to_vec(),
        ..NgiiFixtureState::default()
    }
}

/// `unexpectedProductStopsBeforeConsentOrApplication` (`test_topographic_browser.cpp:656-659`).
pub fn unexpected_product_stops_before_consent_or_application() -> NgiiFixtureState {
    NgiiFixtureState {
        portal_fixture: UNEXPECTED_PRODUCT_PORTAL_HTML.as_bytes().to_vec(),
        search_fixture: UNEXPECTED_PRODUCT_SEARCH_JSON.as_bytes().to_vec(),
        ..NgiiFixtureState::default()
    }
}

/// `emptyAvailabilityIsReportedWithoutApplication` (`test_topographic_browser.cpp:670-672`).
pub fn empty_availability_is_reported_without_application() -> NgiiFixtureState {
    NgiiFixtureState {
        portal_fixture: EMPTY_AVAILABILITY_PORTAL_HTML.as_bytes().to_vec(),
        search_fixture: EMPTY_AVAILABILITY_SEARCH_JSON.as_bytes().to_vec(),
        ..NgiiFixtureState::default()
    }
}
