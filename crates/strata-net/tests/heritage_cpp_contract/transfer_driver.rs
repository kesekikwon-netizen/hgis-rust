//! Public transfer inputs only, from `target/cpp-ref/src/app/KaHeritageBrowser.cpp:1293-1395`.

use std::path::{Path, PathBuf};

use strata_net::heritage_auto::{DatasetOutcome, HeritageEvent, PageHost, PageId, PageRef};

use super::driver::Session;

struct Host;
impl PageHost for Host {
    fn pages(&self) -> Vec<PageRef> {
        vec![PageRef {
            id: PageId(1),
            url: "about:blank".into(),
            opener: None,
        }]
    }
    fn navigate(&mut self, _: &str) {}
    fn run_script(&mut self, _: PageId, _: &str) -> Option<u64> {
        None
    }
}

pub(super) struct TransferRun {
    pub session: Session,
    pub events: Vec<HeritageEvent>,
}

impl TransferRun {
    pub fn waiting() -> Self {
        let mut session = Session::new();
        session.reach(strata_net::heritage_auto::HeritageStage::Waiting);
        Self {
            session,
            events: Vec::new(),
        }
    }

    pub fn offer(&mut self, name: &str) -> PathBuf {
        // Paths identify synthetic offers, not actual archives or GIS records.
        let path = PathBuf::from(name);
        self.session.machine.file_offered(path.clone());
        path
    }

    pub fn complete(&mut self, path: &Path) {
        self.events
            .extend(self.session.machine.zip_arrived(path.to_owned()));
    }

    pub fn drain(&mut self) {
        self.events
            .extend(self.session.machine.timers(self.session.now_ms));
    }

    pub fn tick(&mut self) {
        // KaHeritageBrowser.cpp:103,1150-1158: exact 700ms logical poll, no clock tolerance.
        self.session.now_ms += 700;
        self.drain();
    }

    pub fn loaded(&mut self, files: &[PathBuf], outcome: DatasetOutcome) -> bool {
        self.session.machine.observe_time(self.session.now_ms);
        self.session.machine.dataset_loaded(files, outcome)
    }

    pub fn groups(&self) -> Vec<(&'static str, Vec<PathBuf>)> {
        self.events
            .iter()
            .filter_map(|event| match event {
                HeritageEvent::DatasetReady { kind, files } => Some((*kind, files.clone())),
                _ => None,
            })
            .collect()
    }

    pub fn assert_no_success(&self) {
        assert!(!self.events.iter().any(|event| matches!(
            event,
            HeritageEvent::Done | HeritageEvent::KindAbsent { .. }
        )));
    }

    pub fn assert_no_script(&mut self) {
        self.session.now_ms += 700;
        assert!(
            self.session
                .machine
                .next_step(&mut Host, self.session.now_ms)
                .is_none(),
            "an unacknowledged archive group cannot submit another browser action"
        );
    }

    pub fn one_file_group() -> (Self, Vec<PathBuf>) {
        let mut run = Self::waiting();
        let path = run.offer("fixture-first.zip");
        run.complete(&path);
        for _ in 0..6 {
            run.tick();
        }
        assert_eq!(run.groups(), vec![("지정유산", vec![path.clone()])]);
        (run, vec![path])
    }
}
