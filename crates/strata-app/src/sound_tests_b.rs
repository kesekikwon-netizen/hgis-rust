//! SND1b tests: frame priority coalescing over LazyAudioBackend (1 frame and 2000 frames)
//! and download ending rate limits / already-finished initial job suppression.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use eframe::egui::{self, Event, PointerButton, Pos2, RawInput, Rect, Vec2};
use strata_types::theme_display::SoundLevel;

use super::{
    attach_controller_for_test, on_frame_end, synthesize, AudioDeviceSink, LazyAudioBackend,
    RecordingBackend, SoundBackend, SoundController, SoundJobPhase, SoundJobSnapshot, SoundKind,
    SAMPLE_RATE_HZ,
};
use crate::app_state::StrataAppState;

struct RecordingSinkState {
    sample_lengths: Vec<usize>,
    last_samples: Vec<f32>,
}

type SharedSink = Arc<(Mutex<RecordingSinkState>, Condvar)>;

struct RecordingDeviceSink {
    shared: SharedSink,
}

impl AudioDeviceSink for RecordingDeviceSink {
    fn play_mono(&mut self, sample_rate: u32, samples: Vec<f32>) {
        assert_eq!(sample_rate, SAMPLE_RATE_HZ);
        let (lock, cvar) = &*self.shared;
        if let Ok(mut guard) = lock.lock() {
            guard.sample_lengths.push(samples.len());
            guard.last_samples = samples;
            cvar.notify_all();
        }
    }
}

fn new_shared_sink(cap: usize) -> SharedSink {
    Arc::new((
        Mutex::new(RecordingSinkState {
            sample_lengths: Vec::with_capacity(cap),
            last_samples: Vec::new(),
        }),
        Condvar::new(),
    ))
}

fn wait_for_count(shared: &SharedSink, target: usize) -> Vec<usize> {
    let (lock, cvar) = &**shared;
    let deadline = Instant::now() + Duration::from_millis(500);
    let mut guard = lock.lock().unwrap();
    while guard.sample_lengths.len() < target {
        let now = Instant::now();
        if now >= deadline {
            break;
        }
        let (next, _) = cvar.wait_timeout(guard, deadline - now).unwrap();
        guard = next;
    }
    guard.sample_lengths.clone()
}

fn job(id: u64, fraction: Option<f32>, phase: SoundJobPhase) -> SoundJobSnapshot {
    SoundJobSnapshot { id, fraction, phase }
}

#[test]
fn tick_and_saved_in_one_frame_delivers_only_saved_to_lazy_device() {
    let shared = new_shared_sink(4);
    let sink_shared = Arc::clone(&shared);
    let backend = LazyAudioBackend::with_opener(move || {
        Ok(Box::new(RecordingDeviceSink { shared: sink_shared }))
    });

    let ctx = egui::Context::default();
    attach_controller_for_test(&ctx, SoundController::new(Box::new(backend)));
    let mut app = StrataAppState::new_startup();
    let dir = tempfile::tempdir().unwrap();
    let gpkg = dir.path().join("snd1b_click_save.gpkg");
    let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 900.0));

    let mut save_btn = Pos2::ZERO;
    let _ = ctx.run(RawInput { screen_rect: Some(screen), ..Default::default() }, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            save_btn = ui.button("저장").rect.center();
        });
        on_frame_end(ctx, &app);
    });

    let press = vec![
        Event::PointerMoved(save_btn),
        Event::PointerButton {
            pos: save_btn,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        },
    ];
    let _ = ctx.run(RawInput { screen_rect: Some(screen), events: press, ..Default::default() }, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let _ = ui.button("저장");
        });
        on_frame_end(ctx, &app);
    });

    // Release frame: clicking 「저장」 writes the survey in the same frame (`gui_ribbon_click_a.rs:56`).
    let release = vec![Event::PointerButton {
        pos: save_btn,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    }];
    let _ = ctx.run(RawInput { screen_rect: Some(screen), events: release, ..Default::default() }, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("저장").clicked() {
                let _ = app.write_survey_to(&gpkg);
            }
        });
        on_frame_end(ctx, &app);
    });

    let lengths = wait_for_count(&shared, 1);
    std::thread::sleep(Duration::from_millis(25));
    let expected_saved = synthesize(SoundKind::SurveySaved, SoundLevel::Quiet);
    let guard = shared.0.lock().unwrap();
    assert_eq!(
        guard.sample_lengths,
        vec![expected_saved.len()],
        "tick + saved in one frame must deliver only SurveySaved (4851 samples), got {:?}",
        lengths
    );
    assert_eq!(guard.last_samples, expected_saved);
}

#[test]
fn two_thousand_tick_plus_saved_frames_lose_zero_saved_notes_through_lazy_device() {
    let shared = new_shared_sink(2_000);
    let sink_shared = Arc::clone(&shared);
    let backend = LazyAudioBackend::with_opener(move || {
        Ok(Box::new(RecordingDeviceSink { shared: sink_shared }))
    });
    let mut ctrl = SoundController::new(Box::new(backend));
    let saved_len = synthesize(SoundKind::SurveySaved, SoundLevel::Quiet).len();

    // Warm up worker thread once so every one of the 2,000 frames starts with an idle worker.
    ctrl.begin_frame();
    ctrl.on_survey_saved(Duration::ZERO, SoundLevel::Quiet);
    ctrl.end_frame();
    assert_eq!(wait_for_count(&shared, 1), vec![saved_len]);
    shared.0.lock().unwrap().sample_lengths.clear();

    for frame in 0..2_000_u64 {
        let prev_count = shared.0.lock().unwrap().sample_lengths.len();
        let now = Duration::from_millis((frame + 1) * 200);
        ctrl.begin_frame();
        ctrl.on_button_click(now, SoundLevel::Quiet);
        ctrl.on_survey_saved(now, SoundLevel::Quiet);
        ctrl.end_frame();
        let _ = wait_for_count(&shared, prev_count + 1);
    }

    std::thread::sleep(Duration::from_millis(20));
    let recorded = shared.0.lock().unwrap().sample_lengths.clone();
    let saved_received = recorded.iter().filter(|&&n| n == saved_len).count();
    let lost = 2_000_usize.saturating_sub(saved_received);
    assert_eq!(
        (saved_received, recorded.len()),
        (2_000, 2_000),
        "lost {lost} of 2000 SurveySaved notes (total device plays: {})",
        recorded.len()
    );
}

#[test]
fn five_jobs_ending_in_one_frame_emit_one_sound_and_already_finished_jobs_emit_none() {
    let rec = RecordingBackend::new();
    let mut ctrl = SoundController::new(Box::new(rec.clone()));

    ctrl.observe_jobs(
        &[
            job(100, Some(1.0), SoundJobPhase::Done),
            job(101, Some(0.5), SoundJobPhase::Stopped { needs_user: true }),
        ],
        Duration::from_secs(1),
        SoundLevel::Quiet,
    );
    assert!(rec.events().is_empty(), "already-finished jobs must emit no sound: {:?}", rec.events());

    let running: Vec<SoundJobSnapshot> = (1..=6).map(|id| job(id, Some(0.10), SoundJobPhase::Running)).collect();
    ctrl.observe_jobs(&running, Duration::from_millis(2_000), SoundLevel::Quiet);
    assert!(rec.events().is_empty());

    let mut five_done: Vec<SoundJobSnapshot> = (1..=5).map(|id| job(id, Some(1.0), SoundJobPhase::Done)).collect();
    five_done.push(job(6, Some(0.15), SoundJobPhase::Running));
    ctrl.observe_jobs(&five_done, Duration::from_millis(3_000), SoundLevel::Quiet);
    assert_eq!(rec.events(), vec![(SoundKind::DownloadDone, SoundLevel::Quiet)]);

    five_done[5] = job(6, Some(1.0), SoundJobPhase::Done);
    five_done.push(job(7, Some(0.10), SoundJobPhase::Running));
    ctrl.observe_jobs(&five_done, Duration::from_millis(3_300), SoundLevel::Quiet);
    assert_eq!(rec.events().len(), 1);

    five_done[6] = job(7, Some(1.0), SoundJobPhase::Done);
    ctrl.observe_jobs(&five_done, Duration::from_millis(4_100), SoundLevel::Quiet);
    assert_eq!(rec.events().len(), 2);

    let stop_run: Vec<SoundJobSnapshot> = (20..25).map(|id| job(id, Some(0.20), SoundJobPhase::Running)).collect();
    ctrl.observe_jobs(&stop_run, Duration::from_millis(6_000), SoundLevel::Quiet);
    let stop_end: Vec<SoundJobSnapshot> =
        (20..25).map(|id| job(id, Some(0.20), SoundJobPhase::Stopped { needs_user: true })).collect();
    ctrl.observe_jobs(&stop_end, Duration::from_millis(7_000), SoundLevel::Quiet);
    assert_eq!(rec.events().len(), 3);
    assert_eq!(rec.events().last().copied(), Some((SoundKind::DownloadStopped, SoundLevel::Quiet)));
}

#[test]
fn non_tick_sounds_beat_tick_in_frame_and_in_lazy_backend_handover() {
    let rec = RecordingBackend::new();
    let mut ctrl = SoundController::new(Box::new(rec.clone()));
    let init = [
        job(1, Some(0.10), SoundJobPhase::Running),
        job(2, Some(0.10), SoundJobPhase::Running),
        job(3, Some(0.10), SoundJobPhase::Running),
    ];
    ctrl.observe_jobs(&init, Duration::from_secs(1), SoundLevel::Quiet);

    for (secs, snap) in [
        (2, [job(1, Some(0.30), SoundJobPhase::Running), init[1], init[2]]),
        (4, [job(1, Some(1.0), SoundJobPhase::Done), init[1], init[2]]),
        (6, [job(1, Some(1.0), SoundJobPhase::Done), job(2, Some(0.20), SoundJobPhase::Stopped { needs_user: true }), init[2]]),
    ] {
        let now = Duration::from_secs(secs);
        ctrl.begin_frame();
        ctrl.on_button_click(now, SoundLevel::Quiet);
        ctrl.observe_jobs(&snap, now, SoundLevel::Quiet);
        ctrl.end_frame();
    }
    assert_eq!(
        rec.events(),
        vec![
            (SoundKind::DownloadProgress, SoundLevel::Quiet),
            (SoundKind::DownloadDone, SoundLevel::Quiet),
            (SoundKind::DownloadStopped, SoundLevel::Quiet),
        ]
    );

    let shared = new_shared_sink(4);
    let sink_shared = Arc::clone(&shared);
    let mut backend = LazyAudioBackend::with_opener(move || {
        std::thread::sleep(Duration::from_millis(25));
        Ok(Box::new(RecordingDeviceSink { shared: sink_shared }))
    });
    backend.play(SoundKind::ButtonPress, SoundLevel::Quiet);
    backend.play(SoundKind::SurveySaved, SoundLevel::Quiet);
    let saved_len = synthesize(SoundKind::SurveySaved, SoundLevel::Quiet).len();
    let _ = wait_for_count(&shared, 1);
    std::thread::sleep(Duration::from_millis(40));
    let played = shared.0.lock().unwrap().sample_lengths.clone();
    assert!(
        played.contains(&saved_len),
        "SurveySaved must never be dropped because ButtonPress came first, got {played:?}"
    );
}
