//! Unit and integration tests for `PERF1`: frame-time recorder with injected clock, ring buffer
//! eviction, zero-allocation disabled fast path, JSON report schema, gate decisions, and offline
//! scripted `@perf_map` snapshot step.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui::{self, Pos2, RawInput, Rect};
use strata_net::network::probe_dry_run;

use crate::app_state::StrataAppState;
use crate::perf_bench::{
    evaluate_perf_gate, scripted_view_for_step, DEFAULT_PERF_FRAMES, PERF1_MIN_BOUND_MS,
};
use crate::perf_recorder::{FrameRecorder, PartStats, PerfPart, PerfReport};
use crate::snap_shot::SnapRequest;

/// (1) Recorder computes exact median, p95, and max for total frame and named parts from scripted durations.
#[test]
fn test_perf1_1_recorder_computes_exact_frame_and_part_stats_with_injected_clock() {
    let t0 = Instant::now();
    let now_cell = Arc::new(Mutex::new(t0));
    let reader = Arc::clone(&now_cell);
    let rec = FrameRecorder::with_clock(32, Arc::new(move || *reader.lock().unwrap()));
    rec.set_enabled(true);

    // 20 frames with total = (i + 1) * 3 ms (3..=60 ms), background_tiles = (i + 1) * 2 ms (2..=40 ms),
    // and ribbon = 0.5 ms on every frame.
    for i in 0..20_u64 {
        let frame_start = t0 + Duration::from_millis(i * 100);
        *now_cell.lock().unwrap() = frame_start;
        let fg = rec.begin_frame().expect("enabled recorder must return FrameGuard");

        let pg_rib = rec.begin_part(PerfPart::Ribbon).expect("enabled PartGuard");
        *now_cell.lock().unwrap() = frame_start + Duration::from_micros(500);
        drop(pg_rib);

        let bg_start = frame_start + Duration::from_millis(1);
        *now_cell.lock().unwrap() = bg_start;
        let pg_bg = rec.begin_part(PerfPart::BackgroundTiles).expect("enabled PartGuard");
        *now_cell.lock().unwrap() = bg_start + Duration::from_millis((i + 1) * 2);
        drop(pg_bg);

        *now_cell.lock().unwrap() = frame_start + Duration::from_millis((i + 1) * 3);
        drop(fg);
    }

    let report = rec.summarize([1440.0, 900.0], 1.5);
    assert_eq!(report.frames, 20);
    // Totals: 3, 6, ..., 30, 33, ..., 57, 60 -> median of 20 = (30 + 33)/2 = 31.5, p95 (idx 18) = 57.0, max = 60.0.
    assert!((report.frame_total.median_ms - 31.5).abs() < 1e-6, "got {:?}", report.frame_total);
    assert!((report.frame_total.p95_ms - 57.0).abs() < 1e-6, "got {:?}", report.frame_total);
    assert!((report.frame_total.max_ms - 60.0).abs() < 1e-6, "got {:?}", report.frame_total);

    let bg = report.parts["background_tiles"];
    // 2, 4, ..., 20, 22, ..., 38, 40 -> median = 21.0, p95 = 38.0, max = 40.0.
    assert!((bg.median_ms - 21.0).abs() < 1e-6 && (bg.p95_ms - 38.0).abs() < 1e-6 && (bg.max_ms - 40.0).abs() < 1e-6);
    let rib = report.parts["ribbon"];
    assert!((rib.median_ms - 0.5).abs() < 1e-6 && (rib.p95_ms - 0.5).abs() < 1e-6 && (rib.max_ms - 0.5).abs() < 1e-6);
    assert!((report.parts["labels"].max_ms - 0.0).abs() < 1e-6);
}

/// (2) Ring buffer keeps at most `capacity` most recent frames in chronological order without growing.
#[test]
fn test_perf1_2_ring_buffer_retains_most_recent_frames_in_order() {
    let t0 = Instant::now();
    let now_cell = Arc::new(Mutex::new(t0));
    let reader = Arc::clone(&now_cell);
    let rec = FrameRecorder::with_clock(4, Arc::new(move || *reader.lock().unwrap()));
    rec.set_enabled(true);
    let initial_cap = rec.ring_capacity();

    let mut cur = t0;
    for step in 1..=7_u64 {
        *now_cell.lock().unwrap() = cur;
        let fg = rec.begin_frame().unwrap();
        cur += Duration::from_millis(step * 10);
        *now_cell.lock().unwrap() = cur;
        drop(fg);
    }

    let snap = rec.snapshot();
    assert_eq!(snap.len(), 4, "ring buffer of capacity 4 must keep only 4 samples");
    let totals_ms: Vec<u128> = snap.iter().map(|s| s.total.as_millis()).collect();
    assert_eq!(totals_ms, vec![40, 50, 60, 70], "must keep the 4 newest frames in order");
    assert_eq!(rec.ring_capacity(), initial_cap, "ring buffer must not reallocate after enable");
}

/// (3) When off, begin_frame and begin_part record no entries, never read the clock, and allocate zero capacity.
#[test]
fn test_perf1_3_disabled_recorder_records_nothing_calls_no_clock_and_allocates_zero() {
    let clock_calls = Arc::new(AtomicUsize::new(0));
    let calls_r = Arc::clone(&clock_calls);
    let rec = FrameRecorder::with_clock(
        64,
        Arc::new(move || {
            calls_r.fetch_add(1, Ordering::SeqCst);
            Instant::now()
        }),
    );
    assert!(!rec.is_enabled());
    assert_eq!(rec.ring_capacity(), 0, "disabled recorder must not allocate ring storage");

    for _ in 0..100 {
        assert!(rec.begin_frame().is_none());
        for part in PerfPart::ALL {
            assert!(rec.begin_part(part).is_none());
        }
    }
    assert_eq!(clock_calls.load(Ordering::SeqCst), 0, "disabled fast path must skip clock completely");
    assert!(rec.snapshot().is_empty());
    assert_eq!(rec.ring_capacity(), 0, "100 disabled frames must perform zero ring allocations");
}

/// (4) JSON serialization includes frames, window_size, screen_scale, frame_total, and all 8 required part keys.
#[test]
fn test_perf1_4_json_report_contains_all_required_fields_and_eight_named_parts() {
    let t0 = Instant::now();
    let now_cell = Arc::new(Mutex::new(t0));
    let reader = Arc::clone(&now_cell);
    let rec = FrameRecorder::with_clock(8, Arc::new(move || *reader.lock().unwrap()));
    rec.set_enabled(true);

    *now_cell.lock().unwrap() = t0;
    let fg = rec.begin_frame().unwrap();
    for (idx, part) in PerfPart::ALL.iter().copied().enumerate() {
        let pg = rec.begin_part(part).unwrap();
        *now_cell.lock().unwrap() = t0 + Duration::from_millis((idx as u64 + 1) * 2);
        drop(pg);
    }
    *now_cell.lock().unwrap() = t0 + Duration::from_millis(25);
    drop(fg);

    let report = rec.summarize([1280.0, 900.0], 1.25);
    let json = serde_json::to_string_pretty(&report).unwrap();
    let val: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(val["frames"], 1);
    assert_eq!(val["window_size"], serde_json::json!([1280.0, 900.0]));
    assert_eq!(val["screen_scale"], serde_json::json!(1.25));
    for stat_key in ["median_ms", "p95_ms", "max_ms"] {
        assert!(val["frame_total"][stat_key].is_number(), "missing frame_total.{stat_key}");
    }
    for required in [
        "background_tiles",
        "cadastral_tiles",
        "survey_layers",
        "labels",
        "left_panel",
        "right_panel",
        "ribbon",
        "status_bar",
    ] {
        for stat_key in ["median_ms", "p95_ms", "max_ms"] {
            assert!(val["parts"][required][stat_key].is_number(), "missing parts.{required}.{stat_key}");
        }
    }
}

/// (5) Gate passes when p95 <= bound, fails when p95 > bound, and reports 'skipped' (passing) when missing/undrawable.
#[test]
fn test_perf1_5_gate_passes_under_bound_fails_over_bound_and_skips_when_missing() {
    let make_report = |p95_ms: f64| PerfReport {
        frames: 60,
        window_size: [1440.0, 900.0],
        screen_scale: 1.0,
        frame_total: PartStats { median_ms: p95_ms * 0.6, p95_ms, max_ms: p95_ms * 1.2 },
        parts: BTreeMap::new(),
    };

    let under = evaluate_perf_gate(Some(&make_report(18.5)), None, 60.0);
    assert!(under.passed, "p95 under bound must pass: {under:?}");
    assert!(under.detail.contains("18.50"));

    let over = evaluate_perf_gate(Some(&make_report(75.2)), None, 60.0);
    assert!(!over.passed, "p95 over bound must fail: {over:?}");
    assert!(over.detail.contains("75.20") && over.detail.contains("60.0"));

    let skipped_missing = evaluate_perf_gate(None, Some("map canvas unavailable on headless runner"), 60.0);
    assert!(skipped_missing.passed, "missing/undrawable runner must skip rather than fail");
    assert!(skipped_missing.detail.contains("skipped") && skipped_missing.detail.contains("headless runner"));

    let skipped_empty = evaluate_perf_gate(Some(&PerfReport::default()), None, PERF1_MIN_BOUND_MS);
    assert!(skipped_empty.passed && skipped_empty.detail.contains("skipped"));
}

/// (6) Scripted `@perf_map` snap step pans/zooms deterministically offline on a temp copy of demo.gpkg and writes JSON.
#[test]
fn test_perf1_6_snap_step_writes_deterministic_json_offline_on_temp_demo_survey() {
    let v_a: Vec<_> = (0..12).map(|i| scripted_view_for_step([200_000.0, 450_000.0], [320_000.0, 520_000.0], i, 12)).collect();
    let v_b: Vec<_> = (0..12).map(|i| scripted_view_for_step([200_000.0, 450_000.0], [320_000.0, 520_000.0], i, 12)).collect();
    assert_eq!(v_a, v_b, "scripted views must be identical across runs");
    assert!(v_a.windows(2).all(|w| w[0] != w[1]), "each step must pan/zoom to a distinct view");

    let demo_src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg");
    let dir = tempfile::tempdir().unwrap();
    let demo_copy = dir.path().join("demo.gpkg");
    std::fs::copy(&demo_src, &demo_copy).unwrap();
    let out_dir = dir.path().join("out");
    std::fs::create_dir_all(&out_dir).unwrap();

    let _guard = crate::perf_bench::lock_perf_session();
    let (launches, _) = probe_dry_run(false, || {
        let ctx = egui::Context::default();
        let mut app = StrataAppState::new_startup();
        app.attach_user_settings_for_snap(None, dir.path());
        assert!(app.open_survey_gpkg(&demo_copy));
        app.snap = Some(SnapRequest {
            steps: vec![crate::snap_shot::SnapStep {
                action: Some("@perf_map=12".to_owned()),
                name: "perf".to_owned(),
            }],
            prefix: out_dir.clone(),
            waiting: false,
            settle: 2,
        });
        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(1280.0, 800.0));
        for _ in 0..40 {
            if !app.snap.as_ref().is_some_and(SnapRequest::has_work) {
                break;
            }
            let _ = ctx.run(RawInput { screen_rect: Some(rect), ..Default::default() }, |ctx| {
                let _ = app.render_workspace_egui(ctx);
                app.snap_tick(ctx);
            });
        }
    });
    assert_eq!(launches, 0, "offline perf snap step must launch zero network processes");
    let json_path = out_dir.join("perf.json");
    assert!(json_path.exists(), "expected {} to be written", json_path.display());
    let report: PerfReport = serde_json::from_str(&std::fs::read_to_string(&json_path).unwrap()).unwrap();
    assert_eq!(report.frames, 12);
    assert_eq!(report.window_size, [1280.0, 800.0]);
    assert_eq!(report.parts.len(), 8);
    let _ = DEFAULT_PERF_FRAMES;
}
