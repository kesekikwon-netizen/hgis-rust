//! Reference layers and the app: a layer gets its legend row, and the checked rows are painted every frame.

use std::sync::Arc;

use eframe::egui::{self, Rect};
use strata_canvas2d::viewport::ViewportTransform;
use strata_types::schema::LegendGroup;

use super::{Layer, OPEN_FAILED};
use crate::app_state::StrataAppState;
use crate::gui_tab_map::s2w;
use crate::left_layer_panel::LayerTreeEntry;
use crate::ref_vector::data::LoadSpec;

impl StrataAppState {
    /// Puts the layer on the map: its legend row, checked, above the tile maps and the background. A layer of
    /// that key that is there already now draws the new file; its row keeps what the user set on it.
    pub(crate) fn show_reference_vector_layer(&mut self, key: &str, title: &str, spec: LoadSpec) {
        let entries = &self.left_layers.entries;
        let at = entries.iter().position(|e| e.group == LegendGroup::Basemap || crate::ref_tile_layers::ref_tile_layer(&e.layer_key).is_some()).unwrap_or(entries.len());
        self.put_vector_row(key, title, spec, LegendGroup::ReferenceMap, at);
    }

    /// The same for the received cadastral map, whose row is a top row of its own, outside 주변·참조
    /// (`LayerOps.cpp:1091-1101`).
    pub(crate) fn show_top_vector_layer(&mut self, key: &str, title: &str, spec: LoadSpec) {
        self.put_vector_row(key, title, spec, LegendGroup::Cadastral, 0);
    }

    /// A file of the user: a row of 조사 데이터 at the top of that group (`addVectorFromPath` marks it a survey
    /// layer, LayerOps.cpp:1040-1044), read-only here (user_files.rs).
    pub(crate) fn show_user_vector_layer(&mut self, key: &str, title: &str, spec: LoadSpec) {
        let entries = &self.left_layers.entries;
        let at = entries.iter().position(|e| e.group == LegendGroup::SurveyData).or_else(|| entries.iter().position(|e| e.group != LegendGroup::Cadastral)).unwrap_or(entries.len());
        self.put_vector_row(key, title, spec, LegendGroup::SurveyData, at);
    }

    fn put_vector_row(&mut self, key: &str, title: &str, spec: LoadSpec, group: LegendGroup, at: usize) {
        self.ref_vectors.layers.retain(|l| l.key != key);
        self.ref_vectors.layers.push(Layer::new(key.to_owned(), spec));
        if !self.left_layers.entries.iter().any(|e| e.layer_key == key) {
            let row = LayerTreeEntry { layer_key: key.to_owned(), label_ko: title.to_owned(), group, visible: true, opacity: 1.0, read_only: true, snap_enabled: false, feature_count: 0 };
            self.left_layers.entries.insert(at, row);
        }
        // The check mark is kept by the row's present name (the user may have renamed the row).
        if let Some(name) = self.left_layers.entries.iter().find(|e| e.layer_key == key).map(|e| e.label_ko.clone()) {
            self.home_map.real_survey.layer_visible.insert(name, true);
        }
        // A received map is unsaved work until 저장 keeps it for the next open (`setDirty(true)` after each
        // received map, MainWindowCadastral.cpp:167, MainWindowDownloads.cpp:156, MainWindowOverlay.cpp:671).
        self.is_dirty = true;
    }

    /// Takes the row of this key away, with its layer.
    pub(crate) fn remove_reference_vector_row(&mut self, key: &str) {
        self.ref_vectors.layers.retain(|l| l.key != key);
        self.left_layers.entries.retain(|e| e.layer_key != key);
    }

    /// Rows of an earlier survey do not belong to the next one.
    pub(crate) fn remove_reference_vector_rows(&mut self) {
        let layers = std::mem::take(&mut self.ref_vectors.layers);
        self.left_layers.entries.retain(|e| !layers.iter().any(|l| l.key == e.layer_key));
    }

    /// Takes the rows whose key starts with `prefix` away, with their layers.
    pub(crate) fn remove_reference_vector_rows_of(&mut self, prefix: &str) {
        self.ref_vectors.layers.retain(|l| !l.key.starts_with(prefix));
        self.left_layers.entries.retain(|e| !e.layer_key.starts_with(prefix));
    }

    /// Paints the checked reference vector rows on the map, the lowest row first.
    pub(crate) fn paint_reference_vector_layers(&mut self, ctx: &egui::Context, p: &egui::Painter, rect: Rect) {
        let viewport = self.home_map.viewport;
        // What follows the view (the colours of 「현재 화면 맞춤」, dem_look.rs) asks for the world box of the map.
        let (north_west, south_east) = (s2w(rect, &viewport, rect.left_top()), s2w(rect, &viewport, rect.right_bottom()));
        self.ref_vectors.map_box = Some([north_west[0], south_east[1], south_east[0], north_west[1]]);
        if self.follow_dem_view(ctx.input(|i| i.time)) {
            ctx.request_repaint_after(std::time::Duration::from_millis(360));
        }
        self.paint_reference_vector_layers_in(ctx, p, rect, &viewport, 1.0, |_| true);
        self.zoom_to_new_dem();
    }

    /// Paints the checked reference vector rows into a view: the map, or the map frame of a sheet, which
    /// shows paper `paper_zoom` times as large (1 on the map). `labels_of` says for a row whether its letters
    /// are written in this view.
    pub(crate) fn paint_reference_vector_layers_in(&mut self, ctx: &egui::Context, p: &egui::Painter, rect: Rect, viewport: &ViewportTransform, paper_zoom: f32, labels_of: fn(&str) -> bool) {
        let rows: Vec<&str> = self.left_layers.entries.iter().map(|e| e.layer_key.as_str()).filter(|k| self.ref_vectors.has(k)).collect();
        let shown: Vec<(String, f32)> = self
            .left_layers
            .entries
            .iter()
            .rev()
            .filter(|e| self.ref_vectors.has(&e.layer_key) && self.home_map.real_survey.is_layer_visible(&e.label_ko))
            .map(|e| (e.layer_key.clone(), e.opacity))
            .collect();
        let shown_keys: Vec<&str> = shown.iter().map(|(key, _)| key.as_str()).collect();
        self.ref_vectors.follow_work_crs(strata_types::crs::CrsCode::from(self.work_crs).epsg_number());
        self.ref_vectors.release(&rows, &shown_keys);
        // Labels go over every reference picture, placed among all layers at once by the map engine.
        let mut labelled = Vec::new();
        for (key, opacity) in &shown {
            let mut painted = self.ref_vectors.paint(key, ctx, p, rect, viewport, *opacity, paper_zoom);
            labelled.extend(painted.labelled.take().filter(|_| labels_of(key)));
            if let Some(rows) = painted.rows {
                self.left_layers.update_feature_count_badge(key, usize::try_from(rows).unwrap_or(usize::MAX));
            }
            if painted.failed_now {
                self.show_status_message(OPEN_FAILED);
            }
            // A scripted run has nobody watching: how long the work took goes to its log.
            if self.snap.is_some() {
                if let (Some(rows), Some(secs)) = (painted.rows, painted.read_secs) {
                    eprintln!("[SNAP] 참조 벡터 {key}: {rows} rows read in {secs:.2} s");
                }
                if let Some((width, height, secs)) = painted.drawn {
                    eprintln!("[SNAP] 참조 벡터 {key}: picture {width} x {height} in {secs:.3} s");
                }
            }
        }
        let (runner, work_epsg) = (Arc::clone(&self.ref_vectors.runner), strata_types::crs::CrsCode::from(self.work_crs).epsg_number());
        self.ref_vectors.labels.paint_zoomed(ctx, p, rect, viewport, work_epsg, labelled, &runner, paper_zoom);
    }
}
