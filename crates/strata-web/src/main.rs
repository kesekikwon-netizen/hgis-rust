//! 웹 화면 도우미 `ka-hgis-web`.
//!
//! C++ `KaTopographicBrowser` 가 QWebEngineView 로 하던 일(사이트를 열고, 자동화 스크립트를
//! 돌리고, 팝업과 파일 받기를 다룬다)을 WebView2 로 한다. 본 앱이 따로 띄워 표준 입출력으로 부린다.
//!
//! 실행: `ka-hgis-web --allow <호스트> [--allow …] --downloads <폴더> [--hidden]`

mod protocol;

use std::cell::RefCell;
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::rc::Rc;

use protocol::{free_path, is_trusted_download_url, is_trusted_url, safe_file_name, url_path, Command, Event};
use tao::dpi::LogicalSize;
use tao::event::{Event as TaoEvent, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopWindowTarget};
use tao::window::{Window, WindowBuilder};
use wry::{NewWindowResponse, WebView, WebViewBuilder, WebViewBuilderExtWindows, WebViewExtWindows};

/// 미리 만들어 두는 팝업용 빈 화면 수(로그인 창이 팝업으로 뜬다).
const SPARE_PAGES: usize = 2;

#[derive(Debug)]
enum UserEvent {
    Command(Command),
    /// 팝업이 빈 화면을 하나 썼다 — 다시 채운다.
    Refill,
    InputClosed,
}

struct Page {
    window: Window,
    webview: WebView,
}

/// 창들이 함께 쓰는 것.
struct Shared {
    allowed: Vec<String>,
    downloads: PathBuf,
    visible: RefCell<bool>,
    spares: RefCell<Vec<Page>>,
    /// 팝업이 가져간 화면(다음 차례에 `pages` 로 옮긴다).
    adopted: RefCell<Vec<Page>>,
    proxy: tao::event_loop::EventLoopProxy<UserEvent>,
}

fn emit(event: &Event) {
    if let Ok(line) = serde_json::to_string(event) {
        let mut out = std::io::stdout().lock();
        let _ = writeln!(out, "{line}");
        let _ = out.flush();
    }
}

/// 창 하나와 그 안의 웹 화면. `sibling` 이 있으면 같은 환경(쿠키·로그인)을 쓴다.
fn make_page(
    target: &EventLoopWindowTarget<UserEvent>,
    shared: &Rc<Shared>,
    sibling: Option<&WebView>,
    visible: bool,
) -> Result<Page, String> {
    let window = WindowBuilder::new()
        .with_title("Strata — 공식 사이트")
        .with_inner_size(LogicalSize::new(1180.0, 820.0))
        .with_visible(visible)
        .build(target)
        .map_err(|e| format!("창을 만들지 못했습니다: {e}"))?;

    let for_popup = Rc::clone(shared);
    let for_nav = Rc::clone(shared);
    let for_download = Rc::clone(shared);
    let mut builder = WebViewBuilder::new()
        // 믿는 사이트 밖으로는 가지 않는다(빈 화면은 팝업이 채워지기 전 상태).
        .with_navigation_handler(move |url| url == "about:blank" || is_trusted_url(&url, &for_nav.allowed))
        .with_new_window_req_handler(move |_url, _features| {
            // 미리 만들어 둔 빈 화면을 팝업으로 내준다.
            let Some(page) = for_popup.spares.borrow_mut().pop() else {
                return NewWindowResponse::Deny;
            };
            page.window.set_visible(*for_popup.visible.borrow());
            let webview = page.webview.webview();
            for_popup.adopted.borrow_mut().push(page);
            let _ = for_popup.proxy.send_event(UserEvent::Refill);
            NewWindowResponse::Create { webview }
        })
        .with_download_started_handler(move |url, path| {
            // 믿는 사이트의 파일만, 안전한 이름으로, 있는 파일을 덮어쓰지 않고 받는다.
            if !is_trusted_download_url(&url, &for_download.allowed) {
                return false;
            }
            let name = path.file_name().and_then(|n| n.to_str()).and_then(safe_file_name);
            let Some(free) = name.and_then(|n| free_path(&for_download.downloads, &n)) else {
                return false;
            };
            *path = free;
            emit(&Event::DownloadStarted { path: path.display().to_string() });
            true
        })
        .with_download_completed_handler(|_url, path, ok| {
            let path = path.map(|p| p.display().to_string()).unwrap_or_default();
            emit(&Event::DownloadDone { path, ok });
        });
    if let Some(sibling) = sibling {
        builder = builder.with_environment(sibling.environment());
    }
    let webview = builder.build(&window).map_err(|e| format!("웹 화면을 만들지 못했습니다: {e}"))?;
    Ok(Page { window, webview })
}

/// 경로가 맞는 믿는 화면에서 스크립트를 돌린다(나중에 열린 화면 먼저).
fn run_eval(pages: &[Page], shared: &Shared, id: u64, paths: &[String], script: &str) {
    let hit = pages.iter().rev().find_map(|page| {
        let url = page.webview.url().ok()?;
        (is_trusted_url(&url, &shared.allowed) && paths.iter().any(|p| p == url_path(&url))).then_some((page, url))
    });
    let Some((page, url)) = hit else {
        emit(&Event::Eval { id, found: false, url: String::new(), result: serde_json::Value::Null });
        return;
    };
    let answer_url = url.clone();
    let sent = page.webview.evaluate_script_with_callback(script, move |json| {
        let result = serde_json::from_str(&json).unwrap_or(serde_json::Value::Null);
        emit(&Event::Eval { id, found: true, url: answer_url.clone(), result });
    });
    if sent.is_err() {
        emit(&Event::Eval { id, found: true, url, result: serde_json::Value::Null });
    }
}

fn parse_args() -> (Vec<String>, PathBuf, bool) {
    let (mut allowed, mut downloads, mut hidden) = (Vec::new(), std::env::temp_dir(), false);
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--allow" => allowed.extend(args.next().map(|h| h.to_ascii_lowercase())),
            "--downloads" => downloads = args.next().map_or(downloads, PathBuf::from),
            "--hidden" => hidden = true,
            _ => {}
        }
    }
    (allowed, downloads, hidden)
}

fn main() {
    let (allowed, downloads, hidden) = parse_args();
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let shared = Rc::new(Shared {
        allowed,
        downloads,
        visible: RefCell::new(!hidden),
        spares: RefCell::new(Vec::new()),
        adopted: RefCell::new(Vec::new()),
        proxy: event_loop.create_proxy(),
    });

    let mut pages: Vec<Page> = Vec::new();
    match make_page(&event_loop, &shared, None, !hidden) {
        Ok(page) => pages.push(page),
        Err(message) => {
            emit(&Event::Error { message });
            return;
        }
    }

    // 표준 입력의 명령을 창 쪽으로 넘긴다.
    let proxy = event_loop.create_proxy();
    std::thread::spawn(move || {
        for line in std::io::stdin().lock().lines() {
            let Ok(line) = line else { break };
            match serde_json::from_str::<Command>(&line) {
                Ok(command) => {
                    if proxy.send_event(UserEvent::Command(command)).is_err() {
                        return;
                    }
                }
                Err(_) => emit(&Event::Error { message: "알 수 없는 명령입니다.".to_owned() }),
            }
        }
        let _ = proxy.send_event(UserEvent::InputClosed);
    });
    let _ = shared.proxy.send_event(UserEvent::Refill);
    emit(&Event::Ready);

    event_loop.run(move |event, target, control_flow| {
        *control_flow = ControlFlow::Wait;
        pages.append(&mut shared.adopted.borrow_mut());
        match event {
            TaoEvent::UserEvent(UserEvent::Refill) => {
                while shared.spares.borrow().len() < SPARE_PAGES {
                    let made = make_page(target, &shared, pages.first().map(|p| &p.webview), false);
                    match made {
                        Ok(page) => shared.spares.borrow_mut().push(page),
                        Err(message) => {
                            emit(&Event::Error { message });
                            break;
                        }
                    }
                }
            }
            TaoEvent::UserEvent(UserEvent::Command(command)) => match command {
                Command::Open { url } => {
                    let opened = is_trusted_url(&url, &shared.allowed)
                        && pages.first().is_some_and(|p| p.webview.load_url(&url).is_ok());
                    if !opened {
                        emit(&Event::Error { message: "주소를 열지 못했습니다.".to_owned() });
                    }
                }
                Command::Eval { id, paths, script } => run_eval(&pages, &shared, id, &paths, &script),
                Command::Show { visible } => {
                    *shared.visible.borrow_mut() = visible;
                    for page in &pages {
                        page.window.set_visible(visible);
                    }
                }
                Command::Quit => *control_flow = ControlFlow::Exit,
            },
            TaoEvent::UserEvent(UserEvent::InputClosed) => *control_flow = ControlFlow::Exit,
            TaoEvent::WindowEvent { event: WindowEvent::CloseRequested, window_id, .. } => {
                // 첫 화면을 닫으면 끝, 팝업을 닫으면 그 화면만 없앤다.
                if pages.first().is_some_and(|p| p.window.id() == window_id) {
                    emit(&Event::Closed);
                    *control_flow = ControlFlow::Exit;
                } else {
                    pages.retain(|p| p.window.id() != window_id);
                }
            }
            _ => {}
        }
    });
}
