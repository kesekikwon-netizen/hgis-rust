//! The 「유산」 press asks which city the survey area lies in (`HeritageRegionResolver.cpp:292-353`,
//! `MainWindow.cpp:3101-3166`). The answers are made here; no request leaves the test and no stored key is read.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::{Arc, Mutex};

use eframe::egui;
use strata_refmap::transfer::{Request, TransferFailure};

use crate::app_state::StrataAppState;
use crate::dialogs_modal::ActiveModalDialog;
use crate::ref_vector::map::run::RefMapJobs;
use crate::ref_vector::map::tests::{draw_survey_area, CENTRE};

type Answer = fn(&Request) -> Result<Vec<u8>, TransferFailure>;

struct Bench {
    app: StrataAppState,
    asked: Arc<Mutex<Vec<String>>>,
    _dir: tempfile::TempDir,
}

/// An app with a saved survey and a drawn survey area (EPSG:5187); a made-up key is stored.
fn bench(answer: Answer, online: fn() -> bool) -> Bench {
    let dir = tempfile::tempdir().unwrap();
    let survey = dir.path().join("현장.gpkg");
    std::fs::write(&survey, b"x").unwrap();
    let mut app = StrataAppState::new_startup();
    app.survey_path = Some(survey);
    draw_survey_area(&mut app, CENTRE, 5187);
    let asked = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::clone(&asked);
    app.ref_maps = RefMapJobs::for_test(
        Arc::new(move |request, _| {
            seen.lock().unwrap().push(request.url.clone());
            answer(request)
        }),
        online,
    );
    app.ref_maps.set_key(|| "TESTKEY".to_owned());
    Bench { app, asked, _dir: dir }
}

fn gyeongju(_: &Request) -> Result<Vec<u8>, TransferFailure> {
    Ok(r#"{"response":{"status":"OK","result":[{"structure":{"level1":"경상북도","level2":"경주시"}}]}}"#.as_bytes().to_vec())
}

fn press(bench: &mut Bench) {
    bench.app.press_heritage();
    bench.app.drive_heritage(&egui::Context::default(), 0.0);
}

/// One point inside the survey area is asked of the address service, in degrees, with the stored key; the
/// dialog then opens with the city it named.
#[test]
fn a_press_asks_for_the_city_and_opens_the_dialog_with_it() {
    let mut bench = bench(gyeongju, || true);
    bench.app.press_heritage();
    assert_eq!(bench.app.home_map.draw_guide_message.as_deref(), Some("조사구역이 속한 시·군을 찾는 중입니다…"));
    assert_eq!(bench.app.modals.active, None, "the dialog waits for the answer");

    bench.app.drive_heritage(&egui::Context::default(), 0.0);
    assert_eq!(bench.app.modals.active, Some(ActiveModalDialog::HeritageRegionSelect));
    assert_eq!(bench.app.home_map.draw_guide_message, None, "the search is over: the status line no longer says it runs");
    assert_eq!((bench.app.modals.heritage_sido.as_str(), bench.app.modals.heritage_city.as_str()), ("경상북도", "경주시"));
    assert_eq!(bench.app.heritage_region_words(), "조사구역이 속한 시·군을 확인하세요.\n자료는 선택한 시·군 단위로만 받습니다.");

    let asked = bench.asked.lock().unwrap().clone();
    assert_eq!(asked.len(), 1);
    assert!(asked[0].starts_with("https://api.vworld.kr/req/address?service=address&request=getAddress&version=2.0&crs=EPSG:4326&point="), "{}", asked[0]);
    assert!(asked[0].ends_with("&format=json&type=both&zipcode=false&simple=false&key=TESTKEY"));
    let point = asked[0].split("&point=").nth(1).unwrap().split('&').next().unwrap();
    let degrees: Vec<f64> = point.split(',').map(|part| part.parse().unwrap()).collect();
    assert!((129.20..129.25).contains(&degrees[0]) && (35.90..35.95).contains(&degrees[1]), "the survey area in degrees, longitude first: {point}");
}

/// When no city can be given the dialog still opens: it says why and asks to pick the city, and what was
/// picked before stays.
#[test]
fn without_a_city_the_dialog_says_why_and_asks_to_pick() {
    let pick = "\n조사구역이 속한 시·군을 아래에서 직접 선택하면 계속 받을 수 있습니다.";
    let said = |bench: &mut Bench| {
        press(bench);
        assert_eq!(bench.app.modals.active, Some(ActiveModalDialog::HeritageRegionSelect));
        bench.app.heritage_region_words().strip_suffix(pick).expect("asks to pick").to_owned()
    };

    let mut no_key = bench(gyeongju, || true);
    no_key.app.ref_maps.set_key(String::new);
    assert_eq!(said(&mut no_key), "VWorld 키가 없어 시·군을 판정하지 못했습니다. 더보기에서 키를 넣거나 시·군을 직접 고르세요.");
    assert!(no_key.asked.lock().unwrap().is_empty(), "nothing is asked without a key");

    let mut offline = bench(gyeongju, || false);
    assert_eq!(said(&mut offline), "행정구역 서버에 연결하지 못했습니다. 인터넷 연결을 확인한 뒤 다시 누르세요.");
    assert!(offline.asked.lock().unwrap().is_empty());

    let mut slow = bench(|_| Err(TransferFailure::Timeout), || true);
    assert_eq!(said(&mut slow), "행정구역 서버의 응답 시간이 초과되었습니다. 시·군을 직접 고르세요.");
    let mut cut = bench(|_| Err(TransferFailure::Network), || true);
    assert_eq!(said(&mut cut), "행정구역 서버에 연결하지 못했습니다. 인터넷 연결을 확인한 뒤 다시 누르세요.");

    let mut sea = bench(|_| Ok(r#"{"response":{"status":"OK","result":[{"text":"바다"}]}}"#.as_bytes().to_vec()), || true);
    sea.app.modals.heritage_sido = "경상북도".to_owned();
    sea.app.modals.heritage_city = "안동시".to_owned();
    assert_eq!(said(&mut sea), "좌표로 시·군을 찾지 못했습니다. 시·군을 직접 고르세요.");
    assert_eq!((sea.app.modals.heritage_sido.as_str(), sea.app.modals.heritage_city.as_str()), ("경상북도", "안동시"));

    let mut refused = bench(|_| Ok(br#"{"response":{"status":"ERROR","error":{"code":"INVALID_KEY","text":"key=TESTKEY"}}}"#.to_vec()), || true);
    let why = said(&mut refused);
    assert!(why.starts_with("VWorld API 키가 서버에서 거절되었습니다 (INVALID_KEY)."), "{why}");
    assert!(!why.contains("TESTKEY"));
}

/// A press while the first question is still open is told to wait (`:296`); another survey ends the wait.
#[test]
fn a_second_press_waits_for_the_first_answer() {
    let mut bench = bench(gyeongju, || true);
    bench.app.ref_maps.set_runner(Arc::new(|_| {}));
    bench.app.press_heritage();
    bench.app.press_heritage();
    assert_eq!(bench.app.home_map.draw_guide_message.as_deref(), Some("이전 판정이 아직 끝나지 않았습니다. 잠시 뒤 다시 누르세요."));
    assert!(bench.app.heritage.is_busy());
    bench.app.drop_heritage_run();
    assert!(!bench.app.heritage.is_busy());
}
