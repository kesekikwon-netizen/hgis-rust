//! The files of an order arriving (`requestDownload`, `KaTopographicBrowser.cpp:915-1105`).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use serde_json::json;

use super::tests_support::{needs_input, round, started, to_files, FakeHost, STEP};
use super::{AutoEvent, NgiiAutomation, PageId, Scope, Stage};

const MANIFEST: &str = r#"{"state":"ready","files":3,"manifest":[
  {"name":"377051.zip","id":"a1","number":"377051","bytes":1200},
  {"name":"377051.xml","id":"a2","number":"377051","bytes":300},
  {"name":"377052.zip","id":"b1","number":"377052","bytes":2200}]}"#;
const POPUP: PageId = PageId(2);

/// A started run brought to the point where the transfer page was told to send a file.
fn to_transfer(machine: &mut NgiiAutomation, host: &mut FakeHost, files: &str) -> u64 {
    let mut now = to_files(machine, host);
    round(machine, host, &mut now, files);
    round(machine, host, &mut now, r#"{"state":"downloading"}"#);
    assert_eq!(machine.stage(), Stage::Transfer);
    machine.take_events();
    now
}

/// Takes the offered file as the browser would: asks, writes the file, reports it complete.
fn receive(machine: &mut NgiiAutomation, host: &mut FakeHost, name: &str, now: u64) -> PathBuf {
    let target = machine.download_offered(host, POPUP, name, now).expect("the file is taken");
    assert!(target.directory.is_dir());
    let path = target.directory.join(&target.file_name);
    std::fs::write(&path, b"data").unwrap();
    machine.download_finished(host, &path, now);
    path
}

fn sheets(events: &[AutoEvent]) -> Vec<(&Path, &str)> {
    events.iter().filter_map(|e| if let AutoEvent::SheetDownloaded { path, record } = e { Some((path.as_path(), record["num"].as_str().unwrap())) } else { None }).collect()
}

/// `:927-938, 950-956, 1028-1058` — with a file list, every file is asked for in turn, the files of one
/// sheet share a folder, and a sheet is handed on when its last file is there.
#[test]
fn listed_files_arrive_in_order_into_one_folder_per_sheet() {
    let dir = tempfile::tempdir().unwrap();
    let (mut machine, mut host) = started(dir.path());
    let mut now = to_transfer(&mut machine, &mut host, MANIFEST);
    let zip = receive(&mut machine, &mut host, "377051.zip", now);
    assert_eq!(zip.parent().unwrap().parent().unwrap(), dir.path());
    assert_eq!(zip.parent().unwrap().file_name().unwrap().len(), 36, "a folder named by a random id");
    assert_eq!(machine.stage(), Stage::Download, "the next file is asked for only after this one is saved");
    assert!(sheets(&machine.take_events()).is_empty(), "the sheet's other file is still to come");

    let (page, input) = round(&mut machine, &mut host, &mut now, r#"{"state":"downloading"}"#);
    assert_eq!((page, &input["command"], &input["fileIndex"]), (POPUP, &json!("download"), &json!(1)));
    let xml = receive(&mut machine, &mut host, "377051.xml", now);
    assert_eq!(xml.parent(), zip.parent());
    let events = machine.take_events();
    assert_eq!(sheets(&events), vec![(zip.as_path(), "377051")], "the data file is handed on, the metadata file is not");
    let AutoEvent::SheetDownloaded { record, .. } = &events[0] else { panic!("{events:?}") };
    assert_eq!((&record["name"], &record["source"], &record["requestScope"]["radius"]), (&json!("건천"), &json!("NGII suchiQuery.do"), &json!(5000.0)));

    let (_, input) = round(&mut machine, &mut host, &mut now, r#"{"state":"downloading"}"#);
    assert_eq!(input["fileIndex"], json!(2));
    let second = receive(&mut machine, &mut host, "377052.zip", now);
    assert_ne!(second.parent(), zip.parent());
    assert_eq!(sheets(&machine.take_events()), vec![(second.as_path(), "377052")]);
    assert_eq!((machine.stage(), machine.status()), (Stage::Complete, "선택한 지형도 파일을 모두 받았습니다. 지도 적재 결과를 확인합니다."));
    machine.tick(&mut host, now + STEP);
    assert_eq!(host.calls().iter().filter(|(_, input, _)| input["command"] == "download").count(), 3);
}

/// `:1059-1083` — without a file list a sheet's files come by themselves, and the next sheet is applied
/// for when they are all there.
#[test]
fn without_a_file_list_each_sheet_is_applied_for_in_turn() {
    let dir = tempfile::tempdir().unwrap();
    let (mut machine, mut host) = started(dir.path());
    let mut now = to_transfer(&mut machine, &mut host, r#"{"state":"ready","files":2,"manifest":[]}"#);
    let zip = receive(&mut machine, &mut host, "377051.zip", now);
    assert_eq!(machine.stage(), Stage::Transfer, "the second file comes without another command");
    let xml = receive(&mut machine, &mut host, "meta.xml", now);
    assert_eq!(xml.parent(), zip.parent());
    assert_eq!(sheets(&machine.take_events()), vec![(zip.as_path(), "377051")]);
    assert_eq!((machine.stage(), machine.phase()), (Stage::OpenForm, "로그인"));
    let (page, input) = round(&mut machine, &mut host, &mut now, r#"{"state":"waitingLogin"}"#);
    assert_eq!((page, &input["command"], &input["number"], &input["fileIndex"]), (PageId(1), &json!("openForm"), &json!("377052"), &json!(0)));
}

/// `:915-1105` keeps every arriving file in a request of its own: the page of the fake portal asks for a
/// sheet's metadata file half a second after its data file, while the data file is still arriving
/// (found by the run against the fake portal, 2026-11-15).
#[test]
fn two_files_of_a_sheet_may_arrive_at_the_same_time() {
    let dir = tempfile::tempdir().unwrap();
    let (mut machine, mut host) = started(dir.path());
    let now = to_transfer(&mut machine, &mut host, r#"{"state":"ready","files":2,"manifest":[]}"#);
    let zip = machine.download_offered(&mut host, POPUP, "terrain_377051.zip", now).expect("the data file");
    let xml = machine.download_offered(&mut host, POPUP, "terrain_377051.xml", now + 500).expect("the metadata file, while the data file arrives");
    assert_eq!(zip.directory, xml.directory);
    let (zip, xml) = (zip.directory.join(&zip.file_name), xml.directory.join(&xml.file_name));
    machine.download_progress(&mut host, &zip, 1_048_576, now + 600);
    assert_eq!(machine.current_file(), ("terrain_377051.zip", 1_048_576, None));
    std::fs::write(&xml, b"<xml/>").unwrap();
    machine.download_finished(&mut host, &xml, now + 700);
    assert_eq!(machine.stage(), Stage::Transfer, "the data file is still arriving");
    assert!(sheets(&machine.take_events()).is_empty());
    // A transfer that breaks off stops the run even while another one is fine: only its own end counts.
    machine.download_failed(&mut host, &dir.path().join("not-a-transfer.zip"));
    assert_eq!(machine.stage(), Stage::Transfer);
    std::fs::write(&zip, b"PK").unwrap();
    machine.download_finished(&mut host, &zip, now + 900);
    assert_eq!(sheets(&machine.take_events()), vec![(zip.as_path(), "377051")]);
    assert_eq!(machine.stage(), Stage::OpenForm, "the next sheet");
}

/// `:917-921, 923-924, 930-938` — only the file that was asked for, from the page that was asked, during a
/// transfer. (The C++ window also saves files the user fetches by hand; a hidden run has no hand.)
#[test]
fn only_the_file_that_was_asked_for_is_taken() {
    let dir = tempfile::tempdir().unwrap();
    let (mut machine, mut host) = started(dir.path());
    let now = to_transfer(&mut machine, &mut host, MANIFEST);
    assert_eq!(machine.download_offered(&mut host, PageId(1), "377051.zip", now), None, "another page");
    assert_eq!(machine.stage(), Stage::Transfer);
    assert_eq!(machine.download_offered(&mut host, POPUP, "377052.zip", now), None, "not the next listed file");
    assert_eq!(machine.stage(), Stage::Stopped);
    assert_eq!(needs_input(&machine.take_events()), vec!["요청한 파일과 도착한 파일이 달라 전송을 중단했습니다."]);
    assert_eq!(machine.download_offered(&mut host, POPUP, "377051.zip", now), None);
    assert_eq!(needs_input(&machine.take_events()), vec!["중지한 자동 신청에서 늦게 시작된 다운로드를 취소했습니다. 다시 받으려면 공식 신청 내역을 새로 열어 주세요."]);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0, "nothing was made on disk");

    let (mut machine, mut host) = started(dir.path());
    let now = to_files(&mut machine, &mut host);
    assert_eq!(machine.download_offered(&mut host, POPUP, "377051.zip", now), None, "no transfer was asked for yet");
    assert_eq!(machine.stage(), Stage::Files);
    let mut now = now;
    round(&mut machine, &mut host, &mut now, MANIFEST);
    assert_eq!(machine.download_offered(&mut host, POPUP, "377051.zip", now), None, "the file list is known, but no file was asked for");
    assert_eq!(machine.stage(), Stage::Download);
}

/// `:913 (safeFileName), 939-941, 984-989`
#[test]
fn names_are_made_safe_and_sizes_are_capped_at_one_gigabyte() {
    let dir = tempfile::tempdir().unwrap();
    let (mut machine, mut host) = started(dir.path());
    let now = to_transfer(&mut machine, &mut host, r#"{"state":"ready","files":1,"manifest":[]}"#);
    let target = machine.download_offered(&mut host, POPUP, "", now).unwrap();
    assert_eq!(target.file_name, "수치지형도.zip", "the C++ default name");
    let file = target.directory.join(&target.file_name);
    machine.download_progress(&mut host, &file, 1_000_000_000, now);
    assert_eq!(machine.stage(), Stage::Transfer);
    machine.download_progress(&mut host, &file, 1_000_000_001, now);
    assert_eq!(machine.stage(), Stage::Stopped);
    assert_eq!(needs_input(&machine.take_events()), vec!["현재 파일 수신이 취소되어 다음 도엽 신청을 중지했습니다. 기존 지도와 완료한 파일은 유지됩니다."]);

    let big = r#"{"state":"ready","files":2,"manifest":[{"name":"377051.zip","id":"a","number":"377051","bytes":1000000001},{"name":"377052.zip","id":"b","number":"377052","bytes":5}]}"#;
    let (mut machine, mut host) = started(dir.path());
    let now = to_transfer(&mut machine, &mut host, big);
    assert_eq!(machine.download_offered(&mut host, POPUP, "377051.zip", now), None);
    assert_eq!(needs_input(&machine.take_events()), vec!["파일 크기가 1GB를 넘어서 자동 내려받기를 중단했습니다. 공식 목록에서 나누어 내려받아 주세요."]);
}

/// `:885-902` — a file list that does not match the order stops the run before anything is fetched.
#[test]
fn a_file_list_that_does_not_match_the_order_stops_the_run() {
    let words = "신청한 도엽과 파일 연결을 확인하지 못해 전송을 중단했습니다.";
    let file = |name: &str, id: &str, number: &str, bytes: i64| json!({"name": name, "id": id, "number": number, "bytes": bytes});
    let both = [file("377051.zip", "a", "377051", 10), file("377052.zip", "b", "377052", 10)];
    let bad: Vec<(&str, i64, Vec<serde_json::Value>)> = vec![
        ("count differs", 3, both.to_vec()),
        ("a path in a name", 2, vec![file("..\\377051.zip", "a", "377051", 10), both[1].clone()]),
        ("a device name", 2, vec![file("CON.zip", "a", "377051", 10), both[1].clone()]),
        ("no name", 2, vec![file("", "a", "377051", 10), both[1].clone()]),
        ("no id", 2, vec![file("377051.zip", "", "377051", 10), both[1].clone()]),
        ("a name twice", 3, vec![both[0].clone(), file("377051.zip", "c", "377051", 10), both[1].clone()]),
        ("an id twice", 2, vec![both[0].clone(), file("377052.zip", "a", "377052", 10)]),
        ("a sheet that was not applied for", 2, vec![both[0].clone(), file("377099.zip", "b", "377099", 10)]),
        ("an empty file", 2, vec![both[0].clone(), file("377052.zip", "b", "377052", 0)]),
        ("a sheet without a file", 1, vec![both[0].clone()]),
    ];
    for (why, files, manifest) in bad {
        let dir = tempfile::tempdir().unwrap();
        let (mut machine, mut host) = started(dir.path());
        let mut now = to_files(&mut machine, &mut host);
        machine.take_events();
        round(&mut machine, &mut host, &mut now, &json!({"state": "ready", "files": files, "manifest": manifest}).to_string());
        assert_eq!(machine.stage(), Stage::Stopped, "{why}");
        assert_eq!(needs_input(&machine.take_events()), vec![words], "{why}");
    }
}

/// `:229-232, 961-964, 979-981, 1015-1021, 1087-1103`
#[test]
fn a_silent_or_broken_transfer_stops_the_run() {
    let dir = tempfile::tempdir().unwrap();
    let (mut machine, mut host) = started(dir.path());
    let began = to_transfer(&mut machine, &mut host, MANIFEST);
    machine.tick(&mut host, began + 59_999);
    assert_eq!(machine.stage(), Stage::Transfer);
    let target = machine.download_offered(&mut host, POPUP, "377051.zip", began + 50_000).unwrap();
    let file = target.directory.join(&target.file_name);
    machine.download_progress(&mut host, &file, 100, began + 100_000);
    machine.download_progress(&mut host, &file, 100, began + 150_000);
    machine.tick(&mut host, began + 159_999);
    assert_eq!(machine.stage(), Stage::Transfer, "bytes arriving keep the transfer alive; the same count does not");
    machine.tick(&mut host, began + 160_000);
    assert_eq!((machine.stage(), machine.failure_phase()), (Stage::Stopped, "다운로드"));
    assert_eq!(needs_input(&machine.take_events()), vec!["60초 동안 새 파일 데이터가 수신되지 않았습니다. 연결 상태를 확인한 뒤 다시 시도하세요."]);

    let (mut machine, mut host) = started(dir.path());
    let now = to_transfer(&mut machine, &mut host, MANIFEST);
    let target = machine.download_offered(&mut host, POPUP, "377051.zip", now).unwrap();
    let file = target.directory.join(&target.file_name);
    machine.download_failed(&mut host, &file);
    assert_eq!(needs_input(&machine.take_events()), vec!["내려받기가 중단됐습니다. 전송 연결 끊김\n인터넷 연결과 저장 공간을 확인한 뒤 다시 시도하세요."]);
    machine.download_failed(&mut host, &file);
    assert!(machine.take_events().is_empty(), "a stopped run has no transfer to fail");

    let (mut machine, mut host) = started(dir.path());
    let now = to_transfer(&mut machine, &mut host, MANIFEST);
    let target = machine.download_offered(&mut host, POPUP, "377051.zip", now).unwrap();
    machine.download_finished(&mut host, &target.directory.join("377051.zip"), now);
    assert_eq!(needs_input(&machine.take_events()), vec!["받은 파일을 확인하지 못했습니다. 저장 공간과 폴더 권한을 확인한 뒤 다시 시도하세요."]);

    let blocked = dir.path().join("a-file");
    std::fs::write(&blocked, b"x").unwrap();
    let (mut machine, mut host) = started(&blocked);
    let now = to_transfer(&mut machine, &mut host, MANIFEST);
    assert_eq!(machine.download_offered(&mut host, POPUP, "377051.zip", now), None);
    assert_eq!(needs_input(&machine.take_events()), vec!["다운로드 폴더를 만들지 못했습니다. 저장 공간과 쓰기 권한을 확인하세요."]);
}

/// `:1050-1053` — a circle asked for while an order was being fetched starts when the order is done.
#[test]
fn a_waiting_request_starts_when_the_order_is_done() {
    let dir = tempfile::tempdir().unwrap();
    let (mut machine, mut host) = started(dir.path());
    let mut now = to_files(&mut machine, &mut host);
    machine.prepare_sheets(&mut host, Scope { numbers: vec!["377053".into()], ..super::tests_support::scope() }, now);
    round(&mut machine, &mut host, &mut now, MANIFEST);
    round(&mut machine, &mut host, &mut now, r#"{"state":"downloading"}"#);
    receive(&mut machine, &mut host, "377051.zip", now);
    round(&mut machine, &mut host, &mut now, r#"{"state":"downloading"}"#);
    receive(&mut machine, &mut host, "377051.xml", now);
    round(&mut machine, &mut host, &mut now, r#"{"state":"downloading"}"#);
    let before = machine.generation();
    receive(&mut machine, &mut host, "377052.zip", now);
    assert_eq!((machine.stage(), machine.generation()), (Stage::Select, before + 1));
    let (_, input) = round(&mut machine, &mut host, &mut now, r#"{"state":"querying"}"#);
    assert_eq!((&input["command"], &input["numbers"]), (&json!("select"), &json!(["377053"])));
}
