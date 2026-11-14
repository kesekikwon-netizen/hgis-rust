//! Regression tests for `NET0c5`: which `file:` URLs count as local, and curl URL globbing switched off for an
//! offline local launch. Review findings of 2026-11-14 on `NET0c4` and on the first attempt at this fix: real
//! curl 8.22 opened `\\127.0.0.1\c$\...` (a network share) for `file:///%5C127.0.0.1/...`,
//! `file:///.//127.0.0.1/...` and `file:///{,/}127.0.0.1/...`.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;

use super::local_curl_guard_args;
use crate::curl_stdin::output_with_url_on_stdin;
use crate::network::{allow_launch_urls, is_local_target, probe_dry_run};

#[test]
fn only_a_plain_drive_letter_file_url_is_local_and_nothing_else_launches_offline() {
    let bad = [
        // percent-encoded separators: curl decodes them
        "file:///%5Cevil.example/share/f",
        "file:///%2fevil.example/share/f",
        "file://localhost/%5Cevil.example/share/f",
        "file:///C:/a%5C..%5Cb",
        "file:///C:/tmp/a%20b.txt",
        "file:///%255Cevil.example/f",
        // dot segments and doubled slashes: curl collapses them to //host/share
        "file:///.//evil.example/share/f",
        "file://localhost/.//evil.example/share/f",
        "file:///a/..//evil.example/share/f",
        // URL globbing: {,/} expands to a leading slash
        "file:///{,/}evil.example/share/f",
        "file:///{C:/Windows/win.ini,/evil.example/share/f}",
        "file:///C:/tiles/[1-2].png",
        // no drive letter, a backslash, nothing after the drive, another authority
        "file:///tmp/tile.txt",
        "FILE://LOCALHOST/tmp/tile.txt",
        "file:///C:\\Windows\\win.ini",
        "file:///C:",
        "file:///1:/x",
        "file://evil.example/C:/x",
    ];
    for url in bad {
        assert!(!is_local_target(url), "expected non-local: {url}");
        let (launches, recorded) = probe_dry_run(false, || assert!(!allow_launch_urls([url])));
        assert_eq!(launches, 0, "must not launch offline: {url}");
        assert!(recorded.is_empty(), "must record nothing: {url}");
    }
    let good =
        ["file:///C:/Users/test/tile.txt", "file://localhost/d:/tile.txt", "FILE:///C:/자료/시험/a.txt", "file:///C:/"];
    for url in good {
        assert!(is_local_target(url), "expected local: {url}");
    }
}

#[test]
fn offline_local_launch_switches_curl_globbing_off() {
    assert!(local_curl_guard_args().1.contains(&"--globoff"));

    // Real curl against a listener of this test: with globbing on, `{a,b}` would become two requests (`/a`, `/b`).
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let port = listener.local_addr().expect("local addr").port();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("one request");
        let mut buf = [0_u8; 1024];
        let n = stream.read(&mut buf).expect("read request");
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
            .expect("write response");
        String::from_utf8_lossy(&buf[..n]).lines().next().unwrap_or_default().to_owned()
    });
    let mut cmd = Command::new(if cfg!(target_os = "windows") { "curl.exe" } else { "curl" });
    cmd.args(["-sS", "--max-time", "5"]);
    let out = output_with_url_on_stdin(cmd, &format!("http://127.0.0.1:{port}/{{a,b}}")).expect("local curl");
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(out.stdout, b"ok");
    assert_eq!(server.join().expect("server thread"), "GET /{a,b} HTTP/1.1");
}
