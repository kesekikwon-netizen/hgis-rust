//! App UI sound controller, stand-in download job hook, and central egui click hook (SND1).
//!
//! Download job connection note (`ORDER.yaml` SND1): once `DL1a..DL1c` merges into this branch,
//! pass the live job list snapshots to [`SoundController::observe_jobs`] inside [`on_frame_end`].

use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui;
use strata_types::theme_display::SoundLevel;

use crate::app_state::StrataAppState;
pub use crate::sound_output::{
    open_rodio_sink, AudioDeviceSink, AudioOpenError, LazyAudioBackend, RecordingBackend,
    SilentBackend, SoundBackend,
};
pub use crate::sound_synth::{
    recipe_spec, synthesize, SoundKind, SoundRecipeSpec, MAX_PEAK_DBFS, MAX_PEAK_LINEAR,
    SAMPLE_RATE_HZ,
};

const BUTTON_RATE_WINDOW: Duration = Duration::from_secs(1);
const MAX_BUTTON_TICKS_PER_SEC: usize = 8;
const PROGRESS_MIN_INTERVAL: Duration = Duration::from_secs(1);
const SAVED_GUIDE_PREFIX_KO: &str = "조사 데이터와 작업 구성을 저장했습니다: ";

/// Stand-in phase of one download job from the job list (`orders DL1a..DL1c`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundJobPhase {
    Running,
    Recovering,
    Done,
    /// `needs_user` is `false` for informational stop codes such as `NONE-1`, `true` otherwise.
    Stopped { needs_user: bool },
}

/// Stand-in view of one download job (`orders DL1a..DL1c`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SoundJobSnapshot {
    pub id: u64,
    pub fraction: Option<f32>,
    pub phase: SoundJobPhase,
}

#[derive(Debug, Clone, Copy, Default)]
struct JobTrack {
    passed_mask: u8,
    prev_phase: Option<SoundJobPhase>,
}

pub struct SoundController {
    backend: Box<dyn SoundBackend>,
    clock_origin: Instant,
    button_times: VecDeque<Duration>,
    last_progress_at: Option<Duration>,
    jobs: HashMap<u64, JobTrack>,
    prev_save_sig: Option<(PathBuf, usize)>,
}

impl fmt::Debug for SoundController {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SoundController")
            .field("recent_button_ticks", &self.button_times.len())
            .field("tracked_jobs", &self.jobs.len())
            .finish()
    }
}

impl SoundController {
    #[must_use]
    pub fn new(backend: Box<dyn SoundBackend>) -> Self {
        Self {
            backend,
            clock_origin: Instant::now(),
            button_times: VecDeque::with_capacity(MAX_BUTTON_TICKS_PER_SEC),
            last_progress_at: None,
            jobs: HashMap::new(),
            prev_save_sig: None,
        }
    }

    #[must_use]
    pub fn for_cli_args(args: &[String]) -> Self {
        if cfg!(test) || is_silent_cli_args(args) {
            Self::new(Box::new(SilentBackend))
        } else {
            Self::new(Box::new(LazyAudioBackend::new_rodio()))
        }
    }

    fn emit(&mut self, kind: SoundKind, level: SoundLevel) {
        if level != SoundLevel::Off {
            self.backend.play(kind, level);
        }
    }

    pub fn on_button_click(&mut self, now: Duration, level: SoundLevel) {
        if level == SoundLevel::Off {
            return;
        }
        while self
            .button_times
            .front()
            .is_some_and(|&t| now.saturating_sub(t) >= BUTTON_RATE_WINDOW)
        {
            let _ = self.button_times.pop_front();
        }
        if self.button_times.len() < MAX_BUTTON_TICKS_PER_SEC {
            self.button_times.push_back(now);
            self.emit(SoundKind::ButtonPress, level);
        }
    }

    pub fn observe_jobs(&mut self, jobs: &[SoundJobSnapshot], now: Duration, level: SoundLevel) {
        self.jobs.retain(|id, _| jobs.iter().any(|j| j.id == *id));
        let mut progress_crossed = false;

        for job in jobs {
            let mut track = self.jobs.get(&job.id).copied().unwrap_or_default();
            if matches!(
                track.prev_phase,
                Some(SoundJobPhase::Done | SoundJobPhase::Stopped { .. })
            ) && job.phase == SoundJobPhase::Running
                && job.fraction.unwrap_or(0.0) < 0.25
            {
                track.passed_mask = 0;
            }
            match job.phase {
                SoundJobPhase::Running => {
                    if let Some(frac) = job.fraction {
                        for (bit, threshold) in [(1_u8, 0.25_f32), (2, 0.50), (4, 0.75)] {
                            if frac >= threshold && (track.passed_mask & bit) == 0 {
                                track.passed_mask |= bit;
                                progress_crossed = true;
                            }
                        }
                    }
                }
                SoundJobPhase::Recovering => {}
                SoundJobPhase::Done => {
                    track.passed_mask = 0b111;
                    if track.prev_phase != Some(SoundJobPhase::Done) {
                        self.emit(SoundKind::DownloadDone, level);
                    }
                }
                SoundJobPhase::Stopped { needs_user } => {
                    if !matches!(track.prev_phase, Some(SoundJobPhase::Stopped { .. }))
                        && needs_user
                    {
                        self.emit(SoundKind::DownloadStopped, level);
                    }
                }
            }
            track.prev_phase = Some(job.phase);
            self.jobs.insert(job.id, track);
        }

        if progress_crossed
            && level != SoundLevel::Off
            && self
                .last_progress_at
                .is_none_or(|last| now.saturating_sub(last) >= PROGRESS_MIN_INTERVAL)
        {
            self.last_progress_at = Some(now);
            self.emit(SoundKind::DownloadProgress, level);
        }
    }

    pub fn on_survey_saved(&mut self, _now: Duration, level: SoundLevel) {
        self.emit(SoundKind::SurveySaved, level);
    }

    fn observe_app_save(&mut self, app: &StrataAppState, now: Duration) {
        let is_saved_banner = !app.is_dirty
            && app
                .home_map
                .draw_guide_message
                .as_deref()
                .is_some_and(|m| m.starts_with(SAVED_GUIDE_PREFIX_KO));
        let cur_sig = if is_saved_banner {
            app.survey_path
                .as_ref()
                .map(|p| (p.clone(), app.survey_tables.as_ptr() as usize))
        } else {
            None
        };
        if cur_sig.is_some() && cur_sig != self.prev_save_sig {
            self.on_survey_saved(now, app.display_options.sound_level);
        }
        self.prev_save_sig = cur_sig;
    }
}

#[must_use]
pub fn is_silent_cli_args(args: &[String]) -> bool {
    args.iter().any(|a| {
        matches!(
            a.as_str(),
            "--snap" | "--verify-all" | "--smoke-quit" | "--smoke"
        )
    })
}

/// Central end-of-frame click check using `egui 0.31.1`:
/// 1. `Context::output` (`egui-0.31.1/src/context.rs:981`) -> `PlatformOutput::events`
///    (`egui-0.31.1/src/data/output.rs:132-133`) -> `OutputEvent::Clicked(WidgetInfo)`
///    (`egui-0.31.1/src/data/output.rs:469-471`) for standard `egui` buttons, checkboxes,
///    radio buttons, selectable labels, combo boxes, and collapsing headers.
/// 2. `Context::interaction_snapshot` (`egui-0.31.1/src/context.rs:3743`,
///    `InteractionSnapshot::clicked` at `egui-0.31.1/src/interaction.rs:14-15`) +
///    `Context::read_response` (`egui-0.31.1/src/context.rs:1200`) for custom-painted studio
///    tabs, ribbon chips, and popup menu items (`Sense::click()` without `senses_drag()`),
///    excluding map/terrain canvases (`Sense::click_and_drag()`), text edits, sliders, and drags.
#[must_use]
pub fn frame_had_button_click(ctx: &egui::Context) -> bool {
    use egui::output::OutputEvent;
    use egui::WidgetType;

    let std_event = ctx.output(|o| {
        let mut any_clicked = false;
        let mut button_like = false;
        for ev in &o.events {
            if let OutputEvent::Clicked(info) = ev {
                any_clicked = true;
                if info.enabled
                    && matches!(
                        info.typ,
                        WidgetType::Button
                            | WidgetType::Checkbox
                            | WidgetType::RadioButton
                            | WidgetType::RadioGroup
                            | WidgetType::SelectableLabel
                            | WidgetType::ComboBox
                            | WidgetType::ColorButton
                            | WidgetType::ImageButton
                            | WidgetType::CollapsingHeader
                            | WidgetType::Link
                    )
                {
                    button_like = true;
                }
            }
        }
        any_clicked.then_some(button_like)
    });
    if let Some(matched) = std_event {
        return matched;
    }
    let Some(id) = ctx.interaction_snapshot(|i| i.clicked) else {
        return false;
    };
    ctx.read_response(id).is_some_and(|resp| {
        resp.enabled() && resp.clicked() && resp.sense.senses_click() && !resp.sense.senses_drag()
    })
}

fn controller_id() -> egui::Id {
    egui::Id::new("ka_sound_controller")
}

pub fn attach_controller_for_test(ctx: &egui::Context, controller: SoundController) {
    let arc = Arc::new(Mutex::new(controller));
    ctx.data_mut(|d| d.insert_temp(controller_id(), arc));
}

pub fn on_frame_end(ctx: &egui::Context, app: &StrataAppState) {
    if app.snap.is_some() {
        return;
    }
    let handle = ctx.data_mut(|d| {
        d.get_temp_mut_or_insert_with(controller_id(), || {
            let args: Vec<String> = std::env::args().collect();
            Arc::new(Mutex::new(SoundController::for_cli_args(&args)))
        })
        .clone()
    });
    let clicked = frame_had_button_click(ctx);
    if let Ok(mut ctrl) = handle.lock() {
        let now = ctrl.clock_origin.elapsed();
        if clicked {
            ctrl.on_button_click(now, app.display_options.sound_level);
        }
        ctrl.observe_app_save(app, now);
    };
}
