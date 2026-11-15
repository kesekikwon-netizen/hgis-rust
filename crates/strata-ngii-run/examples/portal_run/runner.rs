//! Runs the cases one after another, each with a fake portal of its own and a time limit.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui;
use strata_net::ngii_auto::{Account, AutoConfig, Scope, Stage, PAGE_SCRIPT};
use strata_ngii_fixture::NgiiFixtureSite;
use strata_ngii_run::{NgiiRun, RunOptions};

use crate::cases::{cases, Case, Seen};

/// The C++ tests wait 15 s for a result; a held transfer and the session's start come on top.
const CASE_LIMIT: Duration = Duration::from_secs(45);

struct Live {
    site: NgiiFixtureSite,
    run: NgiiRun,
    began: Instant,
    seen: Seen,
}

pub struct Runner {
    cases: Vec<Case>,
    at: usize,
    only: Option<usize>,
    live: Option<Live>,
    work: PathBuf,
    pub outcome: Rc<RefCell<(u32, u32)>>,
}

impl Runner {
    pub fn new(work: PathBuf, only: Option<usize>) -> Self {
        Self { cases: cases(), at: 0, only, live: None, work, outcome: Rc::default() }
    }

    fn begin(&mut self, frame: &eframe::Frame) -> Result<Live, String> {
        let case = &self.cases[self.at];
        let site = NgiiFixtureSite::start_with((case.portal)()).map_err(|e| e.to_string())?;
        let downloads = self.work.join(format!("case-{}", self.at + 1)).join("지형도").join("원본");
        // The form fields the C++ test types in (:517-520); the account only when the portal asks for one (:514).
        let account = Account {
            login_id: if case.with_account { "fixture-user".into() } else { String::new() },
            login_password: if case.with_account { "fixture-password".into() } else { String::new() },
            birth_date: "1990/01/01".into(),
            purpose: "회귀 테스트용 가상 신청".into(),
        };
        let config = AutoConfig { portal_url: site.url("/ms/map/NlipMap.do?tabGb=total"), script: PAGE_SCRIPT.to_owned(), download_directory: downloads.clone(), account, product: strata_net::ngii_auto::Product::Sheets, test_mode: true, diagnostics: false };
        let options = RunOptions { data_parent: self.work.join("profile"), config, network_allowed: Arc::new(|| false) };
        let mut run = NgiiRun::start(frame, options).map_err(|e| e.to_string())?;
        let list = |numbers: &[&str]| numbers.iter().map(|n| (*n).to_owned()).collect();
        // The circle of the C++ tests (:532).
        run.prepare_sheets(Scope { x: 1_125_000.0, y: 1_980_000.0, radius: 5000.0, numbers: list(case.numbers), cached_numbers: list(case.cached) });
        let seen = Seen { events: Vec::new(), phases: Vec::new(), stage: Stage::Idle, failure_phase: String::new(), downloads, most_received: 0 };
        Ok(Live { site, run, began: Instant::now(), seen })
    }

    fn judge(&mut self, live: &Live, timed_out: bool) {
        let case = &self.cases[self.at];
        let mut failures = match live.site.state().lock() {
            Ok(portal) => (case.check)(&live.seen, &portal),
            Err(_) => vec!["the fake portal's state could not be read".to_owned()],
        };
        if timed_out {
            failures.insert(0, format!("no end within {} s (stage {:?}, phase {:?})", CASE_LIMIT.as_secs(), live.seen.stage, live.seen.phases.last()));
        }
        let mut outcome = self.outcome.borrow_mut();
        if failures.is_empty() {
            println!("PORTAL ok   {}", case.name);
            outcome.0 += 1;
        } else {
            println!("PORTAL FAIL {}", case.name);
            for failure in &failures {
                println!("PORTAL      not held: {failure}");
            }
            let notices: Vec<_> = live.seen.events.iter().filter_map(|e| if let strata_net::ngii_auto::AutoEvent::NeedsInput(t) = e { Some(t.as_str()) } else { None }).collect();
            println!("PORTAL      notices {notices:?}; phases {:?}; stage {:?}; most received {}", live.seen.phases, live.seen.stage, live.seen.most_received);
            outcome.1 += 1;
        }
    }
}

impl eframe::App for Runner {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label(format!("strata-ngii-run portal: case {} of {}", self.at + 1, self.cases.len()));
        });
        ctx.request_repaint_after(Duration::from_millis(30));
        while self.live.is_none() && self.at < self.cases.len() && self.only.is_some_and(|only| only != self.at + 1) {
            self.at += 1;
        }
        if self.live.is_none() {
            if self.at >= self.cases.len() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
            match self.begin(frame) {
                Ok(live) => self.live = Some(live),
                Err(why) => {
                    println!("PORTAL FAIL {}\nPORTAL      not held: the case could not start: {why}", self.cases[self.at].name);
                    self.outcome.borrow_mut().1 += 1;
                    self.at += 1;
                }
            }
            return;
        }
        let held = self.cases[self.at].held_transfers;
        let Some(live) = self.live.as_mut() else {
            return;
        };
        live.seen.events.extend(live.run.frame(&*frame));
        let (stage, phase, failure_phase, received) = live.run.with_state(|m| (m.stage(), m.phase().to_owned(), m.failure_phase().to_owned(), m.received_bytes()));
        if live.seen.phases.last() != Some(&phase) {
            live.seen.phases.push(phase);
        }
        (live.seen.stage, live.seen.failure_phase) = (stage, failure_phase);
        live.seen.most_received = live.seen.most_received.max(received);
        // As the C++ test does (:535-547): the portal holds the file open after its first chunk, and lets
        // it finish once the received bytes have been seen.
        if held && live.site.has_transfer_socket() && received >= strata_ngii_fixture::FIRST_CHUNK_SIZE as u64 {
            let _ = live.site.finish_transfer();
        }
        let over = matches!(stage, Stage::Complete | Stage::Stopped) && live.run.finished();
        let timed_out = live.began.elapsed() > CASE_LIMIT;
        if over || timed_out {
            if let Some(live) = self.live.take() {
                self.judge(&live, timed_out && !over);
            }
            self.at += 1;
        }
    }
}
