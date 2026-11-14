//! Audio output backends: silent, recording (for tests), and lazy background rodio sink (SND1/SND1b).

use std::collections::VecDeque;
use std::fmt;
use std::num::NonZero;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use strata_types::theme_display::SoundLevel;

use crate::sound_synth::{synthesize, SoundKind, SAMPLE_RATE_HZ};

/// Maximum age of a pending sound request before it is dropped as late (`ORDER.yaml` SND1/SND1b).
const MAX_SOUND_LATENESS: Duration = Duration::from_millis(250);
/// Small non-blocking handover cap for non-tick sounds while the device opens or mixes (`SND1b`).
const MAX_PENDING_NON_TICK: usize = 4;

pub trait SoundBackend: Send {
    fn play(&mut self, kind: SoundKind, level: SoundLevel);
}

#[derive(Debug, Default)]
pub struct SilentBackend;

impl SoundBackend for SilentBackend {
    fn play(&mut self, _kind: SoundKind, _level: SoundLevel) {}
}

#[derive(Debug, Clone, Default)]
pub struct RecordingBackend {
    events: Arc<Mutex<Vec<(SoundKind, SoundLevel)>>>,
}

impl RecordingBackend {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn events(&self) -> Vec<(SoundKind, SoundLevel)> {
        self.events.lock().map(|g| g.clone()).unwrap_or_default()
    }

    pub fn clear(&self) {
        if let Ok(mut g) = self.events.lock() {
            g.clear();
        }
    }
}

impl SoundBackend for RecordingBackend {
    fn play(&mut self, kind: SoundKind, level: SoundLevel) {
        if let Ok(mut g) = self.events.lock() {
            g.push((kind, level));
        }
    }
}

pub trait AudioDeviceSink: Send {
    fn play_mono(&mut self, sample_rate: u32, samples: Vec<f32>);
}

struct RodioSink {
    sink: rodio::MixerDeviceSink,
}

impl AudioDeviceSink for RodioSink {
    fn play_mono(&mut self, sample_rate: u32, samples: Vec<f32>) {
        if samples.is_empty() {
            return;
        }
        let Some(ch) = NonZero::new(1_u16) else {
            return;
        };
        let Some(sr) = NonZero::new(sample_rate) else {
            return;
        };
        let buf = rodio::buffer::SamplesBuffer::new(ch, sr, samples);
        self.sink.mixer().add(buf);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioOpenError;

/// Opens the default OS output sink silently: no stderr output on error or drop.
pub fn open_rodio_sink() -> Result<Box<dyn AudioDeviceSink>, AudioOpenError> {
    let builder = rodio::DeviceSinkBuilder::from_default_device().map_err(|_| AudioOpenError)?;
    let mut sink = builder
        .with_error_callback(|_| {})
        .open_sink_or_fallback()
        .map_err(|_| AudioOpenError)?;
    sink.log_on_drop(false);
    Ok(Box::new(RodioSink { sink }))
}

type OpenerFn = Box<dyn FnOnce() -> Result<Box<dyn AudioDeviceSink>, AudioOpenError> + Send + 'static>;

struct Mailbox {
    queue: VecDeque<(Instant, SoundKind, SoundLevel)>,
    closed: bool,
}

struct SharedMailbox {
    state: Mutex<Mailbox>,
    cvar: Condvar,
}

impl SharedMailbox {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(Mailbox {
                queue: VecDeque::with_capacity(MAX_PENDING_NON_TICK),
                closed: false,
            }),
            cvar: Condvar::new(),
        })
    }

    fn close(&self) {
        if let Ok(mut g) = self.state.lock() {
            g.closed = true;
            self.cvar.notify_all();
        }
    }

    fn recv(&self) -> Option<(Instant, SoundKind, SoundLevel)> {
        let mut g = self.state.lock().ok()?;
        loop {
            if let Some(item) = g.queue.pop_front() {
                return Some(item);
            }
            if g.closed {
                return None;
            }
            g = self.cvar.wait(g).ok()?;
        }
    }
}

struct MailboxSender(Arc<SharedMailbox>);

impl MailboxSender {
    fn send(&self, kind: SoundKind, level: SoundLevel) -> bool {
        let Ok(mut g) = self.0.state.lock() else {
            return false;
        };
        if g.closed {
            return false;
        }
        let now = Instant::now();
        if kind == SoundKind::ButtonPress {
            if g.queue.is_empty() {
                g.queue.push_back((now, kind, level));
                self.0.cvar.notify_one();
            }
        } else {
            g.queue
                .retain(|(at, k, _)| *k != SoundKind::ButtonPress && at.elapsed() <= MAX_SOUND_LATENESS);
            if g.queue.len() < MAX_PENDING_NON_TICK {
                g.queue.push_back((now, kind, level));
                self.0.cvar.notify_one();
            }
        }
        true
    }
}

impl Drop for MailboxSender {
    fn drop(&mut self) {
        self.0.close();
    }
}

struct WorkerCloseGuard(Arc<SharedMailbox>);

impl Drop for WorkerCloseGuard {
    fn drop(&mut self) {
        self.0.close();
    }
}

enum WorkerState {
    Unopened(OpenerFn),
    Active(MailboxSender),
    Disabled,
}

/// Opens the audio device lazily on the first sound on its own background thread.
/// Ticks never queue; non-tick sounds evict any unplayed tick so a tick never drops a note.
pub struct LazyAudioBackend {
    state: WorkerState,
}

impl fmt::Debug for LazyAudioBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let tag = match &self.state {
            WorkerState::Unopened(_) => "Unopened",
            WorkerState::Active(_) => "Active",
            WorkerState::Disabled => "Disabled",
        };
        f.debug_struct("LazyAudioBackend")
            .field("state", &tag)
            .finish()
    }
}

impl LazyAudioBackend {
    #[must_use]
    pub fn new_rodio() -> Self {
        Self::with_opener(open_rodio_sink)
    }

    #[must_use]
    pub fn with_opener<F>(opener: F) -> Self
    where
        F: FnOnce() -> Result<Box<dyn AudioDeviceSink>, AudioOpenError> + Send + 'static,
    {
        Self {
            state: WorkerState::Unopened(Box::new(opener)),
        }
    }

    fn ensure_worker(&mut self) -> Option<&MailboxSender> {
        if matches!(self.state, WorkerState::Unopened(_)) {
            let prev = std::mem::replace(&mut self.state, WorkerState::Disabled);
            if let WorkerState::Unopened(opener) = prev {
                let shared = SharedMailbox::new();
                let worker_shared = Arc::clone(&shared);
                let spawned = std::thread::Builder::new()
                    .name("strata-sound".to_owned())
                    .spawn(move || {
                        let _close_on_exit = WorkerCloseGuard(Arc::clone(&worker_shared));
                        let Some((first_at, first_kind, first_level)) = worker_shared.recv() else {
                            return;
                        };
                        let Ok(mut sink) = opener() else {
                            return;
                        };
                        if first_at.elapsed() <= MAX_SOUND_LATENESS {
                            sink.play_mono(SAMPLE_RATE_HZ, synthesize(first_kind, first_level));
                        }
                        while let Some((sent_at, kind, level)) = worker_shared.recv() {
                            if sent_at.elapsed() <= MAX_SOUND_LATENESS {
                                sink.play_mono(SAMPLE_RATE_HZ, synthesize(kind, level));
                            }
                        }
                    });
                if spawned.is_ok() {
                    self.state = WorkerState::Active(MailboxSender(shared));
                }
            }
        }
        match &self.state {
            WorkerState::Active(tx) => Some(tx),
            WorkerState::Unopened(_) | WorkerState::Disabled => None,
        }
    }
}

impl SoundBackend for LazyAudioBackend {
    fn play(&mut self, kind: SoundKind, level: SoundLevel) {
        if level == SoundLevel::Off {
            return;
        }
        let alive = if let Some(tx) = self.ensure_worker() {
            tx.send(kind, level)
        } else {
            false
        };
        if !alive {
            self.state = WorkerState::Disabled;
        }
    }
}
