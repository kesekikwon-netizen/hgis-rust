//! Run cancellation (`target/cpp-ref/src/app/KaHeritageBrowser.cpp:479-489`).

use super::{HeritageAutomation, HeritageEvent, HeritageStage};

impl HeritageAutomation {
    /// Cancels the run, returning its single cancellation event.
    pub fn cancel(&mut self) -> Option<HeritageEvent> {
        if self.is_over() {
            return None;
        }
        self.pending = None;
        self.offered.clear();
        self.retry_at_ms = 0;
        self.go(HeritageStage::Cancelled);
        Some(HeritageEvent::Cancelled)
    }

    /// Moves the run on and invalidates the previous stage's callback (`:658-659`).
    pub(super) fn go(&mut self, stage: HeritageStage) {
        self.pending = None;
        self.stage = stage;
        self.quiet_ticks = 0;
    }

    /// Stops the run with the words the user should see (`:445-477`).
    pub(super) fn stop(&mut self, words: String) -> Option<HeritageEvent> {
        self.go(HeritageStage::Stopped(words.clone()));
        Some(HeritageEvent::Stopped(words))
    }
}
