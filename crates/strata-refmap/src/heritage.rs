//! The heritage data of the 국가유산 intranet (`target/cpp-ref/src/core/HeritageStyle.cpp`,
//! `HeritageImport.cpp`, `SurveyScopeClip.cpp`): its six kinds, the encoding of a received shapefile, the
//! column that names a site, and the copy of the shapes near the survey area that the map draws.
//!
//! Nothing here contacts the intranet; the received archives are handed in as files.

use std::path::Path;

use strata_engine::{read_shapefile, shapefile_facts, write_new_table, Cell, ColumnKind, Row, Scope, ShapeKind, Shapes, TableLayout};

#[cfg(test)]
#[path = "heritage_tests.rs"]
mod tests;

/// The table of the copy, and its two columns.
pub const TABLE: &str = "heritage";
pub const NAME_COLUMN: &str = "ka_name";
pub const CODE_COLUMN: &str = "ka_code";
/// Shapes that touch the survey area grown by this many metres are kept (`SurveyScopeClip.h:15`).
pub const NEAR_M: f64 = 5000.0;

/// Columns that may hold a site's name, the first that is there wins (`HeritageImport.cpp:33-37`).
const NAME_CANDIDATES: [&str; 19] =
    ["유적명", "국가유산명", "문화재명", "명칭", "유적명칭", "사업명", "조사명", "NAME", "name", "NM", "nm", "SITE_NM", "site_nm", "HERITAGE_NM", "CULTURE_NM", "RELIC_NM", "BURIAL_NM", "TITLE", "title"];
/// The column that holds a site's code (the same site may have several shapes).
const CODE_SOURCE: &str = "유산코드";

/// One kind of heritage data.
#[derive(Debug, PartialEq, Eq)]
pub struct HeritageDataset {
    /// The key kept in files (`HeritageLayoutNumbers.cpp:81-88`).
    pub key: &'static str,
    /// Its Korean name: the legend group, and the start of the name of its archive.
    pub name: &'static str,
    /// One colour for the whole kind (`HeritageStyle.cpp:26-33`).
    pub colour: [u8; 3],
    /// The column this kind shows as a site's name before the usual ones (`HeritageImport.cpp:268-271`).
    pub name_first: Option<&'static str>,
}

pub const DATASETS: [HeritageDataset; 6] = [
    HeritageDataset { key: "designated_heritage", name: "지정유산", colour: [0x8E, 0x44, 0xAD], name_first: None },
    HeritageDataset { key: "alteration_standard", name: "현상변경허용기준", colour: [0xE6, 0x7E, 0x22], name_first: Some("지정종목") },
    HeritageDataset { key: "buried_heritage_area", name: "매장유산유존지역", colour: [0x2E, 0x86, 0xC1], name_first: None },
    HeritageDataset { key: "heritage_distribution_map", name: "문화유적분포지도", colour: [0x27, 0xAE, 0x60], name_first: None },
    HeritageDataset { key: "surface_survey_area", name: "지표조사구역", colour: [0xD4, 0xAA, 0x00], name_first: None },
    HeritageDataset { key: "excavation_survey_area", name: "발굴조사구역", colour: [0xC2, 0x18, 0x7D], name_first: Some("사업명") },
];

/// What an import came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Imported {
    /// Shapes in the file, and those kept.
    pub read: u64,
    pub kept: u64,
    pub kind: ShapeKind,
    /// The CRS of the file and of the copy.
    pub epsg: u32,
    /// The column of the file that names a site, if one was found.
    pub name_column: Option<String>,
}

/// The kind an archive of the site holds: the site names it `<kind>_<time>.zip`.
#[must_use]
pub fn dataset_of_archive(file_name: &str) -> Option<&'static HeritageDataset> {
    let kind = file_name.split(['_', '.']).next().unwrap_or_default();
    DATASETS.iter().find(|dataset| dataset.name == kind)
}

/// The encoding GDAL is to read a received shapefile with.
#[must_use]
pub fn shapefile_encoding(shp: &Path) -> String {
    // `LayerOps.cpp:369-391`: what a `.cpg` says.
    let said = std::fs::read(shp.with_extension("cpg")).map(|bytes| String::from_utf8_lossy(&bytes).trim().to_owned()).unwrap_or_default();
    let upper = said.to_uppercase();
    if upper.contains("UTF-8") || upper.contains("UTF8") {
        return "UTF-8".to_owned();
    }
    if ["CP949", "949", "EUC-KR", "ANSI", "SYSTEM"].iter().any(|korean| upper.contains(korean)) {
        return "CP949".to_owned();
    }
    if !said.is_empty() {
        return said;
    }
    // `:393-447`: the first 32 KB of the `.dbf`. UTF-8 only with at least four whole multi-byte characters
    // and no broken one; Korean public data is CP949 otherwise.
    let mut sample = std::fs::read(shp.with_extension("dbf")).unwrap_or_default();
    sample.truncate(32_768);
    let (mut whole, mut at) = (0_u32, 0_usize);
    while at < sample.len() {
        let follows = |count: usize| sample.get(at + 1..=at + count).is_some_and(|rest| rest.iter().all(|byte| byte & 0xC0 == 0x80));
        let length = match sample[at] {
            0..=127 => 0,
            lead if lead & 0xE0 == 0xC0 && lead >= 0xC2 => 1,
            lead if lead & 0xF0 == 0xE0 => 2,
            lead if lead & 0xF8 == 0xF0 && lead <= 0xF4 => 3,
            _ => return "CP949".to_owned(),
        };
        if length > 0 {
            if !follows(length) {
                return "CP949".to_owned();
            }
            whole += 1;
        }
        at += length + 1;
    }
    if whole >= 4 { "UTF-8" } else { "CP949" }.to_owned()
}

/// The column that names a site.
#[must_use]
pub fn name_column(columns: &[(String, ColumnKind)], dataset: &HeritageDataset) -> Option<String> {
    let text = || columns.iter().filter(|(_, kind)| *kind == ColumnKind::Text).map(|(name, _)| name);
    let exact = |wanted: &str| text().find(|name| name.as_str() == wanted);
    let loose = |wanted: &str| text().find(|name| name.trim().to_lowercase() == wanted.to_lowercase());
    dataset
        .name_first
        .and_then(exact)
        .or_else(|| NAME_CANDIDATES.iter().find_map(|wanted| exact(wanted)))
        .or_else(|| NAME_CANDIDATES.iter().find_map(|wanted| loose(wanted)))
        .or_else(|| text().find(|name| name.contains('명')))
        .or_else(|| text().next())
        .cloned()
}

/// Copies the shapes of a received shapefile that touch `scope` (the survey area grown by `NEAR_M`, in
/// `EPSG:{scope.1}`) into the table `TABLE` of a new GeoPackage `out`. No scope: every shape. No shape kept:
/// no file.
///
/// # Errors
/// The file cannot be read as a shapefile with a known CRS, or the copy cannot be written.
pub fn import_shapefile(shp: &Path, dataset: &HeritageDataset, scope: Option<(&Scope, u32)>, out: &Path) -> Result<Imported, String> {
    let encoding = shapefile_encoding(shp);
    let facts = shapefile_facts(shp, &encoding).map_err(|e| e.0)?;
    let epsg = facts.epsg.ok_or_else(|| format!("{}: no known CRS", shp.display()))?;
    let name = name_column(&facts.columns, dataset);
    let code = facts.columns.iter().any(|(column, kind)| column == CODE_SOURCE && *kind == ColumnKind::Text).then_some(CODE_SOURCE);
    // The scope is carried into the CRS of the file: its shapes stay where they are.
    let near = scope.map(|(scope, work_epsg)| scope.reprojected(work_epsg, epsg)).transpose().map_err(|e| e.0)?;
    let touch = near.as_ref().map(Scope::test).transpose().map_err(|e| e.0)?;
    let wanted: Vec<&str> = name.as_deref().into_iter().chain(code).collect();
    let (mut kept, mut failure) = (Vec::new(), None);
    read_shapefile(shp, &encoding, &wanted, &mut |row| {
        let touches = match (&touch, &row.shapes) {
            (None, _) => Ok(true),
            (Some(test), Shapes::Polygons(polygons)) => polygons.iter().try_fold(false, |any, rings| Ok(any || test.touches_polygon(rings)?)),
            (Some(test), Shapes::Lines(lines)) => lines.iter().try_fold(false, |any, line| Ok(any || test.touches_line(line)?)),
        };
        match touches {
            Ok(true) => {
                let mut cells = row.cells.into_iter();
                let text = |cell: Option<Cell>| if let Some(Cell::Text(text)) = cell { Cell::Text(text) } else { Cell::Null };
                let site = if name.is_some() { text(cells.next()) } else { Cell::Null };
                let site_code = if code.is_some() { text(cells.next()) } else { Cell::Null };
                kept.push(Row { shapes: row.shapes, cells: vec![site, site_code] });
            }
            Ok(false) => {}
            Err(e) => failure = Some(strata_engine::EngineError::clone(&e).0),
        }
        failure.is_none()
    })
    .map_err(|e| e.0)?;
    if let Some(reason) = failure {
        return Err(reason);
    }
    let imported = Imported { read: facts.rows, kept: kept.len() as u64, kind: facts.kind, epsg, name_column: name };
    if !kept.is_empty() {
        let layout = TableLayout { table: TABLE, epsg, kind: facts.kind, columns: &[(NAME_COLUMN, ColumnKind::Text), (CODE_COLUMN, ColumnKind::Text)] };
        write_new_table(out, &layout, &mut kept.into_iter()).map_err(|e| e.0)?;
    }
    Ok(imported)
}
