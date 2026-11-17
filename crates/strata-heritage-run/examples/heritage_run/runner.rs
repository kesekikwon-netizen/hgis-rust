//! Runs the cases one after another, each with a fixture of its own and a time limit.

use raw_window_handle::HasWindowHandle;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use strata_heritage_fixture::{Fixture, Presentation};
use strata_heritage_run::{start_run, DatasetOutcome, HeritageRunHandle, RunOptions};
use strata_net::heritage_auto::{HeritageConfig, HeritageEvent, HeritageStage};

use crate::case::{cases, matches, Case, Scenario};

/// The C++ gives a login about 5.6 s plus two settle ticks (`KaHeritageBrowser.cpp:395,929`);
/// a session's start and the fixture's delay come on top.
const CASE_LIMIT: Duration = Duration::from_secs(90);

struct Live {
    fixture: Fixture,
    run: HeritageRunHandle,
    began: Instant,
    download_root: PathBuf,
    proof: crate::assertions::TransportProof,
    events: Vec<HeritageEvent>,
    initial_profile: Option<PathBuf>,
    retry_prepared: bool,
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

    fn record_start_failure(&mut self) {
        let (passed, failed) = *self.outcome.borrow();
        *self.outcome.borrow_mut() = (passed, failed + 1);
    }

    pub fn planned(&self) -> usize {
        self.cases.len()
    }

    fn begin<W: HasWindowHandle>(&mut self, frame: &W) -> Result<Live, String> {
        let case = &self.cases[self.at];
        let fixture = Fixture::start(crate::case::FIXTURE_ACCOUNT, case.city);
        if crate::case::wants_pledge(case) {
            fixture.require_pledge();
        }
        if crate::case::wants_zip(case) {
            fixture.give_zip("지정유산", crate::case::sample_zip()?);
        }
        if case.scenario == Scenario::RetryOnce {
            fixture.give_zip("지정유산", b"not a zip".to_vec());
        }
        if case.expect == HeritageStage::Cancelled {
            fixture.set_download_delay(1500);
        }
        match case.scenario {
            Scenario::Delayed => fixture.set_page_delay(900),
            Scenario::Absent => fixture.set_empty_results(),
            Scenario::Popup => fixture.set_presentation(Presentation::Popup),
            Scenario::NestedLogin => fixture.set_presentation(Presentation::NestedLogin),
            _ => {}
        }
        let case_root = self.work.join(format!("case-{}", self.at + 1));
        std::fs::create_dir_all(&case_root).map_err(|e| e.to_string())?;
        let download_root = case_root.join("받은자료");
        if case.scenario == Scenario::StorageFailure {
            std::fs::write(&download_root, b"keep original").map_err(|e| e.to_string())?;
        }
        let options = RunOptions {
            data_parent: self
                .work
                .join(format!("case-{}", self.at + 1))
                .join("profile"),
            config: HeritageConfig {
                account_id: case.run_id.to_owned(),
                account_password: case.run_password.to_owned(),
                sido: case.city.0.to_owned(),
                city: case.city.1.to_owned(),
                fixture_base: Some(fixture.base_url()),
            },
            // The fixture is on this PC, so the switch stays off as in `strata-ngii-run`.
            network_allowed: Arc::new(|| false),
            download_root: download_root.clone(),
        };
        let profile_parent = options.data_parent.clone();
        let run = start_run(frame, options).map_err(|e| e.to_string())?;
        let initial_profile = if case.scenario == Scenario::RetryOnce {
            Some(
                std::fs::read_dir(profile_parent)
                    .map_err(|error| error.to_string())?
                    .flatten()
                    .map(|entry| entry.path())
                    .find(|path| path.is_dir())
                    .ok_or("initial fixture browser profile missing")?,
            )
        } else {
            None
        };
        Ok(Live {
            fixture,
            run,
            began: Instant::now(),
            download_root,
            proof: crate::assertions::TransportProof {
                accepted_transfer: false,
                acknowledged: Some(0),
                recreated_session: false,
            },
            events: Vec::new(),
            initial_profile,
            retry_prepared: false,
        })
    }
}

impl Runner {
    pub fn tick<W: HasWindowHandle>(&mut self, frame: &W) -> bool {
        // A frame arrives; drive the current case one step.
        if self.live.is_none() {
            if self.at >= self.cases.len() {
                return true;
            }
            match self.begin(frame) {
                Ok(live) => self.live = Some(live),
                Err(why) => {
                    println!("case {} FAILED: {why}", self.cases[self.at].name);
                    self.record_start_failure();
                    self.at += 1;
                }
            }
            return false;
        }

        let Some(live) = self.live.as_mut() else {
            return false;
        };
        let events = live.run.frame(frame);
        if self.cases[self.at].scenario == Scenario::RetryOnce {
            if live.run.stage() == HeritageStage::RetryWait && !live.retry_prepared {
                live.fixture.give_zip(
                    "지정유산",
                    crate::case::sample_zip().expect("stored fixture ZIP"),
                );
                live.fixture.expire_login();
                live.retry_prepared = true;
            }
            if live.retry_prepared && live.fixture.hits("/j_spring_security_check") == 2 {
                live.proof.recreated_session = live
                    .initial_profile
                    .as_ref()
                    .is_some_and(|old| !old.exists());
            }
        }
        if events
            .iter()
            .any(|event| matches!(event, HeritageEvent::Done))
            && events
                .iter()
                .any(|event| matches!(event, HeritageEvent::DatasetReady { .. }))
        {
            live.proof.acknowledged = None;
        }
        for event in &events {
            println!("  event: {event:?}");
            if let HeritageEvent::DatasetReady { files, .. } = event {
                let stored = crate::case::sample_zip();
                let intact = !files.is_empty()
                    && stored.as_ref().is_ok_and(|expected| {
                        files
                            .iter()
                            .all(|file| std::fs::read(file).is_ok_and(|bytes| bytes == *expected))
                    });
                let waiting = live.run.stage() == HeritageStage::AwaitingImport;
                let outcome = if intact {
                    DatasetOutcome::Accepted
                } else {
                    DatasetOutcome::FatalFailure {
                        reason: "fixture transport bytes differ".into(),
                    }
                };
                let accepted = live.run.dataset_loaded(files, outcome);
                live.proof.acknowledged = if intact && waiting && accepted {
                    live.proof.acknowledged.map(|count| count + 1)
                } else {
                    None
                };
                println!(
                    "  transport ACK accepted={accepted}; stored a.txt fixture, no GIS import"
                );
            }
        }
        live.events.extend(events);
        let stage = live.run.stage();
        if self.cases[self.at].expect == HeritageStage::Cancelled
            && !live.proof.accepted_transfer
            && crate::assertions::offer_was_accepted(&live.download_root, self.cases[self.at].city)
        {
            live.proof.accepted_transfer = true;
            live.run.cancel();
            live.run.cancel();
        }
        // The case is over when the run ends, or when it has reached the stage the case is
        // about. A case that expects `Done` must not stop at `DownloadAll` on the way there.
        let terminal = matches!(
            stage,
            HeritageStage::Stopped(_) | HeritageStage::Done | HeritageStage::Cancelled
        );
        let over = if terminal {
            live.run.finished()
        } else {
            matches(&stage, &self.cases[self.at].expect)
        };
        let timed_out = live.began.elapsed() > CASE_LIMIT;
        if over || timed_out || live.run.finished() {
            let case = &self.cases[self.at];
            let held = !timed_out
                && crate::assertions::held(
                    case,
                    &stage,
                    &live.events,
                    &live.fixture,
                    &live.download_root,
                    &live.proof,
                );
            let (passed, failed) = *self.outcome.borrow();
            if held {
                println!(
                    "case {} ok ({} ms)",
                    case.name,
                    live.began.elapsed().as_millis()
                );
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
            for route in [
                "/fixture/panel-open",
                "/fixture/kind-request",
                "/fixture/search-request",
                "/fixture/unfiltered-download",
                "/fixture/confirm-accepted",
                "/user/data/heritageDownload/downloadFilesAll.do",
            ] {
                println!("  {route}: {}", live.fixture.hits(route));
            }
            self.live = None;
            self.at += 1;
            return false;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::Runner;

    #[test]
    fn start_error_is_counted_without_a_refcell_panic() {
        let mut runner = Runner::new(std::path::PathBuf::new());
        runner.record_start_failure();
        assert_eq!(*runner.outcome.borrow(), (0, 1));
    }
}
