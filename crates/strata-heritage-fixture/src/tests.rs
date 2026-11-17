//! The fixture's tests: the pages carry the ids the scripts read, and a wrong password opens
//! no session.

use crate::Fixture;

fn request(url: String) -> std::sync::mpsc::Receiver<String> {
    let (sent, received) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = sent.send(crate::server::http_get(&url, None));
    });
    received
}

#[test]
fn the_configured_transfer_delay_precedes_the_zip_response() {
    let fixture = Fixture::start(("user1", "pass1"), ("경상북도", "경주시"));
    fixture.give_zip("지정유산", b"fixture payload".to_vec());
    fixture.set_download_delay(150);
    let began = std::time::Instant::now();
    let body = request(fixture.url("/user/data/heritageDownload/downloadFilesAll.do"))
        .recv_timeout(std::time::Duration::from_secs(2))
        .expect("ZIP response deadline");
    assert!(body.contains("fixture payload"));
    // A configured lower bound has no upper timing tolerance; a busy scheduler may answer later.
    assert!(began.elapsed() >= std::time::Duration::from_millis(150));
}

#[test]
fn a_delayed_zip_keeps_an_ordinary_route_responsive() {
    use std::time::{Duration, Instant};
    let fixture = Fixture::start(("user1", "pass1"), ("경상북도", "경주시"));
    fixture.give_zip("지정유산", b"fixture payload".to_vec());
    fixture.set_download_delay(800);
    let url = fixture.url("/user/data/heritageDownload/downloadFilesAll.do");
    let began = Instant::now();
    let slow = request(url);
    while fixture
        .state
        .try_lock()
        .map(|state| state.hits("/user/data/heritageDownload/downloadFilesAll.do"))
        .unwrap_or(0)
        == 0
    {
        assert!(
            began.elapsed() < Duration::from_secs(2),
            "download request never arrived"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
    let normal = request(fixture.url("/user/checkNet.do"))
        .recv_timeout(Duration::from_millis(500))
        .expect("ordinary response deadline");
    assert!(normal.contains("id=\"userId\""));
    // The ordinary route has 500 ms, leaving a 300 ms separation from the 800 ms ZIP delay.
    // This tolerates scheduling jitter while detecting a state lock held during the delay.
    assert!(began.elapsed() < Duration::from_millis(500));
    assert!(slow
        .recv_timeout(Duration::from_secs(2))
        .expect("delayed ZIP response deadline")
        .contains("fixture payload"));
    // Completion must respect the artificial minimum; scheduling may make it later.
    assert!(began.elapsed() >= Duration::from_millis(800));
}

#[test]
fn login_page_has_the_ids_the_script_fills() {
    let fx = Fixture::start(("user1", "pass1"), ("경상북도", "경주시"));
    let body = fx.get("/user/checkNet.do");
    assert!(body.contains("id=\"userId\""), "{body}");
    assert!(body.contains("id=\"pwd\""), "{body}");
    assert!(body.contains("function goLogin()"), "{body}");
}

#[test]
fn a_right_password_reaches_the_frames_page() {
    let fx = Fixture::start(("user1", "pass1"), ("경상북도", "경주시"));
    let _ = fx.post_form("/j_spring_security_check", "USER_ID=user1&USER_PW=pass1");
    let body = fx.get("/user/checkNet.do");
    assert!(
        body.contains("<frameset"),
        "no frames page after login: {body}"
    );
    assert!(
        body.contains("heritageDownload.do"),
        "download frame missing: {body}"
    );
}

#[test]
fn wrong_password_leaves_us_on_the_login_page() {
    let fx = Fixture::start(("user1", "right"), ("경상북도", "경주시"));
    let _ = fx.post_form("/j_spring_security_check", "USER_ID=user1&USER_PW=wrong");
    let body = fx.get("/user/checkNet.do");
    assert!(
        body.contains("id=\"pwd\""),
        "wrong password opened a session: {body}"
    );
    assert!(
        !body.contains("<frameset"),
        "wrong password opened a session: {body}"
    );
}

#[test]
fn the_download_panel_has_what_the_scripts_look_for() {
    let fx = Fixture::start(("u", "p"), ("경상북도", "경주시"));
    let body = fx.get("/user/data/heritageDownload.do");
    for id in [
        "id=\"searchForm\"",
        "id=\"mode\"",
        "id=\"codedetaCd0\"",
        "id=\"codeCdSg0\"",
        "id=\"searchGisChaRirList\"",
        "id=\"downloadFilesAll\"",
        "id=\"tabContentDiv\"",
    ] {
        assert!(body.contains(id), "panel has no {id}: {body}");
    }
    assert!(body.contains("name=\"codedetaCd\""), "{body}");
    assert!(body.contains("name=\"codeCdSg\""), "{body}");
    // The 지적 panel's pair comes first in the DOM, as on the site: the region script must
    // skip it and take codedeta/CodeCdSg instead.
    assert!(
        body.find("bjdcd1_newJbn").unwrap_or(usize::MAX) < body.find("codedetaCd0").unwrap_or(0),
        "the 지적 select must come first for the region test to mean anything"
    );
}

#[test]
fn all_six_kind_tabs_are_on_the_panel() {
    let fx = Fixture::start(("u", "p"), ("경상북도", "경주시"));
    let body = fx.get("/user/data/heritageDownload.do");
    for (code, name) in crate::pages::KIND_TABS {
        assert!(
            body.contains(&format!("changeTab('{code}')")),
            "no tab for {code}"
        );
        assert!(body.contains(name), "tab {code} has no Korean name {name}");
    }
}

#[test]
fn hits_are_counted_per_route() {
    let fx = Fixture::start(("u", "p"), ("경상북도", "경주시"));
    assert_eq!(fx.hits("/user/checkNet.do"), 0);
    let _ = fx.get("/user/checkNet.do");
    let _ = fx.get("/user/checkNet.do");
    assert_eq!(fx.hits("/user/checkNet.do"), 2);
}

#[test]
fn an_unknown_city_still_answers_the_panel() {
    // The fixture does not decide which cities exist; it answers the panel for any city so a
    // test can drive the stages. Returning rows for a city the site would refuse is a
    // test-only freedom recorded in FIXTURES.md.
    let fx = Fixture::start(("u", "p"), ("없는도", "없는시"));
    let body = fx.get("/user/data/heritageDownload.do");
    assert!(body.contains("없는시"), "{body}");
}
