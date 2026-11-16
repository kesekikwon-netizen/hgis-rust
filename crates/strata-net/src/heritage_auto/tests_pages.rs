//! The stages between the login and the first file must move the way the C++ callbacks do
//! (`KaHeritageBrowser.cpp:1100-1260`, `HeritageIntranetFlow.cpp:215-394`).

use super::machine::{HeritageConfig, HeritageEvent, HeritageStage};
use super::HeritageAutomation;

fn at(stage: HeritageStage) -> HeritageAutomation {
    let mut m = HeritageAutomation::new(HeritageConfig {
        account_id: "user1".to_owned(),
        account_password: "pass1".to_owned(),
        sido: "경상북도".to_owned(),
        city: "경주시".to_owned(),
        fixture_base: None,
    });
    m.force_stage(stage.clone());
    m
}

/// The tutorial is dismissed before the panel is opened, and a tutorial that was not there is
/// not a problem (`KaHeritageBrowser.cpp:820-822`).
#[test]
fn a_tutorial_is_dismissed_and_its_absence_is_fine() {
    for answer in ["closed", "none"] {
        let mut m = at(HeritageStage::DismissTutorial);
        let ev = m.script_answered(m.current_token(), answer);
        assert!(ev.is_none(), "「{answer}」 was treated as a problem: {ev:?}");
        assert_eq!(m.stage(), &HeritageStage::DownloadPage, "「{answer}」");
    }
}

/// The panel is opened through the site's own `showAgreePopup()`, and `not-found` stops.
#[test]
fn the_panel_opens_or_the_run_stops() {
    let mut m = at(HeritageStage::DownloadPage);
    let _ = m.script_answered(m.current_token(), "opening");
    assert_eq!(m.stage(), &HeritageStage::Terms);

    let mut m = at(HeritageStage::DownloadPage);
    let ev = m.script_answered(m.current_token(), "not-found");
    assert!(matches!(ev, Some(HeritageEvent::Stopped(_))), "{ev:?}");
    assert!(m.is_over());
}

/// Agreeing answers the pledge text, which the run keeps for the receipt.
#[test]
fn agreeing_keeps_the_pledge_text() {
    let mut m = at(HeritageStage::Terms);
    let answer = r#"{"status":"agreed","text":"국가유산 공간정보 원본자료 사용 서약서"}"#;
    let ev = m.script_answered(m.current_token(), answer);
    assert!(matches!(ev, Some(HeritageEvent::Note(_))), "{ev:?}");
    assert_eq!(m.stage(), &HeritageStage::Kind);
    assert_eq!(
        m.pledge_text(),
        Some("국가유산 공간정보 원본자료 사용 서약서"),
        "the pledge text was not kept"
    );
}

/// A pledge box with no 확인 button stops with the C++ words rather than ticking silently.
#[test]
fn a_pledge_without_a_confirm_button_stops() {
    let mut m = at(HeritageStage::Terms);
    let ev = m.script_answered(m.current_token(), "checked-no-confirm");
    let Some(HeritageEvent::Stopped(words)) = ev else {
        panic!("did not stop: {ev:?}");
    };
    assert!(words.contains("확인"), "{words}");
}

/// A missing 시·도 is a fact about the survey, and the words say which one.
#[test]
fn an_unknown_sido_names_it() {
    let mut m = at(HeritageStage::Region);
    let ev = m.script_answered(m.current_token(), "no-sido");
    let Some(HeritageEvent::Stopped(words)) = ev else {
        panic!("did not stop: {ev:?}");
    };
    assert!(words.contains("경상북도"), "the words do not name the 시·도: {words}");
}

/// A 시·군 the site does not list stops, and the words name the city and say how many it has.
#[test]
fn an_unknown_city_names_it_and_counts_the_rest() {
    let mut m = at(HeritageStage::Region);
    let ev = m.script_answered(m.current_token(), "sigungu:22");
    let Some(HeritageEvent::Stopped(words)) = ev else {
        panic!("did not stop: {ev:?}");
    };
    assert!(words.contains("경주시"), "the words do not name the city: {words}");
    assert!(words.contains("22"), "the words do not say how many the site has: {words}");
}

/// A region that lands moves on to the search.
#[test]
fn a_chosen_region_moves_on_to_the_search() {
    let mut m = at(HeritageStage::Region);
    let _ = m.script_answered(m.current_token(), "selected");
    assert_eq!(m.stage(), &HeritageStage::Search);
}

/// Pressing 검색 only means the button was pressed; the result probe is a later stage.
#[test]
fn pressing_search_moves_on() {
    let mut m = at(HeritageStage::Search);
    let _ = m.script_answered(m.current_token(), "searching");
    assert_eq!(m.stage(), &HeritageStage::DownloadAll);
}

/// Every page stage asks its own script, so a stage can never run another stage's script.
#[test]
fn each_page_stage_asks_its_own_script() {
    let cases = [
        (HeritageStage::DismissTutorial, "downloadForm"),
        (HeritageStage::DownloadPage, "showAgreePopup"),
        (HeritageStage::Terms, "agreementBox"),
        (HeritageStage::Kind, "changeTab"),
        (HeritageStage::Region, "codedeta"),
        (HeritageStage::Search, "findSearch"),
    ];
    for (stage, mark) in cases {
        let m = at(stage.clone());
        let script = m.page_script().unwrap_or_default();
        assert!(!script.is_empty(), "{stage:?} asks nothing");
        assert!(script.contains(mark), "{stage:?} does not carry 「{mark}」");
        assert!(script.contains("var W=pick();"), "{stage:?} does not wrap");
    }
}
