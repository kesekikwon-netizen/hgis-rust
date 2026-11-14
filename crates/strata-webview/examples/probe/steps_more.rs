//! Further checks of the probe (steps 10 to 16): layout in a hidden view, windows that pages close, requests
//! to other addresses, the first empty page of a popup.

use strata_webview::{BrowserEvent, HiddenBrowser, ViewId};

use crate::steps::Probe;

fn closed(probe: &Probe, view: ViewId) -> bool {
    probe.seen.iter().any(|e| matches!(e, BrowserEvent::ViewClosed { view: v } if *v == view))
}

fn open_views(probe: &Probe) -> Vec<ViewId> {
    probe.browser.as_ref().map(HiddenBrowser::views).unwrap_or_default().iter().map(|v| v.id).collect()
}

pub fn advance(probe: &mut Probe, enter: bool) -> Option<bool> {
    let main = ViewId(1);
    match probe.step_number() {
        10 => {
            // The page script of the NGII site only clicks what is laid out (`getClientRects`,
            // ngii-topographic.js:6-11): a hidden view must still compute layout.
            if enter {
                probe.run(main, "document.body.getClientRects().length > 0 && window.innerWidth >= 1000 && window.innerHeight >= 600");
            }
            probe.answer().map(|json| json == "true")
        }
        11 => {
            let Some(popup) = probe.popup else {
                return Some(false);
            };
            if enter {
                probe.run(popup, "window.close(); 1");
            }
            closed(probe, popup).then(|| open_views(probe) == vec![main])
        }
        12 => {
            if enter {
                probe.popup = None;
                // The first view shows the /ready page by now, which has no pop() of its own.
                probe.run(main, "window.__p = window.open('/pop', 'downForm'); !!window.__p");
            }
            let opened = probe.seen.iter().find_map(|e| match e {
                BrowserEvent::PopupOpened { view, .. } => Some(*view),
                _ => None,
            });
            let loaded = opened.is_some_and(|p| probe.seen.iter().any(|e| matches!(e, BrowserEvent::PageLoaded { view, .. } if *view == p)));
            if let (None, Some(p), true) = (probe.popup, opened, loaded) {
                probe.popup = Some(p);
                probe.run(main, "window.__p.close(); 1");
            }
            probe.popup.filter(|p| closed(probe, *p)).map(|_| open_views(probe) == vec![main])
        }
        13 => {
            // Everything a page can ask of another address without leaving its own: none of it may arrive.
            if enter {
                let other = probe.other.origin();
                let script = format!(
                    "(function (b) {{ try {{ fetch(b + '/fetch', {{mode: 'no-cors'}}).catch(function () {{}}); }} catch (e) {{}}                      new Image().src = b + '/img'; var f = document.createElement('iframe'); f.src = b + '/iframe'; document.body.appendChild(f);                      try {{ navigator.sendBeacon(b + '/beacon'); }} catch (e) {{}}                      setTimeout(function () {{ location.href = b + '/top'; }}, 300); return 1; }})('{other}')"
                );
                probe.run(main, &script);
            }
            (probe.step_started.elapsed() > std::time::Duration::from_millis(2500)).then(|| {
                let hits = probe.other.hits();
                if !hits.is_empty() {
                    println!("PROBE detail: the other site was reached: {hits:?}");
                }
                hits.is_empty() && probe.answer() == Some("1")
            })
        }
        14 => {
            // A page may call a dialog of a window it has just opened, before any page is loaded there.
            if enter {
                probe.run(main, "(function () { var w = window.open('about:blank'); var s = String(w.alert); w.close(); return s.indexOf('[native code]') < 0; })()");
            }
            probe.answer().map(|json| json == "true")
        }
        15 => {
            // Sites of this kind are full of javascript: links. They are script, not addresses: the engine
            // must run them without asking the navigation rule (which would refuse the scheme).
            if enter {
                probe.run(main, "var a = document.createElement('a'); a.href = 'javascript:void(window.__j = 7)'; document.body.appendChild(a); a.click(); 1");
            }
            let refused = probe.seen.iter().any(|e| matches!(e, BrowserEvent::NavigationRefused { .. }));
            match (probe.part, probe.answer()) {
                (0, Some("1")) | (1, Some("null")) => {
                    probe.part = 1;
                    probe.run(main, "window.__j === undefined ? null : window.__j");
                    None
                }
                (1, Some(json)) => Some(json == "7" && !refused),
                _ => None,
            }
        }
        16 => {
            if enter {
                probe.accept.set(true);
                probe.run(main, "location.href = '/second.xml'; 1");
            }
            probe.seen.iter().find_map(|e| match e {
                BrowserEvent::DownloadFinished { path, bytes, .. } => Some(path.ends_with("second.xml") && *bytes == 23 && path.is_file()),
                BrowserEvent::DownloadFailed { .. } => Some(false),
                _ => None,
            })
        }
        _ => Some(false),
    }
}
