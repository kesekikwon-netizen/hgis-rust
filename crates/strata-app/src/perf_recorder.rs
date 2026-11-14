//! Lightweight frame-time and per-part recorder (`PERF1`).
//!
//! Switched on by CLI flag (`--perf` / `--perf-record`) or environment variable
//! (`STRATA_PERF=1` / `KA_HGIS_PERF=1`), or programmatically during scripted `--snap` /
//! `--verify-all` measurement runs. When disabled, `begin_part` is a single relaxed atomic load
//! and branch with zero allocation and zero clock reads.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

pub use crate::perf_stats::{
    summarize_samples, FrameSample, PartStats, PerfPart, PerfReport, PART_COUNT,
};

pub const DEFAULT_RING_CAPACITY: usize = 256;

const STATE_UNINIT: u8 = 0;
const STATE_OFF: u8 = 1;
const STATE_ON: u8 = 2;

static GLOBAL_STATE: AtomicU8 = AtomicU8::new(STATE_UNINIT);
static GLOBAL_RECORDER: OnceLock<FrameRecorder> = OnceLock::new();

struct ActiveFrame {
    started_at: Instant,
    parts: [Duration; PART_COUNT],
}

struct RecorderInner {
    capacity: usize,
    ring: VecDeque<FrameSample>,
    active: Option<ActiveFrame>,
}

/// Ring-buffer recorder of frame and per-part durations.
pub struct FrameRecorder {
    enabled: AtomicBool,
    inner: Mutex<RecorderInner>,
    clock: Arc<dyn Fn() -> Instant + Send + Sync>,
}

impl std::fmt::Debug for FrameRecorder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FrameRecorder")
            .field("enabled", &self.enabled.load(Ordering::Relaxed))
            .finish_non_exhaustive()
    }
}

/// Completes the active frame measurement when dropped.
pub struct FrameGuard<'a> {
    rec: &'a FrameRecorder,
}

impl std::fmt::Debug for FrameGuard<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FrameGuard").finish_non_exhaustive()
    }
}

impl Drop for FrameGuard<'_> {
    fn drop(&mut self) {
        let end = (self.rec.clock)();
        let mut guard = self.rec.lock_inner();
        let Some(active) = guard.active.take() else {
            return;
        };
        let cap = guard.capacity;
        if cap == 0 {
            return;
        }
        if guard.ring.len() == cap {
            guard.ring.pop_front();
        }
        guard.ring.push_back(FrameSample {
            total: end.saturating_duration_since(active.started_at),
            parts: active.parts,
        });
    }
}

/// Accumulates one named part's elapsed time into the active frame when dropped.
pub struct PartGuard<'a> {
    rec: &'a FrameRecorder,
    part: PerfPart,
    started_at: Instant,
}

impl std::fmt::Debug for PartGuard<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PartGuard")
            .field("part", &self.part)
            .finish_non_exhaustive()
    }
}

impl Drop for PartGuard<'_> {
    fn drop(&mut self) {
        let elapsed = (self.rec.clock)().saturating_duration_since(self.started_at);
        if let Some(active) = self.rec.lock_inner().active.as_mut() {
            active.parts[self.part as usize] += elapsed;
        }
    }
}

impl FrameRecorder {
    fn lock_inner(&self) -> std::sync::MutexGuard<'_, RecorderInner> {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    #[must_use]
    pub fn with_clock(capacity: usize, clock: Arc<dyn Fn() -> Instant + Send + Sync>) -> Self {
        Self {
            enabled: AtomicBool::new(false),
            inner: Mutex::new(RecorderInner {
                capacity,
                ring: VecDeque::new(),
                active: None,
            }),
            clock,
        }
    }

    pub fn set_enabled(&self, on: bool) {
        if on {
            let mut g = self.lock_inner();
            let cap = g.capacity;
            if g.ring.capacity() < cap {
                let extra = cap - g.ring.capacity();
                g.ring.reserve_exact(extra);
            }
        } else {
            self.lock_inner().active = None;
        }
        self.enabled.store(on, Ordering::Release);
    }

    #[cfg(test)]
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    pub fn clear(&self) {
        let mut g = self.lock_inner();
        g.ring.clear();
        g.active = None;
    }

    #[inline]
    #[must_use]
    pub fn begin_frame(&self) -> Option<FrameGuard<'_>> {
        if !self.enabled.load(Ordering::Relaxed) {
            return None;
        }
        self.begin_frame_slow()
    }

    #[cold]
    #[inline(never)]
    fn begin_frame_slow(&self) -> Option<FrameGuard<'_>> {
        let now = (self.clock)();
        let mut g = self.lock_inner();
        if g.active.is_some() {
            return None;
        }
        g.active = Some(ActiveFrame {
            started_at: now,
            parts: [Duration::ZERO; PART_COUNT],
        });
        Some(FrameGuard { rec: self })
    }

    #[inline]
    #[must_use]
    pub fn begin_part(&self, part: PerfPart) -> Option<PartGuard<'_>> {
        if !self.enabled.load(Ordering::Relaxed) {
            return None;
        }
        self.begin_part_slow(part)
    }

    #[cold]
    #[inline(never)]
    fn begin_part_slow(&self, part: PerfPart) -> Option<PartGuard<'_>> {
        Some(PartGuard {
            rec: self,
            part,
            started_at: (self.clock)(),
        })
    }

    #[must_use]
    pub fn snapshot(&self) -> Vec<FrameSample> {
        self.lock_inner().ring.iter().copied().collect()
    }

    #[cfg(test)]
    #[must_use]
    pub fn ring_capacity(&self) -> usize {
        self.lock_inner().ring.capacity()
    }

    #[cfg(test)]
    #[must_use]
    pub fn summarize(&self, window_size: [f32; 2], screen_scale: f32) -> PerfReport {
        summarize_samples(&self.snapshot(), window_size, screen_scale)
    }
}

fn global_recorder() -> &'static FrameRecorder {
    GLOBAL_RECORDER
        .get_or_init(|| FrameRecorder::with_clock(DEFAULT_RING_CAPACITY, Arc::new(Instant::now)))
}

fn env_flag_on(name: &str) -> bool {
    std::env::var(name).is_ok_and(|v| {
        let t = v.trim();
        !t.is_empty() && t != "0" && !t.eq_ignore_ascii_case("false")
    })
}

#[cold]
#[inline(never)]
fn init_global_from_env() -> bool {
    let want = env_flag_on("STRATA_PERF")
        || env_flag_on("KA_HGIS_PERF")
        || std::env::args().any(|a| a == "--perf" || a == "--perf-record");
    set_enabled(want);
    want
}

#[inline]
#[must_use]
pub fn begin_frame() -> Option<FrameGuard<'static>> {
    let s = GLOBAL_STATE.load(Ordering::Relaxed);
    if s != STATE_ON && (s == STATE_OFF || !init_global_from_env()) {
        return None;
    }
    global_recorder().begin_frame()
}

#[inline]
#[must_use]
pub fn begin_part(part: PerfPart) -> Option<PartGuard<'static>> {
    if GLOBAL_STATE.load(Ordering::Relaxed) != STATE_ON {
        return None;
    }
    begin_part_global_slow(part)
}

#[cold]
#[inline(never)]
fn begin_part_global_slow(part: PerfPart) -> Option<PartGuard<'static>> {
    global_recorder().begin_part(part)
}

pub fn set_enabled(on: bool) {
    global_recorder().set_enabled(on);
    GLOBAL_STATE.store(if on { STATE_ON } else { STATE_OFF }, Ordering::Release);
}

#[must_use]
pub fn is_enabled() -> bool {
    GLOBAL_STATE.load(Ordering::Relaxed) == STATE_ON
}

pub fn clear_global() {
    global_recorder().clear();
}

#[must_use]
pub fn snapshot_global() -> Vec<FrameSample> {
    global_recorder().snapshot()
}
