//! Raw-HTTP tests for held sockets, chunked/streamed transfers, and release methods
//! (`test_topographic_browser.cpp:70-125, 142-167, 392, 784-785`).

use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::thread;
use std::time::{Duration, Instant};
use strata_ngii_fixture::{
    NgiiFixtureSite, NgiiFixtureState, FIRST_CHUNK_SIZE, TRANSFER_SIZE,
};

fn open_get(addr: SocketAddr, target: &str) -> TcpStream {
    let mut stream =
        TcpStream::connect_timeout(&addr, Duration::from_secs(2)).expect("connect 127.0.0.1");
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("read timeout");
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .expect("write timeout");
    let req = format!("GET {target} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");
    stream.write_all(req.as_bytes()).expect("write GET");
    stream.flush().expect("flush GET");
    stream
}

fn read_until_headers_and_body_len(stream: &mut TcpStream, min_body: usize) -> (String, Vec<u8>) {
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut buf = Vec::new();
    let mut scratch = [0u8; 16384];
    loop {
        if let Some(sep) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            if buf.len() - (sep + 4) >= min_body {
                let head = std::str::from_utf8(&buf[..sep]).expect("ASCII headers").to_owned();
                return (head, buf[sep + 4..].to_vec());
            }
        }
        assert!(Instant::now() < deadline, "timed out waiting for {min_body} body bytes");
        let n = stream.read(&mut scratch).expect("read stream");
        assert!(n > 0, "unexpected EOF after {} bytes", buf.len());
        buf.extend_from_slice(&scratch[..n]);
    }
}

fn assert_socket_held_open(stream: &mut TcpStream) {
    stream
        .set_read_timeout(Some(Duration::from_millis(120)))
        .expect("short read timeout");
    let mut probe = [0u8; 16];
    let err = stream
        .read(&mut probe)
        .expect_err("expected socket to stay open without EOF");
    assert!(
        matches!(err.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut),
        "expected timeout on held socket, got {err:?}"
    );
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("restore read timeout");
}

fn wait_until(cond: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while !cond() {
        assert!(Instant::now() < deadline, "condition timed out");
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn slow_zip_writes_partial_payload_and_stays_open() {
    let site = NgiiFixtureSite::start().expect("start");
    let mut sock = open_get(site.addr(), "/slow.zip");
    let (head, body) = read_until_headers_and_body_len(&mut sock, 9);
    assert!(head.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(head.contains("Content-Disposition: attachment; filename=slow.zip"));
    assert!(head.contains("Content-Length: 1000000"));
    assert_eq!(body, b"PKpartial");
    assert_socket_held_open(&mut sock);
    assert_eq!(site.held_socket_count(), 1);
}

#[test]
fn controlled_zip_streams_first_chunk_and_completes_on_finish_transfer() {
    let site = NgiiFixtureSite::start().expect("start");
    assert!(!site.finish_transfer());
    let mut sock = open_get(site.addr(), "/controlled.zip");
    let (head, mut body) = read_until_headers_and_body_len(&mut sock, FIRST_CHUNK_SIZE);
    assert!(head.contains("Content-Type: application/zip"));
    assert!(head.contains("Content-Disposition: attachment; filename=controlled.zip"));
    assert!(head.contains(&format!("Content-Length: {TRANSFER_SIZE}")));
    assert_eq!(body.len(), FIRST_CHUNK_SIZE);
    assert_eq!(&body[..2], b"PK");
    assert!(body[2..].iter().all(|&b| b == b'x'));
    wait_until(|| site.has_transfer_socket());
    assert_socket_held_open(&mut sock);

    assert!(site.finish_transfer());
    sock.read_to_end(&mut body).expect("read rest");
    assert_eq!(body.len(), TRANSFER_SIZE);
    assert!(body[2..].iter().all(|&b| b == b'x'));
}

#[test]
fn chunked_zip_uses_hex_chunk_framing_and_zero_terminator() {
    let site = NgiiFixtureSite::start().expect("start");
    let mut sock = open_get(site.addr(), "/chunked.zip");
    let expected_first_frame = format!("{FIRST_CHUNK_SIZE:x}\r\n").len() + FIRST_CHUNK_SIZE + 2;
    let (head, mut raw_body) = read_until_headers_and_body_len(&mut sock, expected_first_frame);
    assert!(head.contains("Transfer-Encoding: chunked"));
    assert!(!head.contains("Content-Length:"));
    assert!(raw_body.starts_with(b"100000\r\nPK"));
    assert!(raw_body.ends_with(b"\r\n"));
    wait_until(|| site.has_transfer_socket());
    assert_socket_held_open(&mut sock);

    assert!(site.finish_transfer());
    sock.read_to_end(&mut raw_body).expect("read chunked rest");
    let rest_header = b"300000\r\n";
    assert_eq!(
        &raw_body[expected_first_frame..expected_first_frame + rest_header.len()],
        rest_header
    );
    assert!(raw_body.ends_with(b"\r\n0\r\n\r\n"));
}

#[test]
fn interrupted_zip_aborts_first_transfer_and_returns_503_on_second() {
    let site = NgiiFixtureSite::start().expect("start");
    let mut first = open_get(site.addr(), "/interrupted.zip");
    let (head, mut body) = read_until_headers_and_body_len(&mut first, FIRST_CHUNK_SIZE);
    assert!(head.contains("filename=interrupted.zip"));
    wait_until(|| site.has_transfer_socket());
    assert!(site.abort_transfer());
    let _ = first.read_to_end(&mut body);
    assert!(body.len() < TRANSFER_SIZE);
    assert_eq!(site.state().lock().expect("lock").interrupted_requests, 1);

    let mut second = open_get(site.addr(), "/interrupted.zip");
    let mut raw = Vec::new();
    second.read_to_end(&mut raw).expect("read 503");
    assert_eq!(
        raw,
        b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    );
    assert_eq!(site.state().lock().expect("lock").interrupted_requests, 2);
}

#[test]
fn held_login_script_and_held_login_response_behave_like_cpp_server() {
    let mut init = NgiiFixtureState::default();
    init.hold_login_response = true;
    let site = NgiiFixtureSite::start_with(init).expect("start");

    let mut script_sock = open_get(site.addr(), "/held-login-script.js");
    wait_until(|| site.has_pending_login_script());
    assert_socket_held_open(&mut script_sock);
    assert!(site.finish_pending_login_script());
    let mut script_resp = Vec::new();
    script_sock.read_to_end(&mut script_resp).expect("read released script");
    assert_eq!(
        script_resp,
        b"HTTP/1.1 200 OK\r\nContent-Type: application/javascript\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    );

    let mut login_sock = open_get(site.addr(), "/member/process.do");
    wait_until(|| site.held_socket_count() == 1);
    assert_eq!(site.state().lock().expect("lock").login_submission_count, 1);
    assert_socket_held_open(&mut login_sock);
}

#[test]
fn stream_fixture_archives_releases_pending_xml_when_zip_finishes() {
    let mut init = NgiiFixtureState::default();
    init.stream_fixture_archives = true;
    let site = NgiiFixtureSite::start_with(init).expect("start");

    let mut zip_sock = open_get(site.addr(), "/fixture-378044.zip");
    let (zip_head, mut zip_body) = read_until_headers_and_body_len(&mut zip_sock, FIRST_CHUNK_SIZE);
    assert!(zip_head.contains("filename=terrain_378044.zip"));

    let mut xml_sock = open_get(site.addr(), "/fixture-378044.xml");
    wait_until(|| site.has_pending_xml_socket());
    assert_eq!(
        site.state().lock().expect("lock").pending_xml_name,
        b"terrain_378044.xml"
    );
    assert_socket_held_open(&mut xml_sock);

    assert!(site.finish_transfer());
    zip_sock.read_to_end(&mut zip_body).expect("read zip rest");
    assert_eq!(zip_body.len(), TRANSFER_SIZE);

    let (xml_head, xml_body) = read_until_headers_and_body_len(&mut xml_sock, 16);
    assert!(xml_head.contains("Content-Type: application/xml"));
    assert!(xml_head.contains("filename=terrain_378044.xml"));
    assert_eq!(xml_body, b"fixture metadata");
    assert!(site.state().lock().expect("lock").fixture_archive_finished);
}

#[test]
fn hold_sdk_transfers_holds_dxf_and_xml_independently() {
    let mut init = NgiiFixtureState::default();
    init.hold_sdk_transfers = true;
    let site = NgiiFixtureSite::start_with(init).expect("start");

    let mut dxf_sock = open_get(
        site.addr(),
        "/nlippd/pd/innorix/innorixDownLoad2.do?fileName=terrain_378044.dxf&directory=fixture",
    );
    let (_head, mut dxf_body) = read_until_headers_and_body_len(&mut dxf_sock, FIRST_CHUNK_SIZE);
    assert_eq!(
        site.state().lock().expect("lock").active_sdk_file,
        "terrain_378044.dxf"
    );
    assert!(site.finish_transfer());
    dxf_sock.read_to_end(&mut dxf_body).expect("read dxf rest");
    assert_eq!(dxf_body.len(), TRANSFER_SIZE);
    assert!(site.state().lock().expect("lock").active_sdk_file.is_empty());

    let mut xml_sock = open_get(
        site.addr(),
        "/nlippd/pd/innorix/innorixDownLoad2.do?fileName=terrain_378044.xml&directory=fixture",
    );
    wait_until(|| site.has_sdk_xml_socket());
    assert_eq!(
        site.state().lock().expect("lock").sdk_xml_name,
        "terrain_378044.xml"
    );
    assert_socket_held_open(&mut xml_sock);
    assert!(site.finish_sdk_xml());
    let (xml_head, xml_body) = read_until_headers_and_body_len(&mut xml_sock, 16);
    assert!(xml_head.contains("Content-Type: application/xml"));
    assert!(xml_head.contains("filename=terrain_378044.xml"));
    assert_eq!(xml_body, b"fixture metadata");
}
