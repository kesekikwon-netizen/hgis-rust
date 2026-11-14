//! Audio output backends: silent, recording (for tests), and lazy background rodio sink (SND1).

use std::fmt;
use std::num::NonZero;
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use strata_types::theme_display::SoundLevel;

use crate::sound_synth::{synthesize, SoundKind, SAMPLE_RATE_HZ};

/// Maximum age of a pending sound request before it is dropped as late (`ORDER.yaml` SND1).
const MAX_SOUND_LATENESS: Duration = Duration::from_millis(250);

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

enum WorkerState {
    Unopened(OpenerFn),
    Active(SyncSender<(Instant, SoundKind, SoundLevel)>),
    Disabled,
}

/// Opens the audio device lazily on the first sound on its own background thread.
/// Uses a bounded channel of size 1 (`try_send`) so sounds never queue up and never delay the UI.
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

    fn ensure_worker(&mut self) -> Option<&SyncSender<(Instant, SoundKind, SoundLevel)>> {
        if matches!(self.state, WorkerState::Unopened(_)) {
            let prev = std::mem::replace(&mut self.state, WorkerState::Disabled);
            if let WorkerState::Unopened(opener) = prev {
                let (tx, rx) = mpsc::sync_channel::<(Instant, SoundKind, SoundLevel)>(1);
                let spawned = std::thread::Builder::new()
                    .name("strata-sound".to_owned())
                    .spawn(move || {
                        let Ok((first_at, first_kind, first_level)) = rx.recv() else {
                            return;
                        };
                        let Ok(mut sink) = opener() else {
                            return;
                        };
                        if first_at.elapsed() <= MAX_SOUND_LATENESS {
                            sink.play_mono(SAMPLE_RATE_HZ, synthesize(first_kind, first_level));
                        }
                        while let Ok((sent_at, kind, level)) = rx.recv() {
                            if sent_at.elapsed() <= MAX_SOUND_LATENESS {
                                sink.play_mono(SAMPLE_RATE_HZ, synthesize(kind, level));
                            }
                        }
                    });
                if spawned.is_ok() {
                    self.state = WorkerState::Active(tx);
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
        let disconnected = if let Some(tx) = self.ensure_worker() {
            matches!(
                tx.try_send((Instant::now(), kind, level)),
                Err(mpsc::TrySendError::Disconnected(_))
            )
        } else {
            false
        };
        if disconnected {
            self.state = WorkerState::Disabled;
        }
    }
}
