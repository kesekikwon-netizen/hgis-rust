//! Verbatim fixtures (`include_str!`), runtime builders, and test presets part B
//! (`target/cpp-ref/tests/test_topographic_browser.cpp:685-1415`).

use crate::fixtures_a::{batch_portal_fixture, batch_sdk_fixture};
use crate::NgiiFixtureState;

const KNOWN_LOGIN_SEARCH_JSON: &str = include_str!(
    "../fixtures/known_official_login_submits_only_once_and_stop_prevents_more_work__search_fixture.json"
);
const KNOWN_LOGIN_PORTAL_HTML: &str = include_str!(
    "../fixtures/known_official_login_submits_only_once_and_stop_prevents_more_work__portal_fixture.html"
);
const KNOWN_LOGIN_FORM_HTML: &str = include_str!(
    "../fixtures/known_official_login_submits_only_once_and_stop_prevents_more_work__login_fixture.html"
);
const AUTO_LOGIN_SEARCH_JSON: &str = include_str!(
    "../fixtures/automatic_login_waits_until_the_form_can_submit__search_fixture.json"
);
const AUTO_LOGIN_PORTAL_HTML: &str = include_str!(
    "../fixtures/automatic_login_waits_until_the_form_can_submit__portal_fixture.html"
);
const AUTO_LOGIN_FORM_HTML: &str = include_str!(
    "../fixtures/automatic_login_waits_until_the_form_can_submit__login_fixture.html"
);
const UNRECOGNIZED_LOGIN_HTML: &str = include_str!(
    "../fixtures/completed_unrecognized_login_form_never_receives_credentials__login_fixture.html"
);
const HIDDEN_ANY_ID_HTML: &str = include_str!(
    "../fixtures/hidden_any_id_chooser_opens_id_method_once__any_id_fixture.html"
);
const HIDDEN_ANY_ID_LOGIN_HTML: &str = include_str!(
    "../fixtures/hidden_any_id_chooser_opens_id_method_once__login_fixture.html"
);
const FILE_SELECTION_WAIT_HTML: &str = include_str!(
    "../fixtures/file_selection_waits_until_download_page_is_ready__download_fixture.html"
);
const CONTROLLER_NO_AGENT_HTML: &str = include_str!(
    "../fixtures/official_controller_starts_without_an_external_agent__download_fixture.html"
);
const CONTROLLER_REQUESTED_SHEET_HTML: &str = include_str!(
    "../fixtures/official_controller_transfers_only_the_requested_sheet__download_fixture.html"
);
const BATCH_APPLICATION_SEARCH_JSON: &str = include_str!(
    "../fixtures/batch_application_serializes_actual_http_completion_and_waits_for_each_xml__search_fixture.json"
);

/// Builds `loginFixture` for `automaticLoginWaitsUntilTheFormCanSubmit` (`test_topographic_browser.cpp:748-755`).
pub fn build_automatic_login_fixture(parsing_paused: bool) -> String {
    let pause = if parsing_paused {
        "<script src='/held-login-script.js'></script>"
    } else {
        ""
    };
    let ready = if parsing_paused {
        "document.addEventListener('DOMContentLoaded',finishReady);"
    } else {
        ""
    };
    AUTO_LOGIN_FORM_HTML
        .replace("PAUSE", pause)
        .replace("READY", ready)
}

/// Builds `anyIdFixture` for `hiddenAnyIdChooserOpensIdMethodOnce` (`test_topographic_browser.cpp:883-887`).
pub fn build_hidden_any_id_fixture(matching_contract: bool) -> String {
    if matching_contract {
        HIDDEN_ANY_ID_HTML.to_owned()
    } else {
        HIDDEN_ANY_ID_HTML.replace(
            "href=\"javascript:testFnLogin();\"",
            "href=\"javascript:unrecognizedLogin();\"",
        )
    }
}

/// Builds `downloadFixture` for `officialControllerTransfersOnlyTheRequestedSheet` (`test_topographic_browser.cpp:968-974`).
pub fn build_official_controller_sheet_download_fixture(matching: bool, file_id: &str) -> String {
    let number = if matching { "378044" } else { "378043" };
    CONTROLLER_REQUESTED_SHEET_HTML
        .replace("NUMBER", number)
        .replace("FILE_ID", file_id)
}

/// `knownOfficialLoginSubmitsOnlyOnceAndStopPreventsMoreWork` (`test_topographic_browser.cpp:690-701`).
pub fn known_official_login_submits_only_once_and_stop_prevents_more_work(
    unanswered: bool,
) -> NgiiFixtureState {
    NgiiFixtureState {
        logged_in: false,
        hold_login_response: unanswered,
        search_fixture: KNOWN_LOGIN_SEARCH_JSON.as_bytes().to_vec(),
        portal_fixture: KNOWN_LOGIN_PORTAL_HTML.as_bytes().to_vec(),
        login_fixture: KNOWN_LOGIN_FORM_HTML.as_bytes().to_vec(),
        ..NgiiFixtureState::default()
    }
}

/// `automaticLoginWaitsUntilTheFormCanSubmit` (`test_topographic_browser.cpp:738-755`).
pub fn automatic_login_waits_until_the_form_can_submit(parsing_paused: bool) -> NgiiFixtureState {
    NgiiFixtureState {
        logged_in: false,
        accept_login: true,
        search_fixture: AUTO_LOGIN_SEARCH_JSON.as_bytes().to_vec(),
        portal_fixture: AUTO_LOGIN_PORTAL_HTML.as_bytes().to_vec(),
        login_fixture: build_automatic_login_fixture(parsing_paused).into_bytes(),
        ..NgiiFixtureState::default()
    }
}

/// `genuinePageConnectionFailureStopsAutomaticDownload` (`test_topographic_browser.cpp:795-798`).
pub fn genuine_page_connection_failure_stops_automatic_download() -> NgiiFixtureState {
    NgiiFixtureState {
        fail_portal_connection: true,
        ..NgiiFixtureState::default()
    }
}

/// `completedUnrecognizedLoginFormNeverReceivesCredentials` (`test_topographic_browser.cpp:815-817`).
pub fn completed_unrecognized_login_form_never_receives_credentials() -> NgiiFixtureState {
    NgiiFixtureState {
        login_fixture: UNRECOGNIZED_LOGIN_HTML.as_bytes().to_vec(),
        ..NgiiFixtureState::default()
    }
}

/// `loginStatusChecksAreSerializedAndExpireWithoutReopening` (`test_topographic_browser.cpp:835-836`).
pub fn login_status_checks_are_serialized_and_expire_without_reopening() -> NgiiFixtureState {
    NgiiFixtureState::default()
}

/// `hiddenAnyIdChooserOpensIdMethodOnce` (`test_topographic_browser.cpp:878-888`).
pub fn hidden_any_id_chooser_opens_id_method_once(matching_contract: bool) -> NgiiFixtureState {
    NgiiFixtureState {
        any_id_fixture: build_hidden_any_id_fixture(matching_contract).into_bytes(),
        login_fixture: HIDDEN_ANY_ID_LOGIN_HTML.as_bytes().to_vec(),
        ..NgiiFixtureState::default()
    }
}

/// `fileSelectionWaitsUntilDownloadPageIsReady` (`test_topographic_browser.cpp:919-921`).
pub fn file_selection_waits_until_download_page_is_ready() -> NgiiFixtureState {
    NgiiFixtureState {
        download_fixture: FILE_SELECTION_WAIT_HTML.as_bytes().to_vec(),
        ..NgiiFixtureState::default()
    }
}

/// `officialControllerStartsWithoutAnExternalAgent` (`test_topographic_browser.cpp:939-944`).
pub fn official_controller_starts_without_an_external_agent() -> NgiiFixtureState {
    NgiiFixtureState {
        download_fixture: CONTROLLER_NO_AGENT_HTML.as_bytes().to_vec(),
        ..NgiiFixtureState::default()
    }
}

/// `officialControllerTransfersOnlyTheRequestedSheet` (`test_topographic_browser.cpp:964-974`).
pub fn official_controller_transfers_only_the_requested_sheet(
    matching: bool,
    file_id: &str,
) -> NgiiFixtureState {
    NgiiFixtureState {
        download_fixture: build_official_controller_sheet_download_fixture(matching, file_id)
            .into_bytes(),
        ..NgiiFixtureState::default()
    }
}

/// `batchApplicationSerializesActualHttpCompletionAndWaitsForEachXml` (`test_topographic_browser.cpp:1011-1015`).
pub fn batch_application_serializes_actual_http_completion_and_waits_for_each_xml() -> NgiiFixtureState {
    NgiiFixtureState {
        portal_fixture: batch_portal_fixture().as_bytes().to_vec(),
        download_fixture: batch_sdk_fixture().as_bytes().to_vec(),
        redirect_transfer: true,
        hold_sdk_transfers: true,
        search_fixture: BATCH_APPLICATION_SEARCH_JSON.as_bytes().to_vec(),
        ..NgiiFixtureState::default()
    }
}

/// `popupSharesEphemeralLoginSession` (`test_topographic_browser.cpp:1368-1369`).
pub fn popup_shares_ephemeral_login_session() -> NgiiFixtureState {
    NgiiFixtureState::default()
}

/// `completedAttachmentHasUniquePathAndCancelDoesNotImport` (`test_topographic_browser.cpp:1390-1391`).
pub fn completed_attachment_has_unique_path_and_cancel_does_not_import() -> NgiiFixtureState {
    NgiiFixtureState::default()
}
