//! The last checks of the probe (steps 16 to 18): transfers of one address, and how a session ends.

use std::time::Duration;

use strata_webview::downloads::DownloadRefusal;
use strata_webview::profile::remove_session_folder;
use strata_webview::{BrowserEvent, HiddenBrowser, ViewId};

use crate::steps::Probe;

pub fn advance(probe: &mut Probe, enter: bool, frame: &eframe::Frame) -> Option<bool> {
    let main = ViewId(1);
    let seen = |probe: &Probe, wanted: fn(&BrowserEvent) -> bool| probe.seen.iter().any(wanted);
    match probe.step_number() {
        16 => {
            if enter {
                probe.accept.set(true);
                probe.run(main, "location.href = '/slow.zip'; 1");
            }
            let running = seen(probe, |e| matches!(e, BrowserEvent::DownloadStarted { file_name, .. } if file_name == "slow.zip"));
            // The same address is offered again while the first transfer runs, and is refused: the end the
            // engine reports for the refused one must not be taken for the end of the running one.
            if running && probe.accept.get() && probe.step_started.elapsed() > Duration::from_millis(700) {
                probe.accept.set(false);
                probe.run(main, "location.href = '/slow.zip'; 1");
            }
            let refused = seen(probe, |e| matches!(e, BrowserEvent::DownloadRefused { why: DownloadRefusal::Declined, .. }));
            (refused && probe.step_started.elapsed() > Duration::from_millis(2200)).then(|| {
                let failed = seen(probe, |e| matches!(e, BrowserEvent::DownloadFailed { .. }));
                let part_alive = probe.files().iter().any(|f| f.starts_with("slow.zip.") && f.ends_with(".part"));
                if failed || !part_alive {
                    println!("PROBE detail: failed event {failed}; files {:?}", probe.files());
                }
                probe.released = probe.browser.take().map(HiddenBrowser::shut_down);
                !failed && part_alive && probe.released.is_some()
            })
        }
        17 => {
            // The engine's processes let go of their files a moment after the views are closed.
            let folder_gone = probe.released.as_deref().is_some_and(remove_session_folder);
            (folder_gone && probe.files() == vec!["_evil name_.zip".to_owned()]).then_some(true)
        }
        18 => {
            if enter {
                if probe.start(frame).is_err() {
                    return Some(false);
                }
                probe.released = probe.browser.as_ref().map(|b| b.data_folder().to_path_buf());
            }
            let loaded = seen(probe, |e| matches!(e, BrowserEvent::PageLoaded { url, .. } if url.ends_with("/main")));
            if loaded && probe.browser.is_some() {
                // No shut_down: the session is just dropped, with its spare views.
                probe.browser = None;
            }
            (probe.browser.is_none() && probe.released.as_deref().is_some_and(remove_session_folder)).then_some(true)
        }
        _ => Some(false),
    }
}
