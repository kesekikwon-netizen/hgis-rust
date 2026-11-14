//! `DownloadJobList` storage, worker-channel draining, and expiry rules (`docs/RUST_PLAN.md` §2.8).

use std::fmt;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, Weak};

use super::{
    DownloadJob, DownloadJobHandle, DownloadJobId, DownloadJobState, DownloadJobUpdate, StopCode,
    APP1_STOP_REASON, CANCELLED_LINGER_SECS, DONE_LINGER_SECS,
};

fn sanitize_state(state: DownloadJobState) -> DownloadJobState {
    match state {
        DownloadJobState::Running { fraction, stage } => DownloadJobState::Running {
            fraction: fraction.filter(|f| f.is_finite()).map(|f| f.clamp(0.0, 1.0)),
            stage: stage.filter(|s| !s.trim().is_empty()),
        },
        other => other,
    }
}

fn settled_time_for(state: &DownloadJobState, now: f64) -> Option<f64> {
    matches!(state, DownloadJobState::Done { .. } | DownloadJobState::Cancelled).then_some(now)
}

pub struct DownloadJobList {
    jobs: Vec<DownloadJob>,
    next_id: u64,
    pub popup_open: bool,
    clock: Arc<Mutex<f64>>,
    tx: Sender<(DownloadJobId, u64, DownloadJobUpdate)>,
    rx: Mutex<Receiver<(DownloadJobId, u64, DownloadJobUpdate)>>,
    preset_guards: Vec<Arc<()>>,
}

impl Clone for DownloadJobList {
    /// Produces a detached snapshot with its own channel, clock, and per-job cancel flags
    /// so ticking or cancelling a clone never drains or cancels the original list's workers.
    fn clone(&self) -> Self {
        let (tx, rx) = mpsc::channel();
        let now = self.clock.lock().map_or(0.0, |c| *c);
        let jobs = self
            .jobs
            .iter()
            .map(|j| DownloadJob {
                id: j.id,
                title: j.title.clone(),
                state: j.state.clone(),
                run_seq: j.run_seq,
                cancel: Arc::new(AtomicBool::new(j.is_cancel_requested())),
                settled_at: j.settled_at,
                worker_alive: Weak::clone(&j.worker_alive),
            })
            .collect();
        Self {
            jobs,
            next_id: self.next_id,
            popup_open: self.popup_open,
            clock: Arc::new(Mutex::new(now)),
            tx,
            rx: Mutex::new(rx),
            preset_guards: self.preset_guards.clone(),
        }
    }
}

impl Default for DownloadJobList {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for DownloadJobList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DownloadJobList")
            .field("jobs", &self.jobs)
            .field("popup_open", &self.popup_open)
            .finish_non_exhaustive()
    }
}

impl DownloadJobList {
    #[must_use]
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self { jobs: Vec::new(), next_id: 1, popup_open: false, clock: Arc::new(Mutex::new(0.0)), tx, rx: Mutex::new(rx), preset_guards: Vec::new() }
    }

    fn update_clock(&self, now: f64) {
        if now.is_finite() {
            if let Ok(mut c) = self.clock.lock() {
                *c = now;
            }
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool { self.jobs.is_empty() }

    #[must_use]
    pub fn len(&self) -> usize { self.jobs.len() }

    #[must_use]
    pub fn jobs(&self) -> &[DownloadJob] { &self.jobs }

    #[must_use]
    pub fn get(&self, id: DownloadJobId) -> Option<&DownloadJob> { self.jobs.iter().find(|j| j.id == id) }

    pub fn add_job(
        &mut self,
        title: impl Into<String>,
        initial: DownloadJobState,
        now: f64,
    ) -> (DownloadJobId, DownloadJobHandle) {
        self.update_clock(now);
        let id = DownloadJobId(self.next_id);
        self.next_id += 1;
        let cancel = Arc::new(AtomicBool::new(false));
        let alive = Arc::new(());
        let state = sanitize_state(initial);
        let settled_at = settled_time_for(&state, now);
        self.jobs.push(DownloadJob {
            id,
            title: title.into(),
            state,
            run_seq: 1,
            cancel: Arc::clone(&cancel),
            settled_at,
            worker_alive: Arc::downgrade(&alive),
        });
        let handle = DownloadJobHandle {
            id,
            run_seq: 1,
            cancel,
            tx: self.tx.clone(),
            clock: Arc::clone(&self.clock),
            alive,
        };
        (id, handle)
    }

    /// Updates an unfinished job. Once a job reaches `Done`, `Cancelled`, or `Stopped`,
    /// late updates are ignored; starting again requires `add_job` or `restart_job`.
    pub fn set_state(&mut self, id: DownloadJobId, state: DownloadJobState, now: f64) -> bool {
        self.update_clock(now);
        let Some(job) = self.jobs.iter_mut().find(|j| j.id == id) else { return false };
        if !job.is_unfinished() {
            return false;
        }
        let clean = sanitize_state(state);
        job.settled_at = settled_time_for(&clean, now);
        job.state = clean;
        true
    }

    /// Explicitly restarts an existing job with a fresh generation, cancel flag, and worker handle.
    pub fn restart_job(
        &mut self,
        id: DownloadJobId,
        initial: DownloadJobState,
        now: f64,
    ) -> Option<DownloadJobHandle> {
        self.update_clock(now);
        let job = self.jobs.iter_mut().find(|j| j.id == id)?;
        job.run_seq += 1;
        let run_seq = job.run_seq;
        let cancel = Arc::new(AtomicBool::new(false));
        let alive = Arc::new(());
        let clean = sanitize_state(initial);
        job.settled_at = settled_time_for(&clean, now);
        job.state = clean;
        job.cancel = Arc::clone(&cancel);
        job.worker_alive = Arc::downgrade(&alive);
        Some(DownloadJobHandle { id, run_seq, cancel, tx: self.tx.clone(), clock: Arc::clone(&self.clock), alive })
    }

    /// Sets only the job's cancel flag; the worker responds by reporting `Cancelled`.
    pub fn request_cancel(&self, id: DownloadJobId) -> bool {
        let Some(job) = self.jobs.iter().find(|j| j.id == id) else { return false };
        job.request_cancel();
        true
    }

    pub fn dismiss(&mut self, id: DownloadJobId) -> bool {
        let before = self.jobs.len();
        self.jobs.retain(|j| j.id != id);
        if self.jobs.is_empty() {
            self.popup_open = false;
        }
        self.jobs.len() != before
    }

    pub fn tick(&mut self, now: f64) {
        self.update_clock(now);
        let mut updates = Vec::new();
        if let Ok(rx) = self.rx.lock() {
            while let Ok(msg) = rx.try_recv() {
                updates.push(msg);
            }
        }
        for (id, seq, upd) in updates {
            if !self.jobs.iter().any(|j| j.id == id && j.run_seq == seq) {
                continue;
            }
            let next = match upd {
                DownloadJobUpdate::Waiting => DownloadJobState::Waiting,
                DownloadJobUpdate::Running { fraction, stage } => DownloadJobState::Running { fraction, stage },
                DownloadJobUpdate::Recovering { attempt, reason, next_attempt_at } => {
                    DownloadJobState::Recovering { attempt, reason, next_attempt_at }
                }
                DownloadJobUpdate::Done { result } => DownloadJobState::Done { result },
                DownloadJobUpdate::Stopped { code, reason } => DownloadJobState::Stopped { code, reason },
                DownloadJobUpdate::Cancelled => DownloadJobState::Cancelled,
            };
            let _ = self.set_state(id, next, now);
        }
        for job in &mut self.jobs {
            if job.is_unfinished() && job.worker_alive.strong_count() == 0 {
                job.state = DownloadJobState::Stopped {
                    code: StopCode::App1,
                    reason: APP1_STOP_REASON.to_owned(),
                };
                job.settled_at = None;
            }
        }
        self.jobs.retain(|job| match job.state {
            DownloadJobState::Done { .. } => job.settled_at.is_none_or(|t| now - t < DONE_LINGER_SECS),
            DownloadJobState::Cancelled => job.settled_at.is_none_or(|t| now - t < CANCELLED_LINGER_SECS),
            _ => true,
        });
        if self.jobs.is_empty() {
            self.popup_open = false;
        }
    }

    /// Picture-only presets reachable solely from `--snap` and unit tests (orders DL1a, DL1b).
    pub(crate) fn apply_snap_preset(&mut self, preset: &str, now: f64) {
        *self = Self::new();
        match preset {
            "running" => {
                let (_, h) = self.add_job("DEM", DownloadJobState::Waiting, now);
                h.report_running(Some(0.62), None);
                self.preset_guards.push(Arc::clone(&h.alive));
            }
            "recovering" => {
                let (_, h) = self.add_job("DEM", DownloadJobState::Waiting, now);
                h.report_recovering(3, "파일 대신 웹페이지가 돌아왔습니다.", 20.0);
                self.preset_guards.push(Arc::clone(&h.alive));
            }
            "stopped" => {
                let (_, h) = self.add_job("DEM", DownloadJobState::Waiting, now);
                let code = StopCode::from_code("ACC-2").unwrap_or(StopCode::Acc2);
                h.report_stopped(code, "로그인이 되지 않았습니다. 아이디·비밀번호를 확인해 주세요.");
            }
            "two" => {
                let (_, h1) = self.add_job("DEM", DownloadJobState::Waiting, now);
                h1.report_running(Some(0.62), None);
                self.preset_guards.push(Arc::clone(&h1.alive));
                let (_, h2) = self.add_job("지적도", DownloadJobState::Waiting, now);
                self.preset_guards.push(Arc::clone(&h2.alive));
            }
            "two_stopped" => {
                let (_, h1) = self.add_job("DEM", DownloadJobState::Waiting, now);
                h1.report_running(Some(0.62), None);
                self.preset_guards.push(Arc::clone(&h1.alive));
                let (_, h2) = self.add_job("지적도", DownloadJobState::Waiting, now);
                h2.report_stopped(
                    StopCode::Acc2,
                    "로그인이 되지 않았습니다. 아이디·비밀번호를 확인해 주세요.",
                );
            }
            "none1" => {
                let (_, h) = self.add_job("DEM", DownloadJobState::Waiting, now);
                // Verbatim C++ empty-area text (`target/cpp-ref/src/core/DemDownloadService.cpp:207`).
                h.report_stopped(
                    StopCode::None1,
                    "이 범위에는 제공되는 DEM 자료가 없습니다. 바다를 제외한 조사구역으로 이동하고 다시 내려받으세요.",
                );
            }
            _ => {}
        }
        self.tick(now);
    }
}

#[cfg(test)]
#[path = "download_jobs_list_tests.rs"]
mod tests;
