//! A tiny web site on 127.0.0.1 for the probe: a few fixed pages, one file, one transfer that never ends.
//! Standard library only; it answers one request per connection.

use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
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

// What the page sees of the host's ready script: at its own DOMContentLoaded listener (added after the
// host's), at onload, and inside a sub frame (where the script must not have run).
const READY: &str = r#"<html><head><title>ready</title></head><body><iframe src="/frame"></iframe>
<script>document.addEventListener('DOMContentLoaded', function () { window.__atReady = window.__early; });
window.onload = function () { document.title = 'onload saw ' + window.__early + '/' + window.__atReady + '/' + typeof window.frames[0].__early; };</script></body></html>"#;

const FRAME: &str = "<html><body>frame</body></html>";

// Inline scripts exercise document-start suppression (target/cpp-ref/src/app/KaHeritageBrowser.cpp:119-131).
const DIALOGS: &str = r#"<html><head><script>
window.__dialogs = [confirm('main confirm'), prompt('main prompt')]; alert('main alert');
</script></head><body><iframe src='/dialogs/nested'></iframe></body></html>"#;
const NESTED_DIALOGS: &str = "<html><body><iframe src='/dialogs/leaf'></iframe></body></html>";
const LEAF_DIALOGS: &str = r#"<html><head><script>
window.__dialogs = [confirm('nested confirm'), prompt('nested prompt')]; alert('nested alert');
</script></head><body>leaf</body></html>"#;

pub struct Site {
    addr: SocketAddr,
    hits: Arc<Mutex<Vec<String>>>,
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
        let hits = Arc::new(Mutex::new(Vec::new()));
        let log = Arc::clone(&hits);
        let worker = std::thread::spawn(move || {
            let mut held: Vec<TcpStream> = Vec::new();
            while !flag.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        if let Some(keep) = answer(stream, &log) {
                            held.push(keep);
                        }
                    }
                    Err(_) => std::thread::sleep(Duration::from_millis(15)),
                }
            }
        });
        Ok(Self { addr, hits, stop, worker: Some(worker) })
    }

    pub fn origin(&self) -> String {
        format!("http://127.0.0.1:{}", self.addr.port())
    }

    /// The paths of all requests that reached this site.
    pub fn hits(&self) -> Vec<String> {
        self.hits.lock().map(|h| h.clone()).unwrap_or_default()
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
fn answer(mut stream: TcpStream, log: &Mutex<Vec<String>>) -> Option<TcpStream> {
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
    if let Ok(mut hits) = log.lock() {
        hits.push(path.to_owned());
    }
    match path {
        "/main" => page(&mut stream, MAIN),
        "/pop" => page(&mut stream, POP),
        "/ready" => page(&mut stream, READY),
        "/frame" => page(&mut stream, FRAME),
        "/dialogs" => page(&mut stream, DIALOGS),
        "/dialogs/nested" => page(&mut stream, NESTED_DIALOGS),
        "/dialogs/leaf" => page(&mut stream, LEAF_DIALOGS),
        "/file.zip" | "/file.zip?again" => {
            // The offered name tries to leave the folder and holds characters Windows does not take.
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/zip\r\nContent-Disposition: attachment; filename=\"..\\\\evil name?.zip\"\r\nContent-Length: {FILE_BYTES}\r\nConnection: close\r\n\r\n"
            );
            let _ = stream.write_all(&vec![b'Z'; FILE_BYTES]);
        }
        "/second.xml" => {
            // A small second file, as a sheet's metadata file follows its data file.
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/zip\r\nContent-Disposition: attachment; filename=second.xml\r\nContent-Length: 23\r\nConnection: close\r\n\r\nPK-public-sheet-fixture"
            );
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
