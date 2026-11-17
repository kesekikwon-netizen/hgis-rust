//! Runs the cases one after another, each with a fixture of its own and a time limit.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use strata_heritage_fixture::Fixture;
use strata_heritage_run::{start_run, HeritageRunHandle, RunOptions};
use strata_net::heritage_auto::{HeritageConfig, HeritageStage};

use crate::case::{cases, matches, Case};

/// The C++ gives a login about 5.6 s plus two settle ticks (`KaHeritageBrowser.cpp:395,929`);
/// a session's start and the fixture's delay come on top.
const CASE_LIMIT: Duration = Duration::from_secs(45);

struct Live {
    fixture: Fixture,
    run: HeritageRunHandle,
    began: Instant,
    stage_when_done: Option<HeritageStage>,
}

pub struct Runner {
    cases: Vec<Case>,
    at: usize,
    live: Option<Live>,
    work: PathBuf,
    pub outcome: Rc<RefCell<(u32, u32)>>,
}

impl Runner {
    pub fn new(work: PathBuf) -> Self {
        Self {
            cases: cases(),
            at: 0,
            live: None,
            work,
            outcome: Rc::default(),
        }
    }

    fn begin(&mut self, frame: &eframe::Frame) -> Result<Live, String> {
        let case = &self.cases[self.at];
        let fixture = Fixture::start(crate::case::FIXTURE_ACCOUNT, case.city);
        if crate::case::wants_zip(case) {
            fixture.give_zip("지정유산", crate::case::sample_zip()?);
        }
        let options = RunOptions {
            data_parent: self.work.join(format!("case-{}", self.at + 1)).join("profile"),
            config: HeritageConfig {
                account_id: case.run_id.to_owned(),
                account_password: case.run_password.to_owned(),
                sido: case.city.0.to_owned(),
                city: case.city.1.to_owned(),
                fixture_base: Some(fixture.base_url()),
            },
            // The fixture is on this PC, so the switch stays off as in `strata-ngii-run`.
            network_allowed: Arc::new(|| false),
            download_root: self.work.join(format!("case-{}", self.at + 1)).join("받은자료"),
        };
        let run = start_run(frame, options).map_err(|e| e.to_string())?;
        Ok(Live {
            fixture,
            run,
            began: Instant::now(),
            stage_when_done: None,
        })
    }
}

impl eframe::App for Runner {
    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        // A frame arrives; drive the current case one step.
        if self.live.is_none() {
            if self.at >= self.cases.len() {
                ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Close);
                return;
            }
            match self.begin(frame) {
                Ok(live) => self.live = Some(live),
                Err(why) => {
                    println!("case {} FAILED: {why}", self.cases[self.at].name);
                    *self.outcome.borrow_mut() = (self.outcome.borrow().0, self.outcome.borrow().1 + 1);
                    self.at += 1;
                }
            }
            ctx.request_repaint();
            return;
        }

        let Some(live) = self.live.as_mut() else {
            return;
        };
        let events = live.run.frame(frame);
        for event in &events {
            println!("  event: {event:?}");
        }
        let stage = live.run.stage();
        // The case is over when the run ends, or when it has reached the stage the case is
        // about. A case that expects `Done` must not stop at `DownloadAll` on the way there.
        let over = matches!(stage, HeritageStage::Stopped(_) | HeritageStage::Done)
            || matches(&stage, &self.cases[self.at].expect);
        let timed_out = live.began.elapsed() > CASE_LIMIT;
        if over || timed_out || live.run.finished() {
            let _ = &live.fixture;
            let case = &self.cases[self.at];
            let held = matches(&stage, &case.expect);
            let (passed, failed) = *self.outcome.borrow();
            if held {
                println!("case {} ok ({} frames)", case.name, live.began.elapsed().as_millis());
                *self.outcome.borrow_mut() = (passed + 1, failed);
            } else {
                println!(
                    "case {} FAILED: stage {:?}, expected {:?}{}",
                    case.name,
                    stage,
                    case.expect,
                    if timed_out { " (timed out)" } else { "" }
                );
                *self.outcome.borrow_mut() = (passed, failed + 1);
            }
            println!(
                "  fixture hits: checkNet.do {} / j_spring_security_check {}",
                live.fixture.hits("/user/checkNet.do"),
                live.fixture.hits("/j_spring_security_check")
            );
            self.live = None;
            self.at += 1;
            ctx.request_repaint();
            return;
        }
        live.stage_when_done = Some(stage);
        ctx.request_repaint();
    }
}
