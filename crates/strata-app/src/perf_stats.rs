//! Data types and percentile summary math for the `PERF1` frame-time recorder.

use std::collections::BTreeMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};

pub const PART_COUNT: usize = 8;

/// Named UI/render parts timed within a frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum PerfPart {
    Ribbon = 0,
    StatusBar = 1,
    LeftPanel = 2,
    RightPanel = 3,
    BackgroundTiles = 4,
    CadastralTiles = 5,
    SurveyLayers = 6,
    Labels = 7,
}

impl PerfPart {
    pub const ALL: [Self; PART_COUNT] = [
        Self::Ribbon,
        Self::StatusBar,
        Self::LeftPanel,
        Self::RightPanel,
        Self::BackgroundTiles,
        Self::CadastralTiles,
        Self::SurveyLayers,
        Self::Labels,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ribbon => "ribbon",
            Self::StatusBar => "status_bar",
            Self::LeftPanel => "left_panel",
            Self::RightPanel => "right_panel",
            Self::BackgroundTiles => "background_tiles",
            Self::CadastralTiles => "cadastral_tiles",
            Self::SurveyLayers => "survey_layers",
            Self::Labels => "labels",
        }
    }
}

/// One completed frame's total duration and per-part durations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameSample {
    pub total: Duration,
    pub parts: [Duration; PART_COUNT],
}

/// Summary statistics in milliseconds for the whole frame or a single named part.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PartStats {
    pub median_ms: f64,
    pub p95_ms: f64,
    pub max_ms: f64,
}

/// JSON summary written by the scripted benchmark step and checked by the verify gate.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PerfReport {
    pub frames: usize,
    pub window_size: [f32; 2],
    pub screen_scale: f32,
    pub frame_total: PartStats,
    pub parts: BTreeMap<String, PartStats>,
}

/// Summarizes recorded frame samples into `median_ms`, `p95_ms`, and `max_ms` per part and total.
#[must_use]
pub fn summarize_samples(
    samples: &[FrameSample],
    window_size: [f32; 2],
    screen_scale: f32,
) -> PerfReport {
    let mut parts = BTreeMap::new();
    for part in PerfPart::ALL {
        let vals: Vec<f64> = samples
            .iter()
            .map(|s| s.parts[part as usize].as_secs_f64() * 1_000.0)
            .collect();
        parts.insert(part.as_str().to_owned(), stats_of(vals));
    }
    let totals: Vec<f64> = samples
        .iter()
        .map(|s| s.total.as_secs_f64() * 1_000.0)
        .collect();
    PerfReport {
        frames: samples.len(),
        window_size,
        screen_scale,
        frame_total: stats_of(totals),
        parts,
    }
}

fn stats_of(mut ms: Vec<f64>) -> PartStats {
    if ms.is_empty() {
        return PartStats::default();
    }
    ms.sort_by(f64::total_cmp);
    let n = ms.len();
    let median_ms = if n % 2 == 1 {
        ms[n / 2]
    } else {
        (ms[n / 2 - 1] + ms[n / 2]) * 0.5
    };
    let p95_idx = (n * 95).div_ceil(100).saturating_sub(1).min(n - 1);
    PartStats {
        median_ms,
        p95_ms: ms[p95_idx],
        max_ms: ms[n - 1],
    }
}
