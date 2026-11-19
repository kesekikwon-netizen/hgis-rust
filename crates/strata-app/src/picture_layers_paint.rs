//! Checked picture rows on the map, lowest row first (`picture_layers.rs`).
use super::{place, PICTURE_PREFIX};
use crate::app_state::StrataAppState;
use crate::gui_tab_map::{s2w, w2s};
use eframe::egui::{self, Color32, Mesh, Pos2, Rect, Shape};
use strata_engine::Transformer;

impl StrataAppState {
    /// Paints the checked picture rows on the map, the lowest row first.
    pub(crate) fn paint_picture_layers(&mut self, ctx: &egui::Context, p: &egui::Painter, rect: Rect) {
        let viewport = self.home_map.viewport;
        let (north_west, south_east) = (s2w(rect, &viewport, rect.left_top()), s2w(rect, &viewport, rect.right_bottom()));
        let view = [north_west[0], south_east[1], south_east[0], north_west[1]];
        let work_epsg = strata_types::crs::CrsCode::from(self.work_crs).epsg_number();
        let rows: Vec<(String, f32)> = self
            .left_layers
            .entries
            .iter()
            .rev()
            .filter(|e| e.layer_key.starts_with(PICTURE_PREFIX) && self.home_map.real_survey.is_layer_visible(&e.label_ko))
            .map(|e| (e.layer_key.clone(), e.opacity))
            .collect();
        for (key, opacity) in rows {
            let Some(layer) = self.pictures.layers.iter_mut().find(|l| l.key == key) else {
                continue;
            };
            if layer.align_pending {
                continue;
            }
            drop(layer.take_arrived(ctx));
            if let Some(want) = layer.wanted(view, viewport.meters_per_pixel, work_epsg) {
                if layer.reading.is_none() && !layer.serves(&want) {
                    layer.start_reading(want);
                    ctx.request_repaint_after(std::time::Duration::from_millis(60));
                }
            }
            let Some((shown, texture)) = &layer.shown else {
                continue;
            };
            let carry = layer.source_epsg.filter(|epsg| *epsg != work_epsg).and_then(|epsg| Transformer::new(epsg, work_epsg).ok());
            // Straight lines stay straight under the transform; another CRS bends them a little: a finer grid.
            let steps = if carry.is_some() { 8 } else { 1 };
            let alpha = (opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
            let tint = Color32::from_rgba_premultiplied(alpha, alpha, alpha, alpha);
            let mut mesh = Mesh::with_texture(texture.id());
            for j in 0..=steps {
                for i in 0..=steps {
                    let (u, v) = (i as f32 / steps as f32, j as f32 / steps as f32);
                    let col = shown.window[0] as f64 + f64::from(u) * shown.window[2] as f64;
                    let row = shown.window[1] as f64 + f64::from(v) * shown.window[3] as f64;
                    let mut xy = place(&layer.transform, col, row);
                    if let Some(carry) = &carry {
                        xy = carry.transform(xy).unwrap_or(xy);
                    }
                    let pos: Pos2 = w2s(rect, &viewport, xy);
                    mesh.vertices.push(egui::epaint::Vertex {
                        pos,
                        uv: Pos2::new(u, v),
                        color: tint,
                    });
                }
            }
            let per_row = steps + 1;
            for j in 0..steps {
                for i in 0..steps {
                    let a = j * per_row + i;
                    mesh.add_triangle(a, a + 1, a + per_row + 1);
                    mesh.add_triangle(a, a + per_row + 1, a + per_row);
                }
            }
            p.add(Shape::mesh(mesh));
        }
    }
}
