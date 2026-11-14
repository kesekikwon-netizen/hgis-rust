//! Regression tests for `NET0c4`: offline local stdin verification, `f3` curl 8 option refusals,
//! and real `curl` loopback proxy isolation.

use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, TcpListener};
use std::process::Command;

use crate::curl_stdin::{curl_url_config, output_with_url_on_stdin};
use crate::network::{allow_launch_urls, is_local_target, probe_dry_run, run_curl_url_with_stdin};

fn curl_bin() -> &'static str {
    if cfg!(target_os = "windows") {
        "curl.exe"
    } else {
        "curl"
    }
}

#[test]
fn offline_local_launch_refuses_crafted_stdin_configs() {
    let local_url = "http://127.0.0.1:18080/tile.png";
    let valid_cfg = curl_url_config(local_url);

    let crafted_stdins: Vec<(&str, Vec<u8>)> = vec![
        (
            "extra proxy line",
            format!("proxy = \"http://127.0.0.1:19999\"\n{valid_cfg}").into_bytes(),
        ),
        (
            "second url line",
            format!("{valid_cfg}url = \"https://example.com/off-pc\"\n").into_bytes(),
        ),
        (
            "next stanza bypassing trailing noproxy",
            format!("proxy = \"http://127.0.0.1:19999\"\n{valid_cfg}next\n{valid_cfg}")
                .into_bytes(),
        ),
        ("extra location line", format!("location\n{valid_cfg}").into_bytes()),
        (
            "mismatched local url",
            curl_url_config("http://127.0.0.1:18080/other.png").into_bytes(),
        ),
        ("empty stdin", Vec::new()),
        ("trailing comment", format!("{valid_cfg}# extra\n").into_bytes()),
    ];

    for (label, stdin_bytes) in &crafted_stdins {
        let (launches, cmds) = probe_dry_run(false, || {
            let mut cmd = Command::new("curl");
            cmd.args(["-sS", "--config", "-"]);
            let err = run_curl_url_with_stdin(cmd, local_url, stdin_bytes)
                .expect_err("crafted stdin must fail");
            assert_eq!(
                err.kind(),
                ErrorKind::PermissionDenied,
                "crafted stdin ({label}) must fail with PermissionDenied, got {err:?}"
            );
        });
        assert_eq!(launches, 0, "crafted stdin ({label}) must not launch");
        assert!(cmds.is_empty(), "crafted stdin ({label}) must record nothing");
    }

    // Even with matching stdin bytes, omitting `--config -` on an offline local launch is refused.
    let (missing_cfg_launches, missing_cfg_cmds) = probe_dry_run(false, || {
        let mut cmd = Command::new("curl");
        cmd.args(["-sS"]);
        let err = run_curl_url_with_stdin(cmd, local_url, valid_cfg.as_bytes())
            .expect_err("missing --config - must fail");
        assert_eq!(err.kind(), ErrorKind::PermissionDenied);
    });
    assert_eq!(missing_cfg_launches, 0);
    assert!(missing_cfg_cmds.is_empty());

    // Exact config via output_with_url_on_stdin still succeeds offline, and switch-on stays unchanged.
    let (ok_launches, _) = probe_dry_run(false, || {
        let mut cmd = Command::new("curl");
        cmd.args(["-sS"]);
        let _ = output_with_url_on_stdin(cmd, local_url);
    });
    assert_eq!(ok_launches, 1);

    let (on_launches, on_cmds) = probe_dry_run(true, || {
        let mut cmd = Command::new("curl");
        cmd.args(["-sS", "--config", "-"]);
        let _ = run_curl_url_with_stdin(cmd, local_url, b"custom-when-switch-on\n");
    });
    assert_eq!(on_launches, 1);
    assert_eq!(on_cmds[0].args, vec!["-sS", "--config", "-"]);
    assert_eq!(on_cmds[0].stdin, b"custom-when-switch-on\n");
}

#[test]
fn offline_local_launch_refuses_f3_and_curl8_routing_options() {
    let local_url = "http://127.0.0.1:18080/tile.png";

    let refused_arg_sets: &[&[&str]] = &[
        // All 8 options named in f3 (space-separated, inline `=`, and `--expand-` forms):
        &["-sS", "--alt-svc", "altsvc.cache"],
        &["-sS", "--alt-svc=altsvc.cache"],
        &["-sS", "--expand-alt-svc", "altsvc.cache"],
        &["-sS", "--hsts", "hsts.cache"],
        &["-sS", "--hsts=hsts.cache"],
        &["-sS", "--resolve", "127.0.0.1:18080:93.184.216.34"],
        &["-sS", "--resolve=127.0.0.1:18080:93.184.216.34"],
        &["-sS", "--connect-to", "127.0.0.1:18080:example.com:80"],
        &["-sS", "--connect-to=127.0.0.1:18080:example.com:80"],
        &["-sS", "--dns-servers", "1.1.1.1"],
        &["-sS", "--dns-servers=1.1.1.1"],
        &["-sS", "--interface", "eth0"],
        &["-sS", "--interface=eth0"],
        &["-sS", "--unix-socket", "local.sock"],
        &["-sS", "--unix-socket=local.sock"],
        &["-sS", "--doh-url", "https://dns.example/dns-query"],
        &["-sS", "--doh-url=https://dns.example/dns-query"],
        // Additional curl 8 options that route/tunnel elsewhere, mutate the target, or read mirror/gateway files:
        &["-sS", "--abstract-unix-socket", "abstract.sock"],
        &["-sS", "--dns-interface", "eth0"],
        &["-sS", "--dns-ipv4-addr", "1.1.1.1"],
        &["-sS", "--dns-ipv6-addr", "::1"],
        &["-sS", "--doh-insecure"],
        &["-sS", "--doh-cert-status"],
        &["-sS", "--ech", "true"],
        &["-sS", "--ipfs-gateway", "https://ipfs.example"],
        &["-sS", "--metalink"],
        &["-sS", "--proxy", "http://127.0.0.1:19999"],
        &["-sS", "--proxy=http://127.0.0.1:19999"],
        &["-sS", "-x", "http://127.0.0.1:19999"],
        &["-sS", "-xhttp://127.0.0.1:19999"],
        &["-sSx", "http://127.0.0.1:19999"],
        &["-sS", "--proxy1.0", "127.0.0.1:19999"],
        &["-sS", "--preproxy", "socks5://127.0.0.1:19999"],
        &["-sS", "--proxytunnel"],
        &["-sS", "-p"],
        &["-sS", "--proxy-user", "u:p"],
        &["-sS", "-U", "u:p"],
        &["-sS", "-Uu:p"],
        &["-sS", "--socks4", "127.0.0.1:19999"],
        &["-sS", "--socks4a", "127.0.0.1:19999"],
        &["-sS", "--socks5", "127.0.0.1:19999"],
        &["-sS", "--socks5-hostname", "127.0.0.1:19999"],
        &["-sS", "--noproxy", "localhost"],
        &["-sS", "--ftp-port", "-"],
        &["-sS", "-P", "-"],
        &["-sS", "--proto", "=http"],
        &["-sS", "--proto-default", "https"],
        &["-sS", "--proto-redir", "=http"],
        &["-sS", "--request-target", "http://example.com/"],
        &["-sS", "--url-query", "@extra.query"],
        &["-sS", "--max-redirs", "5"],
    ];

    for bad in refused_arg_sets {
        let (launches, cmds) = probe_dry_run(false, || {
            let mut cmd = Command::new("curl");
            cmd.args(*bad);
            let err = output_with_url_on_stdin(cmd, local_url)
                .expect_err(&format!("args {bad:?} should be refused"));
            assert_eq!(
                err.kind(),
                ErrorKind::PermissionDenied,
                "args {bad:?} must fail with PermissionDenied"
            );
        });
        assert_eq!(launches, 0, "args {bad:?} must be refused before launch");
        assert!(cmds.is_empty(), "args {bad:?} must record nothing");
    }
}

#[test]
fn real_curl_offline_local_launch_proxy_listener_sees_nothing() {
    let origin_listener = TcpListener::bind("127.0.0.1:0").expect("bind origin");
    let origin_port = origin_listener.local_addr().expect("origin addr").port();
    let proxy_listener = TcpListener::bind("127.0.0.1:0").expect("bind proxy");
    let proxy_port = proxy_listener.local_addr().expect("proxy addr").port();
    proxy_listener
        .set_nonblocking(true)
        .expect("proxy nonblocking");

    let origin_thread = std::thread::spawn(move || {
        let (mut stream, _) = origin_listener.accept().expect("accept on origin");
        let mut req = Vec::new();
        let mut buf = [0u8; 512];
        while !req.windows(4).any(|w| w == b"\r\n\r\n") {
            let n = stream.read(&mut buf).expect("read origin req");
            if n == 0 {
                break;
            }
            req.extend_from_slice(&buf[..n]);
        }
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
            .expect("write 200");
        stream.flush().expect("flush 200");
        let _ = stream.shutdown(Shutdown::Write);
    });

    let url = format!("http://127.0.0.1:{origin_port}/tile");
    let base = std::env::temp_dir().join(format!("strata-net0c4-{}", std::process::id()));
    std::fs::create_dir_all(&base).expect("create temp dir");
    std::fs::write(
        base.join(".curlrc"),
        format!("proxy = \"http://127.0.0.1:{proxy_port}\"\n"),
    )
    .expect("write .curlrc");

    // 1. Crafted stdin trying `proxy = ...` + `next` so trailing `--noproxy *` only covers the last transfer:
    //    must be refused before starting curl, and proxy_listener must see nothing.
    let crafted_stdin = format!(
        "proxy = \"http://127.0.0.1:{proxy_port}\"\n{}next\n{}",
        curl_url_config(&url),
        curl_url_config(&url)
    );
    let mut sneaky = Command::new(curl_bin());
    sneaky.args(["-sS", "--max-time", "1", "--config", "-"]);
    let sneaky_res = run_curl_url_with_stdin(sneaky, &url, crafted_stdin.as_bytes());
    let proxy_after_sneaky = proxy_listener.accept();
    assert!(
        sneaky_res.is_err(),
        "crafted stdin with proxy + next must be refused before curl runs"
    );
    assert!(
        matches!(proxy_after_sneaky, Err(ref e) if e.kind() == ErrorKind::WouldBlock),
        "proxy listener must see nothing on crafted stdin, got {proxy_after_sneaky:?}"
    );

    // 2. Legitimate offline local launch with `.curlrc` proxy and `http_proxy`/`ALL_PROXY` env vars
    //    pointing at proxy_listener: origin serves 200 OK and proxy_listener still sees nothing.
    let mut legit = Command::new(curl_bin());
    legit
        .args(["-sS", "--max-time", "2"])
        .env("CURL_HOME", &base)
        .env("http_proxy", format!("http://127.0.0.1:{proxy_port}"))
        .env("ALL_PROXY", format!("http://127.0.0.1:{proxy_port}"))
        .env("NO_PROXY", "");
    let out = output_with_url_on_stdin(legit, &url).expect("legit local curl");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(out.stdout, b"ok");
    let proxy_after_legit = proxy_listener.accept();
    assert!(
        matches!(proxy_after_legit, Err(ref e) if e.kind() == ErrorKind::WouldBlock),
        "proxy listener must see nothing on legitimate local curl, got {proxy_after_legit:?}"
    );

    origin_thread.join().expect("origin thread");
    let _ = std::fs::remove_dir_all(&base);
}

/// Review finding on `NET0c4`: curl decodes a percent-encoded separator in a `file:` path, so
/// `file:///%5Chost/share/f` opens `\\host\share\f`, a network share. Any `%` after `file://` is not local (`NET0c5`).
#[test]
fn file_url_with_a_percent_sign_is_not_local_and_never_launches_offline() {
    let bad = [
        "file:///%5Cevil.example/share/f",
        "file:///%5cevil.example/share/f",
        "file:///%2Fevil.example/share/f",
        "file:///%2fevil.example/share/f",
        "file://localhost/%5Cevil.example/share/f",
        "FILE:///%5C127.0.0.1/c$/Windows/win.ini",
        "file:///C:/a%5C..%5C..%5Cb",
        "file:///C:/tmp/%2e%2e/x",
        "file:///C:/tmp/a%20b.txt",
        "file:///%255Cevil.example/f",
    ];
    for url in bad {
        assert!(!is_local_target(url), "expected non-local: {url}");
        let (launches, recorded) = probe_dry_run(false, || assert!(!allow_launch_urls([url])));
        assert_eq!(launches, 0, "must not launch offline: {url}");
        assert!(recorded.is_empty(), "must record nothing: {url}");
    }
    // Plain local paths stay local, also with letters outside ASCII and with a space.
    for url in ["file:///C:/Users/test/tile.txt", "file:///C:/자료/시험 폴더/a b.txt"] {
        assert!(is_local_target(url), "expected local: {url}");
    }
}
