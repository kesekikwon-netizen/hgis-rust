use serde_json::{json, Map, Value};

use super::*;

fn flow() -> PortalFlow {
    let mut scope = Map::new();
    scope.insert("x".to_owned(), json!(960_350));
    scope.insert("y".to_owned(), json!(1_850_300));
    scope.insert("radius".to_owned(), json!(500));
    PortalFlow::new(scope, 3)
}

fn step(flow: &mut PortalFlow, state: Value) -> String {
    let request = flow.poll("19800101", "문화재 조사").expect("시킬 일");
    let command = request.input["command"].as_str().expect("명령").to_owned();
    flow.on_result(&command, &state);
    command
}

#[test]
fn the_stages_run_in_the_cpp_order_from_sheets_to_download() {
    let mut f = flow();
    let record = json!({"num": "367051", "name": "전의", "historyNo": 7});
    assert_eq!(step(&mut f, json!({"state": "querying"})), "select");
    assert_eq!(f.stage, Stage::Select);
    step(&mut f, json!({"state": "selected", "items": [record.clone()], "downloadItems": [record.clone()]}));
    assert_eq!((f.stage, f.phase.as_str()), (Stage::OpenForm, "로그인"));

    // 신청서를 열 때는 방금 확인한 자료를 그대로 넘긴다.
    let request = f.poll("", "").expect("openForm");
    assert_eq!(request.input["number"], json!("367051"));
    assert_eq!(request.input["records"], json!([record]));
    assert_eq!(request.input["generation"], json!("3"));
    assert!(request.input.get("birthDate").is_none(), "생년월일은 신청서 단계에서만 보낸다");
    assert_eq!(request.paths.first().map(String::as_str), Some("/mn/loginPopup.do"));
    assert_eq!(request.paths.last().map(String::as_str), Some(PORTAL_PATH));
    f.on_result("openForm", &json!({"state": "form"}));
    assert_eq!((f.stage, f.phase.as_str()), (Stage::Form, "신청"));

    let request = f.poll("19800101", " 문화재 조사 ").expect("form");
    assert_eq!((request.input["birthDate"].clone(), request.input["purpose"].clone()), (json!("19800101"), json!("문화재 조사")));
    f.on_result("form", &json!({"state": "ready"}));
    assert_eq!(f.stage, Stage::Submit);

    assert_eq!(step(&mut f, json!({"state": "submitted"})), "submit");
    assert_eq!(f.stage, Stage::Order);
    step(&mut f, json!({"state": "pendingOrder"}));
    assert_eq!(f.stage, Stage::Order);
    step(&mut f, json!({"state": "orderReady"}));
    assert_eq!((f.stage, f.phase.as_str()), (Stage::Files, "파일 준비"));

    let request = f.poll("", "").expect("files");
    assert_eq!(request.paths, vec![DOWNLOAD_PATH.to_owned()]);
    f.on_result("files", &json!({"state": "ready", "files": 1, "manifest": []}));
    assert_eq!((f.stage, f.expected_files), (Stage::Download, 1));
    assert_eq!(step(&mut f, json!({"state": "downloading"})), "download");
    assert_eq!(f.stage, Stage::Transfer);
    assert!(f.poll("", "").is_none(), "전송 중에는 화면에 시키지 않는다");
}

#[test]
fn a_lost_answer_never_sends_the_same_application_twice() {
    let mut f = flow();
    f.stage = Stage::Submit;
    let request = f.poll("19800101", "조사").expect("submit");
    assert_eq!(request.input["command"], json!("submit"));
    // 답이 오지 않아도 이미 「신청 확인」 단계다 — 다음 차례는 제출이 아니다.
    assert_eq!(f.stage, Stage::Order);
    assert_eq!(f.poll("", "").expect("order").input["command"], json!("order"));
}

#[test]
fn a_submit_that_did_not_happen_goes_back_to_the_form() {
    // 로그인 화면이 대신 받았다.
    let mut f = flow();
    f.stage = Stage::Submit;
    f.poll("", "").expect("submit");
    f.on_result("login", &json!({"state": "waitingLogin"}));
    assert_eq!((f.stage, f.phase.as_str()), (Stage::Submit, "로그인"));
    // 화면이 아직 없었다.
    f.poll("", "").expect("submit");
    f.on_missing_page();
    assert_eq!(f.stage, Stage::Submit);
    // 입력이 모자랐다.
    f.poll("", "").expect("submit");
    f.on_result("submit", &json!({"state": "input", "message": "신청에 필요한 생년월일이 비어 있습니다."}));
    assert_eq!(f.stage, Stage::Form);
    assert!(f.message.contains("생년월일"));
}

#[test]
fn blocked_no_data_cached_and_silence_all_stop_with_a_reason() {
    let mut f = flow();
    step(&mut f, json!({"state": "blocked", "message": "공식 지도의 좌표체계를 확인하지 못해 도엽 선택을 중단했습니다."}));
    assert_eq!(f.stage, Stage::Stopped);
    assert!(f.failed.as_deref().is_some_and(|m| m.contains("좌표체계")));
    assert!(f.poll("", "").is_none());

    let mut f = flow();
    step(&mut f, json!({"state": "unavailable"}));
    assert_eq!((f.stage, f.failed.is_none()), (Stage::Stopped, true));

    let mut f = flow();
    step(&mut f, json!({"state": "cached", "items": [{"num": "1"}], "downloadItems": []}));
    assert_eq!(f.stage, Stage::Complete);
    assert!(f.message.contains("다시 신청하지 않습니다"));

    let mut f = flow();
    for _ in 0..MAX_WAIT_TICKS {
        assert!(f.failed.is_none());
        f.poll("", "").expect("select");
        f.on_missing_page();
    }
    assert!(f.failed.as_deref().is_some_and(|m| m.starts_with("공식 사이트의 응답을 확인하지 못했습니다.")));
}
