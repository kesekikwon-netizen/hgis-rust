//! 키가 든 주소가 curl 명령줄에 나오지 않는다(SEC1) — 가짜 키만 쓴다.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::process::Command;

use super::*;

const FAKE_KEY: &str = "FAKE-TEST-KEY-0000";

fn curl() -> Command {
    Command::new(if cfg!(target_os = "windows") { "curl.exe" } else { "curl" })
}

/// 명령줄에는 「--config -」만 붙고 주소·「key=」·키 값은 없다. 주소는 표준입력 설정 글에만.
#[test]
fn the_url_never_reaches_the_command_line() {
    let url = format!("https://api.vworld.kr/req/wmts/1.0.0/{FAKE_KEY}/Satellite/18/1/2.jpeg?key={FAKE_KEY}&domain=x");
    let mut base = curl();
    base.args(["-sS", "-L", "--max-time", "12", "-H", "Referer: https://localhost"]);
    let (cmd, config) = with_url_on_stdin(base, &url);
    let args: Vec<String> = cmd.get_args().map(|a| a.to_string_lossy().into_owned()).collect();
    assert!(args.windows(2).any(|w| w[0] == "--config" && w[1] == "-"), "{args:?}");
    assert!(args.iter().all(|a| !a.contains("key=") && !a.contains(FAKE_KEY)), "{args:?}");
    assert_eq!(config, format!("url = \"{url}\"\n"));
}

/// 설정 글의 따옴표 안에서 \ 와 " 는 \ 로 막는다(curl config 문법).
#[test]
fn quotes_and_backslashes_are_escaped() {
    assert_eq!(curl_url_config(r#"https://h/a"b\c"#), "url = \"https://h/a\\\"b\\\\c\"\n");
}

/// 실제 curl 이 표준입력의 주소로 받는다 — 내려받기 없이 file:// 로(표준출력·-o 파일 둘 다).
#[test]
fn curl_reads_the_url_from_stdin() {
    let dir = std::env::temp_dir().join(format!("strata-sec1-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("dir");
    let src = dir.join("tile.txt");
    std::fs::write(&src, b"tile bytes").expect("write");
    let url = format!("file:///{}", src.to_string_lossy().replace('\\', "/"));
    let mut to_stdout = curl();
    to_stdout.args(["-sS"]);
    let out = output_with_url_on_stdin(to_stdout, &url).expect("curl runs");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(out.stdout, b"tile bytes");
    let dest = dir.join("copy.txt");
    let mut to_file = curl();
    to_file.args(["-sS", "-o"]).arg(&dest);
    let out = output_with_url_on_stdin(to_file, &url).expect("curl runs");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(std::fs::read(&dest).expect("copied"), b"tile bytes");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn local_target_launch_offline_adds_noproxy_refuses_redirects_and_blocks_remote() {
    use crate::network::{allowed_launches, probe_dry_run};

    // Switch off + local target without redirect flags: allowed and carries --noproxy *.
    let before = allowed_launches();
    let local_url = "http://127.0.0.1:18080/tile.png";
    let (launches, cmds) = probe_dry_run(false, || {
        let mut cmd = Command::new("curl");
        cmd.args(["-sS"]);
        let _ = output_with_url_on_stdin(cmd, local_url);
    });
    assert_eq!(launches, 1);
    assert!(allowed_launches() > before);
    assert!(
        cmds[0].args.windows(2).any(|w| w == ["--noproxy", "*"]),
        "offline local launch must carry --noproxy *: {:?}",
        cmds[0].args
    );
    assert_eq!(cmds[0].stdin, curl_url_config(local_url).into_bytes());

    // Switch off + local target with -L or --location: refused, starts nothing.
    let count_before_refusal = allowed_launches();
    for redirect_flag in ["-L", "--location", "-sSL"] {
        let (r_launches, r_cmds) = probe_dry_run(false, || {
            let mut cmd = Command::new("curl");
            cmd.args(["-sS", redirect_flag]);
            assert!(output_with_url_on_stdin(cmd, local_url).is_err());
        });
        assert_eq!(r_launches, 0, "redirect flag {redirect_flag} must be refused");
        assert!(r_cmds.is_empty());
    }
    assert_eq!(allowed_launches(), count_before_refusal);

    // Switch off + non-local target: starts nothing.
    let (remote_launches, remote_cmds) = probe_dry_run(false, || {
        let mut cmd = Command::new("curl");
        cmd.args(["-sS"]);
        assert!(output_with_url_on_stdin(cmd, "https://api.vworld.kr/req/wmts/1").is_err());
    });
    assert_eq!(remote_launches, 0);
    assert!(remote_cmds.is_empty());

    // Switch on + local target (even with -L): unchanged, no --noproxy added, nothing refused.
    let (on_launches, on_cmds) = probe_dry_run(true, || {
        let mut cmd = Command::new("curl");
        cmd.args(["-sS", "-L"]);
        let _ = output_with_url_on_stdin(cmd, local_url);
    });
    assert_eq!(on_launches, 1);
    assert_eq!(on_cmds[0].args, vec!["-sS", "-L", "--config", "-"]);
}
