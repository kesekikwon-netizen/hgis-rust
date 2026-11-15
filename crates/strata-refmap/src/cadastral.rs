//! The digital cadastral map (수치지적도) near the survey area: the import of the county shapefiles the 지적
//! button receives into one table of the survey folder (`target/cpp-ref/src/core/CadastralImport.cpp`,
//! `CadastralEncoding.cpp`). The words and the order of the steps are the C++ ones.
//!
//! Not as the C++: the name of the finished file. The C++ makes it from bytes only QGIS writes (the WKB of
//! its buffer, its WKT of the CRS, its transform settings, `:87-96`), so the two apps do not find each
//! other's table; each writes its own beside the other, from the same received files.

use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use sha2::{Digest, Sha256};
use strata_engine::{polygons_area, read_shapefile_within, shapefile_facts, table_facts, write_new_table, Cell, ColumnKind, Row, Scope, ScopeTest, ShapeKind, Shapes, TableLayout, Transformer};

#[path = "cadastral_encoding.rs"]
mod encoding;
#[path = "cadastral_portal.rs"]
pub mod portal;
#[cfg(test)]
#[path = "cadastral_encoding_tests.rs"]
mod encoding_tests;
#[cfg(test)]
#[path = "cadastral_tests.rs"]
mod tests;

pub use encoding::{dbf_encoding, fallback_encoding};

/// The table of the finished file (`:75`) and its two columns (`:136-137`).
pub const TABLE: &str = "cadastral";
pub const PNU_COLUMN: &str = "PNU";
pub const JIBUN_COLUMN: &str = "JIBUN";

const NEEDS: &str = "지적도 원본과 유효한 조사 범위·좌표계가 필요합니다.";
const UNREADABLE: &str = "지적도 원본 파일을 읽지 못했습니다.";
const SHAPES: &str = "지적도 도형 또는 원본 좌표계를 확인하지 못했습니다.";
const NO_PARCELS: &str = "조사 주변 범위와 겹치는 필지가 없습니다.";
const NOT_CARRIED: &str = "필지 좌표를 조사 좌표계로 변환하지 못했습니다.";
/// A lock older than this was left by a run that died (the default of `QLockFile`, `:117-118`).
const STALE_LOCK: Duration = Duration::from_secs(30);

/// The finished table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ready {
    pub file: PathBuf,
    pub parcels: u64,
    /// The table of an earlier run was found and used as it is.
    pub reused: bool,
}

/// Why there is no table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stopped {
    Cancelled,
    Failed(String),
}

fn failed(words: &str) -> Stopped {
    Stopped::Failed(words.to_owned())
}

/// The lock beside a table while it is made; it goes when the run ends, however it ends.
struct Lock(PathBuf);

impl Lock {
    fn take(table: &Path) -> Option<Self> {
        let path = PathBuf::from(format!("{}.lock", table.display()));
        let age = std::fs::metadata(&path).and_then(|file| file.modified()).ok().and_then(|at| at.elapsed().ok());
        if age.is_some_and(|age| age > STALE_LOCK) {
            let _ = std::fs::remove_file(&path);
        }
        std::fs::OpenOptions::new().write(true).create_new(true).open(&path).ok().map(|_| Self(path))
    }
}

impl Drop for Lock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// The SHA-256 of a file in hexadecimal, read a megabyte at a time (`hashFile`, `:36-47`).
fn digest_of(path: &Path, stop: &dyn Fn() -> bool) -> Result<String, Stopped> {
    let mut file = std::fs::File::open(path).map_err(|_| failed(UNREADABLE))?;
    let (mut hash, mut piece) = (Sha256::new(), vec![0_u8; 1024 * 1024]);
    loop {
        if stop() {
            return Err(Stopped::Cancelled);
        }
        match file.read(&mut piece).map_err(|_| failed(UNREADABLE))? {
            0 => break,
            read => hash.update(&piece[..read]),
        }
    }
    Ok(hash.finalize().iter().map(|byte| format!("{byte:02x}")).collect())
}

/// What a table is named by: the scope, the work CRS, and every received file with its content (`:87-114`;
/// a shapefile with the files beside it, `:100-107`).
fn identity(ordered: &[&PathBuf], scope: &Scope, work_epsg: u32, stop: &dyn Fn() -> bool) -> Result<String, Stopped> {
    let mut identity = Sha256::new();
    identity.update(b"strata-cadastral-scope-v1");
    for polygon in scope.polygons().map_err(|_| failed(NEEDS))? {
        for ring in polygon {
            for [x, y] in ring {
                identity.update(x.to_le_bytes());
                identity.update(y.to_le_bytes());
            }
            identity.update(b"|");
        }
    }
    identity.update(format!("EPSG:{work_epsg}").as_bytes());
    for source in ordered {
        let beside = ["dbf", "shx", "prj", "cpg"].into_iter().filter_map(|extension| encoding::sibling(source, extension));
        let shapefile = source.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("shp"));
        for file in std::iter::once((*source).clone()).chain(beside.filter(|_| shapefile)) {
            let digest = digest_of(&file, stop)?;
            identity.update(file.to_string_lossy().as_bytes());
            identity.update(digest.as_bytes());
        }
    }
    Ok(identity.finalize().iter().map(|byte| format!("{byte:02x}")).collect())
}

/// The parcels of a finished table that is whole (its digest beside it is its own) and is this work CRS's
/// table of lot numbers (`:120-131`).
fn finished(table: &Path, work_epsg: u32, stop: &dyn Fn() -> bool) -> Result<Option<u64>, Stopped> {
    let Ok(said) = std::fs::read_to_string(format!("{}.sha256", table.display())) else {
        return Ok(None);
    };
    if !table.exists() || said.trim() != digest_of(table, stop)? {
        return Ok(None);
    }
    Ok(table_facts(table, TABLE).ok().filter(|facts| facts.epsg == Some(work_epsg) && facts.rows > 0 && facts.columns.iter().any(|(column, _)| column == JIBUN_COLUMN)).map(|facts| facts.rows))
}

/// Same-PNU rings that are the same share their box to the millimetre, their area and their number of
/// corners; the pieces of one PNU almost always differ in their box (`cheapParcelKey`, `:49-63`).
fn parcel_key(id: &str, polygons: &[Vec<Vec<[f64; 2]>>]) -> String {
    let points = || polygons.iter().flatten().flatten();
    let edge = |axis: usize, most: bool| points().map(|point| point[axis]).fold(if most { f64::MIN } else { f64::MAX }, |kept, value| if most { kept.max(value) } else { kept.min(value) });
    // Millimetres of a national grid and thousandths of a parcel's square metres fit an i64 many times over.
    #[allow(clippy::cast_possible_truncation)]
    let whole = |value: f64| (value * 1000.0).round() as i64;
    format!("{id}:{}:{}:{}:{}:{}:{}", whole(edge(0, false)), whole(edge(1, false)), whole(edge(0, true)), whole(edge(1, true)), whole(polygons_area(polygons).unwrap_or(0.0)), points().count())
}

fn meets(test: &ScopeTest<'_>, polygons: &[Vec<Vec<[f64; 2]>>]) -> Result<bool, strata_engine::EngineError> {
    polygons.iter().try_fold(false, |any, rings| Ok(any || test.touches_polygon(rings)?))
}

/// The parcels of the sources that meet the scope, each once, in the work CRS (`:146-199`).
fn gather(ordered: &[&PathBuf], scope: &Scope, work_epsg: u32, stop: &dyn Fn() -> bool, progress: &mut dyn FnMut(u8, &str)) -> Result<Vec<Row>, Stopped> {
    let (test, bbox) = (scope.test().map_err(|_| failed(NEEDS))?, scope.bbox().map_err(|_| failed(NEEDS))?);
    let (mut seen, mut rows) = (HashSet::new(), Vec::new());
    for (index, source) in ordered.iter().enumerate() {
        if stop() {
            return Err(Stopped::Cancelled);
        }
        progress(u8::try_from(index * 90 / ordered.len()).unwrap_or(90), "조사 주변 필지와 지번을 정리하고 있습니다…");
        // No `.cpg`: the lot numbers are read as CP949, as UTF-8 only when the `.dbf` clearly is (`:160-162`).
        let encoding = fallback_encoding(source).map_or_else(|| crate::heritage::shapefile_encoding(source), str::to_owned);
        let facts = shapefile_facts(source, &encoding).map_err(|_| failed(SHAPES))?;
        let (Some(epsg), ShapeKind::Polygons) = (facts.epsg, facts.kind) else {
            return Err(failed(SHAPES));
        };
        let column = |name: &str| facts.columns.iter().find(|(column, kind)| column.eq_ignore_ascii_case(name) && *kind == ColumnKind::Text).map(|(column, _)| column.as_str());
        let Some(jibun) = column(JIBUN_COLUMN) else {
            return Err(failed("지적도 원본에 지번(JIBUN) 항목이 없습니다."));
        };
        let wanted: Vec<&str> = std::iter::once(jibun).chain(column(PNU_COLUMN)).collect();
        let to_work = (epsg != work_epsg).then(|| Transformer::new(epsg, work_epsg)).transpose().map_err(|_| failed(SHAPES))?;
        let rect = if to_work.is_some() { Transformer::new(work_epsg, epsg).and_then(|to_source| to_source.transform_box(bbox)).map_err(|_| failed(NOT_CARRIED))? } else { bbox };
        let mut ended = None;
        read_shapefile_within(source, &encoding, &wanted, rect, &mut |row| {
            if stop() {
                ended = Some(Stopped::Cancelled);
                return false;
            }
            let Shapes::Polygons(mut polygons) = row.shapes else { return true };
            if polygons.is_empty() {
                return true;
            }
            if let Some(to_work) = &to_work {
                for ring in polygons.iter_mut().flatten() {
                    match to_work.transform_all(ring) {
                        Ok(carried) => *ring = carried,
                        Err(_) => {
                            ended = Some(failed(NOT_CARRIED));
                            return false;
                        }
                    }
                }
            }
            match meets(&test, &polygons) {
                Ok(true) => {}
                Ok(false) => return true,
                Err(_) => {
                    ended = Some(failed(SHAPES));
                    return false;
                }
            }
            let text = |cell: Option<&Cell>| if let Some(Cell::Text(text)) = cell { text.clone() } else { String::new() };
            let (lot, id) = (text(row.cells.first()), text(row.cells.get(1)).trim().to_owned());
            // One PNU may have several pieces; the same piece from a second file is the same parcel.
            if seen.insert(parcel_key(&id, &polygons)) {
                rows.push(Row { shapes: Shapes::Polygons(polygons), cells: vec![Cell::Text(id), Cell::Text(lot)] });
            }
            true
        })
        .map_err(|_| failed(SHAPES))?;
        if let Some(stopped) = ended {
            return Err(stopped);
        }
    }
    Ok(rows)
}

/// Writes the parcels of `sources` (county shapefiles) that meet `scope` into one table under `directory`,
/// whole, in the work CRS, with their PNU and lot number and nothing else (`CadastralImport::prepare`,
/// `:66-242`). The same sources and scope find the finished table again. `scope` is in the work CRS.
///
/// # Errors
/// See [`Stopped`]; the words of a failure are the C++ ones.
pub fn prepare(sources: &[PathBuf], scope: Option<&Scope>, work_epsg: u32, directory: &Path, stop: &dyn Fn() -> bool, progress: &mut dyn FnMut(u8, &str)) -> Result<Ready, Stopped> {
    if stop() {
        return Err(Stopped::Cancelled);
    }
    let (Some(scope), false, true) = (scope, sources.is_empty(), Transformer::new(work_epsg, 4326).is_ok()) else {
        return Err(failed(NEEDS));
    };
    std::fs::create_dir_all(directory).map_err(|_| failed("지적도 표시 파일을 저장할 폴더를 만들지 못했습니다."))?;
    let mut ordered: Vec<&PathBuf> = sources.iter().collect();
    ordered.sort();
    ordered.dedup();
    let file = directory.join(format!("{}.gpkg", identity(&ordered, scope, work_epsg, stop)?));
    let _lock = Lock::take(&file).ok_or_else(|| failed("같은 범위의 지적도를 준비 중입니다."))?;
    if let Some(parcels) = finished(&file, work_epsg, stop)? {
        progress(100, "저장된 조사 주변 지적도를 사용합니다.");
        return Ok(Ready { file, parcels, reused: true });
    }
    let staging = tempfile::Builder::new().prefix(".cadastral-").tempdir_in(directory).map_err(|_| failed("지적도 임시 파일을 준비하지 못했습니다."))?;
    let draft = staging.path().join("cadastral.gpkg");
    let rows = gather(&ordered, scope, work_epsg, stop, progress)?;
    if stop() {
        return Err(Stopped::Cancelled);
    }
    let parcels = rows.len() as u64;
    if parcels == 0 {
        return Err(failed(NO_PARCELS));
    }
    let layout = TableLayout { table: TABLE, epsg: work_epsg, kind: ShapeKind::Polygons, columns: &[(PNU_COLUMN, ColumnKind::Text), (JIBUN_COLUMN, ColumnKind::Text)] };
    let stored = write_new_table(&draft, &layout, &mut rows.into_iter()).map_err(|_| failed("지적도 저장을 완료하지 못했습니다."))?;
    if stored != parcels || table_facts(&draft, TABLE).ok().and_then(|facts| facts.epsg) != Some(work_epsg) {
        return Err(failed("저장한 지적도 도형 수와 좌표계를 확인하지 못했습니다."));
    }
    if stop() {
        return Err(Stopped::Cancelled);
    }
    // The finished draft takes the place of whatever stood there (a table whose digest did not hold).
    let _ = std::fs::remove_file(&file);
    std::fs::rename(&draft, &file).map_err(|_| failed("완료한 지적도를 저장 위치로 옮기지 못했습니다."))?;
    if let Ok(digest) = digest_of(&file, &|| false) {
        let _ = std::fs::write(format!("{}.sha256", file.display()), digest);
    }
    progress(100, &format!("주변 필지 {parcels}개를 준비했습니다."));
    Ok(Ready { file, parcels, reused: false })
}
