//! Contour generation runs off the UI thread. The frame only starts the worker
//! and applies a finished result (`AGENTS.md`: nothing waits on the UI thread).

use std::sync::{LazyLock, Mutex};

use super::survey_contour_make::{apply_contour, build_contour, request_from, ContourMade};
use crate::app_state::StrataAppState;

struct Slot {
    running: bool,
    path: String,
    done: Option<Result<ContourMade, String>>,
}

static JOB: LazyLock<Mutex<Slot>> = LazyLock::new(|| {
    Mutex::new(Slot { running: false, path: String::new(), done: None })
});

#[cfg(test)]
pub(crate) fn running() -> bool {
    JOB.lock().unwrap_or_else(|poison| poison.into_inner()).running
}

pub(crate) fn running_for(path: &str) -> bool {
    let job = JOB.lock().unwrap_or_else(|poison| poison.into_inner());
    job.running && job.path == path
}

pub(crate) fn start(app: &StrataAppState) -> Result<(), String> {
    let Some(req) = request_from(app) else {
        return Err("조사를 연 뒤에 등고선을 만들 수 있습니다.".to_owned());
    };
    let mut job = JOB.lock().unwrap_or_else(|poison| poison.into_inner());
    if job.running {
        return Ok(());
    }
    job.running = true;
    job.path = req.path.clone();
    job.done = None;
    drop(job);
    std::thread::spawn(move || {
        let result = build_contour(req);
        let mut job = JOB.lock().unwrap_or_else(|poison| poison.into_inner());
        job.done = Some(result);
        job.running = false;
    });
    Ok(())
}

/// Applies a finished build, if the worker has one.
pub(crate) fn poll(app: &mut StrataAppState) {
    let finished = {
        let mut job = JOB.lock().unwrap_or_else(|poison| poison.into_inner());
        if job.path != app.modals.contour_file_path.trim() {
            None
        } else {
            job.done.take()
        }
    };
    match finished {
        Some(Ok(made)) => {
            apply_contour(app, &made);
            app.modals.active = None;
            app.modals.contour_build_note = None;
        }
        Some(Err(text)) => app.modals.contour_build_note = Some(text),
        None => {}
    }
}
