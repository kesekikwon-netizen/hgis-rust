//! The startup notice — C++ `KaStartupSplash` with the 새 모양 picture (`KaSplashStrata`): a paper card with the
//! name, the status and two notice lines from its first frame, and a section that draws itself in once the
//! app is ready. It is read for five seconds (owner 2026-10-04 「첫 로딩화면을 5초로 하라」); a click never
//! closes it early, and the home screen opens when it is over.

use eframe::egui;

use crate::app_state::StrataAppState;

#[path = "startup_notice_paint.rs"]
pub(crate) mod paint;
#[path = "startup_notice_section.rs"]
mod section;

/// `ReadingDurationMs` (KaStartupSplash.h:22) and `kMinMotionMs` (KaStartupSplash.cpp:37), in seconds.
const READING_SECONDS: f64 = 5.0;
const MIN_MOTION_SECONDS: f64 = 3.0;
/// The level string reaches the staff a little before the notice closes (KaStartupSplash.cpp:201-203).
const PULLED_BY: f64 = 0.92;
/// `kTimelineSeconds` (KaSplashStrata.h:21): the hairlines have faded, the 기준점 stays.
pub(crate) const TIMELINE_SECONDS: f64 = 4.8;
/// The card and the transparent margin around it, wider below for the shadow (KaStartupSplash.cpp:30-31).
const CARD: [f32; 2] = [700.0, 400.0];
const MARGIN: [f32; 3] = [16.0, 10.0, 24.0];

/// OutCubic.
pub(crate) fn ease(x: f64) -> f64 {
    1.0 - (1.0 - x.clamp(0.0, 1.0)).powi(3)
}
/// The timeline, in seconds since the app became ready; each runs 0..1 (KaSplashStrata.cpp:215-223).
/// Boundaries are inked left to right, the lowest (index 0) first.
pub(crate) fn boundary_reveal(index: u8, seconds: f64) -> f64 {
    ease((seconds - 0.4 - 0.35 * f64::from(index)) / 0.35)
}
/// The levelling staff grows from the bedrock up.
pub(crate) fn staff_reveal(seconds: f64) -> f64 {
    ease((seconds - 0.4) / 1.4)
}
/// The pit, the ground line, the peg and Ⅰ–Ⅴ.
pub(crate) fn pit_reveal(seconds: f64) -> f64 {
    ease((seconds - 1.9) / 0.5)
}
/// Stratum Ⅲ fills with clay, left to right.
pub(crate) fn clay_reveal(seconds: f64) -> f64 {
    ease((seconds - 2.6) / 0.6)
}
/// The 기준점 lands.
pub(crate) fn marker_reveal(seconds: f64) -> f64 {
    ease((seconds - 3.2) / 0.4)
}
/// The two hairlines slide in, then fade.
pub(crate) fn cross_visibility(seconds: f64) -> f64 {
    if seconds < 3.2 { 0.0 } else { 1.0 - ease((seconds - 3.6) / 1.2) }
}

/// One frame of the card (`KaSplashStrata::Frame`).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CardFrame {
    /// Seconds of the timeline; 0 while the app is still starting.
    pub(crate) seconds: f64,
    /// 0..1: how far the level string is pulled.
    pub(crate) progress: f64,
    pub(crate) status: &'static str,
    pub(crate) version: &'static str,
}

/// The notice while it shows. Times are seconds since it appeared.
#[derive(Debug, Clone)]
pub(crate) struct StartupNotice {
    reduced_motion: bool,
    /// The time of the first frame, on the clock of the window.
    appeared: Option<f64>,
    ready_at: Option<f64>,
}

impl StartupNotice {
    pub(crate) fn new(reduced_motion: bool) -> Self {
        Self { reduced_motion, appeared: None, ready_at: None }
    }

    /// `markReady`: the app is ready, `now` seconds after the notice appeared.
    pub(crate) fn mark_ready(&mut self, now: f64) {
        self.ready_at.get_or_insert(now);
    }

    /// When the notice closes (`m_doneAtMs`): after the reading time, or later so that the section gets its
    /// three seconds to draw (owner 2026-10-04 「모션그래픽이 안나와」).
    fn done_at(&self) -> Option<f64> {
        let ready = self.ready_at?;
        Some(if self.reduced_motion { READING_SECONDS } else { READING_SECONDS.max(ready + MIN_MOTION_SECONDS) })
    }

    pub(crate) fn progress(&self, now: f64) -> f64 {
        self.done_at().map_or(0.0, |done| (now / (done * PULLED_BY)).min(1.0))
    }

    /// Seconds of the timeline: it is stretched or squeezed to end where the string arrives (`:249-250`).
    pub(crate) fn seconds(&self, now: f64) -> f64 {
        let (Some(ready), Some(done)) = (self.ready_at, self.done_at()) else { return 0.0 };
        (now - ready).max(0.0) * TIMELINE_SECONDS / ((done - ready) * PULLED_BY).max(1.0)
    }

    pub(crate) fn status(&self, now: f64) -> &'static str {
        match self.ready_at {
            None => "앱을 준비하고 있습니다",
            Some(_) if self.progress(now) >= 1.0 => "준비 완료",
            Some(_) => "잠시 후 홈 화면이 열립니다",
        }
    }

    pub(crate) fn done(&self, now: f64) -> bool {
        self.done_at().is_some_and(|done| now >= done)
    }

    /// What to paint now. Reduced motion: the finished picture at once.
    fn frame(&self, now: f64) -> CardFrame {
        let (seconds, progress) = if self.reduced_motion { (10.0, 1.0) } else { (self.seconds(now), self.progress(now)) };
        CardFrame { seconds, progress, status: self.status(now), version: env!("CARGO_PKG_VERSION") }
    }
}

/// Windows 「애니메이션 효과 표시」 is not read yet; `KA_HGIS_REDUCED_MOTION=1` asks for the still picture.
pub(crate) fn reduced_motion_requested() -> bool {
    std::env::var("KA_HGIS_REDUCED_MOTION").is_ok_and(|value| value == "1")
}

/// The first window of the app: the card of the notice, without a frame and see-through around the card,
/// in the middle of the screen; or, without a notice, the main window as `main_window` builds it.
pub(crate) fn first_window(notice: bool, main_window: egui::ViewportBuilder) -> (egui::ViewportBuilder, bool) {
    if !notice {
        return (main_window, false);
    }
    let size = [CARD[0] + 2.0 * MARGIN[0], CARD[1] + MARGIN[1] + MARGIN[2]];
    let card = egui::ViewportBuilder { min_inner_size: None, maximized: None, position: None, ..main_window };
    (card.with_inner_size(size).with_decorations(false).with_transparent(true).with_resizable(false), true)
}

impl StrataAppState {
    /// Paints the notice in place of the app while it is read. `false` once it is over: the window then
    /// becomes the main window (C++ `MainWindow.cpp:243, 277`: 1280 x 900, shown maximized).
    pub(crate) fn show_startup_notice(&mut self, ctx: &egui::Context) -> bool {
        let Some(notice) = self.startup_notice.as_mut() else { return false };
        let clock = ctx.input(|i| i.time);
        let now = clock - *notice.appeared.get_or_insert(clock);
        // The Rust app has nothing to load before its first frame: it is ready when the notice appears.
        notice.mark_ready(now);
        if notice.done(now) {
            self.startup_notice = None;
            ctx.set_fonts(crate::gui_theme_fonts::font_definitions());
            ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Resizable(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(egui::vec2(960.0, 600.0)));
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(1280.0, 900.0)));
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(16.0, 16.0)));
            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(std::env::var("KA_HGIS_WINDOWED").is_err()));
            return false;
        }
        let frame = notice.frame(now);
        egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| {
            let window = ui.max_rect();
            let card = egui::Rect::from_min_max(window.min + egui::vec2(MARGIN[0], MARGIN[1]), window.max - egui::vec2(MARGIN[0], MARGIN[2]));
            paint::paint_card(ui.painter(), card, paint::icon(ctx).as_ref(), &frame);
        });
        ctx.request_repaint();
        true
    }
}

#[cfg(test)]
#[path = "startup_notice_tests.rs"]
mod tests;
