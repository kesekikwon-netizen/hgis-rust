//! Session policy checks against actual WebView2 dialogs (target/cpp-ref/src/app/KaHeritageBrowser.cpp:119-131).

use std::sync::Arc;

use strata_webview::dialogs::{ConfirmPolicy, DialogKind};
use strata_webview::policy::NavigationPolicy;
use strata_webview::profile::remove_session_folder;
use strata_webview::{BrowserEvent, BrowserOptions, HiddenBrowser, ViewId};

use crate::steps::Probe;

pub fn advance(probe: &mut Probe, enter: bool, frame: &eframe::Frame) -> Option<bool> {
    let step = probe.step_number();
    let policy = if step < 24 {
        ConfirmPolicy::Reject
    } else {
        ConfirmPolicy::Accept
    };
    let expected = if step < 24 {
        "[false,null]"
    } else {
        "[true,null]"
    };
    let main = ViewId(1);
    match (step - 20) % 4 {
        0 => {
            if enter {
                let options = BrowserOptions {
                    data_parent: probe.work.join("profile"),
                    navigation: NavigationPolicy::new(
                        Arc::new(|| false),
                        Some(&probe.site.origin()),
                    ),
                    document_ready_script: None,
                    confirm_policy: policy,
                };
                let mut browser = match HiddenBrowser::start(frame, options) {
                    Ok(browser) => browser,
                    Err(why) => {
                        println!("PROBE dialog session start error: {why}");
                        return Some(false);
                    }
                };
                if browser.navigate(main, &probe.site.url("/dialogs")).is_err() {
                    return Some(false);
                }
                probe.browser = Some(browser);
            }
            let loaded = probe.seen.iter().any(
                |e| matches!(e, BrowserEvent::PageLoaded { url, .. } if url.ends_with("/dialogs")),
            );
            if loaded && probe.call.is_none() {
                probe.run(main, "window.__dialogs");
            }
            probe
                .answer()
                .filter(|_| notices(probe, "main"))
                .map(|json| answer_is(json, expected))
        }
        1 => {
            if enter {
                probe.run(main, "window.frames[0].frames[0].__dialogs");
            }
            probe.answer().map(|json| answer_is(json, expected))
        }
        2 => {
            if enter {
                probe.run(main, "(function () { var w = window.open('about:blank'); var a = [w.confirm('popup confirm'), w.prompt('popup prompt')]; w.alert('popup alert'); w.close(); return a; })()");
            }
            probe
                .answer()
                .filter(|_| notices(probe, "popup"))
                .map(|json| answer_is(json, expected))
        }
        3 => {
            if enter {
                probe.released = probe.browser.take().map(HiddenBrowser::shut_down);
            }
            probe
                .released
                .as_deref()
                .is_some_and(remove_session_folder)
                .then_some(true)
        }
        _ => None,
    }
}

fn answer_is(json: &str, expected: &str) -> bool {
    if json != expected {
        println!("PROBE dialog result: expected {expected}, received {json}");
    }
    json == expected
}

fn notices(probe: &Probe, prefix: &str) -> bool {
    [(DialogKind::Confirm, "confirm"), (DialogKind::Prompt, "prompt"), (DialogKind::Alert, "alert")]
        .into_iter()
        .all(|(kind, word)| probe.seen.iter().any(|e| matches!(e, BrowserEvent::Dialog { kind: k, text, .. } if *k == kind && *text == format!("{prefix} {word}"))))
}

pub const NAMES: [&str; 28] = [
    "the local site loads in a hidden view with the network switch off",
    "a script's value comes back as JSON",
    "a popup becomes a second hidden view with its opener",
    "the popup page sees its window name and its opener's data",
    "confirm, alert and prompt are answered silently and reported",
    "a file is written to a part file and put in place under a safe name",
    "a refused file leaves nothing behind",
    "a non-web address is refused and reported",
    "a page cannot leave for a web address while the switch is off",
    "the ready script runs when the document is parsed, in the main frame only",
    "a hidden view lays its page out at desktop size",
    "a popup that closes itself is reported and its view is gone",
    "a new popup opens afterwards and its opener can close it",
    "no request reaches another address while the switch is off",
    "the first empty page of a popup has quiet dialogs too",
    "a javascript: link runs in its page and is no navigation",
    "a second file from the same page arrives complete as well",
    "a refused second offer leaves the running transfer alone, then the session is shut down",
    "shut down leaves no part file and the data folder can be removed",
    "a session that is simply dropped lets go of its folder too",
    "Reject answers false before the main page's inline script and preserves notices/prompt cancellation",
    "Reject answers false before the nested frame's inline script and cancels its prompt",
    "Reject answers false in an immediately used popup and preserves notices/prompt cancellation",
    "the Reject session releases its data folder",
    "Accept answers true before the main page's inline script and preserves notices/prompt cancellation",
    "Accept answers true before the nested frame's inline script and cancels its prompt",
    "Accept answers true in an immediately used popup and preserves notices/prompt cancellation",
    "the Accept session releases its data folder",
];
