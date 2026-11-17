//! Import acceptance and fresh-session retry (`target/cpp-ref/src/app/KaHeritageBrowser.cpp:491-562`).

use super::{DatasetOutcome, HeritageAutomation, HeritageEvent, HeritageStage};
use std::path::PathBuf;

const RETRY_DELAYS_MS: [u64; 4] = [5000, 10000, 20000, 30000];

impl HeritageAutomation {
    /// Consumes only the exact pending group (`KaHeritageBrowser.cpp:1206-1217`).
    /// Resulting events are drained by frame/timers; this never reenters a browser callback.
    pub fn dataset_loaded(&mut self, files: &[PathBuf], outcome: DatasetOutcome) -> bool {
        if self.stage != HeritageStage::AwaitingImport
            || files.is_empty()
            || self.group.files() != files
        {
            return false;
        }
        self.group = super::transfer_group::TransferGroup::default();
        let event = match outcome {
            DatasetOutcome::Accepted => self.advance_kind(),
            DatasetOutcome::RetryableFailure { reason } => {
                self.schedule_retry(&reason, self.observed_at_ms)
            }
            DatasetOutcome::FatalFailure { reason } => self.stop(reason),
        };
        self.queued_events.extend(event);
        true
    }

    /// Closes the failed browser before any new authentication (`:533-562`).
    #[must_use]
    pub fn session_restart_required(&self) -> bool {
        self.stage == HeritageStage::RetryWait
    }

    /// Confirms actual fresh-browser construction after backoff (`:533-562`).
    /// The next next_step navigates once from Idle to the captured account's login page.
    pub fn session_reopened(&mut self, now_ms: u64) -> bool {
        if !self.retry_due(now_ms) {
            return false;
        }
        self.observe_time(now_ms);
        self.group = super::transfer_group::TransferGroup::default();
        self.baseline = None;
        self.pledge = None;
        self.retry_at_ms = 0;
        self.asked_at_ms = now_ms;
        self.go(HeritageStage::Idle);
        true
    }

    /// A failed transfer retries its current kind, never authentication failure (`:1360-1372`).
    pub fn transfer_failed(&mut self, reason: &str, now_ms: u64) -> Option<HeritageEvent> {
        if !self.accepts_files() {
            return None;
        }
        self.schedule_retry(reason, now_ms)
    }

    fn schedule_retry(&mut self, reason: &str, now_ms: u64) -> Option<HeritageEvent> {
        self.observe_time(now_ms);
        self.retry_count = self.retry_count.saturating_add(1);
        let index = usize::try_from(self.retry_count - 1).unwrap_or(3).min(3);
        self.retry_at_ms = now_ms.saturating_add(RETRY_DELAYS_MS[index]);
        self.group = super::transfer_group::TransferGroup::default();
        self.go(HeritageStage::RetryWait);
        Some(HeritageEvent::Note(format!(
            "{reason} · 정상 파일을 다시 받습니다 · 중지 가능"
        )))
    }

    /// The C++ caps delay at 30 seconds, without an attempt cap (`:504-520`).
    #[must_use]
    pub fn retry_due(&self, now_ms: u64) -> bool {
        self.session_restart_required() && now_ms >= self.retry_at_ms
    }
}
