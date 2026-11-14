//! Raw-HTTP tests for immediate routes, switches, and counters (`test_topographic_browser.cpp:126-185`).

use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use strata_ngii_fixture::{
    NgiiFixtureSite, NgiiFixtureState, COMBINE_BUFFER_QUERY_JSON, TRANSFER_SIZE,
};

struct RawHttpResponse {
    status_line: String,
    headers: String,
    body: Vec<u8>,
}

fn send_raw_get(addr: SocketAddr, target: &str) -> std::io::Result<Vec<u8>> {
    assert_eq!(addr.ip(), Ipv4Addr::LOCALHOST);
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(2))?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    let req = format!("GET {target} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n");
    stream.write_all(req.as_bytes())?;
    stream.flush()?;
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf)?;
    Ok(buf)
}

fn parse_http(raw: &[u8]) -> RawHttpResponse {
    let sep = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .expect("expected HTTP header terminator \\r\\n\\r\\n");
    let head = std::str::from_utf8(&raw[..sep]).expect("ASCII HTTP headers");
    let (status_line, headers) = head.split_once("\r\n").expect("status line + headers");
    RawHttpResponse {
        status_line: status_line.to_owned(),
        headers: headers.to_owned(),
        body: raw[sep + 4..].to_vec(),
    }
}

#[test]
fn root_route_sets_shared_cookie_and_local_fixture_body() {
    let site = NgiiFixtureSite::start().expect("start server");
    assert_eq!(
        site.url(""),
        format!("http://127.0.0.1:{}/", site.addr().port())
    );
    let resp = parse_http(&send_raw_get(site.addr(), "/").expect("GET /"));
    assert_eq!(resp.status_line, "HTTP/1.1 200 OK");
    assert!(resp.headers.contains("Content-Type: text/html; charset=utf-8"));
    assert!(resp
        .headers
        .contains("Set-Cookie: fixture=shared; Path=/; SameSite=Lax"));
    assert_eq!(
        resp.body,
        b"<html><head><title>local fixture</title></head><body>fixture</body></html>"
    );
}

#[test]
fn unknown_route_returns_local_fixture_without_cookie() {
    let site = NgiiFixtureSite::start().expect("start server");
    let resp = parse_http(&send_raw_get(site.addr(), "/unmapped").expect("GET /unmapped"));
    assert_eq!(resp.status_line, "HTTP/1.1 200 OK");
    assert!(!resp.headers.contains("Set-Cookie:"));
    assert_eq!(
        resp.body,
        b"<html><head><title>local fixture</title></head><body>fixture</body></html>"
    );
}

#[test]
fn popup_route_returns_popup_html() {
    let site = NgiiFixtureSite::start().expect("start server");
    let resp = parse_http(&send_raw_get(site.addr(), "/popup").expect("GET /popup"));
    assert_eq!(resp.status_line, "HTTP/1.1 200 OK");
    assert!(resp.headers.contains("Content-Type: text/html; charset=utf-8"));
    assert_eq!(
        resp.body,
        b"<html><head><title>popup</title></head><body>popup</body></html>"
    );
}

#[test]
fn file_zip_route_returns_terrain_zip_attachment() {
    let site = NgiiFixtureSite::start().expect("start server");
    let resp = parse_http(&send_raw_get(site.addr(), "/file.zip").expect("GET /file.zip"));
    assert_eq!(resp.status_line, "HTTP/1.1 200 OK");
    assert!(resp.headers.contains("Content-Type: application/zip"));
    assert!(resp
        .headers
        .contains("Content-Disposition: attachment; filename=terrain.zip"));
    assert_eq!(resp.body, b"PK-fixture-complete");
}

#[test]
fn portal_map_route_returns_portal_fixture() {
    let mut init = NgiiFixtureState::default();
    init.portal_fixture = b"<html>portal-ok</html>".to_vec();
    let site = NgiiFixtureSite::start_with(init).expect("start server");
    let resp = parse_http(
        &send_raw_get(site.addr(), "/ms/map/NlipMap.do?tabGb=total").expect("GET NlipMap.do"),
    );
    assert_eq!(resp.status_line, "HTTP/1.1 200 OK");
    assert!(resp.headers.contains("Content-Type: text/html; charset=utf-8"));
    assert_eq!(resp.body, b"<html>portal-ok</html>");
}

#[test]
fn portal_map_route_aborts_when_fail_portal_connection_enabled() {
    let mut init = NgiiFixtureState::default();
    init.portal_fixture = b"<html>should-not-arrive</html>".to_vec();
    init.fail_portal_connection = true;
    let site = NgiiFixtureSite::start_with(init).expect("start server");
    if let Ok(bytes) = send_raw_get(site.addr(), "/ms/map/NlipMap.do") {
        assert!(
            bytes.is_empty(),
            "expected aborted connection without HTTP bytes, got {} bytes",
            bytes.len()
        );
    }
    // Verify flipping fail_portal_connection back to false serves portal_fixture.
    site.state().lock().expect("lock").fail_portal_connection = false;
    let resp = parse_http(&send_raw_get(site.addr(), "/ms/map/NlipMap.do").expect("GET NlipMap"));
    assert_eq!(resp.body, b"<html>should-not-arrive</html>");
}

#[test]
fn combine_buffer_query_route_returns_polygon_json() {
    let site = NgiiFixtureSite::start().expect("start server");
    let resp = parse_http(
        &send_raw_get(site.addr(), "/ms/map/combineBufferQuery.do?radius=5000")
            .expect("GET combineBufferQuery.do"),
    );
    assert_eq!(resp.status_line, "HTTP/1.1 200 OK");
    assert!(resp.headers.contains("Content-Type: application/json"));
    assert_eq!(resp.body, COMBINE_BUFFER_QUERY_JSON.as_bytes());
}

#[test]
fn suchi_query_route_returns_search_fixture_and_increments_search_count() {
    let mut init = NgiiFixtureState::default();
    init.search_fixture = br#"{"items":[{"num":"378044"}]}"#.to_vec();
    let site = NgiiFixtureSite::start_with(init).expect("start server");
    for expected_count in 1..=2 {
        let resp = parse_http(
            &send_raw_get(site.addr(), "/ms/map/suchiQuery.do?scale=25000")
                .expect("GET suchiQuery.do"),
        );
        assert_eq!(resp.status_line, "HTTP/1.1 200 OK");
        assert!(resp.headers.contains("Content-Type: application/json"));
        assert_eq!(resp.body, br#"{"items":[{"num":"378044"}]}"#);
        assert_eq!(site.state().lock().expect("lock").search_count, expected_count);
    }
}

#[test]
fn login_check_route_reflects_logged_in_switch_and_increments_count() {
    let site = NgiiFixtureSite::start().expect("start server");
    let first = parse_http(&send_raw_get(site.addr(), "/mn/loginCheck.do").expect("check 1"));
    assert!(first.headers.contains("Content-Type: application/json"));
    assert_eq!(first.body, br#"{"isLogin":true}"#);

    site.state().lock().expect("lock").logged_in = false;
    let second = parse_http(&send_raw_get(site.addr(), "/mn/loginCheck.do").expect("check 2"));
    assert_eq!(second.body, br#"{"isLogin":false}"#);
    assert_eq!(site.state().lock().expect("lock").login_check_count, 2);
}

#[test]
fn member_login_and_any_id_routes_return_configured_fixtures() {
    let mut init = NgiiFixtureState::default();
    init.login_fixture = b"<html>login-form</html>".to_vec();
    init.any_id_fixture = b"<html>anyid-form</html>".to_vec();
    let site = NgiiFixtureSite::start_with(init).expect("start server");

    let login = parse_http(
        &send_raw_get(site.addr(), "/member/login.do?returnUrl=/").expect("GET login.do"),
    );
    assert_eq!(login.body, b"<html>login-form</html>");

    let any_id = parse_http(
        &send_raw_get(site.addr(), "/anyid/common/anyidLogin.do").expect("GET anyidLogin.do"),
    );
    assert_eq!(any_id.body, b"<html>anyid-form</html>");
}

#[test]
fn member_process_route_respects_accept_login_and_counts_submissions() {
    let mut init = NgiiFixtureState::default();
    init.logged_in = false;
    init.accept_login = false;
    let site = NgiiFixtureSite::start_with(init).expect("start server");

    let rejected = parse_http(&send_raw_get(site.addr(), "/member/process.do").expect("submit 1"));
    assert_eq!(rejected.body, b"<html>Fixture login response</html>");
    assert!(!site.state().lock().expect("lock").logged_in);

    site.state().lock().expect("lock").accept_login = true;
    let accepted = parse_http(&send_raw_get(site.addr(), "/member/process.do").expect("submit 2"));
    assert_eq!(accepted.body, b"<html><script>parent.close();</script></html>");
    let guard = site.state().lock().expect("lock");
    assert!(guard.logged_in);
    assert_eq!(guard.login_submission_count, 2);
}

#[test]
fn purchase_and_innorix_download_routes_respect_redirect_transfer_switch() {
    let mut init = NgiiFixtureState::default();
    init.download_fixture = b"<html>download-list</html>".to_vec();
    let site = NgiiFixtureSite::start_with(init).expect("start server");

    let direct = parse_http(&send_raw_get(site.addr(), "/pd/purchs/download.do").expect("direct"));
    assert_eq!(direct.body, b"<html>download-list</html>");

    site.state().lock().expect("lock").redirect_transfer = true;
    let redirected =
        parse_http(&send_raw_get(site.addr(), "/pd/purchs/download.do").expect("redirect"));
    assert_eq!(
        redirected.body,
        b"<html><script>location.replace('/nlippd/pd/innorix/innorixdownLoad.do');</script></html>"
    );

    let innorix = parse_http(
        &send_raw_get(site.addr(), "/nlippd/pd/innorix/innorixdownLoad.do").expect("innorix"),
    );
    assert_eq!(innorix.body, b"<html>download-list</html>");
}

#[test]
fn public_sheet_fixture_routes_return_immediate_payload_when_not_streaming() {
    let site = NgiiFixtureSite::start().expect("start server");
    for ext in ["zip", "xml"] {
        let target = format!("/fixture-378044.{ext}");
        let resp = parse_http(&send_raw_get(site.addr(), &target).expect("GET fixture"));
        assert!(resp.headers.contains("Content-Type: application/zip"));
        assert!(resp
            .headers
            .contains(&format!("Content-Disposition: attachment; filename=terrain_378044.{ext}")));
        assert_eq!(resp.body, b"PK-public-sheet-fixture");
    }
}

#[test]
fn sdk_download2_route_serves_dxf_xml_fallback_and_notifies_observer() {
    let observed: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let observed_cb = Arc::clone(&observed);
    let mut init = NgiiFixtureState::default();
    init.sdk_request_observed = Some(Arc::new(move |name: &str| {
        observed_cb.lock().expect("obs lock").push(name.to_owned());
    }));
    let site = NgiiFixtureSite::start_with(init).expect("start server");

    let dxf = parse_http(
        &send_raw_get(
            site.addr(),
            "/nlippd/pd/innorix/innorixDownLoad2.do?fileName=terrain_378044.dxf&directory=fixture",
        )
        .expect("GET dxf"),
    );
    assert!(dxf.headers.contains("Content-Type: application/octet-stream"));
    assert!(dxf
        .headers
        .contains("Content-Disposition: attachment; filename=terrain_378044.dxf"));
    assert_eq!(dxf.body.len(), TRANSFER_SIZE);
    assert!(dxf.body.iter().all(|&b| b == b'x'));

    let xml = parse_http(
        &send_raw_get(
            site.addr(),
            "/nlippd/pd/innorix/innorixDownLoad2.do?directory=fixture&fileName=terrain%5F378044.xml",
        )
        .expect("GET xml"),
    );
    assert!(xml
        .headers
        .contains("Content-Disposition: attachment; filename=terrain_378044.xml"));
    assert_eq!(xml.body, b"fixture metadata");

    let other = parse_http(
        &send_raw_get(
            site.addr(),
            "/nlippd/pd/innorix/innorixDownLoad2.do?fileName=terrain_378044.zip",
        )
        .expect("GET zip"),
    );
    assert_eq!(other.body, b"PK-fixture-complete");

    let expected = vec![
        "terrain_378044.dxf".to_owned(),
        "terrain_378044.xml".to_owned(),
        "terrain_378044.zip".to_owned(),
    ];
    assert_eq!(site.state().lock().expect("lock").sdk_requests, expected);
    assert_eq!(*observed.lock().expect("obs lock"), expected);
}
