//! What a press of 「수치지형」 needs and how a run shows in the bottom gauge.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use strata_net::ngii_auto::{Stage, DEFAULT_PURPOSE};
use strata_secrets::accounts::Credentials;
use strata_types::crs::WorkCrs;

use super::{gauge_state, topo_request, RunFacts, TopoFacts, TOPO_RADIUS_M};
use crate::download_jobs::{DownloadJobState, StopCode};

fn facts<'a>(survey: Option<&'a Path>) -> TopoFacts<'a> {
    // 경주 부근, EPSG:5187 (동부원점) 미터.
    TopoFacts { survey_path: survey, centre_work: Some([222_000.0, 430_000.0]), work_crs: WorkCrs::Epsg5187, online: true }
}

fn account() -> Credentials {
    Credentials::new("tester", "pw-for-test")
}

/// A press that is refused for the survey, the survey area or the network must not open the account file.
fn never_read() -> Credentials {
    panic!("the stored account was read although the press was refused before it was needed")
}

/// C++ `openTopographicDownload` (MainWindowTopographic.cpp:18-34) checks the survey first; the survey
/// area is the owner's rule (docs/RUST_PLAN.md 2.9); account and network are known without the site (2.8).
#[test]
fn a_press_is_refused_in_order_with_the_reason_and_its_code() {
    let dir = tempfile::tempdir().unwrap();
    let survey = dir.path().join("현장.gpkg");
    std::fs::write(&survey, b"x").unwrap();

    let no_survey = topo_request(facts(None), never_read).unwrap_err();
    assert_eq!(no_survey, (StopCode::Dir1, "먼저 새 조사를 만들거나 기존 조사를 여세요. 조사 폴더 안의 지형도 폴더에 저장합니다.".to_owned()));
    assert_eq!(topo_request(facts(Some(&dir.path().join("없음.gpkg"))), never_read).unwrap_err().0, StopCode::Dir1);

    let no_area = topo_request(TopoFacts { centre_work: None, ..facts(Some(&survey)) }, never_read).unwrap_err();
    assert_eq!(no_area, (StopCode::Area1, "조사구역을 먼저 그려 주세요. 조사구역에서 반경 5km 안의 수치지형도를 받습니다.".to_owned()));
    assert_eq!(topo_request(TopoFacts { centre_work: Some([f64::NAN, 1.0]), ..facts(Some(&survey)) }, never_read).unwrap_err().0, StopCode::Area1);

    let offline = topo_request(TopoFacts { online: false, ..facts(Some(&survey)) }, never_read).unwrap_err();
    assert_eq!(offline.0, StopCode::Site1);

    // The words of the C++ page script for a missing account (ngii-topographic.js:196).
    for stored in [Credentials::empty(), Credentials::new("tester", ""), Credentials::new("  ", "pw")] {
        let refused = topo_request(facts(Some(&survey)), || stored).unwrap_err();
        assert_eq!(refused, (StopCode::Acc1, "더보기의 수치지형도 계정 설정에서 아이디와 비밀번호를 입력해 주세요.".to_owned()));
    }
    assert!(!dir.path().join("지형도").exists(), "a refused press makes no folder");
}

/// The circle goes to the portal in EPSG:5179 with 5 km (MainWindowDownloads.cpp:183-187); files go to
/// <survey folder>/지형도/원본 (MainWindowTopographic.cpp:25, 36); the form gets no birth date from the app
/// (the C++ never has one: KaTopographicBrowser.cpp:251, 267) and the C++ default purpose (:168).
#[test]
fn a_press_asks_for_five_kilometres_around_the_survey_area() {
    let dir = tempfile::tempdir().unwrap();
    let survey = dir.path().join("현장.gpkg");
    std::fs::write(&survey, b"x").unwrap();
    let request = topo_request(facts(Some(&survey)), account).unwrap();
    assert_eq!(request.scope.radius, TOPO_RADIUS_M);
    assert_eq!(TOPO_RADIUS_M, 5000.0);
    // The point lies 22 km east of 129°E and 170 km south of 38°N (EPSG:5187: false origin 200 km, 600 km).
    // In EPSG:5179 (central meridian 127.5°E, false origin 1000 km, 2000 km) that is about 1157 km, 1830 km.
    assert!((1_140_000.0..1_175_000.0).contains(&request.scope.x) && (1_815_000.0..1_845_000.0).contains(&request.scope.y), "{:?}", (request.scope.x, request.scope.y));
    assert!(!request.scope.numbers.is_empty() && request.scope.numbers.iter().all(|n| n.len() == 6 && n.bytes().all(|b| b.is_ascii_digit())));
    assert!(request.scope.cached_numbers.is_empty());
    assert_eq!(request.downloads, std::path::absolute(dir.path()).unwrap().join("지형도").join("원본"));
    assert_eq!((request.account.login_id.as_str(), request.account.login_password.as_str()), ("tester", "pw-for-test"));
    assert_eq!((request.account.birth_date.as_str(), request.account.purpose.as_str()), ("", DEFAULT_PURPOSE));
}

fn run(stage: Stage, phase: &'static str) -> RunFacts<'static> {
    RunFacts { stage, phase, attention: "", failure_phase: "", status: "", received: 0, total: None, sheets: 0, cancelled: false }
}

/// One line of the gauge per state: the C++ stage text `[n/6] phase` (KaTopographicBrowser.cpp:411-414)
/// while it runs, the reason and a code when it stopped, the result when it is done.
#[test]
fn the_gauge_shows_the_phase_then_the_result_or_the_reason() {
    let running = |fraction: Option<f32>, stage: &str| DownloadJobState::Running { fraction, stage: Some(stage.to_owned()) };
    assert_eq!(gauge_state(&run(Stage::Select, "도엽 확인")), running(None, "[1/6] 도엽 확인"));
    assert_eq!(gauge_state(&run(Stage::OpenForm, "로그인")), running(None, "[2/6] 로그인"));
    assert_eq!(gauge_state(&run(Stage::Submit, "신청")), running(None, "[3/6] 신청"));
    assert_eq!(gauge_state(&run(Stage::Files, "파일 준비")), running(None, "[4/6] 파일 준비"));
    assert_eq!(gauge_state(&RunFacts { received: 1_048_576, total: Some(4_194_304), ..run(Stage::Transfer, "다운로드") }), running(Some(0.25), "[5/6] 다운로드"));
    assert_eq!(gauge_state(&RunFacts { received: 1_048_576, ..run(Stage::Transfer, "다운로드") }), running(None, "[5/6] 다운로드"));
    // A notice while the run goes on is what the user must know: it takes the place of the phase.
    let notice = "자동 받기에 추가 확인이 필요합니다. 로그인 설정과 연결 상태를 확인해 주세요.";
    assert_eq!(gauge_state(&RunFacts { attention: notice, ..run(Stage::OpenForm, "로그인") }), running(None, notice));

    let done = |result: &str| DownloadJobState::Done { result: result.to_owned() };
    assert_eq!(gauge_state(&RunFacts { sheets: 2, ..run(Stage::Complete, "다운로드") }), done("도엽 2장을 받았습니다."));
    let kept = "필요한 지형도를 모두 보관하고 있습니다. 다시 신청하지 않습니다.";
    assert_eq!(gauge_state(&RunFacts { status: kept, ..run(Stage::Complete, "도엽 확인") }), done(kept));

    let stopped = |code: StopCode, reason: &str| DownloadJobState::Stopped { code, reason: reason.to_owned() };
    let late = "이 단계의 서버 응답 시간이 초과되었습니다. 계정 설정과 연결 상태를 확인하세요. 신청은 자동으로 반복하지 않습니다.";
    assert_eq!(gauge_state(&RunFacts { attention: late, failure_phase: "로그인", ..run(Stage::Stopped, "로그인") }), stopped(StopCode::Acc2, late));
    assert_eq!(gauge_state(&RunFacts { attention: late, failure_phase: "파일 준비", ..run(Stage::Stopped, "파일 준비") }), stopped(StopCode::Site1, late));
    let none = "이 범위에서 제공 중인 1:25,000 DXF 도엽이 없습니다.";
    assert_eq!(gauge_state(&RunFacts { attention: none, ..run(Stage::Stopped, "도엽 확인") }), stopped(StopCode::None1, none));
    assert_eq!(gauge_state(&RunFacts { cancelled: true, attention: late, failure_phase: "신청", ..run(Stage::Stopped, "신청") }), DownloadJobState::Cancelled);
    assert_eq!(gauge_state(&run(Stage::Stopped, "신청")), DownloadJobState::Cancelled, "stopped without a word: the user stopped it");
}
