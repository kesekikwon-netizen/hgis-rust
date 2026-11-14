//! Procedural synthesis of quiet UI tones (`docs/EVOLUTION.md` E-12, order SND1).
//!
//! All sounds are synthesized in memory as 44.1 kHz mono `f32` PCM with raised-cosine
//! attack and release envelopes so every buffer starts and ends at `0.0` (click-free)
//! and peaks strictly below `-24 dBFS` (`0.06309573` linear). No audio files are used.

use std::f32::consts::PI;

use strata_types::theme_display::SoundLevel;

pub const SAMPLE_RATE_HZ: u32 = 44_100;
/// Maximum allowed peak amplitude (`-24 dBFS` = `10^(-24/20)`).
pub const MAX_PEAK_DBFS: f32 = -24.0;
pub const MAX_PEAK_LINEAR: f32 = 0.063_095_73;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SoundKind {
    /// Button press: 12 ms tactile paper/wood tick, sine sweep 760 Hz -> 420 Hz,
    /// 1.5 ms raised-cosine attack, exp(-t / 4.5 ms) body decay, 2.5 ms raised-cosine tail;
    /// peak -30.0 dBFS (`소리 작게`, default) / -25.0 dBFS (`소리 보통`).
    ButtonPress,
    /// Download progress (25% / 50% / 75%): 40 ms soft step pip, D5 (587.33 Hz) + 0.15 * D6
    /// (1174.66 Hz), 4 ms raised-cosine attack, exp(-t / 18 ms) decay, 8 ms raised-cosine tail;
    /// peak -32.0 dBFS (`소리 작게`, default) / -27.0 dBFS (`소리 보통`).
    DownloadProgress,
    /// Download done: 180 ms (< 250 ms) two rising notes — Note 1 (0..85 ms): C5 (523.25 Hz) plus
    /// 0.12 * C6 (1046.50 Hz); Note 2 (85..180 ms): E5 (659.25 Hz) plus 0.12 * E6 (1318.51 Hz),
    /// each with 6 ms raised-cosine attack, exp(-t / 48 ms) decay, 12 ms raised-cosine tail;
    /// peak -28.5 dBFS (`소리 작게`, default) / -24.5 dBFS (`소리 보통`).
    DownloadDone,
    /// Download stopped (needs user): 140 ms single low soft note, D4 (293.66 Hz) + 0.20 * D5
    /// (587.33 Hz), 8 ms raised-cosine attack, exp(-t / 65 ms) decay, 18 ms raised-cosine tail;
    /// peak -28.5 dBFS (`소리 작게`, default) / -24.5 dBFS (`소리 보통`).
    DownloadStopped,
    /// Survey saved: 110 ms single warm note, 0.85 * G4 (392.00 Hz) + 0.15 * D5 (587.33 Hz),
    /// 6 ms raised-cosine attack, exp(-t / 52 ms) decay, 15 ms raised-cosine tail;
    /// peak -29.0 dBFS (`소리 작게`, default) / -24.5 dBFS (`소리 보통`).
    SurveySaved,
}

impl SoundKind {
    pub const ALL: [Self; 5] = [
        Self::ButtonPress,
        Self::DownloadProgress,
        Self::DownloadDone,
        Self::DownloadStopped,
        Self::SurveySaved,
    ];
}

#[derive(Debug, Clone, Copy)]
pub struct SoundRecipeSpec {
    pub duration_ms: u32,
    pub quiet_peak_dbfs: f32,
    pub normal_peak_dbfs: f32,
}

#[must_use]
pub const fn recipe_spec(kind: SoundKind) -> SoundRecipeSpec {
    match kind {
        SoundKind::ButtonPress => SoundRecipeSpec {
            duration_ms: 12,
            quiet_peak_dbfs: -30.0,
            normal_peak_dbfs: -25.0,
        },
        SoundKind::DownloadProgress => SoundRecipeSpec {
            duration_ms: 40,
            quiet_peak_dbfs: -32.0,
            normal_peak_dbfs: -27.0,
        },
        SoundKind::DownloadDone => SoundRecipeSpec {
            duration_ms: 180,
            quiet_peak_dbfs: -28.5,
            normal_peak_dbfs: -24.5,
        },
        SoundKind::DownloadStopped => SoundRecipeSpec {
            duration_ms: 140,
            quiet_peak_dbfs: -28.5,
            normal_peak_dbfs: -24.5,
        },
        SoundKind::SurveySaved => SoundRecipeSpec {
            duration_ms: 110,
            quiet_peak_dbfs: -29.0,
            normal_peak_dbfs: -24.5,
        },
    }
}

#[inline]
fn db_to_linear(dbfs: f32) -> f32 {
    10.0_f32.powf(dbfs / 20.0)
}

#[inline]
fn raised_cosine_envelope(idx: usize, len: usize, attack_s: f32, decay_tau_s: f32, release_s: f32) -> f32 {
    if len <= 1 || idx == 0 || idx + 1 >= len {
        return 0.0;
    }
    let sr = SAMPLE_RATE_HZ as f32;
    let t = idx as f32 / sr;
    let rem = (len - 1 - idx) as f32 / sr;
    let att = if t < attack_s {
        0.5 * (1.0 - (PI * t / attack_s).cos())
    } else {
        1.0
    };
    let rel = if rem < release_s {
        0.5 * (1.0 - (PI * rem / release_s).cos())
    } else {
        1.0
    };
    att * (-t / decay_tau_s).exp() * rel
}

fn fill_note(
    out: &mut [f32],
    f1_hz: f32,
    f2_hz: f32,
    w2: f32,
    attack_s: f32,
    decay_tau_s: f32,
    release_s: f32,
) {
    let len = out.len();
    let sr = SAMPLE_RATE_HZ as f32;
    let w1 = 1.0 - w2;
    for (i, slot) in out.iter_mut().enumerate() {
        let t = i as f32 / sr;
        let env = raised_cosine_envelope(i, len, attack_s, decay_tau_s, release_s);
        let osc = w1 * (2.0 * PI * f1_hz * t).sin() + w2 * (2.0 * PI * f2_hz * t).sin();
        *slot = env * osc;
    }
}

#[must_use]
pub fn synthesize(kind: SoundKind, level: SoundLevel) -> Vec<f32> {
    let spec = recipe_spec(kind);
    let target_dbfs = match level {
        SoundLevel::Off => return Vec::new(),
        SoundLevel::Quiet => spec.quiet_peak_dbfs,
        SoundLevel::Normal => spec.normal_peak_dbfs,
    };
    let len = (spec.duration_ms as usize * SAMPLE_RATE_HZ as usize) / 1000;
    let mut buf = vec![0.0_f32; len];
    match kind {
        SoundKind::ButtonPress => {
            let sr = SAMPLE_RATE_HZ as f32;
            let mut phase = 0.0_f32;
            for (i, slot) in buf.iter_mut().enumerate() {
                let frac = i as f32 / len.max(1) as f32;
                let freq = 760.0 + (420.0 - 760.0) * frac;
                let env = raised_cosine_envelope(i, len, 0.0015, 0.0045, 0.0025);
                *slot = env * phase.sin();
                phase += 2.0 * PI * freq / sr;
            }
        }
        SoundKind::DownloadProgress => {
            fill_note(&mut buf, 587.33, 1_174.66, 0.15, 0.004, 0.018, 0.008);
        }
        SoundKind::DownloadDone => {
            let split = (85 * SAMPLE_RATE_HZ as usize) / 1000;
            fill_note(&mut buf[..split], 523.25, 1_046.50, 0.12, 0.006, 0.048, 0.012);
            fill_note(&mut buf[split..], 659.25, 1_318.51, 0.12, 0.006, 0.048, 0.012);
        }
        SoundKind::DownloadStopped => {
            fill_note(&mut buf, 293.66, 587.33, 0.20, 0.008, 0.065, 0.018);
        }
        SoundKind::SurveySaved => {
            fill_note(&mut buf, 392.00, 587.33, 0.15, 0.006, 0.052, 0.015);
        }
    }
    let raw_peak = buf.iter().fold(0.0_f32, |m, s| m.max(s.abs()));
    if raw_peak > 0.0 {
        let scale = db_to_linear(target_dbfs).min(MAX_PEAK_LINEAR) / raw_peak;
        for s in &mut buf {
            *s *= scale;
        }
    }
    buf
}
