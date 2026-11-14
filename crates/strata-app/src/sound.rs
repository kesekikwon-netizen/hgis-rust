//! App UI sound controller and stand-in download job hook (SND1/SND1b).
//!
//! Download job connection note (`ORDER.yaml` SND1): once `DL1a..DL1c` merges into this branch,
//! pass the live job list snapshots to [`SoundController::observe_jobs`] inside [`on_frame_end`].

#[path = "sound_frame.rs"]
mod sound_frame;
#[cfg(test)]
#[path = "sound_tests_b.rs"]
mod sound_tests_b;

use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use strata_types::theme_display::SoundLevel;

pub use self::sound_frame::{attach_controller_for_test, frame_had_button_click, on_frame_end};
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
const ENDING_MIN_INTERVAL: Duration = Duration::from_secs(1);

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

fn info_priority(kind: SoundKind) -> u8 {
    match kind {
        SoundKind::ButtonPress => 0,
        SoundKind::DownloadProgress => 1,
        SoundKind::DownloadDone => 2,
        SoundKind::DownloadStopped => 3,
        SoundKind::SurveySaved => 4,
    }
}

pub struct SoundController {
    backend: Box<dyn SoundBackend>,
    pub(super) clock_origin: Instant,
    button_times: VecDeque<Duration>,
    last_progress_at: Option<Duration>,
    last_done_at: Option<Duration>,
    last_stopped_at: Option<Duration>,
    jobs: HashMap<u64, JobTrack>,
    pub(super) prev_save_sig: Option<(PathBuf, usize)>,
    in_frame: bool,
    frame_tick: Option<(Duration, SoundLevel)>,
    frame_info: Option<(SoundKind, SoundLevel)>,
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
            last_done_at: None,
            last_stopped_at: None,
            jobs: HashMap::new(),
            prev_save_sig: None,
            in_frame: false,
            frame_tick: None,
            frame_info: None,
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

    fn emit_tick_if_allowed(&mut self, now: Duration, level: SoundLevel) {
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

    fn stage_or_emit_info(&mut self, kind: SoundKind, level: SoundLevel) {
        if level == SoundLevel::Off {
            return;
        }
        if self.in_frame {
            let replace = self
                .frame_info
                .is_none_or(|(cur, _)| info_priority(kind) >= info_priority(cur));
            if replace {
                self.frame_info = Some((kind, level));
            }
        } else {
            self.emit(kind, level);
        }
    }

    pub fn begin_frame(&mut self) {
        self.in_frame = true;
        self.frame_tick = None;
        self.frame_info = None;
    }

    pub fn end_frame(&mut self) {
        self.in_frame = false;
        let tick = self.frame_tick.take();
        if let Some((kind, level)) = self.frame_info.take() {
            self.emit(kind, level);
        } else if let Some((now, level)) = tick {
            self.emit_tick_if_allowed(now, level);
        }
    }

    pub fn on_button_click(&mut self, now: Duration, level: SoundLevel) {
        if level == SoundLevel::Off {
            return;
        }
        if self.in_frame {
            self.frame_tick = Some((now, level));
        } else {
            self.emit_tick_if_allowed(now, level);
        }
    }

    pub fn observe_jobs(&mut self, jobs: &[SoundJobSnapshot], now: Duration, level: SoundLevel) {
        self.jobs.retain(|id, _| jobs.iter().any(|j| j.id == *id));
        let mut progress_crossed = false;
        let mut done_crossed = false;
        let mut stopped_crossed = false;

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
                    if matches!(track.prev_phase, Some(p) if p != SoundJobPhase::Done) {
                        done_crossed = true;
                    }
                }
                SoundJobPhase::Stopped { needs_user } => {
                    if matches!(
                        track.prev_phase,
                        Some(SoundJobPhase::Running | SoundJobPhase::Recovering)
                    ) && needs_user
                    {
                        stopped_crossed = true;
                    }
                }
            }
            track.prev_phase = Some(job.phase);
            self.jobs.insert(job.id, track);
        }

        if stopped_crossed
            && level != SoundLevel::Off
            && self.last_stopped_at.is_none_or(|t| now.saturating_sub(t) >= ENDING_MIN_INTERVAL)
        {
            self.last_stopped_at = Some(now);
            self.stage_or_emit_info(SoundKind::DownloadStopped, level);
        }
        if done_crossed
            && level != SoundLevel::Off
            && self.last_done_at.is_none_or(|t| now.saturating_sub(t) >= ENDING_MIN_INTERVAL)
        {
            self.last_done_at = Some(now);
            self.stage_or_emit_info(SoundKind::DownloadDone, level);
        }
        if progress_crossed
            && level != SoundLevel::Off
            && self.last_progress_at.is_none_or(|t| now.saturating_sub(t) >= PROGRESS_MIN_INTERVAL)
        {
            self.last_progress_at = Some(now);
            self.stage_or_emit_info(SoundKind::DownloadProgress, level);
        }
    }

    pub fn on_survey_saved(&mut self, _now: Duration, level: SoundLevel) {
        self.stage_or_emit_info(SoundKind::SurveySaved, level);
    }
}

#[must_use]
pub fn is_silent_cli_args(args: &[String]) -> bool {
    args.iter().any(|a| {
        matches!(a.as_str(), "--snap" | "--verify-all" | "--smoke-quit" | "--smoke")
    })
}
