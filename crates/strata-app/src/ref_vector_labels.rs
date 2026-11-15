//! The letters over the shapes of a reference layer (the unit symbols of the geology map).
//!
//! C++ lets QGIS write them (`target/cpp-ref/src/core/GeologyMapService.cpp:426-455`: the column 기호 over
//! the polygon, Malgun Gothic 8 pt in 40/40/40, a white edge of 0.6 mm at alpha 220). QGIS cuts a shape to
//! the map before it looks for the label's place, so a unit larger than the map is labelled over the part in
//! view, and of a row of several polygons the largest is labelled. The same is done here for every view.
//!
//! Two things are this app's own. QGIS chooses among colliding labels of a whole map by cost; that engine is
//! not rebuilt, the rule of the survey layers is used (`gui_tab_map_survey_labels.rs`): letters that fit
//! inside their polygon first, then the larger part; a label that would cover one already placed is not
//! written. And where the centroid of the part lies outside it (a bent unit, a unit around another), the
//! letters go to the widest stretch of the part at that height, so that a symbol always stands on its unit.

use eframe::egui::{self, Color32, FontFamily, FontId, Painter, Pos2, Rect};
use strata_canvas2d::viewport::ViewportTransform;

use crate::gui_tab_map::{paint_placed_labels, place_labels, s2w, w2s, LabelDraw, LabelReq};
use crate::ref_vector::clip::{clip_ring, ClipBox};
use crate::ref_vector::data::{Label, LabelLook, RefVectorData};
use crate::survey_paint_layers::{polygon_area, polygon_centroid};
use crate::survey_style::{label_font_size_points, mm_to_points, qt_premultiplied};

#[cfg(test)]
#[path = "ref_vector_labels_tests.rs"]
mod tests;

/// Rings of one polygon, closed, as offsets from the layer's origin.
type Rings = Vec<Vec<[f64; 2]>>;

/// The part of a row that is in view: of its polygons cut to the view, the one with the largest area left
/// (holes taken out), with that area.
fn visible_part(data: &RefVectorData, label: &Label, view: &ClipBox) -> Option<(f64, Rings)> {
    let mut best: Option<(f64, Rings)> = None;
    for item in data.items.get(label.first_item as usize..(label.first_item + label.items) as usize)? {
        let b = item.bbox.map(f64::from);
        if b[2] < view[0] || b[0] > view[2] || b[3] < view[1] || b[1] > view[3] {
            continue;
        }
        let mut rings = Rings::new();
        for run in data.runs.get(item.first_run as usize..(item.first_run + item.run_count) as usize)? {
            let points: Vec<[f64; 2]> = data.coords.get(run.start as usize..(run.start + run.len) as usize)?.iter().map(|c| [f64::from(c[0]), f64::from(c[1])]).collect();
            let mut cut = clip_ring(view, &points);
            if cut.len() >= 3 {
                cut.push(cut[0]);
                rings.push(cut);
            } else if rings.is_empty() {
                // Nothing of the outer ring is in view: neither is the polygon.
                break;
            }
        }
        let area = polygon_area(&rings);
        if area > 0.0 && best.as_ref().is_none_or(|(most, _)| area > *most) {
            best = Some((area, rings));
        }
    }
    best
}

/// Whether the point lies in the polygon: inside its outer ring and in no hole (even-odd over all rings).
fn inside(rings: &Rings, point: [f64; 2]) -> bool {
    let mut odd = false;
    for pair in rings.iter().flat_map(|ring| ring.windows(2)) {
        let (a, b) = (pair[0], pair[1]);
        if (a[1] > point[1]) != (b[1] > point[1]) && point[0] < a[0] + (point[1] - a[1]) / (b[1] - a[1]) * (b[0] - a[0]) {
            odd = !odd;
        }
    }
    odd
}

/// Where the letters of a part go: its centroid (`:432`, OverPoint), or, when that lies outside the part,
/// the middle of the widest stretch of the part at the centroid's height (the western one of equals).
fn label_point(rings: &Rings) -> Option<[f64; 2]> {
    let centre = polygon_centroid(rings)?;
    if inside(rings, centre) {
        return Some(centre);
    }
    let mut crossings: Vec<f64> = rings
        .iter()
        .flat_map(|ring| ring.windows(2))
        .filter(|pair| (pair[0][1] > centre[1]) != (pair[1][1] > centre[1]))
        .map(|pair| pair[0][0] + (centre[1] - pair[0][1]) / (pair[1][1] - pair[0][1]) * (pair[1][0] - pair[0][0]))
        .collect();
    crossings.sort_by(f64::total_cmp);
    let widest = crossings.chunks_exact(2).rev().max_by(|p, q| (p[1] - p[0]).total_cmp(&(q[1] - q[0])))?;
    Some([(widest[0] + widest[1]) / 2.0, centre[1]])
}

/// The labels of the layers painted in a frame, before they are chosen and written.
#[derive(Default)]
pub(crate) struct Pending {
    pub(crate) reqs: Vec<LabelReq>,
    pub(crate) draws: Vec<LabelDraw>,
    /// Where each layer's labels end in the two lists, with the width and colour of its white edge.
    parts: Vec<(usize, f32, Color32)>,
}

impl Pending {
    /// Adds the labels of this layer for this view of the map: for every row with a part in view, the
    /// letters centred on that part's label point. A label the edge of the map would cut is left out, as
    /// QGIS leaves out truncated labels.
    pub(crate) fn add(&mut self, ctx: &egui::Context, data: &RefVectorData, look: &LabelLook, rect: Rect, vp: &ViewportTransform) {
        // The app's text font is Malgun Gothic (`:440`); Qt's points are 96/72 of a point here.
        let font = FontId::new(label_font_size_points(look.size_pt), FontFamily::Proportional);
        let ink = Color32::from_rgb(look.ink[0], look.ink[1], look.ink[2]);
        let offset = |corner: Pos2| {
            let world = s2w(rect, vp, corner);
            [world[0] - data.origin[0], world[1] - data.origin[1]]
        };
        let (top_left, bottom_right) = (offset(rect.left_top()), offset(rect.right_bottom()));
        let view: ClipBox = [top_left[0], bottom_right[1], bottom_right[0], top_left[1]];
        for label in &data.labels {
            let Some((area, rings)) = visible_part(data, label, &view) else {
                continue;
            };
            let Some(point) = label_point(&rings) else {
                continue;
            };
            let at = w2s(rect, vp, [data.origin[0] + point[0], data.origin[1] + point[1]]);
            let galley = ctx.fonts(|fonts| fonts.layout_no_wrap(label.text.clone(), font.clone(), ink));
            let size = galley.rect.size();
            let origin = at - size * 0.5;
            let spot = Rect::from_min_size(origin, size);
            if !rect.contains_rect(spot) {
                continue;
            }
            let fits_inside = [spot.left_top(), spot.right_top(), spot.right_bottom(), spot.left_bottom()].iter().all(|corner| inside(&rings, offset(*corner)));
            self.reqs.push(LabelReq { candidates: vec![spot], fits_inside, area });
            self.draws.push(LabelDraw { galley, ink, angle: 0.0, origins: vec![origin] });
        }
        let [r, g, b, a] = qt_premultiplied(look.halo);
        self.parts.push((self.reqs.len(), mm_to_points(look.halo_mm), Color32::from_rgba_premultiplied(r, g, b, a)));
    }

    /// Chooses among all labels added, and writes the chosen ones, each layer with its own white edge.
    pub(crate) fn paint(&self, p: &Painter) {
        let chosen = place_labels(&self.reqs);
        let mut start = 0;
        for (end, halo, colour) in &self.parts {
            if let (Some(draws), Some(chosen)) = (self.draws.get(start..*end), chosen.get(start..*end)) {
                paint_placed_labels(p, draws, chosen, *halo, *colour);
            }
            start = *end;
        }
    }
}
