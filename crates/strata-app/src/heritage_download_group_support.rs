//! Component-only ACK controls; no native handle proof (target/cpp-ref/src/app/KaHeritageBrowser.cpp:1206-1217).

use super::*;

pub(super) fn start() -> (StrataAppState, tempfile::TempDir) {
    let (mut app, dir) = crate::heritage_run::lifecycle_tests::ready();
    crate::heritage_run_start::start_heritage_fetch(&mut app);
    (app, dir)
}

pub(super) fn archives(dir: &Path) -> Vec<PathBuf> {
    ["west", "east"]
        .into_iter()
        .map(|name| super::super::worker_tests::archive_named(dir, name))
        .collect()
}

pub(super) fn receive(app: &mut StrataAppState, kind: usize, files: &[PathBuf]) -> Reply {
    app.take_heritage_event(HeritageEvent::DatasetReady {
        kind: DATASETS[kind].name,
        files: files.to_vec(),
    });
    app.heritage
        .download
        .as_ref()
        .unwrap()
        .worker
        .replies
        .recv_timeout(Duration::from_secs(15))
        .expect("real import reply")
}

pub(super) fn finish(
    app: &mut StrataAppState,
    reply: Reply,
    files: &[PathBuf],
    expected: DatasetOutcome,
) -> Vec<(Vec<PathBuf>, DatasetOutcome)> {
    let mut calls = Vec::new();
    // This test double checks exact payloads; production must call its real captured handle.
    // target/cpp-ref/src/app/KaHeritageBrowser.cpp:1206-1217.
    app.take_heritage_import_with(reply, |got, outcome| {
        let accepted = got == files && outcome == expected;
        calls.push((got.to_vec(), outcome));
        accepted
    });
    calls
}

pub(super) fn imported(app: &mut StrataAppState, index: usize) {
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        app.drive_heritage_download(&eframe::egui::Context::default());
        if app
            .heritage
            .download
            .as_ref()
            .is_some_and(|d| d.kinds[index] == Kind::Imported)
        {
            return;
        }
        assert!(Instant::now() < deadline, "accepted group did not publish");
        std::thread::sleep(Duration::from_millis(10));
    }
}

pub(super) fn rows(app: &StrataAppState) -> usize {
    app.left_layers
        .entries
        .iter()
        .filter(|row| row.layer_key.starts_with("ref_heritage:"))
        .count()
}

pub(super) fn recent(dir: &Path) -> PathBuf {
    strata_refmap::heritage_recent::region_folder(dir, "경상북도", "경주시")
        .unwrap()
        .join("recent.json")
}

pub(super) fn running(app: &mut StrataAppState) {
    app.download_jobs.tick(1.0);
    assert!(matches!(
        app.download_jobs.jobs().last().unwrap().state,
        DownloadJobState::Running { fraction: None, .. }
    ));
}
