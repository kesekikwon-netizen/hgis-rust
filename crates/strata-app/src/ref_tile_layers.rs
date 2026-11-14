//! Online reference tile layers of the legend: 대동여지도.
//!
//! C++ adds them to the project as live tile layers (`target/cpp-ref/src/core/BasemapOps.cpp:1565-1575`,
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
};

const REF_TILE_LAYERS: [&RefTileLayer; 1] = [&DAEDONGYEOJIDO];

/// The reference tile layer whose legend row has this `layer_key`.
pub(crate) fn ref_tile_layer(key: &str) -> Option<&'static RefTileLayer> {
    REF_TILE_LAYERS.iter().copied().find(|l| l.key == key)
}

/// Adds the row unless it is there. Live tiles sink below the other layers and the satellite stays lowest
/// (`BasemapOps.cpp:362-401` ensureSatelliteAtBottom): the row goes directly above the background rows.
fn ensure_row(panel: &mut LeftLayerPanel, layer: &RefTileLayer) {
    if panel.entries.iter().any(|e| e.layer_key == layer.key) {
        return;
    }
    let at = panel.entries.iter().position(|e| e.group == LegendGroup::Basemap).unwrap_or(panel.entries.len());
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
    /// The ribbon command (옛 지도 → 대동여지도): the legend row, checked, and the view on the whole circle.
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
        // The command frames the circle: the survey area in the middle, the whole radius in view.
        let r = layer.radius_m * 1.1;
        self.home_map.set_view_extent([centre[0] - r, centre[1] - r, centre[0] + r, centre[1] + r]);
        self.show_status_message(layer.added_text);
        true
    }

    /// Rows of an earlier survey do not belong to the next one (its survey area is elsewhere).
    pub(crate) fn remove_reference_tile_rows(&mut self) {
        self.left_layers.entries.retain(|e| ref_tile_layer(&e.layer_key).is_none());
    }

    /// Centre of the bounding box of the drawn survey area(s), work CRS metres.
    fn reference_circle_centre(&self) -> Option<[f64; 2]> {
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
        self.paint_reference_tile_layers_with(ctx, p, rect, |kind, z, x, y| {
            move || crate::basemap_cadastral_jobs::load_basemap_image(kind, "", z, x, y)
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
        let zoom = crate::basemap_tiles::basemap_zoom_for(mpp, rect);
        for (layer, opacity) in rows {
            let circle = ScreenCircle { center: view.meters_to_screen(rect, centre), radius: (layer.radius_m / mpp) as f32 };
            paint_circle_tiles_with(
                ctx,
                p,
                rect,
                layer.kind,
                zoom.min(layer.max_zoom),
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

#[cfg(test)]
#[path = "ref_tile_layers_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "ref_tile_layers_tests_b.rs"]
mod tests_b;
#[cfg(test)]
#[path = "ref_tile_layers_tests_d.rs"]
mod tests_d;
