//! `DownloadJobList` storage, worker-channel draining, and expiry rules (`docs/RUST_PLAN.md` §2.8).

use std::fmt;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

use super::{
    DownloadJob, DownloadJobHandle, DownloadJobId, DownloadJobState, DownloadJobUpdate, StopCode,
    CANCELLED_LINGER_SECS, DONE_LINGER_SECS,
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

#[derive(Clone)]
pub struct DownloadJobList {
    jobs: Vec<DownloadJob>,
    next_id: u64,
    pub popup_open: bool,
    tx: Sender<(DownloadJobId, DownloadJobUpdate)>,
    rx: Arc<Mutex<Receiver<(DownloadJobId, DownloadJobUpdate)>>>,
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
        Self { jobs: Vec::new(), next_id: 1, popup_open: false, tx, rx: Arc::new(Mutex::new(rx)) }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.jobs.is_empty()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.jobs.len()
    }

    #[must_use]
    pub fn jobs(&self) -> &[DownloadJob] {
        &self.jobs
    }

    #[must_use]
    pub fn get(&self, id: DownloadJobId) -> Option<&DownloadJob> {
        self.jobs.iter().find(|j| j.id == id)
    }

    pub fn add_job(
        &mut self,
        title: impl Into<String>,
        initial: DownloadJobState,
        now: f64,
    ) -> (DownloadJobId, DownloadJobHandle) {
        let id = DownloadJobId(self.next_id);
        self.next_id += 1;
        let cancel = Arc::new(AtomicBool::new(false));
        let state = sanitize_state(initial);
        let settled_at = settled_time_for(&state, now);
        self.jobs.push(DownloadJob {
            id,
            title: title.into(),
            state,
            cancel: Arc::clone(&cancel),
            settled_at,
        });
        (id, DownloadJobHandle { id, cancel, tx: self.tx.clone() })
    }

    pub fn set_state(&mut self, id: DownloadJobId, state: DownloadJobState, now: f64) -> bool {
        let Some(job) = self.jobs.iter_mut().find(|j| j.id == id) else { return false };
        let clean = sanitize_state(state);
        job.settled_at = settled_time_for(&clean, now);
        job.state = clean;
        true
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
        let mut updates = Vec::new();
        if let Ok(rx) = self.rx.lock() {
            while let Ok(msg) = rx.try_recv() {
                updates.push(msg);
            }
        }
        for (id, upd) in updates {
            let next = match upd {
                DownloadJobUpdate::Waiting => DownloadJobState::Waiting,
                DownloadJobUpdate::Running { fraction, stage } => DownloadJobState::Running { fraction, stage },
                DownloadJobUpdate::Recovering { attempt, reason, retry_in_secs } => DownloadJobState::Recovering {
                    attempt,
                    reason,
                    next_attempt_at: now + retry_in_secs.max(0.0),
                },
                DownloadJobUpdate::Done { result } => DownloadJobState::Done { result },
                DownloadJobUpdate::Stopped { code, reason } => DownloadJobState::Stopped { code, reason },
                DownloadJobUpdate::Cancelled => DownloadJobState::Cancelled,
            };
            let _ = self.set_state(id, next, now);
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

    /// Picture-only presets reachable solely from `--snap` and unit tests (order DL1a).
    pub(crate) fn apply_snap_preset(&mut self, preset: &str, now: f64) {
        *self = Self::new();
        match preset {
            "running" => {
                let (_, h) = self.add_job("DEM", DownloadJobState::Waiting, now);
                h.report_running(Some(0.62), None);
            }
            "recovering" => {
                let (_, h) = self.add_job("DEM", DownloadJobState::Waiting, now);
                h.report_recovering(3, "파일 대신 웹페이지가 돌아왔습니다.", 20.0);
            }
            "stopped" => {
                let (_, h) = self.add_job("DEM", DownloadJobState::Waiting, now);
                let code = StopCode::from_code("ACC-2").unwrap_or(StopCode::Acc2);
                h.report_stopped(code, "로그인이 되지 않았습니다. 아이디·비밀번호를 확인해 주세요.");
            }
            "two" => {
                let (_, h1) = self.add_job("DEM", DownloadJobState::Waiting, now);
                h1.report_running(Some(0.62), None);
                let _ = self.add_job("지적도", DownloadJobState::Waiting, now);
            }
            _ => {}
        }
        self.tick(now);
    }
}
