//! Unit tests for procedural sound synthesis, rate limits, job transitions, and lazy device errors (SND1).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use strata_types::theme_display::{DisplayOptions, SoundLevel};

use crate::sound::{
    is_silent_cli_args, recipe_spec, synthesize, AudioDeviceSink, AudioOpenError, LazyAudioBackend,
    RecordingBackend, SoundBackend, SoundController, SoundJobPhase, SoundJobSnapshot, SoundKind,
    MAX_PEAK_LINEAR, SAMPLE_RATE_HZ,
};

fn job(id: u64, fraction: Option<f32>, phase: SoundJobPhase) -> SoundJobSnapshot {
    SoundJobSnapshot { id, fraction, phase }
}

#[test]
fn default_sound_level_is_quiet_and_has_three_korean_labels() {
    let opts = DisplayOptions::default();
    assert_eq!(opts.sound_level, SoundLevel::Quiet);
    assert_eq!(SoundLevel::Normal.label_ko(), "소리 보통");
    assert_eq!(SoundLevel::Quiet.label_ko(), "소리 작게");
    assert_eq!(SoundLevel::Off.label_ko(), "소리 끔");
}

#[test]
fn synthesized_sounds_match_stated_lengths_soft_envelopes_and_peak_ceiling() {
    for kind in SoundKind::ALL {
        let spec = recipe_spec(kind);
        let quiet = synthesize(kind, SoundLevel::Quiet);
        let normal = synthesize(kind, SoundLevel::Normal);
        let off = synthesize(kind, SoundLevel::Off);

        assert!(off.is_empty(), "{kind:?} should produce no samples when Off");
        let expected_len = (spec.duration_ms as usize * SAMPLE_RATE_HZ as usize) / 1000;
        assert_eq!(quiet.len(), expected_len, "{kind:?} quiet sample count");
        assert_eq!(normal.len(), expected_len, "{kind:?} normal sample count");

        if kind == SoundKind::ButtonPress {
            assert!((10..=15).contains(&spec.duration_ms));
        }
        if kind == SoundKind::DownloadDone {
            assert!(spec.duration_ms < 250);
        }
        assert!(quiet.iter().all(|s| s.is_finite()));
        assert!(normal.iter().all(|s| s.is_finite()));
        assert!(quiet[0].abs() < 1e-5, "{kind:?} first sample must be ~0");
        assert!(quiet[quiet.len() - 1].abs() < 1e-5, "{kind:?} last sample must be ~0");

        let quiet_peak = quiet.iter().fold(0.0_f32, |m, s| m.max(s.abs()));
        let normal_peak = normal.iter().fold(0.0_f32, |m, s| m.max(s.abs()));
        assert!(quiet_peak > 0.01, "{kind:?} quiet peak must be non-zero");
        assert!(quiet_peak <= MAX_PEAK_LINEAR + 1e-6);
        assert!(normal_peak <= MAX_PEAK_LINEAR + 1e-6);
        assert!(normal_peak > quiet_peak);
    }
}

#[test]
fn download_done_has_two_rising_notes() {
    let samples = synthesize(SoundKind::DownloadDone, SoundLevel::Normal);
    assert!(!samples.is_empty());
    let split = (85 * SAMPLE_RATE_HZ as usize) / 1000;
    let zero_crossings = |slice: &[f32]| -> usize {
        slice
            .windows(2)
            .filter(|w| (w[0] <= 0.0 && w[1] > 0.0) || (w[0] >= 0.0 && w[1] < 0.0))
            .count()
    };
    let first_rate = zero_crossings(&samples[..split]) as f32 / 0.085;
    let second_rate = zero_crossings(&samples[split..]) as f32 / 0.095;
    assert!(
        second_rate > first_rate * 1.15,
        "expected rising second note, got crossings/s {first_rate} -> {second_rate}"
    );
}

#[test]
fn button_rate_limit_caps_at_eight_per_second_and_drops_excess() {
    let rec = RecordingBackend::new();
    let mut ctrl = SoundController::new(Box::new(rec.clone()));
    for i in 0..20 {
        ctrl.on_button_click(Duration::from_millis(i * 40), SoundLevel::Quiet);
    }
    assert_eq!(rec.events().len(), 8);
    assert!(rec
        .events()
        .iter()
        .all(|e| *e == (SoundKind::ButtonPress, SoundLevel::Quiet)));

    ctrl.on_button_click(Duration::from_millis(1_050), SoundLevel::Quiet);
    assert_eq!(rec.events().len(), 9);
}

#[test]
fn progress_steps_fire_at_25_50_75_once_per_job_and_at_most_one_per_second_across_jobs() {
    let rec = RecordingBackend::new();
    let mut ctrl = SoundController::new(Box::new(rec.clone()));

    ctrl.observe_jobs(
        &[job(1, Some(0.26), SoundJobPhase::Running)],
        Duration::from_millis(1_000),
        SoundLevel::Quiet,
    );
    assert_eq!(rec.events(), vec![(SoundKind::DownloadProgress, SoundLevel::Quiet)]);

    // Staying above 25% does not repeat.
    ctrl.observe_jobs(
        &[job(1, Some(0.30), SoundJobPhase::Running)],
        Duration::from_millis(2_500),
        SoundLevel::Quiet,
    );
    assert_eq!(rec.events().len(), 1);

    // Job 1 crosses 50% at 3.0s; job 2 crosses 25% 200ms later -> 1/s global cap drops the second.
    ctrl.observe_jobs(
        &[
            job(1, Some(0.52), SoundJobPhase::Running),
            job(2, Some(0.10), SoundJobPhase::Running),
        ],
        Duration::from_millis(3_000),
        SoundLevel::Quiet,
    );
    assert_eq!(rec.events().len(), 2);

    ctrl.observe_jobs(
        &[
            job(1, Some(0.55), SoundJobPhase::Running),
            job(2, Some(0.28), SoundJobPhase::Running),
        ],
        Duration::from_millis(3_200),
        SoundLevel::Quiet,
    );
    assert_eq!(rec.events().len(), 2);

    // Recovering never plays.
    ctrl.observe_jobs(
        &[job(1, Some(0.80), SoundJobPhase::Recovering)],
        Duration::from_millis(5_000),
        SoundLevel::Quiet,
    );
    assert_eq!(rec.events().len(), 2);

    // Resuming Running at 76% fires the 75% step once.
    ctrl.observe_jobs(
        &[job(1, Some(0.76), SoundJobPhase::Running)],
        Duration::from_millis(6_000),
        SoundLevel::Quiet,
    );
    assert_eq!(rec.events().len(), 3);
}

#[test]
fn done_stopped_none1_and_saved_emit_expected_sounds_and_off_never_calls_backend() {
    let rec = RecordingBackend::new();
    let mut ctrl = SoundController::new(Box::new(rec.clone()));

    ctrl.observe_jobs(
        &[job(10, None, SoundJobPhase::Stopped { needs_user: false })],
        Duration::from_secs(1),
        SoundLevel::Quiet,
    );
    assert!(rec.events().is_empty());

    ctrl.observe_jobs(
        &[job(11, Some(0.4), SoundJobPhase::Stopped { needs_user: true })],
        Duration::from_secs(2),
        SoundLevel::Quiet,
    );
    ctrl.observe_jobs(
        &[job(11, Some(0.4), SoundJobPhase::Stopped { needs_user: true })],
        Duration::from_secs(3),
        SoundLevel::Quiet,
    );
    assert_eq!(rec.events(), vec![(SoundKind::DownloadStopped, SoundLevel::Quiet)]);

    ctrl.observe_jobs(
        &[job(12, Some(1.0), SoundJobPhase::Done)],
        Duration::from_secs(4),
        SoundLevel::Normal,
    );
    assert_eq!(rec.events().last().copied(), Some((SoundKind::DownloadDone, SoundLevel::Normal)));

    ctrl.on_survey_saved(Duration::from_secs(5), SoundLevel::Quiet);
    assert_eq!(rec.events().last().copied(), Some((SoundKind::SurveySaved, SoundLevel::Quiet)));

    let before = rec.events().len();
    ctrl.on_button_click(Duration::from_secs(10), SoundLevel::Off);
    ctrl.on_survey_saved(Duration::from_secs(11), SoundLevel::Off);
    ctrl.observe_jobs(
        &[job(99, Some(1.0), SoundJobPhase::Done)],
        Duration::from_secs(12),
        SoundLevel::Off,
    );
    assert_eq!(rec.events().len(), before);
}

#[test]
fn silent_cli_modes_and_failing_device_opener_never_play_or_panic() {
    for flag in ["--snap", "--verify-all", "--smoke-quit", "--smoke"] {
        let args = vec!["ka-hgis.exe".to_owned(), flag.to_owned()];
        assert!(is_silent_cli_args(&args), "{flag} must activate silent mode");
    }
    assert!(!is_silent_cli_args(&["ka-hgis.exe".to_owned()]));

    let open_calls = Arc::new(AtomicUsize::new(0));
    let calls_ref = Arc::clone(&open_calls);
    let mut backend = LazyAudioBackend::with_opener(move || {
        calls_ref.fetch_add(1, Ordering::SeqCst);
        Err(AudioOpenError)
    });
    assert_eq!(open_calls.load(Ordering::SeqCst), 0);

    let t0 = Instant::now();
    backend.play(SoundKind::ButtonPress, SoundLevel::Quiet);
    assert!(t0.elapsed() < Duration::from_millis(50));

    let deadline = Instant::now() + Duration::from_millis(500);
    while open_calls.load(Ordering::SeqCst) == 0 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(open_calls.load(Ordering::SeqCst), 1);

    backend.play(SoundKind::SurveySaved, SoundLevel::Normal);
    std::thread::sleep(Duration::from_millis(20));
    assert_eq!(open_calls.load(Ordering::SeqCst), 1);
}

#[test]
fn lazy_backend_plays_samples_through_injected_sink() {
    struct CountingSink(Arc<AtomicUsize>);
    impl AudioDeviceSink for CountingSink {
        fn play_mono(&mut self, sample_rate: u32, samples: Vec<f32>) {
            assert_eq!(sample_rate, SAMPLE_RATE_HZ);
            if !samples.is_empty() {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }
    }

    let played = Arc::new(AtomicUsize::new(0));
    let played_ref = Arc::clone(&played);
    let mut backend =
        LazyAudioBackend::with_opener(move || Ok(Box::new(CountingSink(played_ref))));
    backend.play(SoundKind::ButtonPress, SoundLevel::Quiet);

    let deadline = Instant::now() + Duration::from_millis(500);
    while played.load(Ordering::SeqCst) == 0 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(played.load(Ordering::SeqCst), 1);
}

#[test]
fn lazy_backend_drops_late_and_excess_sounds_without_queuing() {
    struct SlowSink(Arc<AtomicUsize>);
    impl AudioDeviceSink for SlowSink {
        fn play_mono(&mut self, _sample_rate: u32, _samples: Vec<f32>) {
            self.0.fetch_add(1, Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(60));
        }
    }

    // 1) A sound delayed past MAX_SOUND_LATENESS during slow device open is dropped.
    let late_played = Arc::new(AtomicUsize::new(0));
    let late_ref = Arc::clone(&late_played);
    let mut slow_open = LazyAudioBackend::with_opener(move || {
        std::thread::sleep(Duration::from_millis(300));
        Ok(Box::new(SlowSink(late_ref)))
    });
    slow_open.play(SoundKind::ButtonPress, SoundLevel::Quiet);
    std::thread::sleep(Duration::from_millis(360));
    assert_eq!(late_played.load(Ordering::SeqCst), 0);

    // 2) Bursting 10 sounds while sink is busy drops excess instead of queuing all 10.
    let burst_played = Arc::new(AtomicUsize::new(0));
    let burst_ref = Arc::clone(&burst_played);
    let mut burst_backend =
        LazyAudioBackend::with_opener(move || Ok(Box::new(SlowSink(burst_ref))));
    for _ in 0..10 {
        burst_backend.play(SoundKind::ButtonPress, SoundLevel::Quiet);
    }
    std::thread::sleep(Duration::from_millis(200));
    let count = burst_played.load(Ordering::SeqCst);
    assert!((1..=2).contains(&count), "expected bounded non-queuing playback, got {count}");
}
