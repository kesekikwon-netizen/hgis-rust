//! The request as curl gets it, without sending anything outside this PC.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::network::probe_dry_run;

const AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ka-hgis/0.3";

/// The limits of a large transfer and the name the C++ gives itself reach curl; the address does not reach
/// its command line.
#[test]
fn the_request_carries_the_limits_and_keeps_the_address_off_the_command_line() {
    let url = "https://data.kigam.re.kr/geoserver/ows?service=WFS&request=GetFeature&bbox=35.7,129.1,35.9,129.3";
    let (launches, commands) = probe_dry_run(true, || {
        let _ = reference_get(url, AGENT, 15, 180, &|| false);
    });
    assert_eq!(launches, 1);
    let command = &commands[0];
    assert!(command.program.starts_with("curl"));
    let has = |pair: [&str; 2]| command.args.windows(2).any(|w| w[0] == pair[0] && w[1] == pair[1]);
    assert!(has(["--max-time", "180"]) && has(["--speed-time", "15"]) && has(["--speed-limit", "1"]) && has(["--connect-timeout", "15"]), "{:?}", command.args);
    assert!(has(["-A", AGENT]), "{:?}", command.args);
    assert!(command.args.iter().any(|a| a == "--fail"));
    assert!(command.args.iter().all(|a| !a.contains("kigam") && !a.contains("bbox")), "{:?}", command.args);
    assert_eq!(String::from_utf8_lossy(&command.stdin), format!("url = \"{url}\"\n"));
}

/// With the network switched off nothing is launched for an outside address, and the caller is told so.
#[test]
fn nothing_is_sent_while_the_network_is_off() {
    let (launches, commands) = probe_dry_run(false, || {
        assert_eq!(reference_get("https://data.kigam.re.kr/geoserver/ows?x=1", AGENT, 15, 180, &|| false), Err(ReferenceFailure::Network));
    });
    assert_eq!(launches, 0);
    assert!(commands.is_empty());
}

/// A time limit that ran out is told apart from every other failure (curl exit code 28).
#[test]
fn a_timeout_is_told_apart() {
    assert_eq!(failure_of(Some(28)), ReferenceFailure::Timeout);
    for other in [Some(6), Some(7), Some(22), Some(35), Some(56), Some(1), None] {
        assert_eq!(failure_of(other), ReferenceFailure::Network, "{other:?}");
    }
}

/// Real curl, a file on this PC: the body comes back as it is; a file that is not there is a failure.
#[test]
fn the_body_of_a_successful_answer_is_returned() {
    let dir = tempfile::tempdir().expect("tempdir");
    let source = dir.path().join("answer.json");
    std::fs::write(&source, br#"{"type":"FeatureCollection","features":[]}"#).expect("write");
    let url = format!("file:///{}", source.to_string_lossy().replace('\\', "/"));
    assert_eq!(reference_get(&url, AGENT, 15, 180, &|| false).expect("body"), br#"{"type":"FeatureCollection","features":[]}"#);
    let missing = format!("file:///{}", dir.path().join("none.json").to_string_lossy().replace('\\', "/"));
    assert_eq!(reference_get(&missing, AGENT, 15, 180, &|| false), Err(ReferenceFailure::Network));
}

/// A request that is given up while curl waits for the answer ends at once. A listener on this PC takes
/// the connection and says nothing for eight seconds.
#[test]
fn a_request_that_is_given_up_ends_at_once() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let url = format!("http://127.0.0.1:{}/slow", listener.local_addr().expect("address").port());
    let silent = std::thread::spawn(move || {
        if let Ok((stream, _)) = listener.accept() {
            std::thread::sleep(std::time::Duration::from_secs(8));
            drop(stream);
        }
    });
    let started = std::time::Instant::now();
    let give_up = || started.elapsed() > std::time::Duration::from_millis(400);
    assert_eq!(reference_get(&url, AGENT, 15, 180, &give_up), Err(ReferenceFailure::GivenUp));
    let took = started.elapsed();
    assert!(took >= std::time::Duration::from_millis(400) && took < std::time::Duration::from_secs(4), "{took:?}");
    drop(silent);
}

/// A service that checks the key against the address it is registered for gets that address as the
/// `Referer`; the key, which is part of the address asked for, stays off the command line.
#[test]
fn a_referer_reaches_curl_and_the_key_stays_off_the_command_line() {
    let url = "https://api.vworld.kr/req/wfs?SERVICE=WFS&REQUEST=GetFeature&KEY=TEST_KEY";
    let (launches, commands) = probe_dry_run(true, || {
        let _ = reference_get_from(url, "ka-hgis/0.3", Some("https://localhost"), 15, 180, &|| false);
        let _ = reference_get_from(url, "ka-hgis/0.3", None, 15, 180, &|| false);
    });
    assert_eq!(launches, 2);
    let has = |n: usize, pair: [&str; 2]| commands[n].args.windows(2).any(|w| w[0] == pair[0] && w[1] == pair[1]);
    assert!(has(0, ["-e", "https://localhost"]) && has(0, ["-A", "ka-hgis/0.3"]), "{:?}", commands[0].args);
    assert!(commands[1].args.iter().all(|a| a != "-e" && !a.contains("localhost")), "{:?}", commands[1].args);
    for command in &commands {
        assert!(command.args.iter().all(|a| !a.contains("TEST_KEY") && !a.contains("vworld")), "{:?}", command.args);
        assert_eq!(String::from_utf8_lossy(&command.stdin), format!("url = \"{url}\"\n"));
    }
}
