//! Paints what egui has cut into triangles as a picture in memory: a sheet at print resolution, where no
//! window and no graphics card can paint it. Colours are mixed as the screen mixes them: in the values as
//! they are stored (gamma space), with the clearness already multiplied in.

use std::collections::HashMap;

use eframe::egui::epaint::{ClippedPrimitive, Primitive, Vertex};
use eframe::egui::{Color32, ImageData, TextureFilter, TextureId, TexturesDelta};

struct Texture {
    size: [usize; 2],
    pixels: Vec<Color32>,
    smooth: bool,
}

/// The pictures a page uses (the letters among them), kept as egui hands them over after a pass.
#[derive(Default)]
pub(crate) struct Textures(HashMap<TextureId, Texture>);

/// A page painted in memory: red, green and blue of every pixel, row by row from the top.
pub(crate) struct Picture {
    pub(crate) size: [usize; 2],
    pub(crate) rgb: Vec<u8>,
    /// How many meshes asked for a picture that was never handed over (they are left out).
    pub(crate) unknown_pictures: usize,
}

impl Picture {
    #[cfg(test)]
    pub(crate) fn at(&self, at: [usize; 2]) -> [u8; 3] {
        let first = (at[1] * self.size[0] + at[0]) * 3;
        [self.rgb[first], self.rgb[first + 1], self.rgb[first + 2]]
    }
}

impl Textures {
    /// Takes over what a pass made new or changed (a part of a picture goes into its place).
    pub(crate) fn apply(&mut self, delta: &TexturesDelta) {
        for (id, change) in &delta.set {
            let (size, pixels): ([usize; 2], Vec<Color32>) = match &change.image {
                ImageData::Color(image) => (image.size, image.pixels.clone()),
                // The letters: how much of a pixel a letter covers, as the screen's renderer reads it.
                ImageData::Font(letters) => (letters.size, letters.srgba_pixels(None).collect()),
            };
            let smooth = change.options.magnification == TextureFilter::Linear;
            match (change.pos, self.0.get_mut(id)) {
                (Some([left, top]), Some(whole)) => {
                    for (row, line) in pixels.chunks_exact(size[0].max(1)).enumerate() {
                        let first = (top + row) * whole.size[0] + left;
                        if let Some(place) = whole.pixels.get_mut(first..first + size[0]) {
                            place.copy_from_slice(line);
                        }
                    }
                }
                _ => drop(self.0.insert(*id, Texture { size, pixels, smooth })),
            }
        }
    }
}

impl Texture {
    fn pixel(&self, x: f32, y: f32) -> [f32; 4] {
        let column = (x.max(0.0) as usize).min(self.size[0].saturating_sub(1));
        let row = (y.max(0.0) as usize).min(self.size[1].saturating_sub(1));
        self.pixels.get(row * self.size[0] + column).map_or([0.0; 4], |c| c.to_array().map(f32::from))
    }

    /// The colour at a place given in parts of the picture (0 to 1): the nearest pixel, or the four around
    /// the place mixed by how near each is. Past the edge the edge is repeated.
    fn colour_at(&self, part: [f32; 2]) -> [f32; 4] {
        let (x, y) = (part[0] * self.size[0] as f32, part[1] * self.size[1] as f32);
        if !self.smooth {
            return self.pixel(x.floor(), y.floor());
        }
        let (x, y) = (x - 0.5, y - 0.5);
        let (left, top) = (x.floor(), y.floor());
        let (right_part, lower_part) = (x - left, y - top);
        let mix = |a: [f32; 4], b: [f32; 4], t: f32| [0, 1, 2, 3].map(|i| a[i] + (b[i] - a[i]) * t);
        let upper = mix(self.pixel(left, top), self.pixel(left + 1.0, top), right_part);
        let lower = mix(self.pixel(left, top + 1.0), self.pixel(left + 1.0, top + 1.0), right_part);
        mix(upper, lower, lower_part)
    }
}

/// Twice the area of the triangle a, b, q: positive when q lies to the right of a → b on a screen (y down).
fn side(a: [f64; 2], b: [f64; 2], q: [f64; 2]) -> f64 {
    (b[0] - a[0]) * (q[1] - a[1]) - (b[1] - a[1]) * (q[0] - a[0])
}

/// Paints one triangle. A pixel belongs to it when its middle lies inside; a middle exactly on an edge
/// belongs to the triangle whose upper or left edge it is, so two triangles that share an edge paint every
/// pixel once.
fn paint_triangle(picture: &mut Picture, texture: &Texture, cut: [usize; 4], corners: [Vertex; 3], scale: f32) {
    let mut at = corners.map(|v| [f64::from(v.pos.x) * f64::from(scale), f64::from(v.pos.y) * f64::from(scale)]);
    let mut corners = corners;
    let area = side(at[0], at[1], at[2]);
    if area == 0.0 || !area.is_finite() {
        return;
    }
    if area < 0.0 {
        at.swap(1, 2);
        corners.swap(1, 2);
    }
    let area = area.abs();
    // The edge opposite each corner, and whether it is an upper or a left edge.
    let edges = [[at[1], at[2]], [at[2], at[0]], [at[0], at[1]]];
    let owns = edges.map(|[a, b]| b[1] < a[1] || (b[1] == a[1] && b[0] > a[0]));
    let low = |axis: usize| at.iter().map(|p| p[axis]).fold(f64::INFINITY, f64::min);
    let high = |axis: usize| at.iter().map(|p| p[axis]).fold(f64::NEG_INFINITY, f64::max);
    let first_row = (low(1) - 0.5).ceil().max(cut[1] as f64) as usize;
    let last_row = ((high(1) - 0.5).floor() + 1.0).clamp(0.0, cut[3] as f64) as usize;
    let one_colour = corners.iter().all(|v| v.uv == corners[0].uv).then(|| texture.colour_at([corners[0].uv.x, corners[0].uv.y]));
    for row in first_row..last_row {
        let y = row as f64 + 0.5;
        // The stretch of the row inside all three edges: each edge's value changes evenly along the row.
        let (mut from, mut to) = (low(0).max(cut[0] as f64), high(0).min(cut[2] as f64));
        for [a, b] in edges {
            let (at_zero, step) = (side(a, b, [0.0, y]), -(b[1] - a[1]));
            if step > 0.0 {
                from = from.max(-at_zero / step - 1.0);
            } else if step < 0.0 {
                to = to.min(-at_zero / step + 1.0);
            } else if at_zero < 0.0 {
                to = f64::NEG_INFINITY;
            }
        }
        if to < from {
            continue;
        }
        let (first, last) = ((from - 0.5).floor().max(cut[0] as f64) as usize, ((to + 0.5).ceil().max(0.0) as usize).min(cut[2]));
        for column in first..last {
            let middle = [column as f64 + 0.5, y];
            let weights = edges.map(|[a, b]| side(a, b, middle));
            if (0..3).any(|i| weights[i] < 0.0 || (weights[i] == 0.0 && !owns[i])) {
                continue;
            }
            let share = weights.map(|w| (w / area) as f32);
            let blend = |of: [f32; 3]| share[0] * of[0] + share[1] * of[1] + share[2] * of[2];
            let ink = [0, 1, 2, 3].map(|channel| blend(corners.map(|v| f32::from(v.color.to_array()[channel]))));
            let texel = one_colour.unwrap_or_else(|| texture.colour_at([blend(corners.map(|v| v.uv.x)), blend(corners.map(|v| v.uv.y))]));
            let over = [0, 1, 2, 3].map(|channel| ink[channel] * texel[channel] / 255.0);
            let keep = 1.0 - over[3] / 255.0;
            let pixel = (row * picture.size[0] + column) * 3;
            for (slot, value) in picture.rgb[pixel..pixel + 3].iter_mut().zip(over) {
                *slot = (value + f32::from(*slot) * keep + 0.5).clamp(0.0, 255.0) as u8;
            }
        }
    }
}

/// Paints `primitives` (in points) on white paper of `size` pixels, `scale` pixels to the point.
pub(crate) fn paint(primitives: &[ClippedPrimitive], textures: &Textures, scale: f32, size: [usize; 2]) -> Picture {
    let mut picture = Picture { size, rgb: vec![255; size[0] * size[1] * 3], unknown_pictures: 0 };
    for clipped in primitives {
        let Primitive::Mesh(mesh) = &clipped.primitive else { continue };
        let Some(texture) = textures.0.get(&mesh.texture_id) else {
            picture.unknown_pictures += 1;
            continue;
        };
        // The box a mesh is cut to, in whole pixels as a renderer rounds it.
        let edge = |points: f32, most: usize| ((points * scale).round().max(0.0) as usize).min(most);
        let cut = [edge(clipped.clip_rect.min.x, size[0]), edge(clipped.clip_rect.min.y, size[1]), edge(clipped.clip_rect.max.x, size[0]), edge(clipped.clip_rect.max.y, size[1])];
        for corner in mesh.indices.chunks_exact(3) {
            let found = [corner[0], corner[1], corner[2]].map(|index| mesh.vertices.get(index as usize).copied());
            if let [Some(a), Some(b), Some(c)] = found {
                paint_triangle(&mut picture, texture, cut, [a, b, c], scale);
            }
        }
    }
    picture
}

#[cfg(test)]
#[path = "sheet_raster_tests.rs"]
mod tests;
