//! Background HTTP listener and transfer lifecycle ported from `LocalDownloadServer`
//! (`target/cpp-ref/tests/test_topographic_browser.cpp:70-125`).

use crate::server_routes::dispatch_standard_routes;
use crate::{NgiiFixtureState, FIRST_CHUNK_SIZE, TRANSFER_SIZE};
use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub(crate) fn run_accept_loop(
    listener: TcpListener,
    state: Arc<Mutex<NgiiFixtureState>>,
    stop: Arc<AtomicBool>,
) {
    let mut workers: Vec<JoinHandle<()>> = Vec::new();
    while !stop.load(Ordering::SeqCst) {
        let Ok((stream, _peer)) = listener.accept() else {
            break;
        };
        if stop.load(Ordering::SeqCst) {
            let _ = stream.shutdown(Shutdown::Both);
            break;
        }
        let state_conn = Arc::clone(&state);
        let stop_conn = Arc::clone(&stop);
        workers.push(thread::spawn(move || handle_connection(stream, state_conn, stop_conn)));
    }
    for worker in workers {
        let _ = worker.join();
    }
}

pub(crate) fn graceful_close(mut stream: TcpStream) {
    let _ = stream.flush();
    let _ = stream.shutdown(Shutdown::Write);
    let _ = stream.set_read_timeout(Some(Duration::from_millis(50)));
    let mut drain = [0u8; 512];
    while let Ok(n) = stream.read(&mut drain) {
        if n == 0 {
            break;
        }
    }
}

// Ported from target/cpp-ref/tests/test_topographic_browser.cpp:70-80.
pub(crate) fn start_transfer(
    state: &mut NgiiFixtureState,
    mut stream: TcpStream,
    file_name: &[u8],
    chunked: bool,
) {
    let _ = stream.set_read_timeout(None);
    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
    state.transfer_chunked = chunked;
    let length_hdr = if chunked {
        b"Transfer-Encoding: chunked\r\n".to_vec()
    } else {
        format!("Content-Length: {TRANSFER_SIZE}\r\n").into_bytes()
    };
    let mut head = Vec::with_capacity(160 + file_name.len());
    head.extend_from_slice(
        b"HTTP/1.1 200 OK\r\nContent-Type: application/zip\r\nContent-Disposition: attachment; filename=",
    );
    head.extend_from_slice(file_name);
    head.extend_from_slice(b"\r\n");
    head.extend_from_slice(&length_hdr);
    head.extend_from_slice(b"\r\n");
    let _ = stream.write_all(&head);

    let mut first = vec![b'x'; FIRST_CHUNK_SIZE];
    first[0] = b'P';
    first[1] = b'K';
    if chunked {
        let prefix = format!("{:x}\r\n", first.len());
        let _ = stream.write_all(prefix.as_bytes());
        let _ = stream.write_all(&first);
        let _ = stream.write_all(b"\r\n");
    } else {
        let _ = stream.write_all(&first);
    }
    let _ = stream.flush();
    state.transfer_socket = Some(stream);
}

// Ported from target/cpp-ref/tests/test_topographic_browser.cpp:81-96.
// Queues the 3 MiB remainder asynchronously like `QTcpSocket::write` so a caller invoking
// `finish_transfer()` on the same thread before draining the client socket never deadlocks
// on the OS loopback TCP window.
pub(crate) fn finish_transfer(state: &Arc<Mutex<NgiiFixtureState>>) -> bool {
    let Ok(mut guard) = state.lock() else {
        return false;
    };
    let Some(mut transfer) = guard.transfer_socket.take() else {
        return false;
    };
    let chunked = guard.transfer_chunked;
    guard.active_sdk_file.clear();
    guard.fixture_archive_finished = true;
    let xml_pair = guard
        .pending_xml_socket
        .take()
        .map(|sock| (sock, guard.pending_xml_name.clone()));

    let handle = thread::spawn(move || {
        let _ = transfer.set_write_timeout(Some(Duration::from_secs(2)));
        let rest = vec![b'x'; TRANSFER_SIZE - FIRST_CHUNK_SIZE];
        if chunked {
            let prefix = format!("{:x}\r\n", rest.len());
            let _ = transfer.write_all(prefix.as_bytes());
            let _ = transfer.write_all(&rest);
            let _ = transfer.write_all(b"\r\n0\r\n\r\n");
        } else {
            let _ = transfer.write_all(&rest);
        }
        graceful_close(transfer);

        if let Some((mut xml_sock, xml_name)) = xml_pair {
            let _ = xml_sock.set_write_timeout(Some(Duration::from_secs(2)));
            let body = b"fixture metadata";
            let mut msg = Vec::with_capacity(192);
            msg.extend_from_slice(
                b"HTTP/1.1 200 OK\r\nContent-Type: application/xml\r\nContent-Disposition: attachment; filename=",
            );
            msg.extend_from_slice(&xml_name);
            msg.extend_from_slice(
                format!("\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).as_bytes(),
            );
            msg.extend_from_slice(body);
            let _ = xml_sock.write_all(&msg);
            graceful_close(xml_sock);
        }
    });
    guard.bg_writers.push(handle);
    true
}

// Ported from target/cpp-ref/tests/test_topographic_browser.cpp:97-103.
pub(crate) fn finish_sdk_xml(state: &Arc<Mutex<NgiiFixtureState>>) -> bool {
    let Ok(mut guard) = state.lock() else {
        return false;
    };
    let Some(mut sock) = guard.sdk_xml_socket.take() else {
        return false;
    };
    let xml_name = guard.sdk_xml_name.clone();
    let handle = thread::spawn(move || {
        let _ = sock.set_write_timeout(Some(Duration::from_secs(2)));
        let body = b"fixture metadata";
        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/xml\r\nContent-Disposition: attachment; filename={xml_name}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        let _ = sock.write_all(resp.as_bytes());
        let _ = sock.write_all(body);
        graceful_close(sock);
    });
    guard.bg_writers.push(handle);
    true
}

// Ported from target/cpp-ref/tests/test_topographic_browser.cpp:392.
pub(crate) fn abort_transfer(state: &Arc<Mutex<NgiiFixtureState>>) -> bool {
    let Ok(mut guard) = state.lock() else {
        return false;
    };
    let Some(sock) = guard.transfer_socket.take() else {
        return false;
    };
    let _ = sock.shutdown(Shutdown::Both);
    true
}

// Ported from target/cpp-ref/tests/test_topographic_browser.cpp:784-785, 932-933.
pub(crate) fn finish_pending_login_script(state: &Arc<Mutex<NgiiFixtureState>>) -> bool {
    let Ok(mut guard) = state.lock() else {
        return false;
    };
    let Some(mut sock) = guard.pending_login_script.take() else {
        return false;
    };
    let handle = thread::spawn(move || {
        let _ = sock.set_write_timeout(Some(Duration::from_secs(2)));
        let _ = sock.write_all(
            b"HTTP/1.1 200 OK\r\nContent-Type: application/javascript\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        );
        graceful_close(sock);
    });
    guard.bg_writers.push(handle);
    true
}

// Ported from target/cpp-ref/tests/test_topographic_browser.cpp:108-125.
fn handle_connection(
    mut stream: TcpStream,
    state: Arc<Mutex<NgiiFixtureState>>,
    stop: Arc<AtomicBool>,
) {
    let _ = stream.set_read_timeout(Some(Duration::from_millis(100)));
    let mut input = Vec::new();
    let mut buf = [0u8; 4096];
    // Accumulate until "\r\n\r\n" (`target/cpp-ref/tests/test_topographic_browser.cpp:109-112`).
    while !input.windows(4).any(|w| w == b"\r\n\r\n") {
        if stop.load(Ordering::SeqCst) {
            return;
        }
        match stream.read(&mut buf) {
            Ok(0) => return,
            Ok(n) => input.extend_from_slice(&buf[..n]),
            Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => continue,
            Err(_) => return,
        }
    }

    // Extract target = input.split(' ').value(1) (`target/cpp-ref/tests/test_topographic_browser.cpp:113`).
    let target_bytes = input.split(|&b| b == b' ').nth(1).unwrap_or(b"");
    let target = String::from_utf8_lossy(target_bytes).into_owned();

    // Branch 1: `/slow.zip` (`target/cpp-ref/tests/test_topographic_browser.cpp:114-117`).
    if target == "/slow.zip" {
        let _ = stream.set_read_timeout(None);
        let _ = stream.write_all(
            b"HTTP/1.1 200 OK\r\nContent-Type: application/zip\r\nContent-Disposition: attachment; filename=slow.zip\r\nContent-Length: 1000000\r\n\r\nPKpartial",
        );
        let _ = stream.flush();
        if let Ok(mut guard) = state.lock() {
            guard.held_sockets.push(stream);
        }
        return;
    }
    // Branch 2: `/controlled.zip` or `/chunked.zip` (`target/cpp-ref/tests/test_topographic_browser.cpp:118-120`).
    if target == "/controlled.zip" || target == "/chunked.zip" {
        if let Ok(mut guard) = state.lock() {
            start_transfer(&mut guard, stream, &target.as_bytes()[1..], target == "/chunked.zip");
        }
        return;
    }
    // Branch 3: `/interrupted.zip` (`target/cpp-ref/tests/test_topographic_browser.cpp:121-125`).
    if target == "/interrupted.zip" {
        if let Ok(mut guard) = state.lock() {
            guard.interrupted_requests += 1;
            if guard.interrupted_requests == 1 {
                start_transfer(&mut guard, stream, b"interrupted.zip", false);
                return;
            }
        } else {
            return;
        }
        let _ = stream.write_all(
            b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        );
        graceful_close(stream);
        return;
    }

    dispatch_standard_routes(stream, &state, &target);
}
