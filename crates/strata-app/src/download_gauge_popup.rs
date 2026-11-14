//! Popup above the status bar listing every download job and full stop reasons
//! (`docs/RUST_PLAN.md` §2.8; slim bar precedent in `target/cpp-ref/src/app/KaProgressPlate.cpp:15-24`).

use eframe::egui::{self, Align, Color32, FontId, Layout, Pos2, Rect, RichText};

use crate::app_state::StrataAppState;
use crate::download_gauge::{job_fraction, paint_cancel_x, paint_slim_bar, tok, POPUP_BAR_WIDTH};
use crate::download_jobs::{DownloadJob, DownloadJobState};
use crate::status_bar_text::{gauge_job_text, GaugeMultiSummary};

pub(super) fn multi_summary_job(
    summary: &GaugeMultiSummary,
    warn_color: Color32,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let font_id = FontId::proportional(11.5);
    if let Some(pre) = &summary.normal_prefix {
        job.append(
            pre,
            0.0,
            egui::TextFormat {
                font_id: font_id.clone(),
                color: Color32::from_rgb(55, 54, 50),
                ..Default::default()
            },
        );
    }
    if let Some(warn) = &summary.warn_suffix {
        job.append(
            warn,
            0.0,
            egui::TextFormat { font_id, color: warn_color, ..Default::default() },
        );
    }
    job
}

fn estimate_popup_height(jobs: &[DownloadJob]) -> f32 {
    let mut h = 20.0_f32;
    for (idx, job) in jobs.iter().enumerate() {
        if idx > 0 {
            h += 8.0;
        }
        h += match &job.state {
            DownloadJobState::Stopped { reason, .. } => {
                let lines = reason.chars().count().div_ceil(26).max(1) as f32;
                28.0 + lines * 18.0
            }
            _ => 26.0,
        };
    }
    h.min(340.0)
}

pub(super) struct PopupAnchor {
    pub(super) parent_id: egui::Id,
    pub(super) anchor_x: f32,
    pub(super) bar_top_y: f32,
    pub(super) gauge_rect: Rect,
    pub(super) toggled: bool,
    pub(super) now: f64,
}

impl StrataAppState {
    pub(super) fn render_download_popup(&mut self, ctx: &egui::Context, anchor: PopupAnchor) {
        let PopupAnchor { parent_id, anchor_x, bar_top_y, gauge_rect, toggled, now } = anchor;
        let jobs = self.download_jobs.jobs().to_vec();
        let est_h = estimate_popup_height(&jobs);
        let max_x = (ctx.screen_rect().right() - 368.0).max(8.0);
        let popup_pos = Pos2::new(anchor_x.min(max_x), (bar_top_y - 8.0 - est_h).max(44.0));
        let mut cancel_ids = Vec::new();
        let mut dismiss_ids = Vec::new();

        let area_resp = egui::Area::new(parent_id.with("ka_download_gauge_popup"))
            .fixed_pos(popup_pos)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .fill(tok(self.theme.dialog_ground))
                    .stroke(egui::Stroke::new(1.0, tok(self.theme.border_strong)))
                    .corner_radius(6.0)
                    .inner_margin(egui::Margin::symmetric(10, 8))
                    .show(ui, |ui| {
                        ui.set_min_width(300.0);
                        ui.set_max_width(356.0);
                        egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                            for (idx, job) in jobs.iter().enumerate() {
                                if idx > 0 {
                                    ui.separator();
                                }
                                let jt = gauge_job_text(job, now, 14);
                                match &job.state {
                                    DownloadJobState::Stopped { reason, .. } => {
                                        let col = if jt.warning { tok(self.theme.warn) } else { tok(self.theme.ink) };
                                        ui.horizontal(|ui| {
                                            ui.colored_label(col, RichText::new(&jt.full_label).size(11.5).strong());
                                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                if ui.add(egui::Button::new(RichText::new("닫기").size(11.0))).clicked() {
                                                    dismiss_ids.push(job.id);
                                                }
                                            });
                                        });
                                        if !reason.trim().is_empty() {
                                            ui.add(
                                                egui::Label::new(
                                                    RichText::new(reason).size(11.0).color(tok(self.theme.ink_muted)),
                                                )
                                                .wrap(),
                                            );
                                        }
                                    }
                                    _ if job.is_unfinished() => {
                                        ui.horizontal(|ui| {
                                            let mut lbl = ui.label(
                                                RichText::new(&jt.bar_label).size(11.5).color(tok(self.theme.ink)),
                                            );
                                            if let Some(tip) = &jt.tooltip {
                                                lbl = lbl.on_hover_text(tip);
                                            }
                                            let _ = lbl;
                                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                let cancel_col = if job.is_cancel_requested() {
                                                    tok(self.theme.ink_disabled)
                                                } else {
                                                    tok(self.theme.ink_muted)
                                                };
                                                if paint_cancel_x(ui, cancel_col, tok(self.theme.hover)).clicked() {
                                                    cancel_ids.push(job.id);
                                                }
                                                if let Some(pct) = &jt.percent_text {
                                                    ui.label(
                                                        RichText::new(pct).size(11.0).color(tok(self.theme.ink_muted)),
                                                    );
                                                }
                                                let _ = paint_slim_bar(
                                                    ui,
                                                    POPUP_BAR_WIDTH,
                                                    job_fraction(job),
                                                    now,
                                                    tok(self.theme.border),
                                                    tok(self.theme.accent),
                                                );
                                            });
                                        });
                                    }
                                    _ => {
                                        ui.label(
                                            RichText::new(&jt.full_label).size(11.5).color(tok(self.theme.ink_muted)),
                                        );
                                    }
                                }
                            }
                        });
                    });
            });

        for id in cancel_ids {
            let _ = self.download_jobs.request_cancel(id);
        }
        for id in dismiss_ids {
            let _ = self.download_jobs.dismiss(id);
        }
        if !toggled && ctx.input(|i| i.pointer.primary_clicked()) {
            if let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) {
                if !area_resp.response.rect.contains(pos) && !gauge_rect.contains(pos) {
                    self.download_jobs.popup_open = false;
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "download_gauge_popup_tests.rs"]
mod tests;
