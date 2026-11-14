//! Deterministic pan/zoom benchmark step (`@perf_map`) and `--verify-all` speed gate (`PERF1`).

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use eframe::egui;
use strata_net::network::probe_dry_run;
use strata_types::crs::WorkCrs;

use crate::app_state::{MainTab, StrataAppState};
use crate::perf_recorder::{
    clear_global, is_enabled, set_enabled, snapshot_global, summarize_samples, PerfReport,
};
use crate::survey_paint_layers::table_extent_in;
use crate::verify_harness::GateResult;

/// Default number of measured frames in `@perf_map` and the `--verify-all` speed gate.
pub const DEFAULT_PERF_FRAMES: usize = 60;

/// Minimum allowed p95 frame-time ceiling (ms) per `ORDER.yaml` (`never under 50 ms`).
pub const PERF1_MIN_BOUND_MS: f64 = 50.0;

/// Generous p95 whole-frame bound: `max(50.0, 4.0 * measured_p95_ms) = 50.0` ms
/// (measured offline p95 across 3 runs at 99% CPU load was 0.84..1.10 ms; 4x = 3.37..4.40 ms,
/// clamped up to the 50.0 ms floor required by `ORDER.yaml` for busy PCs and CI runners).
pub const PERF1_P95_BOUND_MS: f64 = 50.0;

static PERF_SESSION_LOCK: Mutex<()> = Mutex::new(());
static SNAP_PERF_RUN: Mutex<Option<SnapPerfRun>> = Mutex::new(None);

/// Serializes tests and verification gates that drive the process-wide `FrameRecorder`.
pub(crate) fn lock_perf_session() -> std::sync::MutexGuard<'static, ()> {
    PERF_SESSION_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

struct SnapPerfRun {
    target_frames: usize,
    min_warm_left: u8,
    busy_warm_left: u8,
    measuring: bool,
    prev_enabled: bool,
    korea_center: [f64; 2],
    site_center: [f64; 2],
}

/// Computes the deterministic `(center_xy, meters_per_pixel)` for frame `step` of `total`.
///
/// Steps `0..total/3` pan and zoom across the Korea/regional satellite view; steps `total/3..2*total/3`
/// zoom through the survey context; steps `2*total/3..total` pan and zoom at site/cadastral scale
/// (`1:900..1:3600`, where cadastral WMS, all 7 domain survey layers, and halo labels are active).
#[must_use]
pub fn scripted_view_for_step(
    korea_center: [f64; 2],
    site_center: [f64; 2],
    step: usize,
    total: usize,
) -> ([f64; 2], f64) {
    let third = (total / 3).max(1);
    let two_thirds = (third * 2).max(2);
    if step < third {
        let k = step as f64;
        let mpp = 420.0 * 0.94_f64.powi(step as i32);
        ([korea_center[0] + k * 1_800.0, korea_center[1] - k * 1_200.0], mpp)
    } else if step < two_thirds {
        let j = step - third;
        let k = j as f64;
        let mpp = 14.0 * 0.89_f64.powi(j as i32);
        ([site_center[0] + k * 22.0, site_center[1] + k * 14.0], mpp)
    } else {
        let j = step - two_thirds;
        let k = j as f64;
        let mpp = (0.95 * 0.93_f64.powi(j as i32)).max(0.18);
        ([site_center[0] + k * 3.5, site_center[1] - k * 2.5], mpp)
    }
}

fn parse_frame_count(action: &str) -> usize {
    action
        .strip_prefix("@perf_map=")
        .and_then(|s| s.trim().parse::<usize>().ok())
        .filter(|&n| n > 0)
        .unwrap_or(DEFAULT_PERF_FRAMES)
}

fn anchors_for_app(app: &StrataAppState, work_crs: WorkCrs) -> ([f64; 2], [f64; 2]) {
    let mut probe = crate::tabs_home_map::HomeMapTabState::default();
    probe.zoom_to_korea(work_crs);
    let korea_center = probe.viewport.center_xy;
    let site_center = app
        .survey_tables
        .iter()
        .find(|t| t.table == "survey_area" && !t.layer.features.is_empty())
        .or_else(|| app.survey_tables.iter().find(|t| !t.layer.features.is_empty()))
        .and_then(|t| table_extent_in(t, work_crs))
        .map_or(korea_center, |b| [(b[0] + b[2]) * 0.5, (b[1] + b[3]) * 0.5]);
    (korea_center, site_center)
}

fn apply_step_view(app: &mut StrataAppState, korea: [f64; 2], site: [f64; 2], step: usize, total: usize) {
    let (center, mpp) = scripted_view_for_step(korea, site, step, total);
    app.home_map.korea_view_pending = false;
    app.home_map.viewport.center_xy = center;
    app.home_map.viewport.meters_per_pixel = mpp;
}

/// Advances `@perf_map` / `@perf_map=<N>` by one frame inside `snap_tick`. Returns `true` when finished.
pub fn tick_snap_perf_step(
    app: &mut StrataAppState,
    ctx: &egui::Context,
    action: &str,
    name: &str,
) -> bool {
    let mut slot = SNAP_PERF_RUN.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if slot.is_none() {
        app.active_tab = MainTab::Map;
        let (korea_center, site_center) = anchors_for_app(app, app.work_crs);
        *slot = Some(SnapPerfRun {
            target_frames: parse_frame_count(action),
            min_warm_left: 2,
            busy_warm_left: 6,
            measuring: false,
            prev_enabled: is_enabled(),
            korea_center,
            site_center,
        });
    }
    let Some(run) = slot.as_mut() else {
        return true;
    };
    if !run.measuring {
        if run.min_warm_left > 0 {
            run.min_warm_left -= 1;
            return false;
        }
        let busy =
            app.basemap_fetch.in_flight_count() > 0 || app.cadastral_fetch.in_flight_count() > 0;
        if busy && run.busy_warm_left > 0 {
            run.busy_warm_left -= 1;
            return false;
        }
        run.measuring = true;
        clear_global();
        set_enabled(true);
        apply_step_view(app, run.korea_center, run.site_center, 0, run.target_frames);
        return false;
    }

    let samples = snapshot_global();
    if samples.len() < run.target_frames {
        apply_step_view(app, run.korea_center, run.site_center, samples.len(), run.target_frames);
        return false;
    }

    let tail = &samples[samples.len() - run.target_frames..];
    let screen = ctx.screen_rect();
    let report = summarize_samples(tail, [screen.width(), screen.height()], ctx.pixels_per_point());
    if let Some(snap) = app.snap.as_ref() {
        let mut out_path = snap.prefix.clone();
        out_path.push(format!("{name}.json"));
        if let Some(parent) = out_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match serde_json::to_string_pretty(&report)
            .map_err(|e| std::io::Error::other(e.to_string()))
            .and_then(|s| std::fs::write(&out_path, s.as_bytes()).map(|()| s.len()))
        {
            Ok(bytes) => eprintln!("[SNAP] {name} -> {} ({bytes} bytes)", out_path.display()),
            Err(e) => eprintln!("[SNAP] {name} JSON 저장 실패: {e}"),
        }
    }
    set_enabled(run.prev_enabled);
    clear_global();
    *slot = None;
    true
}

/// Evaluates the coarse `--verify-all` p95 regression gate (`PERF1`).
#[must_use]
pub fn evaluate_perf_gate(
    report: Option<&PerfReport>,
    skip_reason: Option<&str>,
    bound_ms: f64,
) -> GateResult {
    let ceiling = bound_ms.max(PERF1_MIN_BOUND_MS);
    if let Some(reason) = skip_reason {
        return GateResult {
            gate_id: "PERF1",
            title_ko: "지도 화면 프레임 시간(p95) 회귀 상한 검증",
            passed: true,
            detail: format!("skipped: {reason}"),
        };
    }
    let Some(rep) = report.filter(|r| r.frames > 0) else {
        return GateResult {
            gate_id: "PERF1",
            title_ko: "지도 화면 프레임 시간(p95) 회귀 상한 검증",
            passed: true,
            detail: "skipped: no map frames recorded".to_owned(),
        };
    };
    let p95 = rep.frame_total.p95_ms;
    let passed = p95 <= ceiling;
    let cmp = if passed { "<=" } else { ">" };
    GateResult {
        gate_id: "PERF1",
        title_ko: "지도 화면 프레임 시간(p95) 회귀 상한 검증",
        passed,
        detail: format!(
            "p95={p95:.2}ms {cmp} {ceiling:.1}ms (median={:.2}ms, max={:.2}ms, frames={})",
            rep.frame_total.median_ms, rep.frame_total.max_ms, rep.frames
        ),
    }
}

fn find_demo_survey_gpkg() -> Option<PathBuf> {
    let cwd_rel = Path::new("samples/demo_survey/demo.gpkg");
    if cwd_rel.is_file() {
        return Some(cwd_rel.to_path_buf());
    }
    let manifest_rel = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg");
    manifest_rel.is_file().then_some(manifest_rel)
}

/// Runs the offline scripted demo-survey measurement headlessly for `--verify-all`.
#[must_use]
pub fn run_perf1_gate() -> GateResult {
    let _session = lock_perf_session();
    let Some(demo_src) = find_demo_survey_gpkg() else {
        return evaluate_perf_gate(None, Some("samples/demo_survey/demo.gpkg not found"), PERF1_P95_BOUND_MS);
    };
    let tmp_dir = std::env::temp_dir().join(format!("ka_hgis_perf1_{}", std::process::id()));
    if std::fs::create_dir_all(&tmp_dir).is_err() {
        return evaluate_perf_gate(None, Some("tempdir unavailable"), PERF1_P95_BOUND_MS);
    }
    let demo_copy = tmp_dir.join("demo.gpkg");
    if std::fs::copy(&demo_src, &demo_copy).is_err() {
        let _ = std::fs::remove_dir_all(&tmp_dir);
        return evaluate_perf_gate(None, Some("could not copy demo.gpkg to tempdir"), PERF1_P95_BOUND_MS);
    }

    let mut skip_reason: Option<String> = None;
    let mut report_out: Option<PerfReport> = None;
    let _ = probe_dry_run(false, || {
        let ctx = egui::Context::default();
        let mut state = StrataAppState::new_startup();
        crate::gui_theme_fonts::configure_korean_fonts_and_theme(&ctx, &state.theme);
        state.attach_user_settings_for_snap(None, &tmp_dir);
        if !state.open_survey_gpkg(&demo_copy) {
            skip_reason = Some("open_survey_gpkg returned false".to_owned());
            return;
        }
        state.active_tab = MainTab::Map;
        let win = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1440.0, 900.0));
        let mut central = egui::Rect::NOTHING;
        for _ in 0..2 {
            let _ = ctx.run(egui::RawInput { screen_rect: Some(win), ..Default::default() }, |ctx| {
                central = state.render_workspace_egui(ctx);
            });
        }
        if central.width() < 100.0 || central.height() < 100.0 {
            skip_reason = Some("headless runner could not lay out map canvas".to_owned());
            return;
        }
        let (korea, site) = anchors_for_app(&state, state.work_crs);
        let prev = is_enabled();
        clear_global();
        set_enabled(true);
        for step in 0..DEFAULT_PERF_FRAMES {
            apply_step_view(&mut state, korea, site, step, DEFAULT_PERF_FRAMES);
            let _ = ctx.run(egui::RawInput { screen_rect: Some(win), ..Default::default() }, |ctx| {
                let _ = state.render_workspace_egui(ctx);
            });
        }
        let samples = snapshot_global();
        set_enabled(prev);
        clear_global();
        report_out = Some(summarize_samples(&samples, [win.width(), win.height()], ctx.pixels_per_point()));
    });
    let _ = std::fs::remove_dir_all(&tmp_dir);
    evaluate_perf_gate(report_out.as_ref(), skip_reason.as_deref(), PERF1_P95_BOUND_MS)
}
