//! 웹 화면 도우미 `ka-hgis-web`.
//!
//! C++ `KaTopographicBrowser` 가 QWebEngineView 로 하던 일(사이트를 열고, 자동화 스크립트를
//! 돌리고, 팝업과 파일 받기를 다룬다)을 WebView2 로 한다. 본 앱이 따로 띄워 표준 입출력으로 부린다.
//!
//! 실행: `ka-hgis-web --allow <호스트> [--allow …] [--allow-nav <호스트> …] --downloads <폴더> [--hidden]`
//! `--allow`: 스크립트를 돌리고 파일을 받는 믿는 사이트. `--allow-nav`: 지나가는 것만 허용하는
//! 주소(공식 로그인이 거치는 인증 사이트) — 그 화면에서는 스크립트도 계정 입력도 하지 않는다.

mod protocol;

use std::cell::RefCell;
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use protocol::{free_path, is_trusted_download_url, is_trusted_url, safe_file_name, url_path, Command, Event};
use tao::dpi::LogicalSize;
use tao::event::{Event as TaoEvent, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopWindowTarget};
use tao::window::{Window, WindowBuilder};
use wry::{NewWindowResponse, WebView, WebViewBuilder, WebViewBuilderExtWindows, WebViewExtWindows};

/// 팝업이 스스로 닫히면(`window.close()`) 화면은 사라지고 창만 남는다. 그런 화면은 스크립트에
/// 빈 답만 준다 — 이만큼 잇달아 빈 답이면 닫힌 것으로 보고 치운다.
const DEAD_AFTER_EMPTY_ANSWERS: u32 = 3;

/// 미리 만들어 두는 팝업용 빈 화면 수(로그인 창이 팝업으로 뜬다).
const SPARE_PAGES: usize = 2;

#[derive(Debug)]
enum UserEvent {
    Command(Command),
    /// 팝업이 빈 화면을 하나 썼다 — 다시 채운다.
    Refill,
    /// 닫힌 팝업을 치운다.
    DropPage(tao::window::WindowId),
    InputClosed,
}

struct Page {
    // 화면을 창보다 먼저 없앤다(선언 순서대로 없어진다).
    webview: WebView,
    window: Window,
    /// 잇달아 받은 빈 답의 수.
    empty_answers: Arc<AtomicU32>,
}

/// 창들이 함께 쓰는 것.
struct Shared {
    allowed: Vec<String>,
    /// 지나가는 것만 허용하는 호스트.
    pass_through: Vec<String>,
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
        // 믿는 사이트(https) 밖으로는 가지 않는다. 공식 로그인이 거치는 주소는 모두 그 안에 있다
        // (map.ngii.go.kr/anyid/…, www.ngii.go.kr/member/…, 2026-11-11 실제 확인). 빈 화면은 팝업이 채워지기 전 상태.
        .with_navigation_handler(move |url| {
            let ok = url == "about:blank"
                || is_trusted_url(&url, &for_nav.allowed)
                || is_trusted_url(&url, &for_nav.pass_through);
            if !ok {
                // 막은 주소는 호스트만 알린다(뒤의 값에는 인증 정보가 있을 수 있다).
                let host = url.split_once("://").map_or("", |(_, rest)| rest).split(['/', '?', '#']).next().unwrap_or("");
                emit(&Event::Error { message: format!("막은 이동: {}://{host}", url.split(':').next().unwrap_or("")) });
            }
            ok
        })
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
    Ok(Page { window, webview, empty_answers: Arc::new(AtomicU32::new(0)) })
}

/// 경로가 맞는 믿는 화면에서 스크립트를 돌린다. `paths` 의 앞 경로가 먼저(로그인 화면 우선),
/// 같은 경로면 나중에 열린 화면이 먼저다.
fn run_eval(pages: &[Page], shared: &Shared, id: u64, paths: &[String], script: &str) {
    let urls: Vec<String> = pages.iter().map(|page| page.webview.url().unwrap_or_default()).collect();
    let hit = paths.iter().find_map(|path| {
        (0..pages.len())
            .rev()
            .find(|&i| is_trusted_url(&urls[i], &shared.allowed) && url_path(&urls[i]) == path)
            .map(|i| (&pages[i], urls[i].clone()))
    });
    let Some((page, url)) = hit else {
        emit(&Event::Eval { id, found: false, url: String::new(), result: serde_json::Value::Null });
        return;
    };
    let answer_url = url.clone();
    // 첫 화면은 치우지 않는다(이동 중에 빈 답이 올 수 있다).
    let is_popup = pages.first().is_some_and(|first| first.window.id() != page.window.id());
    let (empty, window_id, proxy) = (Arc::clone(&page.empty_answers), page.window.id(), shared.proxy.clone());
    let sent = page.webview.evaluate_script_with_callback(script, move |json| {
        let result = serde_json::from_str(&json).unwrap_or(serde_json::Value::Null);
        if !result.is_null() {
            empty.store(0, Ordering::Relaxed);
        } else if is_popup && empty.fetch_add(1, Ordering::Relaxed) + 1 >= DEAD_AFTER_EMPTY_ANSWERS {
            let _ = proxy.send_event(UserEvent::DropPage(window_id));
        }
        emit(&Event::Eval { id, found: true, url: answer_url.clone(), result });
    });
    if sent.is_err() {
        // 스크립트를 돌릴 수 없는 팝업은 이미 닫힌 화면이다.
        if is_popup {
            let _ = shared.proxy.send_event(UserEvent::DropPage(window_id));
        }
        emit(&Event::Eval { id, found: true, url, result: serde_json::Value::Null });
    }
}

fn parse_args() -> (Vec<String>, Vec<String>, PathBuf, bool) {
    let (mut allowed, mut pass_through, mut downloads, mut hidden) = (Vec::new(), Vec::new(), std::env::temp_dir(), false);
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--allow" => allowed.extend(args.next().map(|h| h.to_ascii_lowercase())),
            "--allow-nav" => pass_through.extend(args.next().map(|h| h.to_ascii_lowercase())),
            "--downloads" => downloads = args.next().map_or(downloads, PathBuf::from),
            "--hidden" => hidden = true,
            _ => {}
        }
    }
    (allowed, pass_through, downloads, hidden)
}

fn main() {
    let (allowed, pass_through, downloads, hidden) = parse_args();
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let shared = Rc::new(Shared {
        allowed,
        pass_through,
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
            TaoEvent::UserEvent(UserEvent::DropPage(window_id)) => {
                if pages.first().is_some_and(|p| p.window.id() != window_id) {
                    pages.retain(|p| p.window.id() != window_id);
                }
            }
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
