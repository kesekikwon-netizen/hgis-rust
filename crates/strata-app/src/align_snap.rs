//! Session-owned target resolution, target/cpp-ref/src/app/KaAlignMapTool.cpp:464-537.
use super::{
    align_snap_sources::{Availability, Sources},
    gui_ribbon_align::AlignPair,
};
use std::sync::{
    mpsc::{self, Receiver, TryRecvError},
    Arc,
};
use strata_geo::snap_index::SnapIndex;

#[derive(Debug)]
struct Job {
    sources: Sources,
    receive: Receiver<Result<SnapIndex, String>>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Operation {
    Pick,
    Move(usize),
}

#[derive(Debug)]
struct Action {
    operation: Operation,
    raw: [f64; 2],
    reach: f64,
    pairs: Vec<AlignPair>,
    source: Option<[f64; 2]>,
}

#[derive(Debug, Default)]
pub(crate) struct Resolver {
    desired: Option<Sources>,
    active: Option<Job>,
    ready: Option<Result<Arc<SnapIndex>, String>>,
    action: Option<Action>,
}
impl Clone for Resolver {
    fn clone(&self) -> Self {
        Self::default()
    }
}

#[derive(Debug, PartialEq)]
pub(super) enum Resolution {
    Ready { point: [f64; 2], snapped: bool },
    Waiting,
    Failed(String),
}

impl Resolver {
    pub(super) fn prepare(&mut self, sources: Sources) -> bool {
        // Cheap immutable revision identity; one owned build may finish after its sources change.
        // Coalesce changes until that worker finishes, never accept its stale result.
        // target/cpp-ref/src/app/KaAlignMapTool.cpp:98-108,482-497.
        let changed = self.desired.as_ref().is_none_or(|old| !old.same_revision(&sources));
        if changed {
            self.desired = Some(sources.clone());
            self.ready = None;
            self.cancel();
        }
        if let Some(job) = self.active.take() {
            match job.receive.try_recv() {
                Err(TryRecvError::Empty) => self.active = Some(job),
                result if job.sources.same_revision(&sources) => {
                    self.ready = Some(match result {
                        Ok(result) => result.map(Arc::new),
                        Err(TryRecvError::Disconnected) => Err(crate::ref_vector::OPEN_FAILED.to_owned()),
                        Err(TryRecvError::Empty) => unreachable!(),
                    });
                }
                Ok(_) | Err(TryRecvError::Disconnected) => {}
            }
        }
        if self.ready.is_some() {
            return changed;
        }
        match &sources.availability {
            Availability::Failed(why) => self.ready = Some(Err(why.clone())),
            Availability::Waiting => {}
            Availability::Ready if !sources.policy.enabled || sources.layers.is_empty() => self.ready = Some(Ok(Arc::new(SnapIndex::new()))),
            Availability::Ready if self.active.is_none() => {
                let (send, receive) = mpsc::channel();
                let owned = sources.clone();
                std::thread::spawn(move || {
                    drop(send.send(super::align_snap_worker::build(&owned)));
                });
                self.active = Some(Job { sources, receive });
            }
            Availability::Ready => {}
        }
        changed
    }
    pub(super) fn resolve(&self, sources: &Sources, raw: [f64; 2], reach: f64) -> Resolution {
        if self.desired.as_ref().is_none_or(|old| !old.same_revision(sources)) {
            return Resolution::Waiting;
        }
        if !sources.policy.enabled {
            return Resolution::Ready { point: raw, snapped: false };
        }
        match &self.ready {
            Some(Ok(index)) => match index.query(raw, reach) {
                Some(hit) => Resolution::Ready { point: hit.point, snapped: true },
                None => Resolution::Ready { point: raw, snapped: false },
            },
            Some(Err(why)) => Resolution::Failed(why.clone()),
            None => Resolution::Waiting,
        }
    }
    pub(super) fn defer(&mut self, operation: Operation, raw: [f64; 2], reach: f64, pairs: &[AlignPair], source: Option<[f64; 2]>) {
        self.action = Some(Action {
            operation,
            raw,
            reach,
            pairs: pairs.to_vec(),
            source,
        });
    }
    pub(super) fn take_ready_action(&mut self, pairs: &[AlignPair], source: Option<[f64; 2]>) -> Result<Option<(Operation, [f64; 2])>, String> {
        let Some(action) = &self.action else { return Ok(None) };
        if action.pairs != pairs || action.source != source {
            self.cancel();
            return Ok(None);
        }
        let Some(sources) = &self.desired else { return Ok(None) };
        match self.resolve(sources, action.raw, action.reach) {
            Resolution::Ready { point, .. } => Ok(self.action.take().map(|a| (a.operation, point))),
            Resolution::Waiting => Ok(None),
            Resolution::Failed(why) => {
                self.cancel();
                Err(why)
            }
        }
    }
    pub(super) fn waiting(&self) -> bool {
        self.ready.is_none() || self.active.is_some()
    }
    pub(crate) fn cancel(&mut self) {
        self.action = None;
    }
    pub(crate) fn has_action(&self) -> bool {
        self.action.is_some()
    }
}

#[cfg(test)]
#[path = "align_snap_input_tests_a.rs"]
mod tests;
