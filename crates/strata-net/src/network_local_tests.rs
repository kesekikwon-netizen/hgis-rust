//! Tests for offline local `curl` launch hardening and guard helper (`NET0c3`).

use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, TcpListener};
use std::process::Command;

use crate::curl_stdin::{curl_url_config, output_with_url_on_stdin};
use crate::network::{
    allow_launch_urls, allowed_launches, local_curl_guard_args, probe_dry_run,
};

fn curl_bin() -> &'static str {
    if cfg!(target_os = "windows") {
        "curl.exe"
    } else {
        "curl"
    }
}

#[test]
fn offline_local_launch_records_disable_first_noproxy_and_max_redirs_zero_last() {
    let local_url = "http://127.0.0.1:18080/tile.png";
    let before = allowed_launches();
    let work_dir = std::env::temp_dir();

    let (launches, cmds) = probe_dry_run(false, || {
        let mut cmd = Command::new("curl");
        cmd.args(["-sS", "--max-time", "5"])
            .env("STRATA_TEST_ENV_KEPT", "1")
            .current_dir(&work_dir);
        let _ = output_with_url_on_stdin(cmd, local_url);
    });

    assert_eq!(launches, 1);
    assert!(allowed_launches() > before);
    assert_eq!(cmds[0].program, "curl");
    assert_eq!(cmds[0].args.first().map(String::as_str), Some("-q"));
    assert!(cmds[0].args.windows(2).any(|w| w == ["--noproxy", "*"]));
    assert!(cmds[0]
        .args
        .ends_with(&["--max-redirs".to_owned(), "0".to_owned()]));
    assert_eq!(
        cmds[0].args,
        vec![
            "-q",
            "-sS",
            "--max-time",
            "5",
            "--config",
            "-",
            "--noproxy",
            "*",
            "--max-redirs",
            "0",
        ]
    );
    assert_eq!(cmds[0].stdin, curl_url_config(local_url).into_bytes());
}

#[test]
fn offline_local_launch_refuses_every_curl8_redirect_spelling() {
    let local_url = "http://127.0.0.1:18080/tile.png";

    for flag in [
        "-L",
        "--location",
        "--location-trusted",
        "--follow",
        "-sSL",
        "-LsS",
    ] {
        let (launches, cmds) = probe_dry_run(false, || {
            let mut cmd = Command::new("curl");
            cmd.args(["-sS", flag]);
            assert!(
                output_with_url_on_stdin(cmd, local_url).is_err(),
                "redirect spelling {flag} should fail"
            );
        });
        assert_eq!(launches, 0, "redirect spelling {flag} must be refused");
        assert!(cmds.is_empty(), "redirect spelling {flag} must record nothing");
    }
}

#[test]
fn offline_local_launch_refuses_positional_url_or_extra_config() {
    let local_url = "http://127.0.0.1:18080/tile.png";

    let bad_arg_sets: &[&[&str]] = &[
        &["-sS", "https://example.com/off-pc"],
        &["-sS", "http://127.0.0.1:18080/second"],
        &["-sS", "example.com"],
        &["-sS", "--", "http://127.0.0.1:18080/second"],
        &["-sS", "--url", "http://127.0.0.1:18080/second"],
        &["-sS", "--url=http://127.0.0.1:18080/second"],
        &["-sS", "-K", "extra.curlrc"],
        &["-sS", "-K", "-"],
        &["-sS", "-Kextra.curlrc"],
        &["-sS", "-sSK", "extra.curlrc"],
        &["-sS", "--config", "extra.curlrc"],
        &["-sS", "--config=extra.curlrc"],
        &["-sS", "--config=-"],
        &["-sS", "--config", "-"],
        &["-sS", "--connect-to", "127.0.0.1:18080:example.com:80"],
        &["-sS", "--resolve", "localhost:18080:93.184.216.34"],
        &["-sS", "--doh-url", "https://dns.example/dns-query"],
        &["-sS", "--next"],
        &["-sS", "-:"],
    ];

    for bad in bad_arg_sets {
        let (launches, cmds) = probe_dry_run(false, || {
            let mut cmd = Command::new("curl");
            cmd.args(*bad);
            assert!(
                output_with_url_on_stdin(cmd, local_url).is_err(),
                "args {bad:?} should fail"
            );
        });
        assert_eq!(launches, 0, "args {bad:?} must be refused");
        assert!(cmds.is_empty(), "args {bad:?} must record nothing");
    }

    // Legitimate option values containing URLs or paths (like -H Referer or -o file) remain allowed.
    let (ok_launches, _) = probe_dry_run(false, || {
        let mut cmd = Command::new("curl");
        cmd.args([
            "-sS",
            "-H",
            "Referer: https://localhost",
            "-e",
            "http://localhost",
            "-o",
            "out.bin",
        ]);
        let _ = output_with_url_on_stdin(cmd, local_url);
    });
    assert_eq!(ok_launches, 1);
}

#[test]
fn local_curl_guard_args_builds_guarded_command() {
    let (first, last) = local_curl_guard_args();
    assert_eq!(first, ["-q"]);
    assert_eq!(last, ["--noproxy", "*", "--max-redirs", "0"]);

    let urls = [
        "http://127.0.0.1:18080/a.png",
        "http://localhost:18080/b.png",
    ];
    let (launches, _) = probe_dry_run(false, || {
        assert!(allow_launch_urls(urls));
    });
    assert_eq!(launches, 1);

    let mut cmd = Command::new(curl_bin());
    cmd.args(first).args(["-sS", "--max-time", "5"]).args(last);
    let args: Vec<String> = cmd
        .get_args()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    assert_eq!(args.first().map(String::as_str), Some("-q"));
    assert!(args.windows(2).any(|w| w == ["--noproxy", "*"]));
    assert!(args.ends_with(&["--max-redirs".to_owned(), "0".to_owned()]));
}

fn serve_302_n_times(
    listener: TcpListener,
    target_port: u16,
    count: usize,
) -> std::thread::JoinHandle<Vec<String>> {
    std::thread::spawn(move || {
        let mut seen_requests = Vec::with_capacity(count);
        for _ in 0..count {
            let (mut stream, _) = listener.accept().expect("accept on first port");
            let mut req = Vec::new();
            let mut buf = [0u8; 512];
            while !req.windows(4).any(|w| w == b"\r\n\r\n") {
                let n = stream.read(&mut buf).expect("read request");
                if n == 0 {
                    break;
                }
                req.extend_from_slice(&buf[..n]);
            }
            seen_requests.push(String::from_utf8_lossy(&req).into_owned());
            let resp = format!(
                "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:{target_port}/hop\r\nContent-Length: 4\r\nConnection: close\r\n\r\n302\n"
            );
            stream.write_all(resp.as_bytes()).expect("write 302");
            stream.flush().expect("flush 302");
            let _ = stream.shutdown(Shutdown::Write);
        }
        seen_requests
    })
}

#[test]
fn real_curl_offline_local_launch_stops_at_302_even_with_curlrc_location() {
    let first_listener = TcpListener::bind("127.0.0.1:0").expect("bind first");
    let first_port = first_listener.local_addr().expect("first addr").port();
    let second_listener = TcpListener::bind("127.0.0.1:0").expect("bind second");
    let second_port = second_listener.local_addr().expect("second addr").port();
    second_listener
        .set_nonblocking(true)
        .expect("second nonblocking");

    let server_thread = serve_302_n_times(first_listener, second_port, 3);
    let url = format!("http://127.0.0.1:{first_port}/start");

    let base = std::env::temp_dir().join(format!("strata-net0c3-{}", std::process::id()));
    let rc_dir = base.join("rc");
    let work_dir = base.join("work");
    std::fs::create_dir_all(&rc_dir).expect("create rc dir");
    std::fs::create_dir_all(&work_dir).expect("create work dir");
    std::fs::write(rc_dir.join(".curlrc"), b"location\n").expect("write .curlrc");

    // 1. Plain offline local launch: answer is the 302 and second port sees no request.
    let mut plain = Command::new(curl_bin());
    plain.args(["-sS", "-i", "--max-time", "2"]);
    let out1 = output_with_url_on_stdin(plain, &url).expect("plain local curl");
    let text1 = String::from_utf8_lossy(&out1.stdout);
    assert!(out1.status.success(), "stderr: {}", String::from_utf8_lossy(&out1.stderr));
    assert!(text1.starts_with("HTTP/1.1 302"), "expected 302, got: {text1}");
    assert!(
        matches!(second_listener.accept(), Err(ref e) if e.kind() == ErrorKind::WouldBlock),
        "second port must see no request on plain launch"
    );

    // 2. CURL_HOME points at .curlrc (`location`), plus env + working dir preserved on rebuild.
    let mut with_rc = Command::new(curl_bin());
    with_rc
        .args([
            "-sS",
            "-i",
            "-o",
            "saved_302.txt",
            "--variable",
            "%STRATA_ENV_CHECK",
            "--expand-header",
            "X-Strata-Env: {{STRATA_ENV_CHECK}}",
            "--max-time",
            "2",
        ])
        .env("CURL_HOME", &rc_dir)
        .env("STRATA_ENV_CHECK", "kept-across-rebuild")
        .env("http_proxy", format!("http://127.0.0.1:{second_port}"))
        .current_dir(&work_dir);
    let out2 = output_with_url_on_stdin(with_rc, &url).expect("curlrc local curl");
    assert!(out2.status.success(), "stderr: {}", String::from_utf8_lossy(&out2.stderr));
    let saved = std::fs::read_to_string(work_dir.join("saved_302.txt")).expect("cwd saved file");
    assert!(saved.starts_with("HTTP/1.1 302"), "expected 302 with .curlrc, got: {saved}");
    assert!(
        matches!(second_listener.accept(), Err(ref e) if e.kind() == ErrorKind::WouldBlock),
        "second port must see no request even with .curlrc location and http_proxy"
    );

    // 3. Direct caller command built with local_curl_guard_args() + CURL_HOME .curlrc `location`.
    let (first_args, last_args) = local_curl_guard_args();
    assert!(allow_launch_urls([&url]));
    let mut helper_cmd = Command::new(curl_bin());
    helper_cmd
        .args(first_args)
        .args(["-sS", "-i", "--max-time", "2", &url])
        .args(last_args)
        .env("CURL_HOME", &rc_dir)
        .env("http_proxy", format!("http://127.0.0.1:{second_port}"));
    let out3 = helper_cmd.output().expect("helper local curl");
    let text3 = String::from_utf8_lossy(&out3.stdout);
    assert!(out3.status.success(), "stderr: {}", String::from_utf8_lossy(&out3.stderr));
    assert!(text3.starts_with("HTTP/1.1 302"), "expected 302 with helper, got: {text3}");
    assert!(
        matches!(second_listener.accept(), Err(ref e) if e.kind() == ErrorKind::WouldBlock),
        "second port must see no request when built with local_curl_guard_args"
    );

    let seen = server_thread.join().expect("server thread");
    assert!(
        seen[1].contains("X-Strata-Env: kept-across-rebuild"),
        "rebuilt command must preserve caller env: {}",
        seen[1]
    );
    let _ = std::fs::remove_dir_all(&base);
}
