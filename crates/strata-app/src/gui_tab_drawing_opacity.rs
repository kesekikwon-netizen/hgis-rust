//! Drawing opacity card values. The card itself is the map rail (`gui_tab_map_opacity_placement.rs`).
//! `KaDrawingStudio.cpp:1196-1234` remembers `m_railLayer` and asks the layout cache to draw again.
use super::RailKind;
use crate::app_state::StrataAppState;

impl StrataAppState {
    pub(super) fn drawing_brightness_unit(&self, title: &str) -> f32 {
        let value = self
            .picture_index(title)
            .map(|i| self.pictures.layers[i].brightness)
            .or_else(|| self.drawing.rail_brightness.get(title).copied())
            .unwrap_or(0);
        (value as f32 / 510.0) + 0.5
    }

    /// Opacity of the layer the sheet remembered, then drop its picture cache (`:1221-1227`, `:1211`).
    pub(super) fn apply_drawing_rail_opacity(&mut self, title: &str, t: f32) {
        let before = self.rail_opacity(title);
        self.apply_rail_opacity(title, t);
        if (self.rail_opacity(title) - before).abs() > f32::EPSILON {
            self.invalidate_drawing_layer_cache(title);
        }
    }

    /// Brightness of that picture only. The map's shared brightness slots stay (`:1198-1218`).
    pub(super) fn apply_drawing_rail_brightness(&mut self, title: &str, t: f32) {
        if !self.rail_brightness_enabled(RailKind::Drawing, title) {
            return;
        }
        let value = ((t - 0.5) * 510.0).round().clamp(-255.0, 255.0) as i32;
        if let Some(index) = self.picture_index(title) {
            let layer = &mut self.pictures.layers[index];
            if layer.brightness == value {
                return;
            }
            layer.brightness = value;
            layer.clear_picture_cache();
        } else if self.drawing.rail_brightness.get(title).copied() == Some(value) {
            return;
        } else {
            self.drawing.rail_brightness.insert(title.to_owned(), value);
        }
        self.drawing.map_cache_epoch = self.drawing.map_cache_epoch.saturating_add(1);
    }

    fn invalidate_drawing_layer_cache(&mut self, title: &str) {
        if let Some(index) = self.picture_index(title) {
            self.pictures.layers[index].clear_picture_cache();
        }
        self.drawing.map_cache_epoch = self.drawing.map_cache_epoch.saturating_add(1);
    }
}
