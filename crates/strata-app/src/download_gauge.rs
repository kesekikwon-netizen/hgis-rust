//! Compact download gauge in the bottom status bar
//! (`docs/RUST_PLAN.md` §2.8; 4 px bar precedent in
//! `target/cpp-ref/src/app/KaProgressPlate.cpp:15-24` and
//! `target/cpp-ref/data/theme/ka-hgis.qss:1567-1568`).

use std::time::Duration;

use eframe::egui::{self, Color32, FontId, Pos2, Rect, RichText, Sense, Vec2};

use crate::app_state::StrataAppState;
use crate::download_jobs::{DownloadJob, DownloadJobState};
use crate::status_bar_text::{gauge_job_text, gauge_multi_summary_text};

#[path = "download_gauge_popup.rs"]
mod download_gauge_popup;

/// Slim download bar dimensions (`KaProgressPlate.cpp:19`).
pub(crate) const GAUGE_BAR_WIDTH: f32 = 120.0;
pub(crate) const GAUGE_BAR_HEIGHT: f32 = 4.0;
pub(super) const POPUP_BAR_WIDTH: f32 = 84.0;

pub(super) fn tok(c: [u8; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3])
}

#[must_use]
pub(crate) fn gauge_title_budget(avail_w: f32) -> usize {
    if avail_w >= 620.0 {
        16
    } else if avail_w >= 480.0 {
        8
    } else {
        5
    }
}

pub(super) fn job_fraction(job: &DownloadJob) -> Option<f32> {
    match &job.state {
        DownloadJobState::Running { fraction, .. } => *fraction,
        _ => None,
    }
}

pub(super) fn paint_slim_bar(
    ui: &mut egui::Ui,
    width: f32,
    fraction: Option<f32>,
    now: f64,
    track_color: Color32,
    fill_color: Color32,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, 14.0), Sense::click());
    let track = Rect::from_center_size(rect.center(), Vec2::new(width, GAUGE_BAR_HEIGHT));
    ui.painter().rect_filled(track, 2.0, track_color);
    if let Some(f) = fraction {
        let w = (width * f.clamp(0.0, 1.0)).max(if f > 0.0 { 2.0 } else { 0.0 });
        if w > 0.0 {
            let fill = Rect::from_min_size(track.min, Vec2::new(w, GAUGE_BAR_HEIGHT));
            ui.painter().rect_filled(fill, 2.0, fill_color);
        }
    } else {
        let seg_w = (width * 0.30).max(16.0);
        let travel = (width - seg_w).max(0.0);
        let phase = (now * 0.9).rem_euclid(2.0);
        let t = if phase <= 1.0 { phase as f32 } else { (2.0 - phase) as f32 };
        let seg = Rect::from_min_size(
            Pos2::new(track.min.x + travel * t, track.min.y),
            Vec2::new(seg_w, GAUGE_BAR_HEIGHT),
        );
        ui.painter().rect_filled(seg, 2.0, fill_color);
    }
    resp
}

/// Paints a crisp 7x7 diagonal `✕` button (`menu_popup.rs:30-40`: Korean UI font lacks U+2715).
pub(super) fn paint_cancel_x(ui: &mut egui::Ui, color: Color32, hover_fill: Color32) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(16.0, 16.0), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(rect, 3.0, hover_fill);
    }
    let c = rect.center();
    let d = 3.5_f32;
    let stroke = egui::Stroke::new(1.4, color);
    ui.painter().line_segment([c + Vec2::new(-d, -d), c + Vec2::new(d, d)], stroke);
    ui.painter().line_segment([c + Vec2::new(-d, d), c + Vec2::new(d, -d)], stroke);
    resp.on_hover_text("받기 취소")
}

impl StrataAppState {
    #[must_use]
    pub(crate) fn measure_download_gauge_width(
        &self,
        ui: &egui::Ui,
        now: f64,
        max_title_chars: usize,
    ) -> f32 {
        let jobs = self.download_jobs.jobs();
        if jobs.is_empty() {
            return 0.0;
        }
        let text_w = |s: &str, sz: f32| -> f32 {
            ui.fonts(|f| f.layout_no_wrap(s.to_owned(), FontId::proportional(sz), Color32::BLACK).size().x)
        };
        let mut w = 18.0;
        if jobs.len() == 1 {
            let job = &jobs[0];
            let jt = gauge_job_text(job, now, max_title_chars);
            w += text_w(&jt.bar_label, 11.5) + 8.0;
            if job.is_unfinished() {
                w += GAUGE_BAR_WIDTH + 8.0;
                if let Some(pct) = &jt.percent_text {
                    w += text_w(pct, 11.0) + 8.0;
                }
                w += 16.0 + 8.0;
            }
        } else if let Some(unfin) = jobs.iter().find(|j| j.is_unfinished()) {
            let summary = gauge_multi_summary_text(jobs).unwrap_or_default();
            let jt = gauge_job_text(unfin, now, max_title_chars);
            w += text_w(&summary, 11.5) + 8.0 + GAUGE_BAR_WIDTH + 8.0;
            if let Some(pct) = &jt.percent_text {
                w += text_w(pct, 11.0) + 8.0;
            }
        } else {
            let pick = jobs
                .iter()
                .find(|j| matches!(j.state, DownloadJobState::Stopped { .. }))
                .unwrap_or(&jobs[0]);
            let jt = gauge_job_text(pick, now, max_title_chars);
            w += text_w(&jt.bar_label, 11.5) + 8.0;
        }
        w
    }

    pub(crate) fn render_download_gauge(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        now: f64,
        max_title_chars: usize,
    ) {
        if self.download_jobs.is_empty() {
            return;
        }
        let needs_anim = self.download_jobs.jobs().iter().any(|j| {
            j.is_unfinished()
                || matches!(j.state, DownloadJobState::Done { .. } | DownloadJobState::Cancelled)
        });
        if needs_anim {
            ctx.request_repaint_after(Duration::from_millis(60));
        }

        ui.add_space(4.0);
        ui.separator();
        let anchor_x = ui.cursor().min.x;
        let bar_top_y = ui.max_rect().top();
        let mut toggled = false;
        let mut gauge_rect;

        if self.download_jobs.len() == 1 {
            let job = self.download_jobs.jobs()[0].clone();
            let jt = gauge_job_text(&job, now, max_title_chars);
            let color = if jt.warning { tok(self.theme.warn) } else { Color32::from_rgb(55, 54, 50) };
            let mut lbl_resp = ui.add(egui::Button::new(RichText::new(&jt.bar_label).size(11.5).color(color)).frame(false));
            if let Some(tip) = &jt.tooltip {
                lbl_resp = lbl_resp.on_hover_text(tip);
            }
            toggled |= lbl_resp.clicked();
            gauge_rect = lbl_resp.rect;

            if job.is_unfinished() {
                let mut bar_resp = paint_slim_bar(
                    ui,
                    GAUGE_BAR_WIDTH,
                    job_fraction(&job),
                    now,
                    tok(self.theme.border),
                    tok(self.theme.accent),
                );
                if let Some(tip) = &jt.tooltip {
                    bar_resp = bar_resp.on_hover_text(tip);
                }
                toggled |= bar_resp.clicked();
                gauge_rect = gauge_rect.union(bar_resp.rect);

                if let Some(pct) = &jt.percent_text {
                    let pct_resp = ui.add(
                        egui::Button::new(RichText::new(pct).size(11.0).color(Color32::from_rgb(75, 74, 70))).frame(false),
                    );
                    toggled |= pct_resp.clicked();
                    gauge_rect = gauge_rect.union(pct_resp.rect);
                }

                let cancel_col = if job.is_cancel_requested() {
                    tok(self.theme.ink_disabled)
                } else {
                    Color32::from_rgb(94, 93, 89)
                };
                let cancel_resp = paint_cancel_x(ui, cancel_col, tok(self.theme.hover));
                gauge_rect = gauge_rect.union(cancel_resp.rect);
                if cancel_resp.clicked() {
                    let _ = self.download_jobs.request_cancel(job.id);
                }
            }
        } else if let Some(unfin) = self.download_jobs.jobs().iter().find(|j| j.is_unfinished()).cloned() {
            let summary = gauge_multi_summary_text(self.download_jobs.jobs()).unwrap_or_default();
            let jt = gauge_job_text(&unfin, now, max_title_chars);
            let mut sum_resp = ui.add(
                egui::Button::new(RichText::new(&summary).size(11.5).color(Color32::from_rgb(55, 54, 50))).frame(false),
            );
            if let Some(tip) = &jt.tooltip {
                sum_resp = sum_resp.on_hover_text(tip);
            }
            toggled |= sum_resp.clicked();
            gauge_rect = sum_resp.rect;

            let bar_resp = paint_slim_bar(
                ui,
                GAUGE_BAR_WIDTH,
                job_fraction(&unfin),
                now,
                tok(self.theme.border),
                tok(self.theme.accent),
            );
            toggled |= bar_resp.clicked();
            gauge_rect = gauge_rect.union(bar_resp.rect);

            if let Some(pct) = &jt.percent_text {
                let pct_resp = ui.add(
                    egui::Button::new(RichText::new(pct).size(11.0).color(Color32::from_rgb(75, 74, 70))).frame(false),
                );
                toggled |= pct_resp.clicked();
                gauge_rect = gauge_rect.union(pct_resp.rect);
            }
        } else {
            let pick = self
                .download_jobs
                .jobs()
                .iter()
                .find(|j| matches!(j.state, DownloadJobState::Stopped { .. }))
                .unwrap_or(&self.download_jobs.jobs()[0])
                .clone();
            let jt = gauge_job_text(&pick, now, max_title_chars);
            let color = if jt.warning { tok(self.theme.warn) } else { Color32::from_rgb(55, 54, 50) };
            let mut lbl_resp = ui.add(egui::Button::new(RichText::new(&jt.bar_label).size(11.5).color(color)).frame(false));
            if let Some(tip) = &jt.tooltip {
                lbl_resp = lbl_resp.on_hover_text(tip);
            }
            toggled |= lbl_resp.clicked();
            gauge_rect = lbl_resp.rect;
        }

        if toggled {
            self.download_jobs.popup_open = !self.download_jobs.popup_open;
        }
        if self.download_jobs.popup_open {
            self.render_download_popup(
                ctx,
                download_gauge_popup::PopupAnchor {
                    parent_id: ui.id(),
                    anchor_x,
                    bar_top_y,
                    gauge_rect,
                    toggled,
                    now,
                },
            );
        }
    }
}

#[cfg(test)]
#[path = "download_gauge_tests.rs"]
mod tests;
