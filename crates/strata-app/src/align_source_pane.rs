//! Split pane and original-pixel interaction, target/cpp-ref/src/app/MainWindowAlign.cpp:219-328,379-487.
use crate::app_state::StrataAppState;
use eframe::egui::{self, Color32, Pos2, Stroke, Vec2};

pub(crate) fn mark(p: &egui::Painter, at: Pos2, index: usize, pending: bool) {
    let ink = if pending { Color32::from_rgb(234, 179, 8) } else { Color32::from_rgb(220, 38, 38) };
    p.line_segment([at - Vec2::new(13.0, 0.0), at + Vec2::new(13.0, 0.0)], Stroke::new(2.0, ink));
    p.line_segment([at - Vec2::new(0.0, 13.0), at + Vec2::new(0.0, 13.0)], Stroke::new(2.0, ink));
    p.circle(at, 9.0, Color32::from_white_alpha(240), Stroke::new(2.4, ink));
    p.text(at + Vec2::new(0.0, -15.0), egui::Align2::CENTER_BOTTOM, (index + 1).to_string(), egui::FontId::proportional(12.0), Color32::from_rgb(15, 23, 42));
}

impl StrataAppState {
    pub(crate) fn align_split_shown(&self) -> bool {
        self.align.raster.is_some() && !self.align.applied
    }

    pub(crate) fn render_align_source(&mut self, ui: &mut egui::Ui) {
        let available = ui.available_width();
        let mut source_error = None;
        egui::SidePanel::left("align_source_pane").default_width(available * 0.5).width_range(150.0..=(available - 150.0).max(150.0)).show_inside(ui, |ui| {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
            ui.strong("왼쪽 · 그림 — 여기를 먼저 찍기");
            ui.horizontal_wrapped(|ui| {
                if ui.button("화면 맞춤").clicked() {
                    if let Some(session) = &mut self.align.raster {
                        session.view.zoom = 1.0;
                        if let Some(layer) = self.pictures.layers.iter().find(|l| l.key == session.original.key) {
                            session.view.centre = [layer.size[0] as f64 * 0.5, layer.size[1] as f64 * 0.5];
                        }
                        session.view.error = None;
                    }
                }
                if self.align.raster.as_ref().is_some_and(|s| s.task.0.is_some()) {
                    ui.spinner();
                    ui.label("도면을 정리하는 중…");
                }
            });
            egui::TopBottomPanel::bottom("align_pairs").max_height((ui.available_height() * 0.35).min(150.0)).show_inside(ui, |ui| {
                egui::ScrollArea::vertical().id_salt("align_pair_list").auto_shrink([false, false]).show(ui, |ui| {
                    let report = self.align.report();
                    for index in 0..self.align.pairs.len() {
                        let selected = self.align.raster.as_ref().is_some_and(|s| s.selected == Some(index));
                        if ui.selectable_label(selected, strata_geo::align_fit::row_text(&report, index)).clicked() {
                            if let Some(session) = &mut self.align.raster {
                                session.selected = Some(index);
                            }
                        }
                    }
                    if self.align.pending_source.is_some() {
                        ui.label(format!("{}번  왼쪽만 — 오른쪽 모서리를 찍으세요", self.align.pairs.len() + 1));
                    }
                    for note in strata_geo::align_fit::notes(&report) {
                        ui.label(note);
                    }
                });
            });
            let (rect, response) = ui.allocate_exact_size(ui.available_size().max(Vec2::splat(1.0)), egui::Sense::click_and_drag());
            let p = ui.painter_at(rect);
            p.rect_filled(rect, 4.0, Color32::from_rgb(225, 223, 216));
            let rect = rect.shrink(20.0_f32.min(rect.width().min(rect.height()).max(0.0) * 0.25));
            let Some(session) = &mut self.align.raster else { return };
            session.source_rect = Some(rect);
            let busy = session.task.0.is_some();
            if response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                let wheel = ui.input(|i| i.raw_scroll_delta.y);
                if let Some(at) = response.hover_pos().filter(|_| wheel != 0.0) {
                    let before = session.view.pixel(rect, at);
                    session.view.zoom = (session.view.zoom * 1.15_f64.powf(f64::from(wheel) / 50.0)).clamp(0.05, 1000.0);
                    let after = session.view.pixel(rect, at);
                    for axis in 0..2 {
                        session.view.centre[axis] += before[axis] - after[axis];
                    }
                }
            }
            if response.dragged_by(egui::PointerButton::Middle) {
                let delta = response.drag_delta();
                let scale = session.view.scale(rect);
                session.view.centre[0] -= f64::from(delta.x) / scale;
                session.view.centre[1] -= f64::from(delta.y) / scale;
            }
            if !busy {
                let mut changed = false;
                if response.drag_started_by(egui::PointerButton::Primary) {
                    if let Some(at) = ui.input(|i| i.pointer.press_origin()) {
                        session.drag =
                            self.align.pairs.iter().enumerate().find(|(_, pair)| session.view.screen(rect, pair.src).distance(at) <= 10.0).map(|(index, _)| (true, index));
                    }
                }
                if response.drag_stopped_by(egui::PointerButton::Primary) {
                    if let (Some((true, index)), Some(at)) = (session.drag.take(), response.interact_pointer_pos()) {
                        let point = session.view.pixel(rect, at);
                        self.align.move_pair(index, true, point);
                        changed = true;
                    }
                } else if response.clicked() {
                    if let Some(at) = response.interact_pointer_pos() {
                        let point = session.view.pixel(rect, at);
                        self.align.set_source(point);
                        changed = true;
                    }
                }
                if changed {
                    self.status_bar_hint = self.align.status_text();
                }
            }
            let Some(session) = &mut self.align.raster else { return };
            let previous_error = session.view.error.clone();
            session.view.paint(ui.ctx(), &p, rect);
            if session.view.error != previous_error {
                source_error = session.view.error.clone();
            }
            for (index, pair) in self.align.pairs.iter().enumerate() {
                mark(&p, session.view.screen(rect, pair.src), index, false);
            }
            if let Some(point) = self.align.pending_source {
                mark(&p, session.view.screen(rect, point), self.align.pairs.len(), true);
            }
        });
        if let Some(why) = source_error {
            self.show_status_message(format!("왼쪽에 그림을 띄우지 못했습니다: {why}"));
        }
    }

    pub(crate) fn delete_selected_align_point(&mut self) {
        let selected = self.align.raster.as_ref().and_then(|s| s.selected);
        if let Some(index) = selected.filter(|i| *i < self.align.pairs.len()) {
            self.align.remove_pair_at(index);
        } else {
            self.align.remove_last_pair();
        }
        if let Some(session) = &mut self.align.raster {
            session.selected = None;
        }
        self.status_bar_hint = self.align.status_text();
    }
}
