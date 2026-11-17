//! Right-map pairing and link overlay, target/cpp-ref/src/app/KaAlignMapTool.cpp:464-555, KaImageView.cpp:29-86.
use super::align_source_pane::mark;
use crate::app_state::StrataAppState;
use crate::gui_tab_map::{s2w, w2s};
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

impl StrataAppState {
    pub(crate) fn apply_pending_align_view(&mut self, ctx: &egui::Context) {
        let Some((epsg, extent)) = self.align.pending_view.take() else { return };
        if epsg != strata_types::crs::CrsCode::from(self.work_crs).epsg_number() { return; }
        self.home_map.set_view_extent(extent);
        self.home_map.viewport.meters_per_pixel *= 1.25;
        self.home_map.note_view();
        ctx.request_repaint();
    }
    fn align_map_point(&self, rect: Rect, at: Pos2) -> [f64; 2] {
        let point = s2w(rect, &self.home_map.viewport, at);
        if !self.home_map.cad.snap_enabled {
            return point;
        }
        let reach = self.home_map.cad.snap_tolerance_px * self.home_map.viewport.meters_per_pixel;
        let (mut lines, points) = self.survey_snap_parts();
        let within = [point[0] - reach, point[1] - reach, point[0] + reach, point[1] + reach];
        for row in self.left_layers.entries.iter().filter(|row| row.snap_enabled && self.home_map.real_survey.is_layer_visible(&row.label_ko)) {
            lines.extend(self.ref_vectors.runs_within(&row.layer_key, within));
        }
        crate::gui_tab_drawing::sheet_coord_points::nearest_on(point, &lines, &points, reach).unwrap_or(point)
    }

    pub(crate) fn handle_align_map_input(&mut self, ui: &egui::Ui, response: &egui::Response, rect: Rect) -> bool {
        if !self.align_split_shown() {
            return false;
        }
        let Some(session) = &mut self.align.raster else { return false };
        session.map_rect = Some(rect);
        if session.task.0.is_some() {
            return true;
        }
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
        }
        if response.secondary_clicked() {
            self.align.pending_source = None;
            session.drag = None;
            self.status_bar_hint = self.align.status_text();
            return true;
        }
        if self.align.pending_source.is_none() && response.drag_started_by(egui::PointerButton::Primary) {
            if let Some(at) = ui.input(|i| i.pointer.press_origin()) {
                session.drag = self.align.pairs.iter().enumerate().find(|(_, p)| w2s(rect, &self.home_map.viewport, p.map).distance(at) <= 10.0).map(|(index, _)| (false, index));
            }
        }
        if response.drag_stopped_by(egui::PointerButton::Primary) {
            if let (Some((false, index)), Some(at)) = (session.drag.take(), response.interact_pointer_pos()) {
                let point = self.align_map_point(rect, at);
                self.align.move_pair(index, false, point);
                self.status_bar_hint = self.align.status_text();
            }
        } else if response.clicked() {
            if let Some(at) = response.interact_pointer_pos() {
                let point = self.align_map_point(rect, at);
                if self.align.accept_map(point) {
                    self.status_bar_hint = self.align.status_text();
                } else {
                    self.status_bar_hint = "왼쪽 도면에서 먼저 모서리를 찍으세요".to_owned();
                }
            }
        }
        true
    }

    pub(crate) fn paint_align_marks(&self, p: &egui::Painter, rect: Rect) {
        if !self.align_split_shown() {
            return;
        }
        for (index, pair) in self.align.pairs.iter().enumerate() {
            mark(p, w2s(rect, &self.home_map.viewport, pair.map), index, false);
        }
    }

    // O(pairs) per frame; endpoints come from the same source/map transforms as the painted marks.
    // target/cpp-ref/src/app/MainWindowAlign.cpp:550-609 and KaImageView.cpp:29-86.
    pub(crate) fn paint_align_links(&self, ctx: &egui::Context) {
        if !self.align_split_shown() {
            return;
        }
        let Some(session) = &self.align.raster else { return };
        let (Some(source), Some(map)) = (session.source_rect, session.map_rect) else { return };
        let p = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("alignment_links"))).with_clip_rect(source.union(map));
        for pair in &self.align.pairs {
            let (a, b) = (session.view.screen(source, pair.src), w2s(map, &self.home_map.viewport, pair.map));
            if source.contains(a) && map.contains(b) {
                link(&p, a, b, false);
            }
        }
        if let (Some(point), Some(cursor)) = (self.align.pending_source, ctx.input(|i| i.pointer.hover_pos()).filter(|p| map.contains(*p))) {
            link(&p, session.view.screen(source, point), cursor, true);
        }
    }
}

fn link(p: &egui::Painter, a: Pos2, b: Pos2, pending: bool) {
    let colour = if pending { Color32::from_rgb(234, 88, 12) } else { Color32::from_rgb(30, 103, 198) };
    let stroke = Stroke::new(2.4, colour);
    if pending {
        for shape in egui::Shape::dashed_line(&[a, b], stroke, 6.0, 4.0) {
            p.add(shape);
        }
    } else {
        p.line_segment([a, b], stroke);
    }
    let direction = b - a;
    if direction.length() > 4.0 {
        let unit = direction.normalized();
        let normal = Vec2::new(-unit.y, unit.x) * 5.0;
        for end in [b - unit * 12.0 + normal, b - unit * 12.0 - normal] {
            p.line_segment([b, end], stroke);
        }
    }
}
