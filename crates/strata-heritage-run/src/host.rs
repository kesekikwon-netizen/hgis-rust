//! Native document ownership (`target/cpp-ref/src/app/KaHeritageBrowser.cpp:639-778`).

#[path = "host_scope.rs"]
mod scope;
use scope::Scope;

use strata_net::heritage_auto::{
    scripts::{document_probe_script, parse_document_probe, DocumentFacts},
    HeritageAutomation, HeritageEvent, HeritageStage, PageHost, PageId, PageRef,
    ScriptCall as EngineCall,
};
use strata_webview::{HiddenBrowser, ScriptCall as BrowserCall, ViewId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Wanted {
    Script {
        page: PageId,
        script: String,
        token: u64,
    },
    Navigate(String),
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SessionEpoch(u64);
#[derive(Debug, Clone, Copy)]
struct Ticket {
    epoch: SessionEpoch,
    token: u64,
}
#[derive(Debug)]
struct Probe {
    page: PageId,
    call: Option<BrowserCall>,
    facts: Option<DocumentFacts>,
}
#[derive(Debug)]
enum Phase {
    Probing(Vec<Probe>),
    Acting { page: PageId, call: BrowserCall },
}
struct Pending {
    ticket: Ticket,
    request: EngineCall,
    scope: Scope,
    phase: Phase,
}
#[derive(Default)]
pub(crate) struct QueuedHost {
    pub(crate) pages: Vec<PageRef>,
    pub(crate) wanted: Vec<Wanted>,
    tokens: u64,
    epoch: SessionEpoch,
    pending: Option<Pending>,
}

impl QueuedHost {
    pub(crate) fn epoch(&self) -> SessionEpoch {
        self.epoch
    }

    /// Call before browser recreation, cancel or terminal cleanup (`KaHeritageBrowser.cpp:458,485`).
    pub(crate) fn reset_documents(&mut self) {
        self.pending = None;
        self.wanted.clear();
        self.epoch.0 = self.epoch.0.wrapping_add(1);
    }

    pub(crate) fn begin(
        &mut self,
        browser: &mut HiddenBrowser,
        machine: &HeritageAutomation,
        request: EngineCall,
    ) {
        if self.pending.is_some() || !machine.accepts_script_token(request.token) {
            return;
        }
        let script = document_probe_script();
        let probes = self
            .pages
            .iter()
            .map(|page| Probe {
                page: page.id,
                call: browser.run_script(ViewId(page.id.0), &script).ok(),
                facts: None,
            })
            .collect();
        self.pending = Some(Pending {
            ticket: Ticket {
                epoch: self.epoch,
                token: request.token,
            },
            scope: Scope::for_stage(machine.stage()),
            request,
            phase: Phase::Probing(probes),
        });
    }

    pub(crate) fn record_probe(
        &mut self,
        epoch: SessionEpoch,
        call: BrowserCall,
        json: &str,
    ) -> bool {
        if epoch != self.epoch {
            return false;
        }
        let Some(Pending {
            ticket,
            phase: Phase::Probing(probes),
            ..
        }) = self.pending.as_mut()
        else {
            return false;
        };
        if ticket.epoch != epoch {
            return false;
        }
        let Some(probe) = probes.iter_mut().find(|probe| probe.call == Some(call)) else {
            return false;
        };
        probe.call = None;
        probe.facts = parse_document_probe(json);
        true
    }

    pub(crate) fn action_answer(
        &mut self,
        epoch: SessionEpoch,
        call: BrowserCall,
        json: &str,
        machine: &mut HeritageAutomation,
    ) -> Option<HeritageEvent> {
        if epoch != self.epoch {
            return None;
        }
        let pending = self.pending.as_ref()?;
        let Phase::Acting {
            page,
            call: expected,
        } = pending.phase
        else {
            return None;
        };
        if call != expected {
            return None;
        }
        let valid =
            pending.ticket.epoch == epoch && machine.accepts_script_token(pending.ticket.token);
        let live = self.pages.iter().any(|view| view.id == page);
        let token = pending.ticket.token;
        let empty = pending.scope.empty();
        self.pending = None;
        valid
            .then(|| machine.script_answered(token, if live { json } else { empty }))
            .flatten()
    }

    fn take_ready(&mut self, token_valid: bool) -> Option<(Pending, Option<PageId>)> {
        let pending = self.pending.as_mut()?;
        if !token_valid || pending.ticket.epoch != self.epoch {
            self.pending = None;
            return None;
        }
        let Phase::Probing(probes) = &mut pending.phase else {
            return None;
        };
        for probe in probes.iter_mut() {
            if !self.pages.iter().any(|live| live.id == probe.page) {
                probe.call = None;
                probe.facts = None;
            }
        }
        if probes.iter().any(|probe| probe.call.is_some()) {
            return None;
        }
        // min_by_key keeps the first equal candidate, independent of callback completion order.
        // C++ uses the smallest inventory index (`target/cpp-ref/src/app/KaHeritageBrowser.cpp:718-720`).
        let page = probes
            .iter()
            .filter_map(|probe| {
                pending
                    .scope
                    .rank(probe.facts?)
                    .map(|rank| (rank, probe.page))
            })
            .min_by_key(|(rank, _)| *rank)
            .map(|(_, page)| page);
        self.pending.take().map(|pending| (pending, page))
    }

    pub(crate) fn advance(
        &mut self,
        browser: &mut HiddenBrowser,
        machine: &mut HeritageAutomation,
    ) -> Option<HeritageEvent> {
        let valid = self
            .pending
            .as_ref()
            .is_some_and(|pending| machine.accepts_script_token(pending.ticket.token));
        if let Some(Pending {
            phase: Phase::Acting { page, .. },
            ticket,
            scope,
            ..
        }) = &self.pending
        {
            if valid && !self.pages.iter().any(|live| live.id == *page) {
                let (token, empty) = (ticket.token, scope.empty());
                self.pending = None;
                return machine.script_answered(token, empty);
            }
        }
        let (pending, page) = self.take_ready(valid)?;
        let Some(page) = page else {
            return machine.script_answered(pending.ticket.token, pending.scope.empty());
        };
        // The pending engine token and live view are checked again immediately before the only action.
        if pending.ticket.epoch != self.epoch
            || !machine.accepts_script_token(pending.ticket.token)
            || !self.pages.iter().any(|live| live.id == page)
        {
            return None;
        }
        match browser.run_script(ViewId(page.0), &pending.request.script) {
            Ok(call) => {
                self.pending = Some(Pending {
                    phase: Phase::Acting { page, call },
                    ..pending
                })
            }
            Err(_) => return machine.script_answered(pending.ticket.token, pending.scope.empty()),
        }
        None
    }
}

impl PageHost for QueuedHost {
    fn pages(&self) -> Vec<PageRef> {
        self.pages.clone()
    }
    fn run_script(&mut self, page: PageId, script: &str) -> Option<u64> {
        self.pages.iter().any(|p| p.id == page).then(|| {
            self.tokens += 1;
            self.wanted.push(Wanted::Script {
                page,
                script: script.to_owned(),
                token: self.tokens,
            });
            self.tokens
        })
    }
    fn navigate(&mut self, url: &str) {
        self.wanted.push(Wanted::Navigate(url.to_owned()));
    }
}

#[cfg(test)]
#[path = "host_tests.rs"]
mod tests;
