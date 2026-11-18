//! Right-map pairing and link overlay, target/cpp-ref/src/app/KaAlignMapTool.cpp:464-555, KaImageView.cpp:29-86.
use super::align_source_pane::mark;
use super::{
    align_snap::{Operation, Resolution},
    align_snap_sources::Sources,
};
use crate::app_state::StrataAppState;
use crate::gui_tab_map::{s2w, w2s};
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

impl StrataAppState {
    pub(crate) fn apply_pending_align_view(&mut self, ctx: &egui::Context) {
        let Some((epsg, extent)) = self.align.pending_view.take() else { return };
        if epsg != strata_types::crs::CrsCode::from(self.work_crs).epsg_number() {
            return;
        }
        self.home_map.set_view_extent(extent);
        self.home_map.viewport.meters_per_pixel *= 1.25;
        self.home_map.note_view();
        ctx.request_repaint();
    }
    pub(crate) fn handle_align_map_input(&mut self, ui: &egui::Ui, response: &egui::Response, rect: Rect) -> bool {
        if !self.align_split_shown() || self.active_tool != strata_canvas2d::map_tools::MapToolKind::GeorefAlign {
            return false;
        }
        let sources = Sources::capture(self);
        let blocked = !response.enabled() || self.modal_window_open() || self.app_popup_menu_open();
        let Some(session) = &mut self.align.raster else { return false };
        session.map_rect = Some(rect);
        session.target_cursor = None;
        if session.task.0.is_some() || blocked {
            session.snap.cancel();
            session.drag = None;
            return true;
        }
        if session.snap.prepare(sources) && matches!(session.drag, Some((false, _))) {
            session.drag = None;
        }
        if session.snap.waiting() {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(30));
        }
        if response.hovered() || response.dragged_by(egui::PointerButton::Primary) {
            session.target_cursor = ui.input(|i| i.pointer.hover_pos()).filter(|at| rect.contains(*at));
        }
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
        }
        if response.secondary_clicked() {
            self.align.pending_source = None;
            session.drag = None;
            session.snap.cancel();
            self.status_bar_hint = self.align.status_text();
            return true;
        }
        if self.align.pending_source.is_none() && response.drag_started_by(egui::PointerButton::Primary) {
            if let Some(at) = ui.input(|i| i.pointer.press_origin()) {
                session.drag = self
                    .align
                    .pairs
                    .iter()
                    .enumerate()
                    .find(|(_, p)| w2s(rect, &self.home_map.viewport, p.map).distance(at) <= 10.0)
                    .map(|(index, _)| (false, index));
            }
        }
        if response.drag_stopped_by(egui::PointerButton::Primary) {
            if let (Some((false, index)), Some(at)) = (session.drag.take(), response.interact_pointer_pos()) {
                if rect.contains(at) {
                    let raw = s2w(rect, &self.home_map.viewport, at);
                    let reach = self.home_map.cad.snap_tolerance_px * self.home_map.viewport.meters_per_pixel;
                    session.snap.defer(Operation::Move(index), raw, reach, &self.align.pairs, self.align.pending_source);
                }
            }
        } else if response.clicked() {
            if let Some(at) = response.interact_pointer_pos() {
                if self.align.pending_source.is_some() {
                    let raw = s2w(rect, &self.home_map.viewport, at);
                    let reach = self.home_map.cad.snap_tolerance_px * self.home_map.viewport.meters_per_pixel;
                    session.snap.defer(Operation::Pick, raw, reach, &self.align.pairs, self.align.pending_source);
                } else {
                    self.status_bar_hint = "왼쪽 도면에서 먼저 모서리를 찍으세요".to_owned();
                }
            }
        }
        // Popup owners consume Escape later in the frame; do not commit delayed input before them.
        // target/cpp-ref/src/app/KaAlignMapTool.cpp:538-555 and MainWindow.cpp:256-268.
        if !ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            let outcome = session.snap.take_ready_action(&self.align.pairs, self.align.pending_source);
            match outcome {
                Ok(Some((Operation::Pick, point))) => {
                    self.align.accept_map(point);
                    self.status_bar_hint = self.align.status_text();
                }
                Ok(Some((Operation::Move(index), point))) => {
                    self.align.move_pair(index, false, point);
                    self.status_bar_hint = self.align.status_text();
                }
                Err(why) => self.show_status_message(why),
                Ok(None) if session.snap.has_action() => self.status_bar_hint = "도면을 정리하는 중…".to_owned(),
                Ok(None) => {}
            }
        }
        true
    }

    fn align_target_preview(&self, rect: Rect) -> Option<([f64; 2], bool)> {
        let session = self.align.raster.as_ref()?;
        let raw = s2w(rect, &self.home_map.viewport, session.target_cursor?);
        let reach = self.home_map.cad.snap_tolerance_px * self.home_map.viewport.meters_per_pixel;
        match session.snap.resolve(&Sources::capture(self), raw, reach) {
            Resolution::Ready { point, snapped } => Some((point, snapped)),
            Resolution::Waiting | Resolution::Failed(_) => None,
        }
    }

    pub(crate) fn paint_align_marks(&self, p: &egui::Painter, rect: Rect) {
        if !self.align_split_shown() {
            return;
        }
        let preview = self.align_target_preview(rect);
        let drag = self.align.raster.as_ref().and_then(|s| s.drag);
        for (index, pair) in self.align.pairs.iter().enumerate() {
            let point = if drag == Some((false, index)) { preview.map_or(pair.map, |(point, _)| point) } else { pair.map };
            mark(p, w2s(rect, &self.home_map.viewport, point), index, false);
        }
        if let Some((point, true)) = preview {
            // target/cpp-ref/src/app/KaAlignMapTool.cpp:464-479, 16-point blue circle.
            p.circle(w2s(rect, &self.home_map.viewport, point), 8.0, Color32::from_white_alpha(220), Stroke::new(2.0, Color32::from_rgb(30, 103, 198)));
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
        let preview = self.align_target_preview(map);
        for (index, pair) in self.align.pairs.iter().enumerate() {
            let target = if session.drag == Some((false, index)) { preview.map_or(pair.map, |(point, _)| point) } else { pair.map };
            let (a, b) = (session.view.screen(source, pair.src), w2s(map, &self.home_map.viewport, target));
            if source.contains(a) && map.contains(b) {
                link(&p, a, b, false);
            }
        }
        if let (Some(point), Some(cursor)) = (self.align.pending_source, session.target_cursor) {
            let endpoint = preview.map_or(cursor, |(point, _)| w2s(map, &self.home_map.viewport, point));
            link(&p, session.view.screen(source, point), endpoint, true);
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
