//! After the 「DEM」 run: the received DEM files are unpacked and listed in the survey's DEM folder, so that
//! a sheet's DEM is not ordered again (`<survey folder>/DEM/strata-dem.json`), and the DEM goes on the map
//! as the legend rows 「DEM」 and 「지형 음영」 (`target/cpp-ref/src/core/BasemapOps.cpp:1996-2037`
//! addDemElevationRaster, `:2039-2123` ensureDemRelief; drawn by `ref_vector_elevation.rs`).
//!
//! Fetching it is the app's own: the C++ takes the NGII DEM only as a file the user picks
//! (`MainWindowRibbon.cpp:405-413`); the owner asked for the DEM of the same area as the topographic
//! sheets to be fetched by the button (2026-11-15).

use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};
use strata_topo::archive::{prepare, Limits};
use strata_types::crs::CrsCode;

use super::post::TopoPost;
use crate::app_state::StrataAppState;
use crate::download_jobs::{DownloadJobHandle, StopCode};
use crate::ref_vector::data::{HeightRamp, LoadSpec, Look};

/// 「DEM 표현」: the colour preset, the relief and its strength.
#[path = "dem_look.rs"]
pub(crate) mod look;
/// The colour bar under the DEM's legend row.
#[path = "dem_legend.rs"]
pub(crate) mod legend;
/// 「국토지리원 DEM 불러오기(.img)…」.
#[path = "dem_import.rs"]
pub(crate) mod import;
#[cfg(test)]
#[path = "dem_post_tests.rs"]
mod tests;

/// The list's file in the survey's DEM folder.
pub(crate) const LIST_FILE: &str = "strata-dem.json";
/// `layer_key` and title of the two legend rows (`BasemapOps.cpp:2007`, `:2064`).
pub(crate) const DEM_KEY: &str = "ref_dem";
pub(crate) const DEM_TITLE: &str = "DEM";
pub(crate) const RELIEF_KEY: &str = "ref_dem_relief";
pub(crate) const RELIEF_TITLE: &str = "지형 음영";
/// The end words of the run, as the sheet run has them (docs/RUST_PLAN.md 4.3 F4).
pub(crate) const DEM_READY: &str = "DEM을 준비했습니다.";
/// This app's own words: the C++ has no order that could end without a file.
const NO_DEM: &str = "지도에 올릴 DEM 이 없습니다.";
const NO_RASTER: &str = "받은 묶음에 DEM 파일(.img)이 없습니다.";
/// The folder of the DEM folder that archives are unpacked into.
const UNPACKED: &str = "풀림";

/// One received DEM: the 1:50,000 sheet it is of, the year of the product, and its file, relative to the
/// DEM folder.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DemEntry {
    pub(crate) num: String,
    pub(crate) name: String,
    pub(crate) year: String,
    pub(crate) file: PathBuf,
}

/// The list of a DEM folder; empty when there is none or it cannot be read.
pub(crate) fn read_list(root: &Path) -> Vec<DemEntry> {
    let listed = std::fs::read(root.join(LIST_FILE)).ok().and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok());
    let entry = |listed: &Value| {
        let text = |key: &str| listed[key].as_str().unwrap_or_default().to_owned();
        let (num, file) = (text("num"), text("file"));
        (!num.is_empty() && !file.is_empty()).then(|| DemEntry { num, name: text("name"), year: text("year"), file: PathBuf::from(file) })
    };
    listed.as_ref().filter(|list| list["schema"].as_u64() == Some(1)).and_then(|list| list["sheets"].as_array()).map(|sheets| sheets.iter().filter_map(entry).collect()).unwrap_or_default()
}

/// Puts the entries into the list of the folder, each in the place of an older entry of its sheet.
///
/// # Errors
/// The folder cannot be written.
pub(crate) fn add_to_list(root: &Path, entries: &[DemEntry]) -> std::io::Result<()> {
    let mut list = read_list(root);
    for entry in entries {
        match list.iter_mut().find(|known| known.num == entry.num) {
            Some(known) => *known = entry.clone(),
            None => list.push(entry.clone()),
        }
    }
    let sheets: Vec<Value> = list.iter().map(|e| json!({"num": e.num, "name": e.name, "year": e.year, "file": e.file.to_string_lossy().replace('\\', "/")})).collect();
    std::fs::create_dir_all(root)?;
    // Written beside and moved into place: a list cut short by a crash would read as an empty one, and the
    // next press would order every sheet again.
    let beside = root.join(format!("{LIST_FILE}.{}.tmp", std::process::id()));
    std::fs::write(&beside, json!({"schema": 1, "sheets": sheets}).to_string())?;
    std::fs::rename(&beside, root.join(LIST_FILE)).inspect_err(|_| drop(std::fs::remove_file(&beside)))
}

/// The listed sheets whose DEM file is still in the folder.
fn kept(root: &Path) -> impl Iterator<Item = DemEntry> + '_ {
    read_list(root).into_iter().filter(|entry| root.join(&entry.file).is_file())
}

/// The sheets whose DEM file is still in the folder: they are not ordered again.
pub(crate) fn received_numbers(root: &Path) -> Vec<String> {
    kept(root).map(|entry| entry.num).collect()
}

/// The files of the listed sheets that are still in the folder.
pub(crate) fn files_of(root: &Path) -> Vec<PathBuf> {
    kept(root).map(|entry| root.join(entry.file)).collect()
}

fn is_raster(file: &Path) -> bool {
    file.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("img"))
}

/// The raster of a received file: the file itself, or the raster of the archive it came in, unpacked under
/// the DEM folder (the archive is left as it came).
fn raster_of(root: &Path, received: &Path) -> Result<PathBuf, String> {
    if is_raster(received) {
        return Ok(received.to_path_buf());
    }
    let prepared = prepare(received, &root.join(UNPACKED), &Limits::default(), &|| false);
    if !prepared.error.is_empty() {
        return Err(prepared.error);
    }
    prepared.files.into_iter().find(|file| is_raster(file)).ok_or_else(|| NO_RASTER.to_owned())
}

/// What a download handed on, as entries of the list, and what could not be made ready, said by sheet
/// (`<도엽> · <이유>`). A file the portal has no DEM record for is left out.
pub(crate) fn entries_of(root: &Path, received: &[(PathBuf, Map<String, Value>)]) -> (Vec<DemEntry>, Vec<String>) {
    let text = |record: &Map<String, Value>, key: &str| record.get(key).and_then(Value::as_str).unwrap_or_default().to_owned();
    // The unpacker names its files by the folder's full path; the list keeps them from the DEM folder.
    let full_root = std::path::absolute(root).unwrap_or_else(|_| root.to_path_buf());
    let (mut entries, mut problems) = (Vec::new(), Vec::new());
    for (path, record) in received.iter().filter(|(_, record)| text(record, "source") == "NGII demQuery.do" && !text(record, "num").is_empty()) {
        match raster_of(root, path) {
            Ok(raster) => {
                let file = raster.strip_prefix(&full_root).or_else(|_| raster.strip_prefix(root)).unwrap_or(&raster).to_path_buf();
                entries.push(DemEntry { num: text(record, "num"), name: text(record, "name"), year: text(record, "year"), file });
            }
            Err(reason) => problems.push(format!("{} · {reason}", text(record, "num"))),
        }
    }
    (entries, problems)
}

impl StrataAppState {
    /// The two rows of a DEM: 「지형 음영」 over 「DEM」. `root` is the survey's DEM folder (the layers read
    /// the files of its list) or one DEM file of the user.
    fn show_dem_layers(&mut self, root: &Path) {
        let work_epsg = CrsCode::from(self.work_crs).epsg_number();
        let spec = |look| LoadSpec { file: root.to_path_buf(), table: String::new(), work_epsg, look, label: None };
        // The relief first: the next row comes in under it.
        let had_relief = self.left_layers.entries.iter().any(|e| e.layer_key == RELIEF_KEY);
        let (look, ramp) = (self.dem_look, self.dem_ramp(self.dem_look.preset).unwrap_or(HeightRamp::National));
        self.show_reference_vector_layer(RELIEF_KEY, RELIEF_TITLE, spec(Look::Relief { z_factor: look.z_factor }));
        if let Some(row) = self.left_layers.entries.iter_mut().find(|e| e.layer_key == RELIEF_KEY && !had_relief) {
            row.opacity = look.relief_opacity();
        }
        self.show_reference_vector_layer(DEM_KEY, DEM_TITLE, spec(Look::Heights { ramp }));
        // A relief that 「DEM 표현」 switched off stays off under a new DEM.
        if let (false, Some(name)) = (look.relief, self.relief_row_name()) {
            self.home_map.real_survey.layer_visible.insert(name, false);
        }
    }

    /// Whether the DEM is on the map and checked: the ribbon button then looks switched on
    /// (`m_btnDem->setChecked(makeVisible)`, MainWindowOverlay.cpp:317).
    pub(crate) fn dem_shown(&self) -> bool {
        self.dem_row_name().is_some_and(|name| self.home_map.real_survey.is_layer_visible(&name))
    }

    /// The present name of the DEM's legend row, when the DEM is on the map (`BasemapDsm::findDem`).
    pub(crate) fn dem_row_name(&self) -> Option<String> {
        self.legend_name_of(DEM_KEY)
    }

    /// The present name of the relief's legend row (the user may have renamed it).
    pub(crate) fn relief_row_name(&self) -> Option<String> {
        self.legend_name_of(RELIEF_KEY)
    }

    fn legend_name_of(&self, key: &str) -> Option<String> {
        self.left_layers.entries.iter().find(|e| e.layer_key == key).map(|e| e.label_ko.clone())
    }

    /// Puts the DEM the survey folder has on the map (a scripted screenshot shows it without an order).
    pub(crate) fn show_survey_dem(&mut self) {
        if let Some(root) = crate::survey_folder::survey_folder(self.survey_path.as_deref()).map(|folder| folder.join("DEM")) {
            self.show_dem_layers(&root);
        }
    }

    /// The end of a DEM run: what it brought is unpacked and listed, and the DEM the survey now has goes on
    /// the map. Nothing here takes long (a sheet's DEM is some hundred kilobytes), so nothing is left running.
    pub(crate) fn begin_dem_post(&mut self, received: Vec<(PathBuf, Map<String, Value>)>, job: DownloadJobHandle) -> Option<TopoPost> {
        let Some(root) = crate::survey_folder::survey_folder(self.survey_path.as_deref()).map(|folder| folder.join("DEM")) else {
            job.report_stopped(StopCode::Dir1, "먼저 새 조사를 만들거나 기존 조사를 여세요. 조사 폴더 안의 DEM 폴더에 저장합니다.");
            return None;
        };
        let (entries, problems) = entries_of(&root, &received);
        if self.snap.is_some() {
            eprintln!("[SNAP] DEM 올리기: 받은 파일 {} · 목록에 넣을 도엽 {entries:?} · 문제 {problems:?}", received.len());
        }
        if !entries.is_empty() && add_to_list(&root, &entries).is_err() {
            job.report_stopped(StopCode::Dir2, "DEM 보관 정보를 저장하지 못했습니다. 받은 원본은 유지됩니다.");
            return None;
        }
        let sheets = received_numbers(&root).len();
        if sheets == 0 {
            if problems.is_empty() {
                job.report_done(NO_DEM);
                self.show_status_message(NO_DEM);
            } else {
                self.show_status_message(problems.join(" "));
                job.report_stopped(StopCode::Data1, problems.join(" "));
            }
            return None;
        }
        self.show_dem_layers(&root);
        job.report_done(DEM_READY);
        let mut said = vec![format!("{DEM_READY} 도엽 {sheets}장을 지도에 올렸습니다.")];
        said.extend(problems);
        self.show_status_message(said.join(" "));
        None
    }
}
