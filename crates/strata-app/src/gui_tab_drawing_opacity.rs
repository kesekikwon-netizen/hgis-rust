//! Drawing opacity card values. The card itself is the map rail (`gui_tab_map_opacity_placement.rs`).
//! `KaDrawingStudio.cpp:1196-1234` remembers `m_railLayer` and asks the layout cache to draw again.
//! Positions are `ui/opacityRail/map` and `ui/opacityRail/drawing` (`KaLayerOpacityRail.cpp:13-15`, `:101-105`).
use super::RailKind;
use crate::app_state::StrataAppState;
use eframe::egui::Vec2;
use strata_storage::qt_ini::{escaped_point, read_point, IniFile};

const RAIL_SECTION: &str = "ui";

fn rail_key(kind: RailKind) -> &'static str {
    // Qt `iniEscapedKey` writes `/` as `\` (`qsettings.cpp:505-506`), so `ui/opacityRail/map` is this key.
    match kind {
        RailKind::Map => "opacityRail\\map",
        RailKind::Drawing => "opacityRail\\drawing",
    }
}

impl StrataAppState {
    /// Read both points. A card the user has already dragged keeps its offset and does not take the new point.
    pub(crate) fn reload_opacity_rails(&mut self) {
        let Some(path) = self.settings_ini.clone() else { return };
        let Ok(file) = IniFile::read(&path) else { return };
        self.opacity_rail_ini = [RailKind::Map, RailKind::Drawing].map(|kind| file.value(RAIL_SECTION, rail_key(kind)).and_then(read_point));
    }

    pub(super) fn write_opacity_rail(&self, kind: RailKind, offset: Vec2) {
        let Some(path) = self.settings_ini.clone() else { return };
        let Ok(mut file) = IniFile::read(&path) else { return };
        let point = (offset.x.round() as i32, offset.y.round() as i32);
        file.set_value(RAIL_SECTION, rail_key(kind), &escaped_point(point.0, point.1));
        let _ = file.write(&path);
    }

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
