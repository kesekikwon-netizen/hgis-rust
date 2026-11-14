//! The cases of the C++ browser test that a hidden run must pass
//! (`target/cpp-ref/tests/test_topographic_browser.cpp`), with the fake portal set up as each test sets it.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use strata_net::ngii_auto::{AutoEvent, Stage};
use strata_ngii_fixture as portal;
use strata_ngii_fixture::NgiiFixtureState;

/// What a case saw by the time its run was over.
pub struct Seen {
    pub events: Vec<AutoEvent>,
    pub phases: Vec<String>,
    pub stage: Stage,
    pub failure_phase: String,
    pub downloads: PathBuf,
    /// Most bytes of one file reported while it was being received.
    pub most_received: u64,
}

impl Seen {
    fn selections(&self) -> Vec<(&Value, &Value, &Value)> {
        self.events.iter().filter_map(|e| if let AutoEvent::SelectionUpdated { items, cached, unavailable } = e { Some((items, cached, unavailable)) } else { None }).collect()
    }

    fn sheets(&self) -> Vec<(&Path, &serde_json::Map<String, Value>)> {
        self.events.iter().filter_map(|e| if let AutoEvent::SheetDownloaded { path, record } = e { Some((path.as_path(), record)) } else { None }).collect()
    }

    fn notices(&self) -> Vec<&str> {
        self.events.iter().filter_map(|e| if let AutoEvent::NeedsInput(text) = e { Some(text.as_str()) } else { None }).collect()
    }
}

pub struct Case {
    pub name: &'static str,
    pub portal: fn() -> NgiiFixtureState,
    pub numbers: &'static [&'static str],
    pub cached: &'static [&'static str],
    /// Whether the account is given (the C++ test sets it only when the portal asks for a login).
    pub with_account: bool,
    /// Whether the fake portal holds its transfers open until the test lets them finish.
    pub held_transfers: bool,
    pub check: fn(&Seen, &NgiiFixtureState) -> Vec<String>,
}

fn expect(failures: &mut Vec<String>, ok: bool, what: &str) {
    if !ok {
        failures.push(what.to_owned());
    }
}

/// The closing checks of `automaticSelectionPreservesProductAndSubmitsOnce` for a completed order
/// (`:596-637`): one selection naming the sheet that does not exist, the sheet's own record, the files
/// in the survey folder, one search.
fn completed(seen: &Seen, portal: &NgiiFixtureState, searches: usize) -> Vec<String> {
    let mut failures = Vec::new();
    expect(&mut failures, seen.stage == Stage::Complete, "the run completes");
    expect(&mut failures, seen.selections().len() == searches, "one selection per search");
    expect(&mut failures, seen.selections().first().is_some_and(|s| *s.2 == json!(["378999"])), "the sheet that does not exist is named as unavailable");
    let sheets = seen.sheets();
    expect(&mut failures, sheets.len() == 1, "one sheet is handed on");
    if let Some((path, record)) = sheets.first() {
        let field = |key: &str| record.get(key).and_then(Value::as_str).unwrap_or_default().to_owned();
        expect(&mut failures, (field("num"), field("projection"), field("fileExt")) == ("378044".into(), "GRS80".into(), "dxf".into()), "the record is the portal's own");
        expect(&mut failures, path.starts_with(&seen.downloads) && path.extension().is_some_and(|e| e == "zip") && path.is_file(), "the data file is in the survey folder");
        expect(&mut failures, path.with_file_name("terrain_378044.xml").is_file(), "its metadata file lies beside it");
        expect(&mut failures, path.parent().is_some_and(|d| std::fs::read_dir(d).map_or(0, Iterator::count) == 2), "nothing else is left in the sheet's folder");
    }
    expect(&mut failures, portal.search_count == searches, "the portal was searched as often as the C++ test expects");
    for phase in ["도엽 확인", "로그인", "신청", "파일 준비", "다운로드"] {
        expect(&mut failures, seen.phases.iter().any(|p| p == phase), &format!("the phase {phase} was shown"));
    }
    expect(&mut failures, seen.most_received >= portal::FIRST_CHUNK_SIZE as u64, "received bytes were reported while the file was held open");
    failures
}

pub fn cases() -> Vec<Case> {
    vec![
        Case {
            name: "all sheets kept: no login, no application (cachedActualSheetSkipsLoginAndApplication)",
            portal: portal::cached_actual_sheet_skips_login_and_application,
            numbers: &["378999"],
            cached: &["378044"],
            with_account: false,
            held_transfers: false,
            check: |seen, portal| {
                let mut failures = Vec::new();
                expect(&mut failures, seen.stage == Stage::Complete, "the run completes");
                expect(&mut failures, seen.selections().len() == 1, "one selection");
                expect(&mut failures, seen.selections().first().is_some_and(|s| s.0.as_array().is_some_and(|a| a.len() == 1) && *s.1 == json!(["378044"])), "one sheet, kept");
                expect(&mut failures, portal.login_submission_count == 0 && seen.sheets().is_empty(), "nothing is applied for or fetched");
                failures
            },
        },
        Case {
            name: "nothing available is said without an application (emptyAvailabilityIsReportedWithoutApplication)",
            portal: portal::empty_availability_is_reported_without_application,
            numbers: &["378044"],
            cached: &[],
            with_account: false,
            held_transfers: false,
            check: |seen, _| {
                let mut failures = Vec::new();
                expect(&mut failures, seen.selections().len() == 1, "one selection");
                expect(&mut failures, seen.selections().first().is_some_and(|s| s.0.as_array().is_some_and(Vec::is_empty) && s.2.as_array().is_some_and(|a| a.len() == 1)), "no sheet, one unavailable");
                expect(&mut failures, seen.sheets().is_empty() && seen.stage == Stage::Stopped, "nothing is fetched");
                expect(&mut failures, seen.notices() == vec!["이 범위에서 제공 중인 1:25,000 DXF 도엽이 없습니다."], "the C++ words");
                failures
            },
        },
        Case {
            name: "another product stops before consent or application (unexpectedProductStopsBeforeConsentOrApplication)",
            portal: portal::unexpected_product_stops_before_consent_or_application,
            numbers: &["378044"],
            cached: &[],
            with_account: false,
            held_transfers: false,
            check: |seen, _| {
                let mut failures = Vec::new();
                expect(&mut failures, !seen.notices().is_empty() && seen.stage == Stage::Stopped, "the run stops and says why");
                expect(&mut failures, seen.selections().is_empty() && seen.sheets().is_empty(), "no selection, no file");
                failures
            },
        },
        Case {
            name: "a hidden order completes in the survey folder (automaticSelection…: compact-hidden-portal-completes-in-survey-folder)",
            portal: || portal::automatic_selection_preserves_product_and_submits_once(true, false, false, true, false, false, false),
            numbers: &["378999"],
            cached: &[],
            with_account: false,
            held_transfers: true,
            check: |seen, portal| completed(seen, portal, 1),
        },
        Case {
            name: "a hidden login resumes one application and one download (…: hidden-login-completion-resumes-one-application-and-download)",
            portal: || portal::automatic_selection_preserves_product_and_submits_once(true, false, false, true, true, false, false),
            numbers: &["378999"],
            cached: &[],
            with_account: true,
            held_transfers: true,
            check: |seen, portal| {
                let mut failures = completed(seen, portal, 1);
                expect(&mut failures, portal.logged_in && portal.login_check_count >= 2, "the portal saw the login");
                expect(&mut failures, portal.login_submission_count == 1, "the account was submitted exactly once");
                failures
            },
        },
        Case {
            name: "a transfer page of an unknown kind stops at file preparation (…: unsupported-final-transfer-stays-visible-at-file-preparation)",
            portal: || portal::automatic_selection_preserves_product_and_submits_once(true, false, false, true, false, true, false),
            numbers: &["378999"],
            cached: &[],
            with_account: false,
            held_transfers: false,
            check: |seen, portal| {
                let mut failures = Vec::new();
                expect(&mut failures, seen.notices().last().is_some_and(|n| n.contains("공식 파일 전송")), "the notice names the official file transfer");
                expect(&mut failures, seen.stage == Stage::Stopped && seen.failure_phase == "파일 준비", "it stopped at 파일 준비");
                expect(&mut failures, seen.sheets().is_empty() && portal.search_count == 1, "nothing was fetched, one search");
                failures
            },
        },
        Case {
            name: "the transfer page behind a redirect keeps its order (…: official-final-transfer-redirect-retains-order)",
            portal: || portal::automatic_selection_preserves_product_and_submits_once(true, false, false, true, false, false, true),
            numbers: &["378999"],
            cached: &[],
            with_account: false,
            held_transfers: true,
            check: |seen, portal| completed(seen, portal, 1),
        },
        Case {
            name: "an order the server rejects stays before the download (…: server-rejected-order-stays-before-download)",
            portal: || portal::automatic_selection_preserves_product_and_submits_once(false, false, false, true, false, false, false),
            numbers: &["378999"],
            cached: &[],
            with_account: false,
            held_transfers: false,
            check: |seen, _| {
                let mut failures = Vec::new();
                expect(&mut failures, seen.notices().last().is_some_and(|n| n.contains("공식 서버")), "the notice names the official server");
                expect(&mut failures, seen.sheets().is_empty(), "nothing was fetched");
                failures
            },
        },
        Case {
            name: "a page that loads anew after the form starts over and still applies once (…: document-reload-after-form-before-submit)",
            portal: || portal::automatic_selection_preserves_product_and_submits_once(true, false, true, true, false, false, false),
            numbers: &["378999"],
            cached: &[],
            with_account: false,
            held_transfers: true,
            check: |seen, portal| completed(seen, portal, 2),
        },
    ]
}
