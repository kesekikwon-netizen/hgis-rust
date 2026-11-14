//! What each answer of the page script does to the run (the callback of `pollAutomation`, `:799-913`),
//! the time limits (`:224-236`) and stopping (`:647-659`).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use serde_json::json;

use super::tests_support::{needs_input, round, started, to_files, RESET, SELECTED, STEP};
use super::{AutoEvent, PageId, Stage, PORTAL_HOME};

/// `:810-816`
#[test]
fn an_answer_without_a_state_is_waited_for_75_times_and_blocked_stops_at_once() {
    let dir = tempfile::tempdir().unwrap();
    let (mut machine, mut host) = started(dir.path());
    let mut now = 0;
    for answer in ["", "null", "42", "{}", r#"{"state":""}"#] {
        round(&mut machine, &mut host, &mut now, answer);
    }
    for _ in 5..74 {
        round(&mut machine, &mut host, &mut now, "{}");
    }
    assert_eq!(machine.stage(), Stage::Select);
    round(&mut machine, &mut host, &mut now, "{}");
    assert_eq!(machine.stage(), Stage::Stopped);
    assert_eq!(needs_input(&machine.take_events()), vec!["포털 자동 진행 응답을 확인하지 못했습니다. 연결 상태를 확인한 뒤 다시 시도하세요."]);

    let (mut machine, mut host) = started(dir.path());
    let mut now = 0;
    round(&mut machine, &mut host, &mut now, r#"{"state":"blocked","message":"공식 지도의 좌표체계를 확인하지 못해 도엽 선택을 중단했습니다."}"#);
    assert_eq!((machine.stage(), machine.failure_phase()), (Stage::Stopped, "도엽 확인"));
    assert_eq!(needs_input(&machine.take_events()), vec!["공식 지도의 좌표체계를 확인하지 못해 도엽 선택을 중단했습니다."]);
    assert_eq!(machine.attention(), "공식 지도의 좌표체계를 확인하지 못해 도엽 선택을 중단했습니다.");
}

/// `:817-822` — the run waits and says why, once per text; a submit or download that did not go out is
/// taken back.
#[test]
fn input_and_login_answers_ask_for_attention_and_take_the_marked_stage_back() {
    let dir = tempfile::tempdir().unwrap();
    let (mut machine, mut host) = started(dir.path());
    let mut now = 0;
    round(&mut machine, &mut host, &mut now, SELECTED);
    machine.take_events();
    let login = r#"{"state":"login","message":"수치지형도 계정으로 로그인하고 있습니다."}"#;
    round(&mut machine, &mut host, &mut now, login);
    round(&mut machine, &mut host, &mut now, login);
    assert_eq!(needs_input(&machine.take_events()), vec!["수치지형도 계정으로 로그인하고 있습니다."], "the same words are told once");
    assert_eq!(machine.stage(), Stage::OpenForm);
    round(&mut machine, &mut host, &mut now, r#"{"state":"form"}"#);
    assert_eq!(machine.attention(), "", "progress clears the notice");
    round(&mut machine, &mut host, &mut now, r#"{"state":"ready"}"#);
    let (_, input) = round(&mut machine, &mut host, &mut now, r#"{"state":"input","message":"신청서의 생년월일을 확인해 주세요."}"#);
    assert_eq!(input["command"], json!("submit"));
    assert_eq!(machine.stage(), Stage::Form, "the submit did not go out");
    assert_eq!(needs_input(&machine.take_events()), vec!["신청서의 생년월일을 확인해 주세요."]);

    let (mut machine, mut host) = started(dir.path());
    let mut now = to_files(&mut machine, &mut host);
    round(&mut machine, &mut host, &mut now, r#"{"state":"ready","files":1,"manifest":[]}"#);
    let (_, input) = round(&mut machine, &mut host, &mut now, r#"{"state":"input","message":"전송 화면을 확인해 주세요."}"#);
    assert_eq!((&input["command"], machine.stage()), (&json!("download"), Stage::Files));
}

/// `:839-880`
#[test]
fn the_selection_answer_decides_between_nothing_everything_kept_and_an_order() {
    let dir = tempfile::tempdir().unwrap();
    let (mut machine, mut host) = started(dir.path());
    let mut now = 0;
    round(&mut machine, &mut host, &mut now, r#"{"state":"unavailable","items":[],"downloadItems":[],"unavailable":["377051"]}"#);
    assert_eq!(machine.stage(), Stage::Stopped);
    let events = machine.take_events();
    assert_eq!(events[0], AutoEvent::SelectionUpdated { items: json!([]), cached: json!(null), unavailable: json!(["377051"]) });
    assert_eq!(needs_input(&events), vec!["이 범위에서 제공 중인 1:25,000 DXF 도엽이 없습니다."]);
    machine.tick(&mut host, now + STEP);
    assert_eq!(host.calls().len(), 1, "the run is over");

    let (mut machine, mut host) = started(dir.path());
    let mut now = 0;
    round(&mut machine, &mut host, &mut now, r#"{"state":"cached","items":[{"num":"377064","name":"불국사"}],"downloadItems":[],"cached":["377064"],"unavailable":[]}"#);
    assert_eq!((machine.stage(), machine.status()), (Stage::Complete, "필요한 지형도를 모두 보관하고 있습니다. 다시 신청하지 않습니다."));
    assert_eq!(machine.take_events(), vec![AutoEvent::SelectionUpdated { items: json!([{"num":"377064","name":"불국사"}]), cached: json!(["377064"]), unavailable: json!([]) }]);
    machine.tick(&mut host, now + STEP);
    assert_eq!(host.calls().len(), 1, "nothing is applied for");

    let (mut machine, mut host) = started(dir.path());
    let mut now = 0;
    round(&mut machine, &mut host, &mut now, SELECTED);
    assert_eq!(machine.stage(), Stage::OpenForm);
    assert_eq!(machine.order_sheets_text(), "공식 신청할 도엽 2장 · 377051 건천, 377052 경주 — 중지하려면 「취소」를 누르세요.");
    assert!(matches!(&machine.take_events()[..], [AutoEvent::SelectionUpdated { cached, .. }] if *cached == json!(["377064"])));
    let (_, input) = round(&mut machine, &mut host, &mut now, r#"{"state":"openingForm"}"#);
    assert_eq!(input["numbers"], json!(["377051", "377052", "377064"]), "the portal's own list replaces the app's");
    assert_eq!(machine.phase(), "신청");

    let many: Vec<_> = (0..11).map(|i| json!({"num": format!("{:06}", 377_001 + i), "name": format!("도엽{i}")})).collect();
    let (mut machine, mut host) = started(dir.path());
    let mut now = 0;
    round(&mut machine, &mut host, &mut now, &json!({"state": "selected", "items": many, "downloadItems": many}).to_string());
    assert_eq!(
        machine.order_sheets_text(),
        "공식 신청할 도엽 11장 · 377001 도엽0, 377002 도엽1, 377003 도엽2, 377004 도엽3, 377005 도엽4, 377006 도엽5, 377007 도엽6, 377008 도엽7, 외 3장 — 중지하려면 「취소」를 누르세요."
    );
}

/// `:780, 804-809` — with diagnostics on, the page script says at which of its checks it stopped (a stage
/// and a step name, never page content); the run keeps the last such note for the log.
#[test]
fn the_page_scripts_diagnostic_note_is_asked_for_and_kept() {
    let dir = tempfile::tempdir().unwrap();
    let (mut machine, mut host) = started(dir.path());
    let mut now = 0;
    let (_, input) = round(&mut machine, &mut host, &mut now, SELECTED);
    assert_eq!(input["diagnostics"], json!(false), "off unless the run was set up with it");
    assert_eq!(machine.last_diagnostic(), "");

    let mut host = super::tests_support::FakeHost::with_portal();
    let mut machine = super::NgiiAutomation::new(super::AutoConfig { diagnostics: true, ..super::tests_support::config(dir.path()) });
    machine.prepare_sheets(&mut host, super::tests_support::scope(), 0);
    let mut now = 0;
    let (_, input) = round(&mut machine, &mut host, &mut now, SELECTED);
    assert_eq!(input["diagnostics"], json!(true));
    round(&mut machine, &mut host, &mut now, r#"{"state":"blocked","message":"공식 로그인 또는 신청 화면을 확인하지 못했습니다.","diagnostics":{"stage":"openForm","step":"authentication"}}"#);
    assert_eq!(machine.stage(), Stage::Stopped);
    assert_eq!(machine.last_diagnostic(), r#"{"stage":"openForm","step":"authentication"}"#);
    // The course of the run for the log: which page was asked what and what it answered, each change once.
    assert_eq!(machine.answer_trace(), ["p1 select>selected", "p1 openForm>blocked"]);
}

/// `:829-838, 881-884`
#[test]
fn the_order_moves_on_by_the_answers_of_its_own_stage() {
    let dir = tempfile::tempdir().unwrap();
    let (mut machine, mut host) = started(dir.path());
    let mut now = 0;
    round(&mut machine, &mut host, &mut now, SELECTED);
    round(&mut machine, &mut host, &mut now, r#"{"state":"ready"}"#);
    assert_eq!(machine.stage(), Stage::OpenForm, "ready is not an answer of openForm");
    round(&mut machine, &mut host, &mut now, r#"{"state":"form"}"#);
    round(&mut machine, &mut host, &mut now, r#"{"state":"form"}"#);
    assert_eq!(machine.stage(), Stage::Form, "form is not an answer of form");
    round(&mut machine, &mut host, &mut now, r#"{"state":"ready"}"#);
    round(&mut machine, &mut host, &mut now, r#"{"state":"waitingForm"}"#);
    assert_eq!(machine.stage(), Stage::Form, "the form was not there: nothing was submitted");
    round(&mut machine, &mut host, &mut now, r#"{"state":"ready"}"#);
    round(&mut machine, &mut host, &mut now, r#"{"state":"submitted"}"#);
    assert_eq!(machine.stage(), Stage::Order);
    machine.take_events();
    let (_, input) = round(&mut machine, &mut host, &mut now, r#"{"state":"delayed","message":"신청 처리가 늦어지고 있습니다."}"#);
    assert_eq!((&input["command"], machine.stage()), (&json!("order"), Stage::Order));
    assert_eq!(needs_input(&machine.take_events()), vec!["신청 처리가 늦어지고 있습니다."]);
    round(&mut machine, &mut host, &mut now, r#"{"state":"orderReady"}"#);
    assert_eq!((machine.stage(), machine.phase(), machine.attention()), (Stage::Files, "파일 준비", ""));
    assert_eq!(host.calls().iter().filter(|(_, input, _)| input["command"] == "submit").count(), 2, "one per form that was ready, never a repeat of a sent one");
}

/// `:591-596, 800` — when the map page loads anew before the order is out the selection starts over, and
/// an answer of the old page is not listened to.
#[test]
fn a_reloaded_map_page_starts_the_selection_again() {
    let dir = tempfile::tempdir().unwrap();
    let (mut machine, mut host) = started(dir.path());
    let mut now = 0;
    round(&mut machine, &mut host, &mut now, SELECTED);
    now += STEP;
    machine.tick(&mut host, now);
    let (_, _, old) = host.calls().last().cloned().unwrap();
    let before = machine.generation();
    machine.page_loaded(&mut host, PageId(1), now);
    assert_eq!((machine.stage(), machine.generation()), (Stage::Select, before + 1));
    machine.script_answered(&mut host, old, r#"{"state":"form"}"#, now);
    assert_eq!(machine.stage(), Stage::Select, "the old page's answer is ignored");
    let (_, input) = round(&mut machine, &mut host, &mut now, r#"{"state":"querying"}"#);
    assert_eq!((&input["command"], &input["generation"]), (&json!("select"), &json!((before + 1).to_string())));

    // Once the order is out, a reload must not start it again.
    let (mut machine, mut host) = started(dir.path());
    to_files(&mut machine, &mut host);
    let before = machine.generation();
    machine.page_loaded(&mut host, PageId(1), 9000);
    assert_eq!((machine.stage(), machine.generation()), (Stage::Files, before));
}

/// Seen on the real site on 2026-11-15: after the login the map page loads anew and takes seconds to do
/// so. Its address is the map's from the first moment, and a page script asked then finds the member
/// logged in and no sheet list, and stops the run ("checkboxes"). The page is left alone while it loads;
/// when it has loaded, the selection starts again (`:591-596`).
#[test]
fn a_map_page_that_is_still_loading_anew_is_not_asked_to_open_the_form() {
    let dir = tempfile::tempdir().unwrap();
    let (mut machine, mut host) = started(dir.path());
    let mut now = 0;
    round(&mut machine, &mut host, &mut now, SELECTED);
    round(&mut machine, &mut host, &mut now, r#"{"state":"waitingLogin"}"#);
    // The login moved the first page away and brings it back: the load of the map page starts.
    machine.page_load_started(&host, PageId(1));
    let asked = host.calls().len();
    for _ in 0..80 {
        now += STEP;
        machine.tick(&mut host, now);
    }
    assert_eq!(host.calls().len(), asked, "nothing is asked of the page while it loads");
    assert_eq!(machine.stage(), Stage::OpenForm, "and waiting for it is not counted as a missing page");
    let before = machine.generation();
    machine.page_loaded(&mut host, PageId(1), now);
    assert_eq!((machine.stage(), machine.generation()), (Stage::Select, before + 1));
    let (_, input) = round(&mut machine, &mut host, &mut now, r#"{"state":"querying"}"#);
    assert_eq!(input["command"], json!("select"));

    // The first load of the map page is asked at once, as before: the selection waits for the map itself.
    let (mut machine, mut host) = started(dir.path());
    machine.page_load_started(&host, PageId(1));
    let mut now = 0;
    let (_, input) = round(&mut machine, &mut host, &mut now, r#"{"state":"waitingLayout"}"#);
    assert_eq!(input["command"], json!("select"));
}

/// `:224-236`
#[test]
fn every_phase_has_a_time_limit_and_login_a_longer_one() {
    let words = "이 단계의 서버 응답 시간이 초과되었습니다. 계정 설정과 연결 상태를 확인하세요. 신청은 자동으로 반복하지 않습니다.";
    let dir = tempfile::tempdir().unwrap();
    let (mut machine, mut host) = started(dir.path());
    let mut now = 0;
    while now + STEP < 90_000 {
        round(&mut machine, &mut host, &mut now, r#"{"state":"querying"}"#);
    }
    assert_eq!(machine.stage(), Stage::Select);
    machine.tick(&mut host, 90_000);
    assert_eq!((machine.stage(), machine.failure_phase()), (Stage::Stopped, "도엽 확인"));
    assert_eq!(needs_input(&machine.take_events()), vec![words]);

    let (mut machine, mut host) = started(dir.path());
    host.open(2, super::tests_support::LOGIN_POPUP, Some(1));
    let mut now = 0;
    round(&mut machine, &mut host, &mut now, r#"{"state":"waitingLogin"}"#);
    let login_began = now;
    while now + STEP < login_began + 300_000 {
        round(&mut machine, &mut host, &mut now, r#"{"state":"waitingLogin"}"#);
    }
    assert_eq!((machine.stage(), machine.phase()), (Stage::Select, "로그인"));
    machine.tick(&mut host, login_began + 300_000);
    assert_eq!((machine.stage(), machine.failure_phase()), (Stage::Stopped, "로그인"));
    assert_eq!(needs_input(&machine.take_events()), vec![words]);
}

/// `:647-659` — stopping forgets the order, clears the page state and keeps late answers out.
#[test]
fn stopping_ends_the_run_and_clears_the_page_state() {
    let dir = tempfile::tempdir().unwrap();
    let (mut machine, mut host) = started(dir.path());
    let mut now = 0;
    round(&mut machine, &mut host, &mut now, SELECTED);
    now += STEP;
    machine.tick(&mut host, now);
    let (_, _, pending) = host.calls().last().cloned().unwrap();
    host.open(9, "https://evil.example/", None);
    let scripts = host.other_scripts().len();
    machine.stop(&mut host);
    assert_eq!((machine.stage(), machine.failure_phase(), machine.order_sheets_text()), (Stage::Stopped, "", ""));
    assert_eq!(machine.status(), "자동 진행을 중지했습니다. 이미 신청한 자료는 공식 신청 내역에서 확인할 수 있습니다.");
    assert_eq!(&host.other_scripts()[scripts..], &[(PageId(1), RESET.to_owned())], "only the portal's own pages are touched");
    machine.script_answered(&mut host, pending, r#"{"state":"form"}"#, now);
    machine.tick(&mut host, now + STEP);
    assert_eq!(machine.stage(), Stage::Stopped);
    assert!(host.navigations.is_empty());
    // The same sheets can be asked for again after a stop.
    machine.prepare_sheets(&mut host, super::tests_support::scope(), now + 1000);
    assert_eq!(machine.stage(), Stage::Select);
    assert_eq!(host.navigations.len(), 0, "the portal page ({PORTAL_HOME}) is still open");
}
