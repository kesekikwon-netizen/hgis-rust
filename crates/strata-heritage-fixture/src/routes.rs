//! The routes the fixture answers, and the HTML of each page.
//!
//! Element ids are the ones the ported scripts look for: `#userId`, `#pwd`, `goLogin()`,
//! `#searchForm`, `#mode` with a `[SPBURE]` letter, `#codedetaCd0`, `#codeCdSg0`,
//! `#searchGisChaRirList`, `#downloadFilesAll`, and `#tabContentDiv`.
//! Route table: `FIXTURES.md` beside this crate.

use std::sync::{Arc, Mutex};

use crate::pages;
use crate::state::FixtureState;

pub(crate) const STATUS_200: &str = "200 OK";
pub(crate) const STATUS_204: &str = "204 No Content";
pub(crate) const STATUS_404: &str = "404 Not Found";
pub(crate) const CT_HTML: &str = "text/html; charset=utf-8";
pub(crate) const CT_ZIP: &str = "application/zip";

/// Answers one request: `(status line, content type, body)`.
pub(crate) fn respond(
    method: &str,
    path: &str,
    body: &str,
    state: &Arc<Mutex<FixtureState>>,
) -> (&'static str, &'static str, Vec<u8>) {
    // The site's login form posts here; the answer is the login landing page.
    if method == "POST" && path == "/j_spring_security_check" {
        if let Ok(mut s) = state.lock() {
            let account = s.account.clone();
            if matches_login(body, &account) {
                s.logged_in = true;
            }
        }
        return html(&login_or_main(state));
    }

    match path {
        // Before login: the form. After login: the frames page, whose child frame holds
        // the download panel — the frame walker in the scripts needs that layout.
        "/user/checkNet.do" => html(&login_or_main(state)),
        "/main.do" => html(&pages::main_frames(&city(state))),
        "/user/data/heritageDownload.do" => html(&pages::download_page(&city(state))),
        "/user/data/heritageDownloadList.do" => html(&pages::search_result(&city(state))),
        "/user/data/heritageDownload/downloadFilesAll.do" => {
            let zip = state.lock().ok().and_then(|s| s.zips.values().next().cloned());
            match zip {
                Some(bytes) => (STATUS_200, CT_ZIP, bytes),
                None => (STATUS_404, CT_HTML, b"no zip".to_vec()),
            }
        }
        "/favicon.ico" => (STATUS_204, CT_HTML, Vec::new()),
        _ => (STATUS_404, CT_HTML, b"not found".to_vec()),
    }
}

fn html(body: &str) -> (&'static str, &'static str, Vec<u8>) {
    (STATUS_200, CT_HTML, body.as_bytes().to_vec())
}

fn city(state: &Arc<Mutex<FixtureState>>) -> (String, String) {
    state.lock().map(|s| s.city.clone()).unwrap_or_default()
}

fn login_or_main(state: &Arc<Mutex<FixtureState>>) -> String {
    let (logged_in, city) = state
        .lock()
        .map(|s| (s.logged_in, s.city.clone()))
        .unwrap_or((false, (String::new(), String::new())));
    if logged_in {
        pages::main_frames(&city)
    } else {
        pages::login().to_owned()
    }
}

/// `USER_ID=…&USER_PW=…` matching the fixture's account.
fn matches_login(body: &str, account: &(String, String)) -> bool {
    let mut id = None;
    let mut pw = None;
    for pair in body.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            match k {
                "USER_ID" => id = Some(v.to_owned()),
                "USER_PW" => pw = Some(v.to_owned()),
                _ => {}
            }
        }
    }
    matches!((id, pw), (Some(i), Some(p)) if i == account.0 && p == account.1)
}
