//! Regression tests for `MAP2a2` findings (`f1`..`f6`, `note`, and pre-spawn gate; part 3/3).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::HashMap;
use std::fs;
use std::io::{self, Cursor, Read};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::tests_a::*;
use super::*;

struct PanickingAfterFirstRead {
    first_chunk: Option<Vec<u8>>,
}

impl Read for PanickingAfterFirstRead {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if let Some(chunk) = self.first_chunk.take() {
            let n = chunk.len().min(buf.len());
            buf[..n].copy_from_slice(&chunk[..n]);
            Ok(n)
        } else {
            panic!("simulated reader thread panic");
        }
    }
}

/// `f1` + `f6`: killing `curl` externally in mid-batch delivers a failure result for every unrun
/// item and marks the batch incomplete (`!ended_normally()`).
#[test]
fn killed_curl_in_mid_batch_delivers_results_for_all_items_and_marks_incomplete() {
    let tmp = TempDir::new("extkill");
    let gate = HoldGate::new();
    let routes = HashMap::from([
        ("/0".to_owned(), Reply::Ok(b"ok-0".to_vec())),
        ("/1".to_owned(), Reply::HoldThenOk(gate.clone(), b"never-1".to_vec())),
        ("/2".to_owned(), Reply::Ok(b"never-2".to_vec())),
    ]);
    let server = TestServer::start(routes);
    let items: Vec<BatchItem> = (0..3)
        .map(|i| BatchItem::new(server.url(&format!("/{i}")), tmp.path().join(format!("{i}.bin"))))
        .collect();

    let handle = start(&items, &[], Duration::from_secs(10), 1).expect("start batch");
    let r0 = handle.recv_timeout(TEST_TIMEOUT).unwrap().expect("item 0 result");
    assert_eq!(r0, BatchResult { index: 0, http_status: 200, exit_code: 0 });
    assert!(r0.is_saved());

    assert!(gate.wait_entered(1, TEST_TIMEOUT), "item 1 held at server");
    let pid = handle.pid().expect("spawned curl pid");
    assert!(os_process_exists(pid));
    kill_os_process(pid);
    assert!(wait_process_gone(pid, TEST_TIMEOUT), "external kill terminated curl");
    gate.release();

    let r1 = handle.recv_timeout(TEST_TIMEOUT).unwrap().expect("fallback result for item 1");
    let r2 = handle.recv_timeout(TEST_TIMEOUT).unwrap().expect("fallback result for item 2");
    assert_eq!(handle.recv_timeout(TEST_TIMEOUT).unwrap(), None);

    assert_eq!(r1.index, 1);
    assert_eq!(r2.index, 2);
    assert_eq!((r1.http_status, r2.http_status), (0, 0));
    assert!(r1.exit_code != 0 && r2.exit_code != 0, "{r1:?} {r2:?}");
    assert!(!r1.is_saved() && !r2.is_saved());
    assert!(handle.is_done());
    assert!(!handle.ended_normally());
    assert_eq!(handle.end_state(), BatchEnd::Incomplete { missing_records: 2 });
    assert_eq!(fs::read(&items[0].destination).unwrap(), b"ok-0");
    assert!(!items[1].destination.exists() && !items[2].destination.exists());
    assert!(!has_part_files(tmp.path()));
}

/// `f1` + `f6`: a missing record or a reader-thread panic still delivers one result per item,
/// never hangs, and distinguishes incomplete/panicked batches from normal completion.
#[test]
fn missing_record_and_reader_panic_deliver_fallback_results_without_hanging() {
    let tmp = TempDir::new("missing-rec");
    let (d0, d1) = (tmp.path().join("0.bin"), tmp.path().join("1.bin"));
    let items = [BatchItem::new("http://127.0.0.1:1/0", &d0), BatchItem::new("http://127.0.0.1:1/1", &d1)];
    fs::write(part_path(&d0), b"saved-0").unwrap();
    fs::write(part_path(&d1), b"orphan-1").unwrap();

    let rec0 = format!("{RECORD_PREFIX}200\t0\t{}\n", part_path(&d0).to_string_lossy());
    let handle = start_from_reader_for_test(&items, Cursor::new(rec0.into_bytes()));
    let r0 = handle.recv_timeout(TEST_TIMEOUT).unwrap().expect("item 0");
    let r1 = handle.recv_timeout(TEST_TIMEOUT).unwrap().expect("missing item 1 fallback");
    assert_eq!(handle.recv_timeout(TEST_TIMEOUT).unwrap(), None);
    assert_eq!(r0, BatchResult { index: 0, http_status: 200, exit_code: 0 });
    assert_eq!(r1, BatchResult { index: 1, http_status: 0, exit_code: EXIT_NO_RECORD });
    assert!(!handle.ended_normally());
    assert_eq!(handle.end_state(), BatchEnd::Incomplete { missing_records: 1 });
    assert_eq!(fs::read(&d0).unwrap(), b"saved-0");
    assert!(!d1.exists() && !has_part_files(tmp.path()));

    let (p0, p1) = (tmp.path().join("p0.bin"), tmp.path().join("p1.bin"));
    let p_items = [BatchItem::new("http://127.0.0.1:1/p0", &p0), BatchItem::new("http://127.0.0.1:1/p1", &p1)];
    fs::write(part_path(&p0), b"ok-before-panic").unwrap();
    fs::write(part_path(&p1), b"orphan-panic").unwrap();
    let p_rec0 = format!("{RECORD_PREFIX}200\t0\t{}\n", part_path(&p0).to_string_lossy());
    let panic_handle = start_from_reader_for_test(&p_items, PanickingAfterFirstRead { first_chunk: Some(p_rec0.into_bytes()) });
    let pr0 = panic_handle.recv_timeout(TEST_TIMEOUT).unwrap().expect("item 0 before panic");
    let pr1 = panic_handle.recv_timeout(TEST_TIMEOUT).unwrap().expect("item 1 after panic");
    assert_eq!(panic_handle.recv_timeout(TEST_TIMEOUT).unwrap(), None);
    assert_eq!(pr0, BatchResult { index: 0, http_status: 200, exit_code: 0 });
    assert_eq!(pr1, BatchResult { index: 1, http_status: 0, exit_code: EXIT_NO_RECORD });
    assert!(panic_handle.is_done() && !panic_handle.ended_normally());
    assert_eq!(panic_handle.end_state(), BatchEnd::ReaderPanicked { missing_records: 1 });
    assert!(!has_part_files(tmp.path()));

    let server = TestServer::start(HashMap::from([("/ok".to_owned(), Reply::Ok(b"ok".to_vec()))]));
    let bad_items = [BatchItem::new(server.url("/ok"), tmp.path().join("ok.bin")), BatchItem::new("://bad", tmp.path().join("bad.bin"))];
    let bh = start(&bad_items, &[], Duration::from_secs(5), 1).expect("start malformed url batch");
    let (mut seen, mut bad_res) = ([false; 2], None);
    while let Some(r) = bh.recv_timeout(TEST_TIMEOUT).unwrap() { seen[r.index] = true; if r.index == 1 { bad_res = Some(r); } }
    assert_eq!(seen, [true, true]);
    assert_eq!(bad_res.unwrap().http_status, 0);
    assert_ne!(bad_res.unwrap().exit_code, 0);
}

/// `f2`: `location` is absent and `max-redirs = 0` is set; HTTP 302 returns status 302 without
/// contacting the redirect target server or forwarding headers.
#[test]
fn redirect_302_is_not_followed_and_target_server_gets_no_request() {
    let tmp = TempDir::new("redir");
    let target_server = TestServer::start(HashMap::from([("/secret".to_owned(), Reply::Ok(b"leaked".to_vec()))]));
    let origin_server = TestServer::start(HashMap::from([("/go".to_owned(), Reply::Redirect302(target_server.url("/secret")))]));

    let dest = tmp.path().join("redir.bin");
    let item = BatchItem::new(origin_server.url("/go"), &dest);
    let cfg = build_batch_config(std::slice::from_ref(&item), &[("Referer", "https://localhost")], Duration::from_secs(5), 1).unwrap();
    assert!(!cfg.lines().any(|l| l.trim() == "location"), "{cfg}");
    assert!(cfg.lines().any(|l| l.trim() == "max-redirs = 0"), "{cfg}");

    let handle = start(&[item], &[("Referer", "https://localhost")], Duration::from_secs(5), 1).expect("start 302");
    let res = handle.recv_timeout(TEST_TIMEOUT).unwrap().expect("302 result");
    assert_eq!(res, BatchResult { index: 0, http_status: 302, exit_code: 0 });
    assert!(!res.is_saved());
    assert_eq!(handle.recv_timeout(TEST_TIMEOUT).unwrap(), None);
    assert!(handle.ended_normally());
    assert!(!dest.exists() && !has_part_files(tmp.path()));
    assert_eq!(target_server.accepted_connections(), 0, "redirect target must receive 0 connections");
    assert!(target_server.request_paths().is_empty());
}

/// `f3`: `globoff` is set; URLs containing `[ ] { }` and lone `[` arrive byte-for-byte once each.
#[test]
fn glob_characters_in_urls_are_sent_verbatim_as_single_requests() {
    let tmp = TempDir::new("glob");
    let paths = ["/g/[1-2]?q={a,b}", "/g/{a,b}", "/g/[1"];
    let routes: HashMap<String, Reply> = paths
        .iter()
        .enumerate()
        .map(|(i, &p)| (p.to_owned(), Reply::Ok(format!("glob-{i}").into_bytes())))
        .collect();
    let server = TestServer::start(routes);
    let items: Vec<BatchItem> = paths
        .iter()
        .enumerate()
        .map(|(i, p)| BatchItem::new(server.url(p), tmp.path().join(format!("{i}.bin"))))
        .collect();

    let cfg = build_batch_config(&items, &[], Duration::from_secs(5), 1).unwrap();
    assert!(cfg.lines().any(|l| l.trim() == "globoff"), "{cfg}");

    let handle = start(&items, &[], Duration::from_secs(5), 1).expect("start glob batch");
    let mut got = [None, None, None];
    while let Some(res) = handle.recv_timeout(TEST_TIMEOUT).unwrap() {
        got[res.index] = Some(res);
    }
    for (i, slot) in got.iter().enumerate() {
        let res = slot.expect("every glob item gets a result");
        assert_eq!(res, BatchResult { index: i, http_status: 200, exit_code: 0 });
        assert_eq!(fs::read(&items[i].destination).unwrap(), format!("glob-{i}").as_bytes());
    }
    assert!(handle.ended_normally());
    assert_eq!(server.raw_targets(), paths.iter().map(|&s| s.to_owned()).collect::<Vec<_>>());
}

/// `f4`: when renaming `<destination>.part` to `<destination>` fails, `BatchResult.exit_code`
/// reports `EXIT_RENAME_FAILED` (never mistaken for success) and `.part` is cleaned up.
#[test]
fn failed_rename_reports_failure_code_and_leaves_no_part_file() {
    let tmp = TempDir::new("rename-fail");
    let server = TestServer::start(HashMap::from([("/ok".to_owned(), Reply::Ok(b"tile-data".to_vec()))]));
    let dest_is_dir = tmp.path().join("occupied_by_dir");
    fs::create_dir_all(&dest_is_dir).unwrap();

    let handle = start(&[BatchItem::new(server.url("/ok"), &dest_is_dir)], &[], Duration::from_secs(5), 1).expect("start");
    let res = handle.recv_timeout(TEST_TIMEOUT).unwrap().expect("rename-fail result");
    assert_eq!(handle.recv_timeout(TEST_TIMEOUT).unwrap(), None);
    assert_eq!(res.index, 0);
    assert_eq!(res.http_status, 200);
    assert_eq!(res.exit_code, EXIT_RENAME_FAILED);
    assert!(!res.is_saved());
    assert!(dest_is_dir.is_dir());
    assert!(!has_part_files(tmp.path()));
}

/// `f5`: duplicate destinations (after matching path normalisation) are refused at `start` with an
/// error naming the item index — never the path or URL — without creating folders or spawning `curl`.
#[test]
fn duplicate_destinations_are_refused_by_index_without_leaking_path_or_spawning() {
    let tmp = TempDir::new("dup-dest");
    let server = TestServer::start(HashMap::from([("/a".to_owned(), Reply::Ok(b"a".to_vec()))]));
    let uncreated = tmp.path().join("should_not_be_created");
    let d0 = uncreated.join("sub").join("secret_tile_name.jpeg");
    let d1 = uncreated.join("sub").join("other.jpeg");
    let d2 = PathBuf::from(format!("{}/./sub//secret_tile_name.jpeg", uncreated.to_string_lossy()));

    let items = [
        BatchItem::new(format!("{}?key={FAKE_KEY}", server.url("/a")), &d0),
        BatchItem::new(server.url("/a"), &d1),
        BatchItem::new(server.url("/a"), &d2),
    ];
    let err = start(&items, &[], Duration::from_secs(5), 2).expect_err("duplicate destination must fail");
    assert_eq!(err, BatchError::DuplicateDestination { index: 2, first_index: 0 });
    let msg = format!("{err} | {err:?}");
    assert!(msg.contains('2') && msg.contains('0'), "{msg}");
    assert!(!msg.contains("secret_tile_name") && !msg.contains(FAKE_KEY) && !msg.contains("http://"), "{msg}");
    assert!(!uncreated.exists(), "refused batch must not create directories");
    assert_eq!(server.accepted_connections(), 0, "refused batch must not contact server");
}

/// `f6` + `note`: Korean folder/file names with spaces (equal file names in distinct folders) work
/// with real `curl`, and raw non-UTF-8 bytes on `stderr` do not abort record parsing.
#[test]
fn korean_paths_with_spaces_and_non_utf8_stderr_succeed() {
    let tmp = TempDir::new("kor-utf8");
    let server = TestServer::start(HashMap::from([
        ("/k0".to_owned(), Reply::Ok(b"kor-0".to_vec())),
        ("/k1".to_owned(), Reply::Ok(b"kor-1".to_vec())),
    ]));
    let k0 = tmp.path().join("조사 구역 A").join("위성 타일 01.jpeg");
    let k1 = tmp.path().join("조사 구역 B").join("위성 타일 01.jpeg");
    let handle = start(&[BatchItem::new(server.url("/k0"), &k0), BatchItem::new(server.url("/k1"), &k1)], &[], Duration::from_secs(5), 2).expect("start korean paths");
    while handle.recv_timeout(TEST_TIMEOUT).unwrap().is_some() {}
    assert!(handle.ended_normally());
    assert_eq!(fs::read(&k0).unwrap(), b"kor-0");
    assert_eq!(fs::read(&k1).unwrap(), b"kor-1");

    let (n0, n1) = (tmp.path().join("n0.bin"), tmp.path().join("n1.bin"));
    fs::write(part_path(&n0), b"bytes-0").unwrap();
    fs::write(part_path(&n1), b"bytes-1").unwrap();
    let mut raw_stderr = vec![0xFF, 0xFE, 0xB0, 0xA1, b' ', b'c', b'p', b'9', b'4', b'9', b'\r', b'\n'];
    raw_stderr.extend_from_slice(format!("{RECORD_PREFIX}200\t0\t{}\n", part_path(&n0).to_string_lossy()).as_bytes());
    raw_stderr.extend_from_slice(&[0x80, 0xC0, 0xFE, b'\n']);
    raw_stderr.extend_from_slice(format!("{RECORD_PREFIX}200\t0\t{}\n", part_path(&n1).to_string_lossy()).as_bytes());

    let n_items = [BatchItem::new("http://127.0.0.1:1/0", &n0), BatchItem::new("http://127.0.0.1:1/1", &n1)];
    let nh = start_from_reader_for_test(&n_items, Cursor::new(raw_stderr));
    let nr0 = nh.recv_timeout(TEST_TIMEOUT).unwrap().expect("record 0 after non-utf8");
    let nr1 = nh.recv_timeout(TEST_TIMEOUT).unwrap().expect("record 1 after non-utf8");
    assert_eq!(nh.recv_timeout(TEST_TIMEOUT).unwrap(), None);
    assert!(nr0.is_saved() && nr1.is_saved() && nh.ended_normally());
    assert_eq!(fs::read(&n0).unwrap(), b"bytes-0");
    assert_eq!(fs::read(&n1).unwrap(), b"bytes-1");
}

/// Pre-spawn decision hook (`start_checked`) inspects the URL list and prevents directory creation
/// and process launch when refused.
#[test]
fn pre_spawn_hook_receives_urls_and_blocks_launch_without_spawning() {
    let tmp = TempDir::new("gate");
    let server = TestServer::start(HashMap::from([("/x".to_owned(), Reply::Ok(b"ok".to_vec()))]));
    let blocked_dir = tmp.path().join("blocked_sub");
    let u0 = server.url("/x?tile=0");
    let u1 = server.url("/x?tile=1");
    let items = [BatchItem::new(&u0, blocked_dir.join("0.bin")), BatchItem::new(&u1, blocked_dir.join("1.bin"))];

    let captured: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let cap = Arc::clone(&captured);
    let err = start_checked(&items, &[], Duration::from_secs(5), 2, move |urls| {
        *cap.lock().unwrap() = urls.iter().map(|&s| s.to_owned()).collect();
        Err("offline mode enabled".to_owned())
    })
    .expect_err("blocked launch must return Err");

    assert_eq!(err, BatchError::LaunchBlocked("offline mode enabled".to_owned()));
    assert_eq!(*captured.lock().unwrap(), vec![u0, u1]);
    assert!(!blocked_dir.exists(), "blocked launch must not create folders");
    assert_eq!(server.accepted_connections(), 0, "blocked launch must spawn no curl process");

    let called = Arc::new(AtomicBool::new(false));
    let called_bg = Arc::clone(&called);
    let ok_dest = tmp.path().join("allowed.bin");
    let handle = start_checked(&[BatchItem::new(server.url("/x"), &ok_dest)], &[], Duration::from_secs(5), 1, move |urls| {
        called_bg.store(urls.len() == 1, Ordering::Release);
        Ok(())
    })
    .expect("allowed launch succeeds");
    assert!(called.load(Ordering::Acquire));
    assert!(handle.recv_timeout(TEST_TIMEOUT).unwrap().unwrap().is_saved());
    assert_eq!(fs::read(&ok_dest).unwrap(), b"ok");
}
