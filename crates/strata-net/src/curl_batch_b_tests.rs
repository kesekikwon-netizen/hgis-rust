//! Batch downloader behavioural tests covering `MAP2a` requirements (part 2/2).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::time::Duration;

use super::tests_a::*;
use super::*;

/// Writes via `<destination>.part`, creates missing parent dirs, renames only on exit 0 + HTTP 200,
/// and leaves 200 text/XML bodies untouched for the caller to inspect.
#[test]
fn saves_via_part_creates_parents_and_keeps_200_xml_body_for_caller() {
    let tmp = TempDir::new("part");
    let xml = b"<?xml version=\"1.0\"?><ServiceException>InvalidKey</ServiceException>".to_vec();
    let routes = HashMap::from([
        ("/img".to_owned(), Reply::Ok(b"\xff\xd8\xff\xe0tile-bytes".to_vec())),
        ("/xml".to_owned(), Reply::Ok(xml.clone())),
        ("/404".to_owned(), Reply::NotFound(b"missing".to_vec())),
    ]);
    let server = TestServer::start(routes);

    let img_dest = tmp.path().join("nested").join("18").join("1").join("2.jpeg");
    let xml_dest = tmp.path().join("nested").join("xml_reply.jpeg");
    let miss_dest = tmp.path().join("nested").join("missing.jpeg");
    fs::create_dir_all(miss_dest.parent().unwrap()).unwrap();
    fs::write(part_path(&miss_dest), b"stale-part").unwrap();

    let handle = start(
        &[
            BatchItem::new(server.url("/img"), &img_dest),
            BatchItem::new(server.url("/xml"), &xml_dest),
            BatchItem::new(server.url("/404"), &miss_dest),
        ],
        &[],
        Duration::from_secs(5),
        3,
    )
    .expect("start batch");

    let mut count = 0usize;
    while handle.recv_timeout(TEST_TIMEOUT).expect("no timeout").is_some() {
        count += 1;
    }
    assert_eq!(count, 3);
    assert_eq!(fs::read(&img_dest).unwrap(), b"\xff\xd8\xff\xe0tile-bytes");
    assert_eq!(fs::read(&xml_dest).unwrap(), xml);
    assert!(!miss_dest.exists());
    assert!(!has_part_files(tmp.path()));
}

/// Nothing containing a URL or key reaches the command line, Debug/Display/error text, or filenames.
#[test]
fn url_and_key_never_reach_cli_errors_debug_or_filenames() {
    let tmp = TempDir::new("sec1");
    let routes = HashMap::from([(format!("/wmts/{FAKE_KEY}/tile.jpeg"), Reply::Ok(b"secret-ok".to_vec()))]);
    let server = TestServer::start(routes);

    let keyed_url = format!("{}/wmts/{FAKE_KEY}/tile.jpeg?key={FAKE_KEY}&q=%22", server.url(""));
    let dest = tmp.path().join("tiles_utf8_\u{00e9}").join("tile_0.jpeg");
    let item = BatchItem::new(&keyed_url, &dest);

    let cmd = build_curl_command(OsStr::new("curl"));
    let args: Vec<String> = cmd.get_args().map(|a| a.to_string_lossy().into_owned()).collect();
    assert!(args.windows(2).any(|w| w[0] == "--config" && w[1] == "-"), "{args:?}");
    for arg in &args {
        assert!(!arg.contains(FAKE_KEY) && !arg.to_ascii_lowercase().contains("key=") && !arg.contains("http://") && !arg.contains("https://"), "{args:?}");
    }

    let esc_item = BatchItem::new(r#"http://127.0.0.1:1/a"b\c"#, tmp.path().join("e.bin"));
    let esc_cfg = build_batch_config(&[esc_item], &[], Duration::from_secs(5), 1).expect("escaped");
    assert!(esc_cfg.contains("url = \"http://127.0.0.1:1/a\\\"b\\\\c\"\n"), "{esc_cfg}");

    let cfg = build_batch_config(std::slice::from_ref(&item), &[], Duration::from_secs(5), 2).expect("config");
    assert!(cfg.contains(FAKE_KEY));
    assert!(!cfg.contains("%{url"), "write-out must never print url: {cfg}");

    let item_dbg = format!("{item:?}");
    assert!(!item_dbg.contains(FAKE_KEY) && !item_dbg.contains("http://"), "{item_dbg}");

    let bad_err = start_with_program(
        OsStr::new("strata-no-such-curl-0000"),
        std::slice::from_ref(&item),
        &[],
        Duration::from_secs(5),
        2,
    )
    .unwrap_err();
    let err_texts = format!("{bad_err} | {bad_err:?}");
    assert!(!err_texts.contains(FAKE_KEY) && !err_texts.contains("http://"), "{err_texts}");

    let handle = start(std::slice::from_ref(&item), &[], Duration::from_secs(5), 1).expect("start");
    let handle_dbg = format!("{handle:?}");
    assert!(!handle_dbg.contains(FAKE_KEY) && !handle_dbg.contains("http://"), "{handle_dbg}");
    let res = handle.recv_timeout(TEST_TIMEOUT).unwrap().unwrap();
    let res_dbg = format!("{res:?}");
    assert!(!res_dbg.contains(FAKE_KEY) && !res_dbg.contains("http://"), "{res_dbg}");
    assert_eq!(handle.recv_timeout(TEST_TIMEOUT).unwrap(), None);
    assert!(handle.is_done());

    for entry in fs::read_dir(dest.parent().unwrap()).unwrap() {
        let name = entry.unwrap().file_name().to_string_lossy().into_owned();
        assert!(!name.contains(FAKE_KEY) && !name.to_ascii_lowercase().contains("key="), "{name}");
    }
}

/// Files are requested in the exact order given.
#[test]
fn files_are_requested_in_the_order_given() {
    let tmp = TempDir::new("order");
    let mut routes = HashMap::new();
    let mut expected_paths = Vec::new();
    for i in 0..8 {
        let p = format!("/seq/{i}");
        routes.insert(p.clone(), Reply::Ok(format!("b{i}").into_bytes()));
        expected_paths.push(p);
    }
    let server = TestServer::start(routes);
    let items: Vec<BatchItem> = (0..8)
        .map(|i| BatchItem::new(server.url(&format!("/seq/{i}")), tmp.path().join(format!("{i}.bin"))))
        .collect();

    let cfg = build_batch_config(&items, &[], Duration::from_secs(5), 4).expect("parallel config");
    let cfg_urls: Vec<&str> = cfg.lines().filter_map(|l| l.strip_prefix("url = ")).collect();
    assert_eq!(cfg_urls.len(), 8);
    for (i, line) in cfg_urls.iter().enumerate() {
        assert!(line.ends_with(&format!("/seq/{i}\"")), "{line}");
    }

    let handle = start(&items, &[], Duration::from_secs(5), 1).expect("start serial");
    while handle.recv_timeout(TEST_TIMEOUT).expect("no timeout").is_some() {}
    assert!(handle.is_done());
    assert_eq!(server.request_paths(), expected_paths);
}

/// 24 files with 6 parallel transfers reuse HTTP/1.1 keep-alive (far fewer TCP connections than files, <= 6 requests open at once).
#[test]
fn reuses_connections_for_24_files_over_6_parallel() {
    let tmp = TempDir::new("reuse");
    let routes: HashMap<String, Reply> = (0..24)
        .map(|i| (format!("/t/{i}"), Reply::DelayedOk(format!("tile-{i}").into_bytes(), Duration::from_millis(20))))
        .collect();
    let server = TestServer::start(routes);
    let items: Vec<BatchItem> = (0..24)
        .map(|i| BatchItem::new(server.url(&format!("/t/{i}")), tmp.path().join(format!("{i}.tile"))))
        .collect();

    let handle = start(&items, &[], Duration::from_secs(10), 6).expect("start 24 over 6");
    let mut seen = [false; 24];
    let mut count = 0usize;
    while let Some(res) = handle.recv_timeout(TEST_TIMEOUT).expect("no timeout") {
        assert_eq!((res.http_status, res.exit_code, seen[res.index]), (200, 0, false));
        seen[res.index] = true;
        count += 1;
    }
    assert_eq!(count, 24);
    assert!(handle.is_done());

    let conns = server.accepted_connections();
    let peak = server.peak_open_requests();
    println!(
        "MAP2a measured: curl=\"{}\", files=24, accepted_connections={conns}, peak_open_requests={peak}",
        curl_version_line()
    );
    // curl limits transfers at once, not connections over time: GitHub's runner (curl 8.21.0) opened a seventh
    // connection once (run 43572224099). What must hold here: connections are reused and never more than six
    // requests are open at once. A cap on connections per host belongs to order MAP2b.
    assert!((1..=12).contains(&conns), "expected reuse (1..=12 TCP connections for 24 files), got {conns}");
    assert!((1..=6).contains(&peak), "expected 1..=6 open requests at once, got {peak}");
}

/// Results stream one by one while `curl` is still running (server holds the second response until first arrives).
#[test]
fn results_arrive_one_by_one_before_batch_ends() {
    let tmp = TempDir::new("stream");
    let gate = HoldGate::new();
    let routes = HashMap::from([
        ("/fast".to_owned(), Reply::Ok(b"fast-bytes".to_vec())),
        ("/held".to_owned(), Reply::HoldThenOk(gate.clone(), b"held-bytes".to_vec())),
    ]);
    let server = TestServer::start(routes);

    let (fast_dest, held_dest) = (tmp.path().join("fast.bin"), tmp.path().join("held.bin"));
    let handle = start(
        &[BatchItem::new(server.url("/fast"), &fast_dest), BatchItem::new(server.url("/held"), &held_dest)],
        &[],
        Duration::from_secs(10),
        2,
    )
    .expect("start streaming batch");

    let first = handle.recv_timeout(TEST_TIMEOUT).expect("no timeout").expect("first result");
    assert_eq!(first, BatchResult { index: 0, http_status: 200, exit_code: 0 });
    assert_eq!(fs::read(&fast_dest).unwrap(), b"fast-bytes");
    assert!(!held_dest.exists());
    assert!(!handle.is_done());

    gate.release();
    let second = handle.recv_timeout(TEST_TIMEOUT).expect("no timeout").expect("second result");
    assert_eq!(second, BatchResult { index: 1, http_status: 200, exit_code: 0 });
    assert_eq!(fs::read(&held_dest).unwrap(), b"held-bytes");
    assert_eq!(handle.recv_timeout(TEST_TIMEOUT).unwrap(), None);
    assert!(handle.is_done());
}

/// After `cancel`, no new request starts, the `curl` process is gone, and no `.part` file remains.
#[test]
fn after_cancel_no_new_request_starts_curl_is_gone_and_no_part_remains() {
    let tmp = TempDir::new("cancel");
    let gate = HoldGate::new();
    let mut routes = HashMap::from([("/0".to_owned(), Reply::HoldThenOk(gate.clone(), b"body-0".to_vec()))]);
    for i in 1..6 {
        routes.insert(format!("/{i}"), Reply::Ok(format!("body-{i}").into_bytes()));
    }
    let server = TestServer::start(routes);
    let items: Vec<BatchItem> = (0..6)
        .map(|i| BatchItem::new(server.url(&format!("/{i}")), tmp.path().join(format!("{i}.bin"))))
        .collect();

    let handle = start(&items, &[], Duration::from_secs(10), 1).expect("start cancel batch");
    assert!(gate.wait_entered(1, TEST_TIMEOUT), "first request reached server");
    let pid = handle.pid().expect("spawned curl pid");
    assert!(os_process_exists(pid), "OS must report curl PID {pid} alive while held");
    let before_cancel = server.request_paths().len();
    assert_eq!(before_cancel, 1);

    handle.cancel();
    assert_eq!(handle.end_state(), BatchEnd::Cancelled);
    assert!(wait_process_gone(pid, TEST_TIMEOUT), "OS must report curl PID {pid} gone after cancel");
    assert!(!has_part_files(tmp.path()), "no .part file may remain after cancel");

    gate.release();
    assert!(server.wait_idle(TEST_TIMEOUT), "server finished handling held request");
    assert_eq!(server.request_paths().len(), before_cancel, "no new request may start after cancel");
    assert!(!has_part_files(tmp.path()));
}

/// Caller distinguishes 404, socket closed without answer, and 200 text body from `BatchResult` alone.
#[test]
fn caller_tells_failures_and_200_text_body_apart_from_result_alone() {
    let tmp = TempDir::new("failures");
    let routes = HashMap::from([
        ("/notfound".to_owned(), Reply::NotFound(b"404 body".to_vec())),
        ("/closed".to_owned(), Reply::CloseWithoutAnswer),
        ("/text200".to_owned(), Reply::Ok(b"ERROR: expired key".to_vec())),
    ]);
    let server = TestServer::start(routes);

    let (d0, d1, d2) = (tmp.path().join("0.bin"), tmp.path().join("1.bin"), tmp.path().join("2.bin"));
    let handle = start(
        &[
            BatchItem::new(server.url("/notfound"), &d0),
            BatchItem::new(server.url("/closed"), &d1),
            BatchItem::new(server.url("/text200"), &d2),
        ],
        &[],
        Duration::from_secs(5),
        3,
    )
    .expect("start failure-discrimination batch");

    let mut by_idx = [None, None, None];
    while let Some(r) = handle.recv_timeout(TEST_TIMEOUT).expect("no timeout") {
        by_idx[r.index] = Some(r);
    }
    assert_eq!(by_idx[0].unwrap().http_status, 404);
    assert!(!d0.exists());
    assert_eq!(by_idx[1].unwrap().http_status, 0);
    assert_ne!(by_idx[1].unwrap().exit_code, 0);
    assert!(!d1.exists());
    assert_eq!((by_idx[2].unwrap().http_status, by_idx[2].unwrap().exit_code), (200, 0));
    assert_eq!(fs::read(&d2).unwrap(), b"ERROR: expired key");
    assert!(!has_part_files(tmp.path()));
}

/// Missing `curl` binary returns an explicit error without panicking.
#[test]
fn missing_curl_binary_returns_error_without_panic() {
    let tmp = TempDir::new("missing-bin");
    let item = BatchItem::new("http://127.0.0.1:1/x", tmp.path().join("x.bin"));
    let err = start_with_program(
        OsStr::new("strata-nonexistent-curl-executable-0000"),
        &[item],
        &[],
        Duration::from_secs(5),
        2,
    )
    .expect_err("missing binary must fail cleanly");
    assert!(matches!(err, BatchError::CurlStartFailed(_)), "{err:?}");
    assert!(err.to_string().contains("curl"), "{err}");
}
