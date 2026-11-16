//! A TCP listener answering one HTTP request per connection, the shape of
//! `crates/strata-ngii-fixture/src/server.rs`.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::state::{Fixture, FixtureState};

pub(crate) fn spawn(state: FixtureState) -> Fixture {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind a port");
    let addr = listener.local_addr().expect("local address");
    let base_url = format!("http://{addr}");
    let state = Arc::new(Mutex::new(state));
    let stop = Arc::new(AtomicBool::new(false));

    let state_conn = Arc::clone(&state);
    let stop_conn = Arc::clone(&stop);
    let join = std::thread::spawn(move || {
        while !stop_conn.load(Ordering::SeqCst) {
            let Ok((stream, _)) = listener.accept() else {
                break;
            };
            if stop_conn.load(Ordering::SeqCst) {
                break;
            }
            let s = Arc::clone(&state_conn);
            std::thread::spawn(move || handle(stream, s));
        }
    });

    Fixture {
        base_url,
        stop,
        state,
        join: Some(join),
    }
}

struct Request {
    method: String,
    path: String,
    body: String,
}

fn read_request(stream: &mut TcpStream) -> Option<Request> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let n = stream.read(&mut chunk).ok()?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(head_end) = find_header_end(&buf) {
            let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
            let len = content_length(&head);
            while buf.len() < head_end + 4 + len {
                let n = stream.read(&mut chunk).ok()?;
                if n == 0 {
                    break;
                }
                buf.extend_from_slice(&chunk[..n]);
            }
            let body = String::from_utf8_lossy(&buf[head_end + 4..]).to_string();
            return Some(parse_head(&head, body));
        }
    }
    None
}

fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

fn content_length(head: &str) -> usize {
    for line in head.lines() {
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                return value.trim().parse().unwrap_or(0);
            }
        }
    }
    0
}

fn parse_head(head: &str, body: String) -> Request {
    let mut lines = head.lines();
    let mut first = lines.next().unwrap_or("").split_whitespace();
    let method = first.next().unwrap_or("GET").to_owned();
    let target = first.next().unwrap_or("/").to_owned();
    let path = target.split('?').next().unwrap_or("/").to_owned();
    Request { method, path, body }
}

fn handle(mut stream: TcpStream, state: Arc<Mutex<FixtureState>>) {
    let Some(req) = read_request(&mut stream) else {
        return;
    };
    if let Ok(mut s) = state.lock() {
        s.count(&req.path);
    }
    let (status, content_type, body) = crate::routes::respond(&req.method, &req.path, &req.body, &state);
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.write_all(&body);
    let _ = stream.flush();
}

pub(crate) fn http_get(url: &str, _body: Option<&str>) -> String {
    let host_port = url.trim_start_matches("http://").split('/').next().unwrap_or("");
    let path = format!("/{}", url.trim_start_matches("http://").split('/').skip(1).collect::<Vec<_>>().join("/"));
    let mut stream = match TcpStream::connect(host_port) {
        Ok(s) => s,
        Err(_) => return String::new(),
    };
    let _ = stream.write_all(
        format!("GET {path} HTTP/1.1\r\nHost: {host_port}\r\nConnection: close\r\n\r\n").as_bytes(),
    );
    read_all(stream)
}

pub(crate) fn http_post_form(url: &str, body: &str) -> String {
    let host_port = url.trim_start_matches("http://").split('/').next().unwrap_or("");
    let path = format!("/{}", url.trim_start_matches("http://").split('/').skip(1).collect::<Vec<_>>().join("/"));
    let mut stream = match TcpStream::connect(host_port) {
        Ok(s) => s,
        Err(_) => return String::new(),
    };
    let _ = stream.write_all(
        format!(
            "POST {path} HTTP/1.1\r\nHost: {host_port}\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .as_bytes(),
    );
    read_all(stream)
}

fn read_all(mut stream: TcpStream) -> String {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
        }
    }
    let text = String::from_utf8_lossy(&buf).to_string();
    match text.find("\r\n\r\n") {
        Some(at) => text[at + 4..].to_owned(),
        None => text,
    }
}
