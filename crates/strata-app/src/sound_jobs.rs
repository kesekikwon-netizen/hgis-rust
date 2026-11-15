//! The download gauge and the sounds: what the jobs of the gauge are to the sound controller, and the weak
//! sound of a gauge that works (owner 2026-11-15: 「다운로드 받을때 게이지에서도 소리가 약하게나게하라. 완료시 에도」).

use std::hash::{Hash, Hasher};
use std::time::Duration;

use strata_types::theme_display::SoundLevel;

use super::{SoundController, SoundJobPhase, SoundJobSnapshot, SoundKind, PROGRESS_MIN_INTERVAL};
use crate::download_jobs::{DownloadJob, DownloadJobState, StopCode};

#[cfg(test)]
#[path = "sound_jobs_tests.rs"]
mod tests;

/// While any job runs the gauge sounds at least this often.
const HEARTBEAT: Duration = Duration::from_millis(2500);

/// The jobs of the gauge for `observe_jobs`, and for `observe_running` the running ones with the step of
/// their work (job id, a number that changes when the job's stage does). A job that waits or was cancelled
/// is not heard.
pub(super) fn heard(jobs: &[DownloadJob]) -> (Vec<SoundJobSnapshot>, Vec<(u64, u64)>) {
    let (mut snapshots, mut running) = (Vec::new(), Vec::new());
    for job in jobs {
        let (fraction, phase) = match &job.state {
            DownloadJobState::Waiting | DownloadJobState::Cancelled => continue,
            DownloadJobState::Running { fraction, stage } => {
                let mut step = std::collections::hash_map::DefaultHasher::new();
                stage.hash(&mut step);
                running.push((job.id.0, step.finish()));
                (*fraction, SoundJobPhase::Running)
            }
            DownloadJobState::Recovering { .. } => (None, SoundJobPhase::Recovering),
            DownloadJobState::Done { .. } => (None, SoundJobPhase::Done),
            // NONE-1 only says there is no data for the area: nothing the user has to mend.
            DownloadJobState::Stopped { code, .. } => (None, SoundJobPhase::Stopped { needs_user: *code != StopCode::None1 }),
        };
        snapshots.push(SoundJobSnapshot { id: job.id.0, fraction, phase });
    }
    (snapshots, running)
}

impl SoundController {
    /// The weak sound of a gauge that works: when a job starts to run or comes to another step of its
    /// work (at most once a second), and every 2.5 s while any job runs. `running`: the running jobs with
    /// the step of their work.
    pub fn observe_running(&mut self, running: &[(u64, u64)], now: Duration, level: SoundLevel) {
        let stepped = running.iter().any(|(id, step)| self.steps.get(id) != Some(step));
        self.steps = running.iter().copied().collect();
        if running.is_empty() || level == SoundLevel::Off {
            return;
        }
        let since = |wait: Duration| self.last_progress_at.is_none_or(|at| now.saturating_sub(at) >= wait);
        if (stepped && since(PROGRESS_MIN_INTERVAL)) || since(HEARTBEAT) {
            self.last_progress_at = Some(now);
            self.stage_or_emit_info(SoundKind::DownloadProgress, level);
        }
    }
}
