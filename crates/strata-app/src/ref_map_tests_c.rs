//! The 지질 button when things go another way than planned: another survey is opened during a fetch, the
//! official colours could not be read, the note of a stored map cannot be written (review of NET-GEO1).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use strata_refmap::transfer::{Download, Prepared, Request, Stopped, TransferFailure};

use super::run::RefMapJobs;
use super::tests::{bench, kigam, Held};
use super::*;
use crate::download_jobs::{DownloadJob, DownloadJobState, StopCode};

/// A fetch belongs to the survey it was started for: when another survey is opened the fetch is stopped and
/// nothing of it reaches the new survey (C++ drops such a result by its survey generation,
/// `MainWindowOverlay.cpp:614`, `:633`, `:656`).
#[test]
fn another_survey_takes_nothing_of_a_fetch_started_before() {
    let mut b = bench(kigam, || true);
    let held: Held = Arc::new(Mutex::new(Vec::new()));
    let keep = Arc::clone(&held);
    b.app.ref_maps.set_runner(Arc::new(move |work| keep.lock().unwrap().push(work)));
    b.app.press_reference_map(&GEOLOGY);
    b.frame(1.0);
    assert!(b.app.ref_maps.is_busy());

    let other = tempfile::tempdir().unwrap();
    let crs = b.app.work_crs;
    b.app.start_new_survey_project_with_path("다른 조사", &other.path().join("다른 조사.gpkg"), crs);
    assert!(!b.app.ref_maps.is_busy());
    b.frame(2.0);
    assert_eq!(b.app.download_jobs.jobs()[0].state, DownloadJobState::Cancelled);

    // The worker ends afterwards: it was told to stop, asks nothing and leaves nothing in the first survey.
    let work = held.lock().unwrap().pop().unwrap();
    work();
    b.frame(3.0);
    assert_eq!(b.requests.load(Ordering::SeqCst), 0);
    assert!(!b.has_row() && b.stored_maps().is_empty());
    assert_eq!(b.app.download_jobs.len(), 1);

    // A press that has not begun when the survey changes goes the same way.
    let mut b = bench(kigam, || true);
    b.app.press_reference_map(&GEOLOGY);
    b.app.start_new_survey_project_with_path("다른 조사", &other.path().join("또 다른 조사.gpkg"), crs);
    b.frame(1.0);
    assert!(b.app.download_jobs.is_empty() && !b.has_row() && !b.app.ref_maps.is_busy());
    assert_eq!(b.requests.load(Ordering::SeqCst), 0);
}

fn no_picture(request: &Request) -> Result<Vec<u8>, TransferFailure> {
    if request.url.contains("request=GetMap") {
        Err(TransferFailure::Network)
    } else {
        kigam(request)
    }
}

/// A map stored without the official colours is shown, with the C++ words about it, and it is not a kept
/// map: the next press for this area fetches again, as those words promise.
#[test]
fn a_map_without_the_official_colours_is_fetched_again() {
    let mut b = bench(no_picture, || true);
    b.app.press_reference_map(&GEOLOGY);
    b.frame(1.0);
    assert!(b.has_row());
    assert_eq!(b.status(), strata_refmap::geology_prepare::NO_OFFICIAL_COLOUR);
    let asked = b.requests.load(Ordering::SeqCst);

    b.app.remove_reference_vector_rows();
    b.app.press_reference_map(&GEOLOGY);
    assert_eq!(b.status(), GEOLOGY.fetching);
    b.frame(2.0);
    assert!(b.requests.load(Ordering::SeqCst) > asked, "fetched again");
}

/// Stores a map in a folder that takes no note: a folder stands where the note would be written.
fn prepare_where_no_note_fits(_: &Scope, requested: &Path, _: Download<'_>, _: &dyn Fn() -> bool) -> Result<Prepared, Stopped> {
    let folder = requested.parent().unwrap().join("ka-hgis-reference-NoNote");
    std::fs::create_dir_all(folder.join("ka-hgis-scope.json")).unwrap();
    let gpkg = folder.join(GEOLOGY.file_name);
    std::fs::write(&gpkg, b"map").unwrap();
    Ok(Prepared { folder, gpkg, rows: 1, warnings: Vec::new() })
}

static NO_NOTE: RefMapKind = RefMapKind { prepare: prepare_where_no_note_fits, ..GEOLOGY };

/// A stored map whose note cannot be written would never be used again: it is taken away, and the gauge
/// says that the map was not saved.
#[test]
fn a_map_whose_note_cannot_be_written_is_not_left_behind() {
    let mut b = bench(kigam, || true);
    b.app.press_reference_map(&NO_NOTE);
    b.frame(1.0);
    let state = &b.app.download_jobs.jobs()[0].state;
    assert_eq!(*state, DownloadJobState::Stopped { code: StopCode::Dir2, reason: strata_refmap::transfer::NOT_SAVED.to_owned() });
    assert!(!b.has_row());
    assert!(!b.dir.path().join("ka-hgis-reference-NoNote").exists());
}

/// The request in flight is told when the user cancels, so that it can end at once (the C++ aborts the
/// transfer, `PreparedReferenceMap.cpp:67-68`); the job then ends as cancelled, not as a failed request.
#[test]
fn a_cancel_reaches_the_request_in_flight() {
    let job: Arc<Mutex<Option<DownloadJob>>> = Arc::default();
    let seen: Arc<Mutex<Vec<bool>>> = Arc::default();
    let (cancel, note) = (Arc::clone(&job), Arc::clone(&seen));
    let mut b = bench(kigam, || true);
    b.app.ref_maps = RefMapJobs::for_test(
        Arc::new(move |_, give_up| {
            let before = give_up();
            // The user presses the gauge's cancel while the request is out.
            cancel.lock().unwrap().as_ref().unwrap().request_cancel();
            note.lock().unwrap().extend([before, give_up()]);
            Err(TransferFailure::Cancelled)
        }),
        || true,
    );
    let held: Held = Arc::new(Mutex::new(Vec::new()));
    let keep = Arc::clone(&held);
    b.app.ref_maps.set_runner(Arc::new(move |work| keep.lock().unwrap().push(work)));
    b.app.press_reference_map(&GEOLOGY);
    b.frame(1.0);
    *job.lock().unwrap() = Some(b.app.download_jobs.jobs()[0].clone());
    let work = held.lock().unwrap().pop().unwrap();
    work();
    b.frame(2.0);
    assert_eq!(*seen.lock().unwrap(), [false, true]);
    assert_eq!(b.app.download_jobs.jobs()[0].state, DownloadJobState::Cancelled);
    assert!(!b.has_row() && b.stored_maps().is_empty());
}
