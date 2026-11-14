//! A tiny web site on 127.0.0.1 for the probe: a few fixed pages, one file, one transfer that never ends.
//! Standard library only; it answers one request per connection.

use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

pub const FILE_BYTES: usize = 70_000;

const MAIN: &str = r#"<html><head><title>main</title></head><body>main
<script>
window.__n = 41;
function pop() { window.__p = window.open('/pop', 'downForm', 'width=300,height=200'); return !!window.__p; }
function ask() { var yes = confirm('sure?'); alert('answer ' + yes); var typed = prompt('name?'); return [yes, typed]; }
</script></body></html>"#;

const POP: &str = r#"<html><head><title>pop</title></head><body>
<script>document.title = 'pop:' + window.name + ':' + (window.opener ? window.opener.__n : 'none');</script></body></html>"#;

const READY: &str = r#"<html><head><title>ready</title></head><body>
<script>window.onload = function () { document.title = 'onload saw ' + window.__early; };</script></body></html>"#;

pub struct Site {
    addr: SocketAddr,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl Site {
    pub fn start() -> std::io::Result<Self> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
        listener.set_nonblocking(true)?;
        let addr = listener.local_addr()?;
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let worker = std::thread::spawn(move || {
            let mut held: Vec<TcpStream> = Vec::new();
            while !flag.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        if let Some(keep) = answer(stream) {
                            held.push(keep);
                        }
                    }
                    Err(_) => std::thread::sleep(Duration::from_millis(15)),
                }
            }
        });
        Ok(Self { addr, stop, worker: Some(worker) })
    }

    pub fn origin(&self) -> String {
        format!("http://127.0.0.1:{}", self.addr.port())
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{path}", self.origin())
    }
}

impl Drop for Site {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn page(stream: &mut TcpStream, html: &str) {
    let _ = write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{html}", html.len());
}

/// Answers one request. Returns the connection when it must stay open (the transfer that never ends).
fn answer(mut stream: TcpStream) -> Option<TcpStream> {
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let mut request = Vec::new();
    let mut buffer = [0_u8; 2048];
    while !request.windows(4).any(|w| w == b"\r\n\r\n") {
        match stream.read(&mut buffer) {
            Ok(0) | Err(_) => return None,
            Ok(n) => request.extend_from_slice(&buffer[..n]),
        }
    }
    let head = String::from_utf8_lossy(&request);
    let path = head.split_whitespace().nth(1).unwrap_or("/");
    match path {
        "/main" => page(&mut stream, MAIN),
        "/pop" => page(&mut stream, POP),
        "/ready" => page(&mut stream, READY),
        "/file.zip" | "/file.zip?again" => {
            // The offered name tries to leave the folder and holds characters Windows does not take.
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/zip\r\nContent-Disposition: attachment; filename=\"..\\\\evil name?.zip\"\r\nContent-Length: {FILE_BYTES}\r\nConnection: close\r\n\r\n"
            );
            let _ = stream.write_all(&vec![b'Z'; FILE_BYTES]);
        }
        "/slow.zip" => {
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/zip\r\nContent-Disposition: attachment; filename=slow.zip\r\nContent-Length: 1000000\r\n\r\nPKpartial"
            );
            let _ = stream.flush();
            return Some(stream);
        }
        _ => {
            let _ = write!(stream, "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        }
    }
    None
}
