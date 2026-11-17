//! Run cancellation (`target/cpp-ref/src/app/KaHeritageBrowser.cpp:479-489`).

use super::{HeritageAutomation, HeritageEvent, HeritageStage};

impl HeritageAutomation {
    /// Records an allocation refusal (`target/cpp-ref/src/app/KaHeritageBrowser.cpp:1303-1306`).
    pub fn storage_failed(&mut self, reason: String) {
        if self.is_over() {
            return;
        }
        let event = self.stop(reason);
        self.queued_events.extend(event);
    }

    /// Cancels the run, returning its single cancellation event.
    pub fn cancel(&mut self) -> Option<HeritageEvent> {
        if self.is_over() {
            return None;
        }
        self.pending = None;
        self.offered.clear();
        self.retry_at_ms = 0;
        self.queued_events.clear();
        self.go(HeritageStage::Cancelled);
        Some(HeritageEvent::Cancelled)
    }

    /// Moves the run on and invalidates the previous stage's callback (`:658-659`).
    pub(super) fn go(&mut self, stage: HeritageStage) {
        self.phase_since_ms = self.asked_at_ms;
        if stage == HeritageStage::Kind {
            self.baseline = None;
        }
        self.pending = None;
        self.stage = stage;
        self.quiet_ticks = 0;
    }

    /// A pending part of the same C++ stage retains its navigation deadline (`:871-875`).
    pub(super) fn wait_for(&mut self, stage: HeritageStage) {
        self.pending = None;
        self.stage = stage;
    }

    /// Stops the run with the words the user should see (`:445-477`).
    pub(super) fn stop(&mut self, words: String) -> Option<HeritageEvent> {
        self.go(HeritageStage::Stopped(words.clone()));
        Some(HeritageEvent::Stopped(words))
    }
}
