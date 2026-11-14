//! The parts of a session that are not its frame-by-frame work: spare views, the end of a transfer, and
//! closing everything.

use std::path::PathBuf;

use raw_window_handle::HasWindowHandle;

use crate::downloads::{discard_download, finish_download};
use crate::events::{BrowserEvent, ViewId};
use crate::session::{HiddenBrowser, SPARE_ATTEMPTS, SPARE_VIEWS};
use crate::view::{build_hidden_view, Engine};

impl HiddenBrowser {
    /// Builds one spare view when one is missing.
    pub(crate) fn add_spare<W: HasWindowHandle>(&mut self, parent: &W) {
        if self.spare_failures >= SPARE_ATTEMPTS || self.shared.spare.borrow().len() >= SPARE_VIEWS {
            return;
        }
        let Some(first) = self.views.first() else {
            return;
        };
        let id = ViewId(self.next_view);
        match build_hidden_view(parent, id, Engine::SameAs(&first.view), &self.shared, &self.tx) {
            Ok(view) => {
                self.next_view += 1;
                self.spare_failures = 0;
                self.shared.spare.borrow_mut().push((id, view));
            }
            // Without a spare view the next popup is refused and reported; nothing else breaks.
            Err(_) => self.spare_failures += 1,
        }
    }

    /// The engine says a download ended. A complete one names its file; one that broke off or was refused
    /// names only its address.
    pub(crate) fn transfer_ended(&mut self, part_path: Option<PathBuf>, url: &str, success: bool, events: &mut Vec<BrowserEvent>) {
        if part_path.is_none() {
            if let Some(at) = self.refused_ends.iter().position(|refused| refused == url) {
                self.refused_ends.remove(at);
                return;
            }
        }
        let at = self.transfers.iter().position(|t| part_path.as_ref().map_or(t.url == url, |p| *p == t.plan.part_path));
        let Some(transfer) = at.map(|i| self.transfers.remove(i)) else {
            return;
        };
        match success.then(|| finish_download(&transfer.plan)) {
            Some(Ok(bytes)) => events.push(BrowserEvent::DownloadFinished { id: transfer.id, path: transfer.plan.final_path, bytes }),
            _ => {
                discard_download(&transfer.plan);
                events.push(BrowserEvent::DownloadFailed { id: transfer.id, url: transfer.url });
            }
        }
    }

    /// Closes every view (the spare ones hold the shared state that holds them: review of 2026-11-15, S4)
    /// and removes what unfinished transfers left.
    pub(crate) fn close_everything(&mut self) {
        self.views.clear();
        if let Ok(mut spare) = self.shared.spare.try_borrow_mut() {
            spare.clear();
        }
        if let Ok(mut adopted) = self.shared.adopted.try_borrow_mut() {
            adopted.clear();
        }
        if let Ok(mut decider) = self.shared.decider.try_borrow_mut() {
            *decider = None;
        }
        for transfer in self.transfers.drain(..) {
            discard_download(&transfer.plan);
        }
    }
}
