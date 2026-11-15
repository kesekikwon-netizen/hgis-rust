//! Online reference tile layers of the legend: 대동여지도 and 지형맵.
//!
//! C++ adds them to the project as live tile layers (`target/cpp-ref/src/core/BasemapOps.cpp:1565-1595`,
//! addXyzBasemap `:860-977`): a legend row the user can uncheck, fade and remove. Owner's decision of
//! 2026-11-15 (「모든 ui버튼은 조사구역을 그린것을 중심으로 생성되어야한다. 대동여지도 조사구역반경100km」):
//! the command works from the drawn survey area and the map covers a circle around it.
//! A layer is painted only while its row is in the legend. A title that merely counts as visible is not
//! enough: an unmerged build painted two such maps on every survey and the owner could not remove them.

use eframe::egui::{self, Color32};
use strata_types::schema::LegendGroup;

use crate::app_state::StrataAppState;
use crate::basemap_paint::TileView;
use crate::basemap_tiles::TileKind;
use crate::left_layer_panel::{LayerTreeEntry, LeftLayerPanel};

#[path = "ref_tile_paint.rs"]
pub(crate) mod ref_tile_paint;

use ref_tile_paint::{paint_circle_tiles_with, ScreenCircle};

/// The centre of the circle, kept between frames. Working it out builds a coordinate transformer for each
/// survey-area table (`survey_paint_layers.rs` table_extent_in: a new PROJ pipeline, milliseconds), so it is done
/// again only when the survey-area shapes or the work CRS change.
#[derive(Debug, Default, Clone)]
pub(crate) struct RefCircleCache {
    key: Option<u64>,
    centre: Option<[f64; 2]>,
    worked_out: u32,
}

impl RefCircleCache {
    /// How many times the centre was worked out.
    #[cfg(test)]
    pub(crate) fn times_worked_out(&self) -> u32 {
        self.worked_out
    }
}

/// One online reference tile layer.
pub(crate) struct RefTileLayer {
    /// `layer_key` of its legend row.
    pub(crate) key: &'static str,
    /// Legend title as in C++ (`BasemapOps.cpp:1566`).
    pub(crate) title: &'static str,
    pub(crate) kind: TileKind,
    /// Deepest zoom asked of the service; a closer view stretches those tiles.
    pub(crate) max_zoom: u8,
    /// Radius of the circle around the survey area, in metres of the work CRS plane (its scale error over
    /// this distance is far below the width of a tile).
    pub(crate) radius_m: f64,
    /// Status text after the command.
    pub(crate) added_text: &'static str,
    /// Status text when no survey area is drawn (modelled on `MainWindowCadastral.cpp:114`).
    pub(crate) needs_area_text: &'static str,
    /// What the command does to the view.
    pub(crate) on_add: ViewOnAdd,
}

/// What adding a layer does to the view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ViewOnAdd {
    /// The whole circle in view, the survey area in the middle (a small-scale map such as 대동여지도).
    FrameCircle,
    /// The view stays, except that a view centred outside the circle (it would show nothing of the map) is
    /// moved onto the survey area at its scale. Not the C++ rule: addXyzBasemap zooms to 1:10,000 around the
    /// view centre when the view is smaller-scale than 1:80,000 (`BasemapOps.cpp:961-972`).
    Keep,
}

/// 대동여지도 (1861). The picture of the service has the resolution of zoom 12 (about 30 m per pixel in
/// Korea): a zoom-15 tile fetched on 2026-11-15 was made of 8-pixel blocks. Deeper tiles add requests, no detail.
pub(crate) const DAEDONGYEOJIDO: RefTileLayer = RefTileLayer {
    key: "ref_daedongyeojido",
    title: "대동여지도",
    kind: TileKind::Daedongyeojido,
    max_zoom: 12,
    radius_m: 100_000.0,
    added_text: "대동여지도를 올렸습니다. 조사구역에서 반경 100km 안에 보입니다.",
    needs_area_text: "조사구역을 먼저 그려 주세요. 조사구역에서 반경 100km 안의 대동여지도를 올립니다.",
    on_add: ViewOnAdd::FrameCircle,
};

/// 지형맵 — OpenTopoMap, zoom 5 to 17 (`BasemapOps.cpp:1580-1590`); the ribbon button 「지형」
/// (`MainWindowRibbon.cpp:369-377`, `MainWindowOverlay.cpp:239-267`).
pub(crate) const TERRAIN_MAP: RefTileLayer = RefTileLayer {
    key: "ref_terrain_map",
    title: "지형맵",
    kind: TileKind::Terrain,
    max_zoom: 17,
    radius_m: 100_000.0,
    added_text: "지형맵을 올렸습니다. 다시 누르면 숨깁니다.",
    needs_area_text: "조사구역을 먼저 그려 주세요. 조사구역에서 반경 100km 안의 지형맵을 올립니다.",
    on_add: ViewOnAdd::Keep,
};

const REF_TILE_LAYERS: [&RefTileLayer; 3] = [&DAEDONGYEOJIDO, &TERRAIN_MAP, &history::MAP_1919];

/// The reference tile layer whose legend row has this `layer_key`.
pub(crate) fn ref_tile_layer(key: &str) -> Option<&'static RefTileLayer> {
    REF_TILE_LAYERS.iter().copied().find(|l| l.key == key)
}

/// Adds the row unless it is there: above the older tile rows and the background rows, so that the map just
/// asked for is not hidden under an older one (the director's choice, docs/EVOLUTION.md E-13). C++ puts a new
/// tile layer BELOW the older ones: addXyzBasemap appends its node (`BasemapOps.cpp:922`) and
/// ensureSatelliteAtBottom (`:362-401`) keeps the tile layers in that order above the satellite.
fn ensure_row(panel: &mut LeftLayerPanel, layer: &RefTileLayer) {
    if panel.entries.iter().any(|e| e.layer_key == layer.key) {
        return;
    }
    let at = panel
        .entries
        .iter()
        .position(|e| e.group == LegendGroup::Basemap || ref_tile_layer(&e.layer_key).is_some())
        .unwrap_or(panel.entries.len());
    panel.entries.insert(
        at,
        LayerTreeEntry {
            layer_key: layer.key.to_owned(),
            label_ko: layer.title.to_owned(),
            group: LegendGroup::ReferenceMap,
            visible: true,
            opacity: 1.0,
            read_only: true,
            snap_enabled: false, // a picture is not snapped to
            feature_count: 0,
        },
    );
}

impl StrataAppState {
    /// The ribbon command: the legend row, checked, and the view as the layer asks (`ViewOnAdd`).
    /// Without a drawn survey area nothing is added and the status bar says what to do first.
    pub(crate) fn add_reference_tile_layer(&mut self, layer: &'static RefTileLayer) -> bool {
        let Some(centre) = self.reference_circle_centre() else {
            self.show_status_message(layer.needs_area_text);
            return false;
        };
        ensure_row(&mut self.left_layers, layer);
        // The check mark is kept by the row's present name (the user may have renamed the row).
        if let Some(name) = self.left_layers.entries.iter().find(|e| e.layer_key == layer.key).map(|e| e.label_ko.clone()) {
            self.home_map.real_survey.layer_visible.insert(name, true);
        }
        match layer.on_add {
            ViewOnAdd::FrameCircle => {
                let r = layer.radius_m * 1.1;
                self.home_map.set_view_extent([centre[0] - r, centre[1] - r, centre[0] + r, centre[1] + r]);
            }
            ViewOnAdd::Keep => self.bring_view_into_circle(layer, centre),
        }
        self.show_status_message(layer.added_text);
        true
    }

    /// A view centred outside the circle would show nothing of the layer: it goes onto the survey area at
    /// its scale (one step of the view history).
    fn bring_view_into_circle(&mut self, layer: &RefTileLayer, centre: [f64; 2]) {
        let view = self.home_map.viewport.center_xy;
        if (view[0] - centre[0]).hypot(view[1] - centre[1]) > layer.radius_m {
            self.home_map.viewport.center_xy = centre;
            self.home_map.note_view();
        }
    }

    /// Whether the layer has a legend row and that row is checked (the ribbon button shows it as pressed:
    /// `MainWindowOverlay.cpp:245-247`).
    pub(crate) fn reference_tile_layer_shown(&self, layer: &RefTileLayer) -> bool {
        self.left_layers
            .entries
            .iter()
            .any(|e| e.layer_key == layer.key && self.home_map.real_survey.is_layer_visible(&e.label_ko))
    }

    /// A checkable ribbon button (지형): the first click adds the layer, later clicks hide and show its row
    /// with the texts of toggleExistingOverlay (`MainWindowOverlay.cpp:221-236`).
    pub(crate) fn toggle_reference_tile_layer(&mut self, layer: &'static RefTileLayer) {
        let Some(name) = self.left_layers.entries.iter().find(|e| e.layer_key == layer.key).map(|e| e.label_ko.clone()) else {
            self.add_reference_tile_layer(layer);
            return;
        };
        let shown = self.home_map.real_survey.is_layer_visible(&name);
        self.home_map.real_survey.layer_visible.insert(name, !shown);
        if let (false, Some(centre)) = (shown, self.reference_circle_centre()) {
            self.bring_view_into_circle(layer, centre);
        }
        self.show_status_message(if shown { format!("{}를 껐습니다.", layer.title) } else { format!("{}를 다시 켰습니다.", layer.title) });
    }

    /// Rows of an earlier survey do not belong to the next one (its survey area is elsewhere).
    pub(crate) fn remove_reference_tile_rows(&mut self) {
        self.left_layers.entries.retain(|e| ref_tile_layer(&e.layer_key).is_none());
    }

    /// Centre of the bounding box of the drawn survey area(s), work CRS metres.
    pub(crate) fn reference_circle_centre(&self) -> Option<[f64; 2]> {
        crate::survey_area_setup::survey_area_extent(self).map(|b| [(b[0] + b[2]) / 2.0, (b[1] + b[3]) / 2.0])
    }

    /// The same centre for the paint pass: from the cache while the work CRS and the survey-area shapes are
    /// what they were. Per frame this is one hash over the WKT of those shapes (a handful of short strings).
    fn reference_circle_centre_for_paint(&mut self) -> Option<[f64; 2]> {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.work_crs.auth_id().hash(&mut hasher);
        for t in self.survey_tables.iter().filter(|t| crate::survey_area_setup::is_survey_area_key(&t.table)) {
            (&t.table, t.srs_id).hash(&mut hasher);
            for f in &t.layer.features {
                f.wkt_geom.hash(&mut hasher);
            }
        }
        let key = hasher.finish();
        if self.ref_circle.key != Some(key) {
            self.ref_circle =
                RefCircleCache { key: Some(key), centre: self.reference_circle_centre(), worked_out: self.ref_circle.worked_out + 1 };
        }
        self.ref_circle.centre
    }

    /// Paints the checked reference tile rows over the background tiles.
    pub(crate) fn paint_reference_tile_layers(&mut self, ctx: &egui::Context, p: &egui::Painter, rect: egui::Rect) {
        let history_key = self.tile_key_for(TileKind::History1919);
        self.paint_reference_tile_layers_with(ctx, p, rect, move |kind, z, x, y| {
            let key = if kind == TileKind::History1919 { history_key.clone() } else { String::new() };
            move || crate::basemap_cadastral_jobs::load_basemap_image(kind, &key, z, x, y)
        });
    }

    pub(crate) fn paint_reference_tile_layers_with<M, F>(
        &mut self,
        ctx: &egui::Context,
        p: &egui::Painter,
        rect: egui::Rect,
        mut make_job: M,
    ) where
        M: FnMut(TileKind, u8, u32, u32) -> F,
        F: FnOnce() -> Option<egui::ColorImage> + Send + 'static,
    {
        // Lowest row first, so that a higher row is painted over a lower one.
        let rows: Vec<(&'static RefTileLayer, f32)> = self
            .left_layers
            .entries
            .iter()
            .rev()
            .filter_map(|e| {
                let layer = ref_tile_layer(&e.layer_key)?;
                self.home_map.real_survey.is_layer_visible(&e.label_ko).then_some((layer, e.opacity.clamp(0.0, 1.0)))
            })
            .collect();
        // The pictures of a layer that is not shown (row removed or unchecked) are not kept in memory.
        for layer in REF_TILE_LAYERS {
            if !rows.iter().any(|(shown, _)| shown.key == layer.key) {
                self.basemap_textures.retain_kinds(&[layer.kind], &std::collections::HashSet::new());
            }
        }
        if rows.is_empty() {
            return;
        }
        let Some(centre) = self.reference_circle_centre_for_paint() else {
            return;
        };
        let mpp = self.home_map.viewport.meters_per_pixel.max(1e-6);
        let view = TileView::from_viewport(&self.home_map.viewport, rect, self.work_crs);
        for (layer, opacity) in rows {
            let circle = ScreenCircle { center: view.meters_to_screen(rect, centre), radius: (layer.radius_m / mpp) as f32 };
            paint_circle_tiles_with(
                ctx,
                p,
                rect,
                layer.kind,
                crate::basemap_tiles::level_for(layer.kind.grid(), mpp, rect).min(layer.max_zoom),
                &view,
                &circle,
                &mut self.basemap_textures,
                &self.basemap_fetch,
                Color32::WHITE.gamma_multiply(opacity),
                &mut make_job,
            );
        }
    }
}

#[path = "ref_tile_history.rs"]
pub(crate) mod history;
#[cfg(test)]
#[path = "ref_tile_layers_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "ref_tile_layers_tests_b.rs"]
mod tests_b;
#[cfg(test)]
#[path = "ref_tile_layers_tests_c.rs"]
mod tests_c;
#[cfg(test)]
#[path = "ref_tile_layers_tests_d.rs"]
mod tests_d;
