//! Which page is asked what, and when (`startAutomation`, `pollAutomation`, `prepareSheets`).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use serde_json::json;

use super::tests_support::{config, needs_input, round, scope, started, to_files, FakeHost, LOGIN_POPUP, RESET, SELECTED, STEP, TRANSFER};
use super::{NgiiAutomation, PageId, Scope, Stage, PORTAL_HOME};

/// `:698-727` and the input of `:757-784`.
#[test]
fn starting_clears_the_page_state_and_the_first_poll_asks_for_the_selection() {
    let dir = tempfile::tempdir().unwrap();
    let (mut machine, mut host) = started(dir.path());
    assert_eq!((machine.stage(), machine.phase()), (Stage::Select, "도엽 확인"));
    assert!(host.navigations.is_empty(), "the portal page is already open");
    assert_eq!(host.other_scripts(), vec![(PageId(1), RESET.to_owned())]);
    let mut now = 0;
    let (page, input) = round(&mut machine, &mut host, &mut now, "{}");
    assert_eq!(page, PageId(1));
    let want = json!({"x": 1_100_000.5, "y": 1_750_000.0, "radius": 5000.0, "numbers": ["377051", "377052"], "cachedNumbers": ["377064"],
        "command": "select", "diagnostics": false, "generation": "1", "record": {}, "records": [], "fileIndex": 0});
    assert_eq!(input, want);
}

#[test]
fn without_a_portal_page_the_first_page_is_sent_there() {
    let dir = tempfile::tempdir().unwrap();
    let mut host = FakeHost::default();
    let mut machine = NgiiAutomation::new(config(dir.path()));
    machine.prepare_sheets(&mut host, scope(), 0);
    assert_eq!(host.navigations, vec![PORTAL_HOME.to_owned()]);
    assert_eq!(machine.stage(), Stage::Select);
}

/// `:663-671`
#[test]
fn a_scope_that_cannot_be_checked_is_refused_in_the_cpp_words() {
    let dir = tempfile::tempdir().unwrap();
    let many: Vec<String> = (0..101).map(|i| format!("{:06}", 300_000 + i)).collect();
    let bad: Vec<Scope> = vec![
        Scope { numbers: many, ..scope() },
        Scope { numbers: vec!["37705".into()], ..scope() },
        Scope { numbers: vec!["37705a".into()], ..scope() },
        Scope { cached_numbers: vec!["12".into()], ..scope() },
        Scope { radius: 0.0, ..scope() },
        Scope { radius: f64::NAN, ..scope() },
        Scope { x: f64::INFINITY, ..scope() },
    ];
    for scope in bad {
        let mut host = FakeHost::with_portal();
        let mut machine = NgiiAutomation::new(config(dir.path()));
        machine.prepare_sheets(&mut host, scope.clone(), 0);
        assert_eq!(needs_input(&machine.take_events()), vec!["자동 선택할 1:25,000 도엽 목록과 반경을 확인하지 못했습니다."], "{scope:?}");
        assert_eq!(machine.stage(), Stage::Idle);
        assert!(host.runs.is_empty() && host.navigations.is_empty());
    }
}

/// `:672-682` — the same circle, or the same sheets again, starts nothing new; a new circle waits while an
/// order is being fetched.
#[test]
fn the_same_request_is_not_started_twice() {
    let dir = tempfile::tempdir().unwrap();
    let (mut machine, mut host) = started(dir.path());
    let first = machine.generation();
    machine.prepare_sheets(&mut host, scope(), 10);
    machine.prepare_sheets(&mut host, Scope { x: 1_100_003.0, ..scope() }, 20);
    assert_eq!(machine.generation(), first, "a few metres of wobble must not start the same sheets again");
    machine.prepare_sheets(&mut host, Scope { numbers: vec!["377053".into()], ..scope() }, 30);
    assert_eq!(machine.generation(), first + 1, "other sheets do start again while nothing is ordered yet");

    let (mut machine, mut host) = started(dir.path());
    to_files(&mut machine, &mut host);
    let ordered = machine.generation();
    machine.prepare_sheets(&mut host, Scope { numbers: vec!["377053".into()], ..scope() }, 9000);
    assert_eq!((machine.generation(), machine.stage()), (ordered, Stage::Files));
    assert_eq!(machine.status(), "현재 신청한 도엽을 받은 뒤 새 조사 범위의 자료를 확인합니다.");
}

/// `:221, 729-730`
#[test]
fn polls_come_every_400_ms_and_wait_for_the_answer() {
    let dir = tempfile::tempdir().unwrap();
    let (mut machine, mut host) = started(dir.path());
    machine.tick(&mut host, STEP - 1);
    assert_eq!(host.calls().len(), 0);
    machine.tick(&mut host, STEP);
    assert_eq!(host.calls().len(), 1);
    machine.tick(&mut host, 2 * STEP);
    machine.tick(&mut host, 3 * STEP);
    assert_eq!(host.calls().len(), 1, "no second call while the first is unanswered");
    let token = host.calls()[0].2;
    machine.script_answered(&mut host, token, r#"{"state":"querying"}"#, 3 * STEP);
    machine.tick(&mut host, 4 * STEP);
    assert_eq!(host.calls().len(), 2);
}

/// `:756-760`
#[test]
fn a_missing_portal_page_stops_the_run_after_75_polls() {
    let dir = tempfile::tempdir().unwrap();
    let mut host = FakeHost::default();
    let mut machine = NgiiAutomation::new(config(dir.path()));
    machine.prepare_sheets(&mut host, scope(), 0);
    for poll in 1..75 {
        machine.tick(&mut host, poll * STEP);
    }
    assert_eq!(machine.stage(), Stage::Select);
    machine.tick(&mut host, 75 * STEP);
    assert_eq!(machine.stage(), Stage::Stopped);
    assert_eq!(needs_input(&machine.take_events()), vec!["공식 사이트의 응답을 확인하지 못했습니다. 계정 설정과 연결 상태를 확인하세요."]);
    assert_eq!(machine.failure_phase(), "도엽 확인");
    machine.tick(&mut host, 76 * STEP);
    assert!(host.calls().is_empty() && machine.take_events().is_empty());
}

/// `:741-755, 780-784` — a login page is asked instead of the map page, and only it gets the account.
#[test]
fn a_login_page_takes_the_poll_and_only_it_gets_the_account() {
    let dir = tempfile::tempdir().unwrap();
    let (mut machine, mut host) = started(dir.path());
    let mut now = 0;
    host.open(2, LOGIN_POPUP, Some(1));
    let (page, input) = round(&mut machine, &mut host, &mut now, r#"{"state":"waitingLogin"}"#);
    assert_eq!(page, PageId(2));
    assert_eq!((&input["command"], &input["loginId"], &input["loginPassword"], &input["loginAttempt"]), (&json!("login"), &json!("tester"), &json!("pw-for-test"), &json!("1")));
    assert!(input.get("birthDate").is_none());
    assert_eq!((machine.stage(), machine.phase()), (Stage::Select, "로그인"));

    host.open(3, "https://www.ngii.go.kr/member/login.do", Some(2));
    let (page, _) = round(&mut machine, &mut host, &mut now, r#"{"state":"waitingLogin"}"#);
    assert_eq!(page, PageId(3), "the member login comes later in the C++ list and wins");

    host.close(2);
    host.close(3);
    host.open(4, "https://evil.example/mn/loginPopup.do", Some(1));
    let (page, input) = round(&mut machine, &mut host, &mut now, r#"{"state":"querying"}"#);
    assert_eq!((page, &input["command"]), (PageId(1), &json!("select")));
    assert!(input.get("loginPassword").is_none() && input.get("loginId").is_none());
}

/// `:762-779, 786-788` — the form fields go to form and submit only; a submit is marked as ordered before
/// the script runs, so that a lost answer can never apply twice.
#[test]
fn form_and_submit_carry_the_form_fields_and_submit_is_marked_first() {
    let dir = tempfile::tempdir().unwrap();
    let (mut machine, mut host) = started(dir.path());
    let mut now = 0;
    round(&mut machine, &mut host, &mut now, SELECTED);
    assert_eq!(machine.stage(), Stage::OpenForm);
    let (_, input) = round(&mut machine, &mut host, &mut now, r#"{"state":"form"}"#);
    assert_eq!((&input["command"], &input["number"], &input["record"]), (&json!("openForm"), &json!("377051"), &json!({"num":"377051","name":"건천","historyNo":2})));
    assert_eq!(input["records"].as_array().map(Vec::len), Some(2));
    assert!(input.get("birthDate").is_none() && input.get("loginPassword").is_none());
    let (_, input) = round(&mut machine, &mut host, &mut now, r#"{"state":"ready"}"#);
    assert_eq!((&input["command"], &input["birthDate"], &input["purpose"]), (&json!("form"), &json!("19800101"), &json!("조사 도면")));
    assert_eq!((machine.stage(), machine.phase()), (Stage::Submit, "신청"));

    machine.tick(&mut host, now + STEP);
    let (page, input, _) = host.calls().last().cloned().unwrap();
    assert_eq!((page, &input["command"], &input["birthDate"]), (PageId(1), &json!("submit"), &json!("19800101")));
    assert_eq!(machine.stage(), Stage::Order, "marked before the answer");
}

/// `:538-563, 735-740` — the files are asked of the popup this very order opened, checked by its window
/// name; no other transfer page will do.
#[test]
fn files_are_asked_of_the_orders_own_transfer_popup_only() {
    let dir = tempfile::tempdir().unwrap();
    let (mut machine, mut host) = started(dir.path());
    let mut now = to_files(&mut machine, &mut host);
    assert_eq!((machine.stage(), machine.phase()), (Stage::Files, "파일 준비"));
    // Another transfer page, opened later by the same page but not checked for this order.
    host.open(9, TRANSFER, Some(1));
    let (page, input) = round(&mut machine, &mut host, &mut now, r#"{"state":"waitingFiles"}"#);
    assert_eq!((page, &input["command"], &input["fileIndex"]), (PageId(2), &json!("files"), &json!(0)));

    // Another run: transfer pages that are not this order's popup.
    let (mut machine, mut host) = started(dir.path());
    let mut now = 0;
    round(&mut machine, &mut host, &mut now, SELECTED);
    host.open(7, "about:blank", Some(1));
    machine.page_load_started(&host, PageId(7));
    host.open(7, TRANSFER, Some(1));
    machine.page_loaded(&mut host, PageId(7), now);
    assert!(host.other_scripts().iter().all(|(page, _)| *page != PageId(7)), "opened before anything was ordered");
    round(&mut machine, &mut host, &mut now, r#"{"state":"form"}"#);
    round(&mut machine, &mut host, &mut now, r#"{"state":"ready"}"#);
    round(&mut machine, &mut host, &mut now, r#"{"state":"submitted"}"#);
    // The window opened before the order went out finishes loading (or is used again) only now: it began
    // to load for no order, so it cannot supply this order's files.
    machine.page_loaded(&mut host, PageId(7), now);
    assert!(host.other_scripts().iter().all(|(page, _)| *page != PageId(7)), "a window that did not start loading for this order");
    host.open(5, TRANSFER, None);
    machine.page_load_started(&host, PageId(5));
    machine.page_loaded(&mut host, PageId(5), now);
    assert!(host.other_scripts().iter().all(|(page, _)| *page != PageId(5)), "not opened by the ordering page");
    host.open(2, TRANSFER, Some(1));
    machine.page_load_started(&host, PageId(2));
    machine.page_loaded(&mut host, PageId(2), now);
    let token = host.token_of("window.name === 'downForm'");
    machine.script_answered(&mut host, token, "false", now);
    round(&mut machine, &mut host, &mut now, r#"{"state":"orderReady"}"#);
    for _ in 1..75 {
        now += STEP;
        machine.tick(&mut host, now);
    }
    assert_eq!(machine.stage(), Stage::Files);
    machine.tick(&mut host, now + STEP);
    assert_eq!(machine.stage(), Stage::Stopped);
    assert_eq!(needs_input(&machine.take_events()), vec!["파일 전송 화면이 열리지 않았습니다. 신청 내역과 연결 상태를 확인하세요."]);
    assert!(host.calls().iter().all(|(page, input, _)| *page == PageId(1) && input["command"] != "files"));
}

/// `:731-734, 789-793` — a download is marked as a transfer before it runs, and a transfer is not polled.
#[test]
fn a_download_is_marked_as_transfer_first_and_a_transfer_is_not_polled() {
    let dir = tempfile::tempdir().unwrap();
    let (mut machine, mut host) = started(dir.path());
    let mut now = to_files(&mut machine, &mut host);
    round(&mut machine, &mut host, &mut now, r#"{"state":"ready","files":1,"manifest":[]}"#);
    assert_eq!(machine.stage(), Stage::Download);
    now += STEP;
    machine.tick(&mut host, now);
    let (page, input, token) = host.calls().last().cloned().unwrap();
    assert_eq!((page, &input["command"]), (PageId(2), &json!("download")));
    assert_eq!((machine.stage(), machine.phase()), (Stage::Transfer, "다운로드"));
    machine.script_answered(&mut host, token, r#"{"state":"downloading"}"#, now);
    let calls = host.calls().len();
    for _ in 0..5 {
        now += STEP;
        machine.tick(&mut host, now);
    }
    assert_eq!((host.calls().len(), machine.stage()), (calls, Stage::Transfer));
}

/// The commands the stages send are the ones the page script of the C++ app answers
/// (`data/providers/ngii-topographic.js`), and the script is the function expression the poll calls.
#[test]
fn every_command_is_one_the_page_script_answers() {
    let script = super::PAGE_SCRIPT;
    assert!(script.contains("(function (input) {") && script.trim_end().ends_with("})"));
    let stages = [Stage::Select, Stage::OpenForm, Stage::Form, Stage::Submit, Stage::Order, Stage::Files, Stage::Download];
    for command in stages.iter().map(|s| s.command()).chain(["login", "configureTransfer"]) {
        assert!(script.contains(&format!("==='{command}'")) || script.contains(&format!("=== '{command}'")), "{command}");
    }
    for never_sent in [Stage::Idle, Stage::Transfer, Stage::Complete, Stage::Stopped] {
        let command = never_sent.command();
        assert!(!script.contains(&format!("command==='{command}'")) && !script.contains(&format!("command === '{command}'")), "{command}");
    }
}
