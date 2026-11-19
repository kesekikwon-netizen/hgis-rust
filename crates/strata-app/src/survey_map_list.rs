//! The received maps a survey had in its legend when it was saved come back when it is opened again (owner
//! 2026-11-15 「맵을받아 레이어에 추가되는맵이면 받은것을 같은개발일때 그다음에 넣을때 적용되게하라」; MORE0 M6,
//! To build #1). The C++ keeps these layers in the project inside the survey file (MainWindowSession.cpp:
//! 299-368). Until this app reads and writes that project (docs/RUST_PLAN.md, 2D), a small list beside the
//! survey file, `<survey name>.strata-maps.json`, keeps which rows there were, in their order, with their
//! names, check marks and opacity, and what each row drew. Opening a survey makes the layers again from the
//! kept files: it never orders, logs in or downloads, and a row whose file is gone is left out. 유산 rows
//! are not kept yet.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use strata_types::crs::CrsCode;

use crate::app_state::StrataAppState;
use crate::left_layer_panel::LayerTreeEntry;
use crate::ref_tile_layers::{ensure_row, history::MAP_1919, ref_tile_layer, RefTileLayer};
use crate::ref_vector::cadastral::layer::{kept_style, CADASTRAL_KEY};
use crate::ref_vector::map::{GEOLOGY, RIVER, SOIL};
use crate::topo_download::dem::{files_of, DEM_KEY, RELIEF_KEY};
use crate::topo_download::post::{held_topographic_merge, topo_spec, TOPO_KEY, TOPO_TITLE};
use crate::app_state::user_files::{user_shapefile, USER_PREFIX};
use crate::app_state::picture_layers::{picture_layer_of, PICTURE_PREFIX};

/// The version of the list; a list of another version is not read.
const VERSION: u32 = 1;

/// One row of a received map.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct KeptRow {
    key: String,
    title: String,
    visible: bool,
    opacity: f32,
    /// What the row draws (a kept file, or the DEM folder), from the survey folder when it lies inside it;
    /// none for a map of tiles.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    file: Option<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize)]
struct KeptList {
    version: u32,
    rows: Vec<KeptRow>,
}

/// The list beside a survey file: `<name>.strata-maps.json`. Surveys of one folder share the received
/// files, not their legends.
pub(crate) fn list_path(survey: &Path) -> PathBuf {
    survey.with_extension("strata-maps.json")
}

/// The rows the list keeps: the maps whose layers can be made again from what the survey folder holds, the
/// maps of tiles, and the files of the user (user_files.rs; the buffer rings are made, not files).
fn kept_key(key: &str) -> bool {
    [TOPO_KEY, DEM_KEY, RELIEF_KEY, CADASTRAL_KEY, SOIL.key, GEOLOGY.key, RIVER.key].contains(&key)
        || ref_tile_layer(key).is_some()
        || (key.starts_with(USER_PREFIX) && !key.starts_with("user:buffer_"))
        || key.starts_with(PICTURE_PREFIX)
}

impl StrataAppState {
    /// Writes the list beside the saved survey file; without received maps an older list is taken away.
    pub(crate) fn write_survey_map_list(&self, survey: &Path) -> std::io::Result<()> {
        let list = list_path(survey);
        let folder = std::path::absolute(survey)?.parent().map(Path::to_path_buf).unwrap_or_default();
        let visible = |name: &str| self.home_map.real_survey.is_layer_visible(name);
        let from_folder = |file: &Path| file.strip_prefix(&folder).unwrap_or(file).to_path_buf();
        let received = self.left_layers.entries.iter().filter(|e| kept_key(&e.layer_key));
        let rows: Vec<KeptRow> = received
            .map(|e| KeptRow {
                key: e.layer_key.clone(),
                title: e.label_ko.clone(),
                visible: visible(&e.label_ko),
                opacity: e.opacity,
                file: self.ref_vectors.spec_of(&e.layer_key).map(|spec| from_folder(&spec.file)).or_else(|| self.pictures.layers.iter().find(|l| l.key == e.layer_key).map(|l| from_folder(&l.file))),
            })
            .collect();
        if rows.is_empty() {
            return match std::fs::remove_file(&list) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
                _ => Ok(()),
            };
        }
        let text = serde_json::to_string_pretty(&KeptList { version: VERSION, rows }).map_err(std::io::Error::other)?;
        std::fs::write(list, text)
    }

    /// Puts the received maps of the list back on the map, in its order, with its names, check marks and
    /// opacity. Nothing is ordered, fetched or logged in to: a row whose kept file is gone is left out.
    pub(crate) fn restore_survey_maps(&mut self) {
        let Some(survey) = self.survey_path.clone() else { return };
        let Some(folder) = crate::survey_folder::survey_folder(Some(&survey)) else { return };
        let read = std::fs::read(list_path(&survey)).ok().and_then(|bytes| serde_json::from_slice::<KeptList>(&bytes).ok());
        let Some(list) = read.filter(|list| list.version == VERSION) else { return };
        let before: Vec<String> = self.left_layers.entries.iter().map(|e| e.layer_key.clone()).collect();
        let back: Vec<&KeptRow> = list.rows.iter().filter(|row| self.restore_row(row, &survey, &folder)).collect();
        // A row made with another (the relief with the DEM) that the list does not have goes again.
        let listed = |key: &String| before.contains(key) || back.iter().any(|row| &row.key == key);
        let extra: Vec<String> = self.left_layers.entries.iter().map(|e| e.layer_key.clone()).filter(|key| !listed(key)).collect();
        for key in &extra {
            self.remove_reference_vector_row(key);
        }
        for row in &back {
            self.rename_layer_to(&row.key, &row.title);
            if let Some(entry) = self.left_layers.entries.iter_mut().find(|e| e.layer_key == row.key) {
                entry.opacity = row.opacity;
            }
            self.home_map.real_survey.layer_visible.insert(row.title.clone(), row.visible);
        }
        self.put_rows_in_order(&back.iter().map(|row| row.key.as_str()).collect::<Vec<_>>());
    }

    /// Makes the row's layer again from what the survey folder holds; false when that is gone.
    fn restore_row(&mut self, row: &KeptRow, survey: &Path, folder: &Path) -> bool {
        if let Some(layer) = ref_tile_layer(&row.key) {
            return self.restore_tile_row(layer);
        }
        // `join` keeps a whole path (a file outside the survey folder, after 다른 이름으로 저장) as it is.
        let file = row.file.as_ref().map(|file| folder.join(file)).filter(|file| file.is_file() || !files_of(file).is_empty());
        let work_epsg = CrsCode::from(self.work_crs).epsg_number();
        if row.key.starts_with(PICTURE_PREFIX) {
            let Some((layer, _)) = file.and_then(|file| picture_layer_of(&file, row.key.clone(), work_epsg)) else { return false };
            self.put_picture_row(layer, &row.title);
            return true;
        }
        if row.key.starts_with(USER_PREFIX) {
            let Some(Ok((spec, _, _))) = file.map(|file| user_shapefile(&file, work_epsg)) else { return false };
            self.show_user_vector_layer(&row.key, &row.title, spec);
            return true;
        }
        if row.key == TOPO_KEY {
            // A later run in this folder replaces the merged file by a newer one that has every sheet.
            let Some(file) = file.or_else(|| held_topographic_merge(Some(survey))) else { return false };
            self.show_reference_vector_layer(TOPO_KEY, TOPO_TITLE, topo_spec(file, work_epsg));
            return true;
        }
        let Some(file) = file else { return false };
        match row.key.as_str() {
            // The two rows of a DEM come together.
            DEM_KEY | RELIEF_KEY => {
                if !self.left_layers.entries.iter().any(|e| e.layer_key == row.key) {
                    self.show_dem_layers(&file);
                }
            }
            CADASTRAL_KEY => {
                // The look is in the note beside the tables (`지적도/strata-kept.json`).
                let style = file.parent().and_then(Path::parent).and_then(|directory| kept_style(directory, &file));
                self.show_cadastral_layer(file, style);
            }
            key => {
                let Some(kind) = [&SOIL, &GEOLOGY, &RIVER].into_iter().find(|kind| kind.key == key) else { return false };
                let spec = self.load_spec(kind, file);
                self.show_reference_vector_layer(kind.key, kind.title, spec);
            }
        }
        true
    }

    fn restore_tile_row(&mut self, layer: &'static RefTileLayer) -> bool {
        if layer.key == MAP_1919.key {
            let key = (self.history_gis_key_reader)();
            if !key.trim().is_empty() {
                key.trim().clone_into(&mut self.history_gis_key);
            }
        }
        ensure_row(&mut self.left_layers, layer);
        true
    }

    /// Puts the rows of these keys into this order, in the places they hold between the other rows.
    fn put_rows_in_order(&mut self, keys: &[&str]) {
        let entries = &mut self.left_layers.entries;
        let places: Vec<usize> = entries.iter().enumerate().filter(|(_, e)| keys.contains(&e.layer_key.as_str())).map(|(at, _)| at).collect();
        let rows: Vec<LayerTreeEntry> = keys.iter().filter_map(|key| entries.iter().find(|e| e.layer_key == *key).cloned()).collect();
        if rows.len() == places.len() {
            for (at, row) in places.into_iter().zip(rows) {
                entries[at] = row;
            }
        }
    }
}

#[cfg(test)]
#[path = "survey_map_list_tests.rs"]
mod tests;
