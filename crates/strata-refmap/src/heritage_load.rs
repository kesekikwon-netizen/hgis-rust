//! Loading one kind of received heritage data for the map (`target/cpp-ref/src/core/HeritageImport.cpp:159-337`
//! loadDataset): every shapefile of its archives becomes one layer, named by the file, holding the sites near
//! the survey area. The archives are unpacked by the function the caller hands in.

use std::path::{Path, PathBuf};

use strata_engine::{Scope, ShapeKind};

use crate::heritage::{import_shapefile, HeritageDataset};

#[cfg(test)]
#[path = "heritage_load_tests.rs"]
mod tests;

/// One layer for the map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loaded {
    /// The name of the shapefile, as the legend row is named (`:237-240`).
    pub title: String,
    /// The copy of the sites near the survey area (`heritage::TABLE`).
    pub file: PathBuf,
    pub sites: u64,
    pub kind: ShapeKind,
    /// Whether the sites have names to write.
    pub named: bool,
}

/// What loading a kind came to.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DatasetLoad {
    pub layers: Vec<Loaded>,
    /// Words for the user (`out.messages`).
    pub messages: Vec<String>,
    /// The data is sound, but nothing of it lies within 5 km of the survey area (`:285-295`).
    pub empty_in_scope: bool,
}

/// Why a kind could not be loaded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadFailure {
    pub reason: String,
    /// Receiving the archive again could help (`retryableDownload`).
    pub download_again: bool,
}

/// Loads a kind from its received archives. `unpack` gives the files an archive holds; `scope` is the survey
/// area grown by 5 km with its CRS; `region` names a neighbour city, empty for the survey's own; the copies
/// are written into `work`.
///
/// # Errors
/// What the C++ stops a kind for, in its words.
pub fn load_dataset(dataset: &HeritageDataset, received: &[PathBuf], unpack: &dyn Fn(&Path) -> Result<Vec<PathBuf>, LoadFailure>, scope: Option<(&Scope, u32)>, region: &str, work: &Path) -> Result<DatasetLoad, LoadFailure> {
    let stop = |reason: String, download_again: bool| LoadFailure { reason, download_again };
    let file_name = |path: &Path| path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
    let has_suffix = |path: &Path, suffix: &str| path.extension().is_some_and(|e| e.to_string_lossy().eq_ignore_ascii_case(suffix));
    if received.is_empty() {
        return Err(stop("받은 파일이 없습니다.".to_owned(), false));
    }
    // The shapefiles of every archive, and whether each came out of an archive (`:186-212`).
    let mut shapefiles = Vec::new();
    for file in received {
        let is_archive = has_suffix(file, "zip");
        let held = unpack(file).map_err(|e| stop(format!("{}: {}", file_name(file), e.reason), e.download_again))?;
        let found: Vec<PathBuf> = held.into_iter().filter(|inner| has_suffix(inner, "shp")).collect();
        if is_archive && found.is_empty() {
            return Err(stop(format!("{}에서 SHP를 찾지 못했습니다. 자료를 다시 받아야 합니다.", file_name(file)), true));
        }
        shapefiles.extend(found.into_iter().map(|shp| (shp, is_archive)));
    }
    if shapefiles.is_empty() {
        return Err(stop("받은 파일에서 SHP를 찾지 못했습니다. 파일 형식을 확인해야 합니다.".to_owned(), false));
    }
    std::fs::create_dir_all(work).map_err(|e| stop(format!("{}: {e}", work.display()), false))?;
    let region = region.trim();
    let (mut load, mut beyond) = (DatasetLoad::default(), 0_u32);
    for (index, (shp, from_archive)) in shapefiles.iter().enumerate() {
        for suffix in ["shx", "dbf", "prj"] {
            if !shp.with_extension(suffix).exists() {
                return Err(stop(format!("{}의 필수 파일 .{suffix}가 없습니다. 적재를 중단합니다.", file_name(shp)), *from_archive));
            }
        }
        // The layer is named by the file, the colour is the kind's (`:226-240`).
        let stem = shp.file_stem().map(|stem| stem.to_string_lossy().into_owned()).filter(|stem| !stem.is_empty());
        let mut title = stem.unwrap_or_else(|| dataset.name.to_owned());
        if !region.is_empty() {
            title = format!("{title} · {region}");
        }
        let file = work.join(format!("{}-{index:02}.gpkg", dataset.key));
        let imported = import_shapefile(shp, dataset, scope, &file).map_err(|reason| stop(reason, false))?;
        if imported.kept == 0 {
            beyond += u32::from(imported.read > 0);
            continue;
        }
        if imported.name_column.is_none() {
            load.messages.push(format!("{title}: 유적명 컬럼을 찾지 못해 범례에 이름을 넣지 못했습니다."));
        }
        load.layers.push(Loaded { title, file, sites: imported.kept, kind: imported.kind, named: imported.name_column.is_some() });
    }
    if load.layers.is_empty() {
        if beyond == 0 {
            return Err(stop(format!("{} 자료를 지도에 올리지 못했습니다.", dataset.name), false));
        }
        // The data is sound: this kind of this city lies only beyond 5 km of the survey (`:285-295`).
        let of_region = if region.is_empty() { String::new() } else { format!(" ({region})") };
        load.empty_in_scope = true;
        load.messages.push(format!("{}{of_region}: 조사구역 주변 5km 안에 든 자료가 없습니다.", dataset.name));
    }
    Ok(load)
}
