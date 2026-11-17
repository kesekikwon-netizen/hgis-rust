//! Accepted offers and quiet completion (`target/cpp-ref/src/app/KaHeritageBrowser.cpp:1150-1217`).

use std::path::{Path, PathBuf};

#[derive(Debug)]
struct Transfer {
    path: PathBuf,
    complete: bool,
}

#[derive(Debug, Default)]
pub(super) struct TransferGroup {
    offers: Vec<Transfer>,
    quiet_since_ms: Option<u64>,
}

impl TransferGroup {
    pub(super) fn offer(&mut self, path: PathBuf) {
        if self.offers.iter().any(|offer| offer.path == path) {
            return;
        }
        self.offers.push(Transfer {
            path,
            complete: false,
        });
        self.quiet_since_ms = None;
    }

    pub(super) fn complete(&mut self, path: &Path, now_ms: u64) {
        let Some(offer) = self.offers.iter_mut().find(|offer| offer.path == path) else {
            return;
        };
        if offer.complete {
            return;
        }
        offer.complete = true;
        if self.offers.iter().all(|offer| offer.complete) {
            self.quiet_since_ms = Some(now_ms);
        }
    }

    pub(super) fn ready(&self, now_ms: u64) -> bool {
        self.quiet_since_ms
            .is_some_and(|since| now_ms.saturating_sub(since) >= 6 * super::LOGIN_POLL_MS)
    }

    pub(super) fn files(&self) -> Vec<PathBuf> {
        self.offers
            .iter()
            .filter(|offer| offer.complete)
            .map(|offer| offer.path.clone())
            .collect()
    }
}
