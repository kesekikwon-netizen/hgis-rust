//! Tests for the process-wide network switch and every `strata-net` launch site (`NET0c`).

use std::path::Path;
use std::process::Command;

use crate::address_search::http_get_bytes;
use crate::curl_stdin::{curl_url_config, output_with_url_on_stdin};
use crate::dem_copernicus::{self, DemProgress, DemTile};
use crate::heritage_intranet::{self, HeritageIntranetFlow};
use crate::internet_check::{vworld_reachable, CHECK_URL};
use crate::network::{
    allow_launch_urls, allowed_launches, is_allowed, is_local_target, offline_from_env,
    probe_dry_run, startup_allows_network, OFFLINE_ENV,
};
use crate::ngii_topographic::{self, NgiiTopographicClient};

fn expected_curl_bin() -> &'static str {
    if cfg!(target_os = "windows") {
        "curl.exe"
    } else {
        "curl"
    }
}

#[test]
fn cargo_config_sets_strata_offline_in_test_process() {
    assert_eq!(std::env::var(OFFLINE_ENV).as_deref(), Ok("1"),
        "STRATA_OFFLINE must be \"1\" in tests (.cargo/config.toml); unset STRATA_OFFLINE in your shell or set STRATA_OFFLINE=1");
    assert!(offline_from_env(Some("1")));
    assert!(!offline_from_env(None));
    assert!(!offline_from_env(Some("0")));
    assert!(!is_allowed());
}

#[test]
fn startup_args_and_env_control_the_switch() {
    assert!(startup_allows_network(&["ka-hgis.exe"], None));
    assert!(!startup_allows_network(&["ka-hgis.exe", "--snap", "out"], None));
    assert!(!startup_allows_network(&["ka-hgis.exe", "--verify-all"], None));
    assert!(!startup_allows_network(&["ka-hgis.exe", "--smoke-quit"], None));
    assert!(!startup_allows_network(&["ka-hgis.exe", "--smoke"], None));

    // --snap-online keeps network on for one --snap run only, and nothing else may use it.
    assert!(startup_allows_network(
        &["ka-hgis.exe", "--snap", "out", "--snap-online"],
        None
    ));
    assert!(!startup_allows_network(
        &["ka-hgis.exe", "--verify-all", "--snap-online"],
        None
    ));
    assert!(!startup_allows_network(
        &["ka-hgis.exe", "--smoke-quit", "--snap-online"],
        None
    ));
    assert!(!startup_allows_network(
        &["ka-hgis.exe", "--smoke", "--snap-online"],
        None
    ));

    // STRATA_OFFLINE=1 turns the switch off, and disable() turns it off for the process.
    assert!(!startup_allows_network(&["ka-hgis.exe"], Some("1")));
    assert!(!startup_allows_network(
        &["ka-hgis.exe", "--snap", "out", "--snap-online"],
        Some("1")
    ));
    crate::network::disable();
    assert!(!is_allowed());
}

#[test]
fn curl_stdin_helper_respects_switch_and_preserves_command() {
    let url = "https://api.vworld.kr/req/wmts/1.0.0/FAKE-KEY/Satellite/15/1/2.jpeg";
    let (off_launches, off_cmds) = probe_dry_run(false, || {
        let mut cmd = Command::new("curl");
        cmd.args(["-sS"]);
        assert!(output_with_url_on_stdin(cmd, url).is_err());
    });
    assert_eq!(off_launches, 0);
    assert!(off_cmds.is_empty());

    let before = allowed_launches();
    let (on_launches, on_cmds) = probe_dry_run(true, || {
        let mut cmd = Command::new("curl");
        cmd.args(["-sS"]);
        let _ = output_with_url_on_stdin(cmd, url);
    });
    assert_eq!(on_launches, 1);
    assert!(allowed_launches() > before);
    assert_eq!(on_cmds[0].program, "curl");
    assert_eq!(on_cmds[0].args, vec!["-sS", "--config", "-"]);
    assert_eq!(on_cmds[0].stdin, curl_url_config(url).into_bytes());
}

#[test]
fn address_geocode_launch_site_respects_switch_and_preserves_command() {
    let url = "https://api.vworld.kr/req/search?service=search&key=FAKE-KEY-0000";
    let (off_launches, off_cmds) = probe_dry_run(false, || {
        assert_eq!(http_get_bytes(url), Err("network_error".to_owned()));
    });
    assert_eq!(off_launches, 0);
    assert!(off_cmds.is_empty());

    let before = allowed_launches();
    let (on_launches, on_cmds) = probe_dry_run(true, || {
        let _ = http_get_bytes(url);
    });
    assert_eq!(on_launches, 1);
    assert!(allowed_launches() > before);
    assert_eq!(on_cmds[0].program, expected_curl_bin());
    assert_eq!(
        on_cmds[0].args,
        vec![
            "-sS", "--max-time", "15",
            "-A", "ka-hgis/0.3 (Korean archaeology HGIS; contact: local)",
            "-H", "Accept: application/json", "--config", "-",
        ]
    );
    assert_eq!(on_cmds[0].stdin, curl_url_config(url).into_bytes());
}

#[test]
fn dem_fetch_launch_site_respects_switch_and_preserves_command() {
    let tile = DemTile { lat: 35, lon: 129 };
    let dest = std::env::temp_dir().join(format!("strata-net0c-{}-N35E129.tif", std::process::id()));
    let progress = DemProgress::none();

    let (off_launches, off_cmds) = probe_dry_run(false, || {
        let res = dem_copernicus::fetch_tile(tile, &dest, &progress);
        assert!(matches!(res, dem_copernicus::TileFetch::Failed));
    });
    assert_eq!(off_launches, 0);
    assert!(off_cmds.is_empty());

    let before = allowed_launches();
    let (on_launches, on_cmds) = probe_dry_run(true, || {
        let res = dem_copernicus::fetch_tile(tile, &dest, &progress);
        assert!(matches!(res, dem_copernicus::TileFetch::Failed));
    });
    assert_eq!(on_launches, 1);
    assert!(allowed_launches() > before);
    assert_eq!(on_cmds[0].program, expected_curl_bin());
    assert_eq!(
        on_cmds[0].args,
        vec![
            "-sS", "--connect-timeout", "5", "--speed-time", "15", "--speed-limit", "1", "-f",
            "-A", "ka-hgis/0.8 (Korean archaeology HGIS; contact: local)",
            "-o", &dest.to_string_lossy(), "-w", "%{http_code}", &tile.url(),
        ]
    );
}

#[test]
fn heritage_http_launch_site_respects_switch_and_preserves_command() {
    let url = HeritageIntranetFlow::download_list_url();
    let body = "pageIndex=1";
    let jar = Path::new("fake_cookies.txt");

    let (off_launches, off_cmds) = probe_dry_run(false, || {
        let err = heritage_intranet::run_curl(&url, Some(body), Some(jar)).expect_err("offline");
        assert_eq!(err, "연결 실패(상태 0)");
    });
    assert_eq!(off_launches, 0);
    assert!(off_cmds.is_empty());

    let before = allowed_launches();
    let (on_launches, on_cmds) = probe_dry_run(true, || {
        let _ = heritage_intranet::run_curl(&url, Some(body), Some(jar));
    });
    assert_eq!(on_launches, 1);
    assert!(allowed_launches() > before);
    assert_eq!(on_cmds[0].program, expected_curl_bin());
    let mut expected_args = vec![
        "-sS".to_owned(),
        "--max-time".to_owned(),
        HeritageIntranetFlow::TIMEOUT_SECS.to_string(),
        "-X".to_owned(),
        "POST".to_owned(),
        "-A".to_owned(),
        HeritageIntranetFlow::USER_AGENT.to_owned(),
        "-w".to_owned(),
        "\n__KA_HTTP_STATUS__:%{http_code}".to_owned(),
    ];
    for (name, value) in HeritageIntranetFlow::request_headers() {
        expected_args.push("-H".to_owned());
        expected_args.push(format!("{name}: {value}"));
    }
    expected_args.extend([
        "-b".to_owned(),
        "fake_cookies.txt".to_owned(),
        "-c".to_owned(),
        "fake_cookies.txt".to_owned(),
        "--data-binary".to_owned(),
        body.to_owned(),
        url,
    ]);
    assert_eq!(on_cmds[0].args, expected_args);
}

#[test]
fn internet_check_launch_site_respects_switch_and_preserves_command() {
    let (off_launches, off_cmds) = probe_dry_run(false, || {
        assert!(!vworld_reachable());
    });
    assert_eq!(off_launches, 0);
    assert!(off_cmds.is_empty());

    let before = allowed_launches();
    let (on_launches, on_cmds) = probe_dry_run(true, || {
        assert!(!vworld_reachable());
    });
    assert_eq!(on_launches, 1);
    assert!(allowed_launches() > before);
    let null_sink = if cfg!(target_os = "windows") {
        "NUL"
    } else {
        "/dev/null"
    };
    assert_eq!(on_cmds[0].program, expected_curl_bin());
    assert_eq!(
        on_cmds[0].args,
        vec!["-s", "-I", "-o", null_sink, "-w", "%{http_code}", "--max-time", "6", CHECK_URL]
    );
}

#[test]
fn ngii_fetch_launch_site_respects_switch_and_preserves_command() {
    let url = NgiiTopographicClient::combine_buffer_query_url();
    let body = NgiiTopographicClient::combine_buffer_body(1_150_000.0, 1_750_000.0, 5_000.0);

    let (off_launches, off_cmds) = probe_dry_run(false, || {
        let err = ngii_topographic::http_post_form(&url, &body).expect_err("offline");
        assert_eq!(err, NgiiTopographicClient::portal_timeout_message());
    });
    assert_eq!(off_launches, 0);
    assert!(off_cmds.is_empty());

    let before = allowed_launches();
    let (on_launches, on_cmds) = probe_dry_run(true, || {
        let _ = ngii_topographic::http_post_form(&url, &body);
    });
    assert_eq!(on_launches, 1);
    assert!(allowed_launches() > before);
    assert_eq!(on_cmds[0].program, expected_curl_bin());
    assert_eq!(
        on_cmds[0].args,
        vec![
            "-sS", "--max-time", "30",
            "-A", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ka-hgis/0.3",
            "-H", "Content-Type: application/x-www-form-urlencoded; charset=UTF-8",
            "-H", "Referer: https://map.ngii.go.kr/ms/map/NlipMap.do",
            "-X", "POST", "--data-binary", &body, &url,
        ]
    );
}

#[test]
fn is_local_target_accepts_only_file_and_loopback_http() {
    for ok in [
        "file:///C:/Users/test/tile.txt", "file:///tmp/tile.txt", "FILE://LOCALHOST/tmp/tile.txt",
        "http://127.0.0.1", "http://127.0.0.1/", "http://127.0.0.1:8080/tile?x=1",
        "http://[::1]", "http://[::1]:9000/api", "http://localhost", "http://localhost:3000/a",
        "HTTP://LOCALHOST:8080/x", "HTTP://127.0.0.1/x", "HTTP://[::1]:80/x",
    ] {
        assert!(is_local_target(ok), "expected local: {ok}");
    }
    for bad in [
        "", "   ", "not a url ::: garbage", "://127.0.0.1", "http://", "file://", "file:///",
        "https://127.0.0.1", "https://localhost:8443/x", "https://[::1]/x",
        "http://127.0.0.1.example.com", "http://localhost.example.com",
        "http://user@127.0.0.1@example.com", "http://user@127.0.0.1", "http://user@localhost/x",
        "http://example.com/127.0.0.1", "http://127.0.0.2/x", "http://0.0.0.0/x",
        "http://127.0.0.1:", "http://127.0.0.1:0/x", "http://127.0.0.1:70000/x",
        "http://127.0.0.1\\@example.com", "file://evil.example.com/share/a", "file:////srv/a",
    ] {
        assert!(!is_local_target(bad), "expected non-local: {bad}");
    }
}

#[test]
fn allow_launch_urls_allows_all_local_offline_and_remote_only_online() {
    let all_local = ["file:///C:/tmp/a.tif", "http://127.0.0.1:8000/b", "http://[::1]/c", "http://localhost/d"];
    let mixed = ["http://127.0.0.1:8000/a", "https://api.vworld.kr/req/wmts/1.0.0/FAKE/1/2.jpeg"];
    let before = allowed_launches();
    let (local_launches, _) = probe_dry_run(false, || assert!(allow_launch_urls(all_local)));
    assert_eq!(local_launches, 1);
    assert!(allowed_launches() > before);

    let after_local = allowed_launches();
    let (mixed_off, _) = probe_dry_run(false, || {
        assert!(!allow_launch_urls(mixed));
        assert!(!allow_launch_urls(std::iter::empty::<&str>()));
    });
    assert_eq!(mixed_off, 0);
    assert_eq!(allowed_launches(), after_local);

    let (mixed_on, _) = probe_dry_run(true, || assert!(allow_launch_urls(mixed)));
    assert_eq!(mixed_on, 1);
    assert!(allowed_launches() > after_local);
}
