//! The word 「rust」 beside 「Strata」 on the startup card (owner 2026-11-16: 「Strata 여기가지는 놔두고 rust를
//! 추가 rust는 3d글자로 좌우로 움직이게하라」): heavy letters with depth that sway left and right and turn as
//! they go, so that their sides show. The letters are the font's own: the mesh of each glyph is set in space
//! and seen in perspective, the sides as thin copies from the back to the face.

use eframe::egui::epaint::{Mesh, Vertex};
use eframe::egui::{Color32, Pos2, Shape, Vec2};

use super::paint::Card;
use crate::gui_tab_map::LABEL_FONT_FAMILY;

/// The word as the owner wrote it, in letters of 66 card units on the baseline of 「Strata」, this far right of it.
const WORD: &str = "rust";
const SIZE: f32 = 66.0;
const GAP: f32 = 28.0;
/// How deep the letters are and how far the eye is (card units).
pub(crate) const DEPTH: f32 = 14.0;
const EYE: f32 = 760.0;
/// The swing: how far the word turns (radians, about 29°) and slides (card units), and how long one takes.
pub(crate) const MAX_YAW: f32 = 0.5;
pub(crate) const SLIDE: f32 = 18.0;
pub(crate) const PERIOD: f64 = 3.6;
/// Without motion the word stands a little turned, so that its depth still shows.
pub(crate) const STILL_YAW: f32 = 0.3;
/// The clay of the card on the face, lighter at the top; a burnt brown on the sides, darker at the back.
const FACE_TOP: Color32 = Color32::from_rgb(0xEE, 0x9A, 0x74);
const FACE_FOOT: Color32 = Color32::from_rgb(0xC4, 0x5F, 0x3F);
const SIDE_FRONT: Color32 = Color32::from_rgb(0x9C, 0x45, 0x2A);
const SIDE_BACK: Color32 = Color32::from_rgb(0x5E, 0x26, 0x16);
/// One copy of the sides every this many screen points of depth: close enough to read as a solid side.
const SIDE_STEP_PX: f32 = 0.6;

/// How the word stands: turned about its upright middle (positive: its right end goes back) and slid along
/// the card (positive: to the right).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Pose {
    pub(crate) yaw: f32,
    pub(crate) slide: f32,
}

/// The pose `clock` seconds after the card appeared: it slides and turns together, as a thing seen from a
/// fixed eye shows the side that faces the middle. `None`: no motion.
pub(crate) fn pose(clock: Option<f64>) -> Pose {
    let Some(clock) = clock else { return Pose { yaw: STILL_YAW, slide: 0.0 } };
    // A sine of a phase that grows without end: f32 is enough for the ±1 it gives.
    #[allow(clippy::cast_possible_truncation)]
    let swing = (std::f64::consts::TAU * clock / PERIOD).sin() as f32;
    Pose { yaw: MAX_YAW * swing, slide: SLIDE * swing }
}

/// A point of the word in space (x right, y down, z away from the eye; the origin in the middle of the face at
/// rest) to the card, relative to that middle, in the same units.
pub(crate) fn project(point: [f32; 3], pose: Pose) -> [f32; 2] {
    let (sin, cos) = pose.yaw.sin_cos();
    let x = point[0] * cos - point[2] * sin;
    let z = point[0] * sin + point[2] * cos;
    let scale = EYE / (EYE + z);
    [x * scale + pose.slide, point[1] * scale]
}

/// Where the word starts on the card: right of 「Strata」 (the serif of 60 units at x 29, `paint_card`).
pub(crate) fn word_left(card: &Card<'_>) -> f32 {
    let strata = card.painter.layout_no_wrap("Strata".to_owned(), card.serif(60.0), Color32::BLACK);
    29.0 + strata.size().x / card.unit + GAP
}

fn mix(from: Color32, to: Color32, part: f32) -> Color32 {
    let channel = |a: u8, b: u8| (f32::from(a) + (f32::from(b) - f32::from(a)) * part.clamp(0.0, 1.0)).round() as u8;
    Color32::from_rgb(channel(from.r(), to.r()), channel(from.g(), to.g()), channel(from.b(), to.b()))
}

/// The layers of the word with its left edge at `left` and its baseline at `baseline` (card units): the sides
/// as copies from the back forward, then the face.
pub(crate) fn word_meshes(card: &Card<'_>, left: f32, baseline: f32, pose: Pose) -> Vec<Mesh> {
    let galley = card.painter.layout_no_wrap(WORD.to_owned(), card.font(LABEL_FONT_FAMILY, SIZE), Color32::WHITE);
    // The rows' meshes give texels as places in the font picture: a mesh needs parts of it (0 to 1).
    let texels = card.painter.ctx().fonts(|fonts| fonts.font_image_size());
    let per_texel = Vec2::new(1.0 / texels[0].max(1) as f32, 1.0 / texels[1].max(1) as f32);
    let ascent = galley.rows.first().and_then(|row| row.glyphs.first()).map_or(0.0, |glyph| glyph.pos.y);
    let size = galley.size();
    // The middle of the face on the screen, and the font's vertices around it in card units.
    let middle = card.at(left, baseline) - Vec2::new(0.0, ascent) + size * 0.5;
    let rows = || galley.rows.iter().map(|row| &row.visuals.mesh);
    let place = |pos: Pos2, depth: f32| {
        let local = (pos.to_vec2() - size * 0.5) / card.unit;
        let [x, y] = project([local.x, local.y, depth], pose);
        middle + Vec2::new(x, y) * card.unit
    };
    let layer = |depth: f32, colour: &dyn Fn(Pos2) -> Color32| {
        let mut mesh = Mesh::default();
        for row in rows() {
            let first = mesh.vertices.len() as u32;
            mesh.indices.extend(row.indices.iter().map(|index| index + first));
            mesh.vertices.extend(row.vertices.iter().map(|v| Vertex { pos: place(v.pos, depth), uv: (v.uv.to_vec2() * per_texel).to_pos2(), color: colour(v.pos) }));
        }
        mesh
    };
    // A count of thin copies: a few dozen at most.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let copies = ((DEPTH * card.unit / SIDE_STEP_PX).ceil() as usize).max(8);
    let mut layers: Vec<Mesh> = (0..copies)
        .map(|copy| {
            let part = copy as f32 / (copies - 1) as f32;
            let depth = DEPTH * (1.0 - copy as f32 / copies as f32);
            layer(depth, &|_| mix(SIDE_BACK, SIDE_FRONT, part))
        })
        .collect();
    layers.push(layer(0.0, &|pos| mix(FACE_TOP, FACE_FOOT, pos.y / size.y.max(1.0))));
    layers
}

/// Paints the word on the card: moving with `clock`, or still.
pub(crate) fn paint_word(card: &Card<'_>, baseline: f32, clock: Option<f64>) {
    for mesh in word_meshes(card, word_left(card), baseline, pose(clock)) {
        card.painter.add(Shape::mesh(mesh));
    }
}

#[cfg(test)]
#[path = "startup_notice_rust_tests.rs"]
mod tests;
