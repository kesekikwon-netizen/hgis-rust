//! What happens after sheets were received: they are listed, placed and merged for the map in one go
//! (C++ `KaTopographicScopePanel.cpp` prepareSheets / prepareOne / restoreNearby and
//! `KaTopographicImportDialog.cpp` publishPrepared). The sheets already in the library are placed and
//! merged with the new ones, so the map always shows every sheet the survey has.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value};
use strata_engine::Scope;
use strata_net::ngii_topographic::TopographicSheets;

use crate::library::{official_bounds, Library, SheetEntry, SheetState};
use crate::merge::{merge, MergeStop, SheetLines};
use crate::sheet::{place, scan};

#[cfg(test)]
#[path = "post_tests.rs"]
mod tests;

/// `KaTopographicScopePanel.cpp:222-223`.
pub const NOT_LINKED: &str = "받은 파일과 공식 1:25,000 DXF 도엽 정보를 연결하지 못했습니다. 자동 적재하지 않았습니다.";
/// `KaTopographicScopePanel.cpp:300`.
pub const LIST_NOT_SAVED: &str = "도엽 보관 정보를 저장하지 못했습니다. 받은 원본은 유지됩니다.";
/// The folder and the name of the merged file (`KaTopographicImportDialog.cpp:108`, `:155`).
pub const MERGED_FOLDER: &str = "합침";
pub const MERGED_PREFIX: &str = "수치지형도-합침-";

/// A file the download handed on, with the portal's record of its sheet.
#[derive(Debug, Clone, PartialEq)]
pub struct Received {
    pub path: PathBuf,
    pub record: Map<String, Value>,
}

/// What the library holds for the map now.
#[derive(Debug, Clone, PartialEq)]
pub struct Posted {
    /// The merged file, when any line lies near the survey.
    pub merged: Option<PathBuf>,
    pub lines: u64,
    /// The numbers of the sheets on the map.
    pub placed: Vec<String>,
    /// The sheets that were not put on the map, each with the reason (the C++ 「검토 필요」).
    pub review: Vec<(String, String)>,
    /// What else the user should know.
    pub warnings: Vec<String>,
}

/// Lists the received files in the library folder `root` (`<survey folder>/지형도`), places every sheet of
/// the library, and merges their lines near the survey into a new file. `portal` holds the records of the
/// portal's list for this area: they bring the official rectangle of sheets listed earlier up to date.
/// `stamp_ms` names the merged file.
///
/// # Errors
/// The user stopped it, or the merged file cannot be written.
pub fn post_process(root: &Path, received: &[Received], portal: &[Map<String, Value>], scope: &Scope, work_epsg: u32, stamp_ms: u128, cancelled: &dyn Fn() -> bool) -> Result<Posted, MergeStop> {
    let text = |record: &Map<String, Value>, key: &str| record.get(key).and_then(Value::as_str).unwrap_or_default().to_owned();
    let is_dxf = |path: &Path| path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("dxf"));
    let mut library = Library::open(root);
    let mut warnings = Vec::new();

    // The new files: each must be tied to the portal's record of one 1:25,000 DXF sheet (`:216-223`).
    for file in received.iter().filter(|file| is_dxf(&file.path)) {
        let record = &file.record;
        let (num, official) = (text(record, "num"), official_bounds(record));
        let linked = text(record, "source") == "NGII suchiQuery.do" && text(record, "scale") == "25000" && text(record, "fileExt").eq_ignore_ascii_case("dxf") && TopographicSheets::resolve(&num).is_some() && official.is_some();
        if !linked {
            warnings.push(NOT_LINKED.to_owned());
            continue;
        }
        let dxf = file.path.strip_prefix(root).unwrap_or(&file.path).to_path_buf();
        library.upsert(SheetEntry { num, name: text(record, "name"), projection: text(record, "projection"), official, dxf, state: SheetState::Received });
    }
    // The rectangle of a sheet listed earlier, as the portal gives it today.
    for record in portal {
        if let (Some(entry), Some(official)) = (library.sheets.iter_mut().find(|entry| entry.num == text(record, "num")), official_bounds(record)) {
            entry.official = Some(official);
        }
    }

    // Every sheet of the library is placed: the new ones and those of earlier days.
    let (mut sheets, mut placed, mut review) = (Vec::new(), Vec::new(), Vec::new());
    for i in 0..library.sheets.len() {
        if cancelled() {
            return Err(MergeStop::Cancelled);
        }
        let dxf = library.dxf_of(&library.sheets[i]);
        if !dxf.is_file() {
            continue;
        }
        let entry = &mut library.sheets[i];
        match scan(&dxf).and_then(|scanned| place(&entry.num, &entry.projection, &scanned, entry.official)) {
            Ok(found) => {
                warnings.extend(found.warnings);
                placed.push(entry.num.clone());
                sheets.push(SheetLines { dxf, number: entry.num.clone(), epsg: found.epsg, shown: found.shown });
                entry.state = SheetState::Placed { epsg: found.epsg };
            }
            Err(reason) => {
                review.push((entry.num.clone(), reason.clone()));
                entry.state = SheetState::Review { reason };
            }
        }
    }
    if library.save().is_err() {
        warnings.push(LIST_NOT_SAVED.to_owned());
    }

    let folder = root.join(MERGED_FOLDER);
    let out = folder.join(format!("{MERGED_PREFIX}{stamp_ms}.gpkg"));
    if std::fs::create_dir_all(&folder).is_err() {
        return Err(MergeStop::Failed("수치지형도 레이어를 지도에 추가하지 못했습니다".to_owned()));
    }
    let lines = merge(&sheets, scope, work_epsg, &out, cancelled)?;
    if lines == 0 {
        return Ok(Posted { merged: None, lines, placed, review, warnings });
    }
    // The merged files of earlier runs are this app's own and are replaced by the new one (`:966`).
    for older in std::fs::read_dir(&folder).into_iter().flatten().flatten().map(|entry| entry.path()) {
        let is_merged = older.file_name().and_then(|name| name.to_str()).is_some_and(|name| name.starts_with(MERGED_PREFIX));
        if is_merged && older != out {
            let _ = std::fs::remove_file(&older);
        }
    }
    Ok(Posted { merged: Some(out), lines, placed, review, warnings })
}
