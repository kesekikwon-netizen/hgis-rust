//! A browser made of lists, for the tests of the stage logic.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use serde_json::Value;

use super::{Account, AutoConfig, AutoEvent, NgiiAutomation, PageHost, PageId, PageRef, Scope, PORTAL_HOME};

pub const STEP: u64 = 400;
pub const LOGIN_POPUP: &str = "https://map.ngii.go.kr/mn/loginPopup.do";
pub const TRANSFER: &str = "https://nlippd.ngii.go.kr/nlippd/pd/innorix/innorixdownLoad.do";
pub const RESET: &str = "window.__kaTopographicSelection=null;window.__kaTopographicOrder=null;window.__kaTopographicSubmission=null;";

#[derive(Debug, Clone)]
pub struct Run {
    pub page: PageId,
    pub script: String,
    pub token: u64,
}

#[derive(Debug, Default)]
pub struct FakeHost {
    pub pages: Vec<PageRef>,
    pub runs: Vec<Run>,
    pub navigations: Vec<String>,
}

impl FakeHost {
    /// A browser whose first page already shows the portal's map.
    pub fn with_portal() -> Self {
        let mut host = Self::default();
        host.open(1, PORTAL_HOME, None);
        host
    }

    pub fn open(&mut self, id: u32, url: &str, opener: Option<u32>) {
        self.pages.retain(|p| p.id != PageId(id));
        self.pages.push(PageRef { id: PageId(id), url: url.to_owned(), opener: opener.map(PageId) });
    }

    pub fn close(&mut self, id: u32) {
        self.pages.retain(|p| p.id != PageId(id));
    }

    /// The calls of the page script so far: page, the JSON it was called with, the token of the call.
    pub fn calls(&self) -> Vec<(PageId, Value, u64)> {
        self.runs
            .iter()
            .filter_map(|r| {
                let json = r.script.strip_prefix("SCRIPT(")?.strip_suffix(");")?;
                Some((r.page, serde_json::from_str(json).expect("the input is JSON"), r.token))
            })
            .collect()
    }

    /// Other scripts run so far (not the page script): page and text.
    pub fn other_scripts(&self) -> Vec<(PageId, String)> {
        self.runs.iter().filter(|r| !r.script.starts_with("SCRIPT(")).map(|r| (r.page, r.script.clone())).collect()
    }

    pub fn token_of(&self, script: &str) -> u64 {
        self.runs.iter().rev().find(|r| r.script == script).expect("that script was run").token
    }
}

impl PageHost for FakeHost {
    fn pages(&self) -> Vec<PageRef> {
        self.pages.clone()
    }

    fn run_script(&mut self, page: PageId, script: &str) -> Option<u64> {
        self.pages.iter().any(|p| p.id == page).then(|| {
            let token = self.runs.len() as u64 + 1;
            self.runs.push(Run { page, script: script.to_owned(), token });
            token
        })
    }

    fn navigate(&mut self, url: &str) {
        self.navigations.push(url.to_owned());
    }
}

pub fn config(downloads: &Path) -> AutoConfig {
    AutoConfig {
        portal_url: PORTAL_HOME.to_owned(),
        script: "SCRIPT".to_owned(),
        download_directory: downloads.to_path_buf(),
        account: Account { login_id: "tester".into(), login_password: "pw-for-test".into(), birth_date: " 19800101 ".into(), purpose: " 조사 도면 ".into() },
        test_mode: false,
        diagnostics: false,
    }
}

pub fn scope() -> Scope {
    Scope { x: 1_100_000.5, y: 1_750_000.0, radius: 5000.0, numbers: vec!["377052".into(), "377051".into(), "377052".into()], cached_numbers: vec!["377064".into()] }
}

/// A run that has started on a browser showing the portal.
pub fn started(downloads: &Path) -> (NgiiAutomation, FakeHost) {
    let mut host = FakeHost::with_portal();
    let mut machine = NgiiAutomation::new(config(downloads));
    machine.prepare_sheets(&mut host, scope(), 0);
    (machine, host)
}

/// One poll and its answer. Returns the page the script ran in and the input it got.
pub fn round(machine: &mut NgiiAutomation, host: &mut FakeHost, now: &mut u64, answer: &str) -> (PageId, Value) {
    let before = host.calls().len();
    *now += STEP;
    machine.tick(host, *now);
    let calls = host.calls();
    assert_eq!(calls.len(), before + 1, "this poll must call the page script once (stage {:?})", machine.stage());
    let (page, input, token) = calls.last().cloned().unwrap();
    machine.script_answered(host, token, answer, *now);
    (page, input)
}

pub const SELECTED: &str = r#"{"state":"selected","message":"","items":[{"num":"377051","name":"건천"},{"num":"377052","name":"경주"},{"num":"377064","name":"불국사"}],
  "downloadItems":[{"num":"377051","name":"건천","historyNo":2},{"num":"377052","name":"경주","historyNo":5}],"cached":["377064"],"unavailable":[]}"#;

/// Drives a started run to the stage where the two sheets of `SELECTED` are applied for and the order's
/// transfer popup (page 2, opened by the portal page) is known. Returns the time.
pub fn to_files(machine: &mut NgiiAutomation, host: &mut FakeHost) -> u64 {
    let mut now = 0;
    round(machine, host, &mut now, SELECTED);
    round(machine, host, &mut now, r#"{"state":"form"}"#);
    round(machine, host, &mut now, r#"{"state":"ready"}"#);
    round(machine, host, &mut now, r#"{"state":"submitted"}"#);
    host.open(2, "about:blank", Some(1));
    machine.page_load_started(host, PageId(2));
    host.open(2, TRANSFER, Some(1));
    machine.page_loaded(host, PageId(2), now);
    let token = host.token_of("window.name === 'downForm'");
    machine.script_answered(host, token, "true", now);
    round(machine, host, &mut now, r#"{"state":"orderReady"}"#);
    now
}

pub fn needs_input(events: &[AutoEvent]) -> Vec<&str> {
    events.iter().filter_map(|e| if let AutoEvent::NeedsInput(text) = e { Some(text.as_str()) } else { None }).collect()
}
