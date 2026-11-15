//! The gauge and the sounds: what the jobs of the gauge are to the sound controller, the weak sound of a
//! gauge that works, and how loud a press is beside it (owner 2026-11-15: 「모든버튼에 소리를 넣어라 눌리는지
//! 적용되는지 확인이 잘안된다 다운로드 받을때 게이지에서도 소리가 약하게나게하라. 완료시 에도」).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Duration;

use strata_types::theme_display::SoundLevel;

use super::heard;
use crate::download_jobs::{DownloadJobList, DownloadJobState, StopCode};
use crate::sound::{synthesize, RecordingBackend, SoundController, SoundJobPhase, SoundKind};

fn ms(at: u64) -> Duration {
    Duration::from_millis(at)
}

fn peak(kind: SoundKind, level: SoundLevel) -> f32 {
    synthesize(kind, level).iter().fold(0.0_f32, |high, sample| high.max(sample.abs()))
}

/// A press must be heard: also at 「소리 작게」 it is a tenth of full scale (-20 dBFS) and lasts long enough
/// to be noticed. The gauge's own sound is the weakest of all; the end of a download is at least as loud
/// as a press.
#[test]
fn a_press_is_heard_and_the_gauge_is_weaker() {
    for level in [SoundLevel::Quiet, SoundLevel::Normal] {
        let (press, pip, done, stopped, saved) =
            (peak(SoundKind::ButtonPress, level), peak(SoundKind::DownloadProgress, level), peak(SoundKind::DownloadDone, level), peak(SoundKind::DownloadStopped, level), peak(SoundKind::SurveySaved, level));
        assert!(press >= 0.099, "{level:?}: a press at {press}");
        assert!(pip < press * 0.6, "{level:?}: the gauge ({pip}) is weak beside a press ({press})");
        assert!(done >= press && stopped >= press && saved >= press * 0.99, "{level:?}: the ends {done} {stopped} {saved} beside {press}");
    }
    let samples = synthesize(SoundKind::ButtonPress, SoundLevel::Quiet).len();
    assert!((40 * 44_100 / 1000..=60 * 44_100 / 1000).contains(&samples), "a press lasts 40 to 60 ms: {samples} samples");
}

/// The jobs of the gauge as the sound controller hears them: a job that waits or was cancelled is not
/// heard; a stop that only says there is no data (NONE-1) asks nothing of the user.
#[test]
fn the_jobs_of_the_gauge_as_the_sounds_hear_them() {
    let mut list = DownloadJobList::default();
    let _ = list.add_job("기다림", DownloadJobState::Waiting, 0.0);
    let (running, _) = list.add_job("받는 중", DownloadJobState::Running { fraction: Some(0.5), stage: Some("[2/6] 로그인".to_owned()) }, 0.0);
    let (done, _) = list.add_job("끝", DownloadJobState::Done { result: "받았습니다.".to_owned() }, 0.0);
    let (stopped, _) = list.add_job("멈춤", DownloadJobState::Stopped { code: StopCode::Dir2, reason: "쓸 수 없음".to_owned() }, 0.0);
    let (none, _) = list.add_job("없음", DownloadJobState::Stopped { code: StopCode::None1, reason: "자료 없음".to_owned() }, 0.0);
    let _ = list.add_job("취소", DownloadJobState::Cancelled, 0.0);

    let (jobs, steps) = heard(list.jobs());
    let of = |id: crate::download_jobs::DownloadJobId| jobs.iter().find(|job| job.id == id.0).map(|job| (job.fraction, job.phase));
    assert_eq!(jobs.len(), 4);
    assert_eq!(of(running), Some((Some(0.5), SoundJobPhase::Running)));
    assert_eq!(of(done), Some((None, SoundJobPhase::Done)));
    assert_eq!(of(stopped), Some((None, SoundJobPhase::Stopped { needs_user: true })));
    assert_eq!(of(none), Some((None, SoundJobPhase::Stopped { needs_user: false })));
    // Only the running job has a step, and another stage is another step.
    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0].0, running.0);
    let mut later = DownloadJobList::default();
    let _ = later.add_job("받는 중", DownloadJobState::Running { fraction: Some(0.5), stage: Some("[3/6] 신청".to_owned()) }, 0.0);
    assert_ne!(heard(later.jobs()).1[0].1, steps[0].1);
}

/// While the gauge works it sounds weakly: when a job starts to run, when a job comes to another step of
/// its work (at most once a second), and every 2.5 s while any job runs. A gauge at rest is silent, and
/// so is everything at 「소리 끔」.
#[test]
fn a_working_gauge_sounds_weakly() {
    let recorder = RecordingBackend::new();
    let mut sounds = SoundController::new(Box::new(recorder.clone()));
    let pips = |recorder: &RecordingBackend| recorder.events().iter().filter(|event| event.0 == SoundKind::DownloadProgress).count();

    sounds.observe_running(&[], ms(0), SoundLevel::Quiet);
    assert_eq!(pips(&recorder), 0, "nothing runs");
    sounds.observe_running(&[(7, 1)], ms(100), SoundLevel::Quiet);
    assert_eq!(pips(&recorder), 1, "a job started to run");
    sounds.observe_running(&[(7, 1)], ms(1500), SoundLevel::Quiet);
    assert_eq!(pips(&recorder), 1, "the same step, 1.4 s later");
    sounds.observe_running(&[(7, 2)], ms(1600), SoundLevel::Quiet);
    assert_eq!(pips(&recorder), 2, "another step");
    sounds.observe_running(&[(7, 3)], ms(2000), SoundLevel::Quiet);
    assert_eq!(pips(&recorder), 2, "a step 0.4 s after the last sound waits");
    sounds.observe_running(&[(7, 3)], ms(4099), SoundLevel::Quiet);
    assert_eq!(pips(&recorder), 2);
    sounds.observe_running(&[(7, 3)], ms(4100), SoundLevel::Quiet);
    assert_eq!(pips(&recorder), 3, "2.5 s after the last sound");
    sounds.observe_running(&[], ms(9000), SoundLevel::Quiet);
    assert_eq!(pips(&recorder), 3, "the job ended");

    sounds.observe_running(&[(8, 1)], ms(20_000), SoundLevel::Off);
    sounds.observe_running(&[(8, 2)], ms(30_000), SoundLevel::Off);
    assert_eq!(recorder.events().len(), 3, "소리 끔");
}
