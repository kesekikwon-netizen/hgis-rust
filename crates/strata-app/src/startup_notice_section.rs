//! The section of the startup notice — C++ `paintSection` (KaSplashStrata.cpp:97-211): four boundaries
//! inked bottom up while the levelling staff grows, then the pit, the ground line, the peg and Ⅰ–Ⅴ, stratum
//! Ⅲ filled with clay, and two hairlines that cross where the 기준점 lands. The level string is the progress
//! line. Places are those of the 700 x 400 card.

use eframe::egui::{epaint::Mesh, Align2, Color32, Pos2, Rect, Shape, Stroke, Vec2};

use super::paint::{Card, CLAY, HAIR, INK, MUTED, PAPER, RING};
use super::{boundary_reveal, clay_reveal, cross_visibility, ease, marker_reveal, pit_reveal, staff_reveal};

/// The levelling staff, where the section ends, and the level string.
const STAFF_X: f32 = 632.0;
const STRING_Y: f32 = 204.5;
/// Points to one cubic of a line.
const STEPS: usize = 24;

type Curve = [f32; 14];
/// A boundary as drawn on the board: a start point and two cubics. Lowest first: Ⅳ|Ⅴ, Ⅲ|Ⅳ, Ⅱ|Ⅲ, Ⅰ|Ⅱ.
const BOUNDARIES: [Curve; 4] = [
    [0.0, 324.0, 100.0, 318.0, 220.0, 328.0, 330.0, 320.0, 440.0, 312.0, 520.0, 312.0, 632.0, 316.0],
    [0.0, 300.0, 120.0, 292.0, 210.0, 306.0, 310.0, 297.0, 410.0, 288.0, 480.0, 286.0, 632.0, 291.0],
    [0.0, 277.0, 110.0, 270.0, 190.0, 283.0, 290.0, 274.0, 390.0, 265.0, 490.0, 262.0, 632.0, 267.0],
    [0.0, 258.0, 100.0, 252.0, 200.0, 262.0, 300.0, 254.0, 400.0, 246.0, 480.0, 243.0, 632.0, 246.0],
];
/// The topsoil, which comes with the pit.
const GROUND: Curve = [0.0, 243.0, 90.0, 237.0, 170.0, 247.0, 262.0, 238.0, 354.0, 229.0, 450.0, 227.0, 632.0, 229.0];
const STONES: [[f32; 2]; 12] = [
    [58.0, 330.0], [74.5, 328.2], [94.7, 328.8], [168.5, 331.6], [282.0, 331.0], [299.5, 328.6], [317.6, 327.3], [338.6, 326.6], [511.8, 320.2], [530.5, 329.9],
    [552.7, 320.4], [570.9, 326.7],
];

fn cubic(from: [f32; 2], c: [f32; 6], out: &mut Vec<[f32; 2]>) {
    for step in 1..=STEPS {
        let t = step as f32 / STEPS as f32;
        let (a, b, d, e) = ((1.0 - t).powi(3), 3.0 * (1.0 - t).powi(2) * t, 3.0 * (1.0 - t) * t * t, t.powi(3));
        out.push([a * from[0] + b * c[0] + d * c[2] + e * c[4], a * from[1] + b * c[1] + d * c[3] + e * c[5]]);
    }
}

fn line(curve: &Curve) -> Vec<[f32; 2]> {
    let mut points = vec![[curve[0], curve[1]]];
    cubic([curve[0], curve[1]], [curve[2], curve[3], curve[4], curve[5], curve[6], curve[7]], &mut points);
    cubic([curve[6], curve[7]], [curve[8], curve[9], curve[10], curve[11], curve[12], curve[13]], &mut points);
    points
}

/// The wall and the floor of the pit, from its left lip to its right lip (`pitOutline`).
fn pit_outline() -> Vec<[f32; 2]> {
    let mut points = vec![[384.0, 248.3]];
    cubic([384.0, 248.3], [386.0, 276.0, 388.0, 293.0, 396.0, 302.0], &mut points);
    cubic([396.0, 302.0], [402.0, 310.0, 414.0, 313.0, 428.0, 313.0], &mut points);
    cubic([428.0, 313.0], [442.0, 313.0, 452.0, 309.0, 456.0, 300.0], &mut points);
    cubic([456.0, 300.0], [458.0, 294.0, 459.0, 289.0, 460.0, 284.0], &mut points);
    points.push([468.0, 282.0]);
    cubic([468.0, 282.0], [471.0, 272.0, 474.0, 260.0, 476.0, 245.3], &mut points);
    points
}

fn inside(polygon: &[[f32; 2]], at: [f32; 2]) -> bool {
    let mut odd = false;
    for (a, b) in polygon.iter().zip(polygon.iter().cycle().skip(1)) {
        if (a[1] > at[1]) != (b[1] > at[1]) && at[0] < a[0] + (b[0] - a[0]) * (at[1] - a[1]) / (b[1] - a[1]) {
            odd = !odd;
        }
    }
    odd
}

fn faded(colour: Color32, opacity: f64) -> Color32 {
    colour.gamma_multiply(opacity.clamp(0.0, 1.0) as f32)
}

impl Card<'_> {
    fn path(&self, points: &[[f32; 2]]) -> Vec<Pos2> {
        points.iter().map(|p| self.at(p[0], p[1])).collect()
    }

    fn stroke(&self, points: &[[f32; 2]], width: f32, ink: Color32) {
        self.painter.add(Shape::line(self.path(points), Stroke::new(width * self.unit, ink)));
    }

    /// Strokes a line from its left end up to a share of the section's width (`strokeLeftPart`).
    fn stroke_left_part(&self, points: &[[f32; 2]], width: f32, share: f64) {
        if share > 0.0 {
            let clip = Rect::from_min_max(self.at(-4.0, 0.0), self.at(-4.0 + (STAFF_X + 8.0) * share as f32, 400.0));
            self.painter.with_clip_rect(clip.intersect(self.painter.clip_rect())).add(Shape::line(self.path(points), Stroke::new(width * self.unit, INK)));
        }
    }

    fn segment(&self, from: [f32; 2], to: [f32; 2], width: f32, ink: Color32) {
        self.painter.line_segment([self.at(from[0], from[1]), self.at(to[0], to[1])], Stroke::new(width * self.unit, ink));
    }
}

pub(super) fn paint_section(card: &Card<'_>, seconds: f64, progress: f64) {
    let boundaries: Vec<Vec<[f32; 2]>> = BOUNDARIES.iter().map(line).collect();
    // Ⅲ is the one colour on the card: it fills from the left once the section stands.
    let clay = clay_reveal(seconds);
    if clay > 0.0 {
        let mut band = Mesh::default();
        for (upper, lower) in boundaries[2].iter().zip(&boundaries[1]) {
            band.colored_vertex(card.at(upper[0], upper[1]), CLAY);
            band.colored_vertex(card.at(lower[0], lower[1]), CLAY);
        }
        for quad in 0..boundaries[2].len() as u32 - 1 {
            band.add_triangle(2 * quad, 2 * quad + 1, 2 * quad + 2);
            band.add_triangle(2 * quad + 1, 2 * quad + 3, 2 * quad + 2);
        }
        let clip = Rect::from_min_max(card.at(0.0, 240.0), card.at(STAFF_X * clay as f32, 320.0));
        card.painter.with_clip_rect(clip.intersect(card.painter.clip_rect())).add(Shape::mesh(band));
    }
    for (index, boundary) in boundaries.iter().enumerate().take(3) {
        card.stroke_left_part(boundary, 1.0, boundary_reveal(index as u8, seconds));
    }
    let pit = pit_reveal(seconds);
    if pit > 0.0 {
        // Small stones in Ⅴ.
        for stone in STONES {
            card.painter.add(Shape::ellipse_stroke(card.at(stone[0], stone[1]), Vec2::new(3.0, 2.0) * card.unit, Stroke::new(card.unit, faded(HAIR, pit))));
        }
        // The pit cuts the strata: paper inside it, two lenses of its fill, and the cut in a heavier line.
        let cut = pit_outline();
        let mut body = Mesh::default();
        body.colored_vertex(card.at(430.0, 262.0), faded(PAPER, pit));
        for point in &cut {
            body.colored_vertex(card.at(point[0], point[1]), faded(PAPER, pit));
        }
        for edge in 1..cut.len() as u32 {
            body.add_triangle(0, edge, edge + 1);
        }
        body.add_triangle(0, cut.len() as u32, 1);
        card.painter.add(Shape::mesh(body));
        for lens in [[378.0, 264.0, 404.0, 279.0, 440.0, 277.0, 482.0, 259.0], [386.0, 288.0, 410.0, 300.0, 438.0, 298.0, 470.0, 281.0]] {
            let mut points = vec![[lens[0], lens[1]]];
            cubic([lens[0], lens[1]], [lens[2], lens[3], lens[4], lens[5], lens[6], lens[7]], &mut points);
            points.retain(|point| inside(&cut, *point));
            card.stroke(&points, 1.0, faded(MUTED, pit));
        }
        card.stroke(&cut, 2.0, faded(INK, pit));
    }
    card.stroke_left_part(&boundaries[3], 1.0, boundary_reveal(3, seconds));
    card.stroke_left_part(&line(&GROUND), 2.0, pit);
    if pit > 0.0 {
        // The peg the string is tied to, and the numbers of the strata.
        card.segment([32.5, 200.5], [32.5, 247.5], 1.5, faded(INK, pit));
        for (numeral, baseline) in [("Ⅰ", 241.2), ("Ⅱ", 259.9), ("Ⅲ", 282.2), ("Ⅳ", 306.7), ("Ⅴ", 333.6)] {
            card.painter.text(card.at(608.0, baseline + 2.0), Align2::CENTER_BOTTOM, numeral, card.serif(11.0), faded(INK, pit));
        }
    }
    // The levelling staff grows from the bedrock: a long tick every metre (60 px), mid ticks between.
    let staff = staff_reveal(seconds) as f32;
    if staff > 0.0 {
        let top = 344.0 - (344.0 - 196.0) * staff;
        card.segment([STAFF_X + 0.5, top], [STAFF_X + 0.5, 344.0], 1.0, INK);
        for tick in 0..24u8 {
            let y = STRING_Y + 6.0 * f32::from(tick);
            if y >= top {
                let length = if tick % 10 == 0 { 8.0 } else if tick % 5 == 0 { 5.0 } else { 3.0 };
                card.segment([STAFF_X, y], [STAFF_X + length, y], 1.0, INK);
            }
        }
        for (label, y) in [("154", 204.5), ("153", 264.5), ("152", 324.5)] {
            if y >= top {
                card.text(645.0, y + 3.5, label, card.mono(10.0), MUTED);
            }
        }
    }
    // The level string: dashed until it is pulled from the peg to the staff.
    let string = [card.at(32.0, STRING_Y), card.at(STAFF_X, STRING_Y)];
    card.painter.extend(Shape::dashed_line(&string, Stroke::new(card.unit, RING), 8.0 * card.unit, 6.0 * card.unit));
    card.segment([32.0, STRING_Y], [32.0 + (STAFF_X - 32.0) * progress.clamp(0.0, 1.0) as f32, STRING_Y], 1.0, INK);
    if progress >= 0.999 {
        card.painter.text(card.at(624.0, 193.0), Align2::RIGHT_CENTER, "EL. 154.00 m", card.mono(10.0), MUTED);
    }
    // Two hairlines slide in from the top and the left, cross where the 기준점 lands, then fade.
    let cross = cross_visibility(seconds);
    if cross > 0.0 {
        let slid = ease((seconds - 3.2) / 0.4) as f32;
        card.segment([236.5, STRING_Y], [236.5, STRING_Y + 47.6 * slid], 1.0, faded(HAIR, cross));
        card.segment([0.0, 234.6], [266.0 * slid, 234.6], 1.0, faded(HAIR, cross));
    }
    let landed = marker_reveal(seconds);
    if landed > 0.0 {
        let drop = -18.0 * (1.0 - landed as f32);
        let mark = vec![card.at(236.0, 225.6 + drop), card.at(244.0, 238.6 + drop), card.at(228.0, 238.6 + drop)];
        card.painter.add(Shape::convex_polygon(mark, faded(PAPER, landed), Stroke::new(1.5 * card.unit, faded(INK, landed))));
        card.painter.circle_filled(card.at(236.0, 234.6 + drop), 1.5 * card.unit, faded(INK, landed));
        card.text(249.0, 233.1, "기준점", card.ui(10.0), faded(MUTED, ease((seconds - 3.6) / 0.4)));
    }
}
