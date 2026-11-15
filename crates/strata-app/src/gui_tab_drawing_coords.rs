//! 「좌표점」 on the sheet — C++ `beginPlaceCoordPoint`, `placeCoordCallout`, `relayoutCoordCallouts` and
//! `updateCoordFrame` (KaDrawingStudio.cpp:1662-1889) with `KaDrawingCoordCallout`: a click on the map frame
//! puts a point on the ground (a magnet takes the survey's lines and points within 18 screen points), and
//! every frame draws each point where its ground is now, so the points follow the map when it is moved or
//! its scale changes.

use eframe::egui::{self, Color32, FontFamily, FontId, Pos2, Rect, Stroke, Vec2};

use super::gui_tab_drawing_frame::SheetFrame;
use super::sheet_coord_points::{callout_shape, free_box_rect, inside_map_frame, nearest_on, tag_for, xy_text, BoxMm};
use crate::app_state::StrataAppState;
use crate::tab_drawing::CoordCallout;

/// The magnet reaches 18 screen points (`snapMm = 18.0 / viewPx`, :1704-1705).
const MAGNET_PX: f64 = 18.0;
/// The ink of the arrows and the words, and the orange of the frame through the points (`:115`, `:1873`).
const INK: Color32 = Color32::from_rgb(0x1C, 0x19, 0x17);
const FRAME: Color32 = Color32::from_rgb(0xC2, 0x41, 0x0C);
/// Millimetres in a typographic point.
const MM_PER_PT: f32 = 25.4 / 72.0;

/// A place on the screen as millimetres of the paper of `f`, and back.
fn paper_mm(f: &SheetFrame, at: Pos2) -> [f64; 2] {
    [f64::from((at.x - f.sheet_rect.left()) / f.px_per_mm), f64::from((at.y - f.sheet_rect.top()) / f.px_per_mm)]
}

fn screen(f: &SheetFrame, at: [f64; 2]) -> Pos2 {
    // Millimetres of a paper: f32 holds them.
    #[allow(clippy::cast_possible_truncation)]
    Pos2::new(f.sheet_rect.left() + at[0] as f32 * f.px_per_mm, f.sheet_rect.top() + at[1] as f32 * f.px_per_mm)
}

fn bold(painter: &egui::Painter) -> FontFamily {
    let bold = FontFamily::Name(crate::gui_tab_map::LABEL_FONT_FAMILY.into());
    if painter.ctx().fonts(|fonts| fonts.families().contains(&bold)) { bold } else { FontFamily::Proportional }
}

impl StrataAppState {
    /// 「좌표점」 (`beginPlaceCoordPoint`, :1662-1675).
    pub(crate) fn begin_place_coord_point(&mut self) {
        self.end_sheet_map_adjust();
        self.drawing.coord_callout_mode = true;
        self.show_status_message("지도 칸에서 찍으세요. 선·점에 자석이 붙습니다. 우클릭·Delete·Ctrl+Z는 마지막 점 지우기.");
    }

    /// A click at `at` while points are placed (`placeCoordCallout`, :1687-1804).
    pub(crate) fn place_coord_point(&mut self, f: &SheetFrame, at: Pos2) {
        let [x, y] = paper_mm(f, f.map_rect.min);
        let frame = BoxMm { x, y, w: f64::from(f.map_rect.width() / f.px_per_mm), h: f64::from(f.map_rect.height() / f.px_per_mm) };
        if !inside_map_frame(paper_mm(f, at), frame) {
            return self.show_status_message("지도 칸 안에서 찍으세요. 흰 용지에는 점이 안 생깁니다.");
        }
        let click = f.m2w(at);
        let (lines, points) = self.survey_snap_parts();
        let snapped = nearest_on(click, &lines, &points, MAGNET_PX / f.px_per_meter.max(1e-9));
        let ground = snapped.unwrap_or(click);
        let tag = tag_for(self.drawing.coord_callouts.len());
        self.drawing.coord_callouts.push(CoordCallout { ground });
        let magnet = if snapped.is_some() { "  (자석)" } else { "" };
        self.show_status_message(format!("{tag}점  X={:.3}  Y={:.3}{magnet}", ground[1], ground[0]));
    }

    /// The pointer on the sheet while points are placed: a click places a point, a right click takes the
    /// last one off (`KaLayoutCoordPointTool`, KaDrawingTools.cpp:28-46).
    pub(crate) fn handle_coord_point_clicks(&mut self, ui: &egui::Ui, f: &SheetFrame) {
        if !self.drawing.coord_callout_mode {
            return;
        }
        let response = ui.interact(f.sheet_rect, ui.id().with("drawing_callout_pick"), egui::Sense::click());
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
        }
        if response.secondary_clicked() {
            return self.undo_last_coord_callout();
        }
        if let (true, Some(at)) = (response.clicked(), response.interact_pointer_pos()) {
            self.place_coord_point(f, at);
        }
    }

    /// The points where their ground is now (`relayoutCoordCallouts`, :1823-1839): the orange frame through
    /// them, then for each its arrow, its box with the words and its letter (`add`, KaDrawingCoordCallout.cpp:71-134).
    pub(crate) fn paint_sheet_coord_points(&self, f: &SheetFrame) {
        let (p, mm) = (&f.sheet_p, f.px_per_mm);
        let tips: Vec<[f64; 2]> = self.drawing.coord_callouts.iter().map(|point| paper_mm(f, f.w2m(point.ground))).collect();
        if tips.len() >= 2 {
            p.add(egui::Shape::closed_line(tips.iter().map(|tip| screen(f, *tip)).collect(), Stroke::new(0.45 * mm, FRAME)));
        }
        let mut used: Vec<BoxMm> = Vec::new();
        for (index, (point, tip)) in self.drawing.coord_callouts.iter().zip(&tips).enumerate() {
            let box_mm = free_box_rect(*tip, &used);
            used.push(box_mm);
            let shape = callout_shape(*tip, box_mm);
            p.line_segment([screen(f, shape.shaft[0]), screen(f, shape.shaft[1])], Stroke::new(0.22 * mm, INK));
            p.add(egui::Shape::convex_polygon(shape.head.iter().map(|corner| screen(f, *corner)).collect(), INK, Stroke::NONE));
            let size = |w: f64, h: f64| Vec2::new(w as f32, h as f32) * mm;
            let frame = Rect::from_min_size(screen(f, [box_mm.x, box_mm.y]), size(box_mm.w, box_mm.h));
            p.rect_filled(frame, 0.0, Color32::from_rgba_unmultiplied(255, 255, 255, 235));
            p.rect_stroke(frame, 0.0, Stroke::new(0.3 * mm, Color32::BLACK), egui::StrokeKind::Inside);
            let words = xy_text(point.ground[0], point.ground[1]);
            p.text(Pos2::new(frame.left() + 0.4 * mm, frame.center().y), egui::Align2::LEFT_CENTER, words, FontId::proportional(6.0 * MM_PER_PT * mm), INK);
            let letter = Rect::from_min_size(screen(f, [shape.letter.x, shape.letter.y]), size(shape.letter.w, shape.letter.h));
            p.text(letter.center(), egui::Align2::CENTER_CENTER, tag_for(index).to_string(), FontId::new(8.0 * MM_PER_PT * mm, bold(p)), INK);
        }
    }
}

#[cfg(test)]
#[path = "gui_tab_drawing_coords_tests.rs"]
mod tests;
