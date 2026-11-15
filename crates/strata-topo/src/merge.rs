//! The lines of the placed sheets that touch the area around the survey, in one table for the map.
//!
//! C++ `KaTopographicImportDialog.cpp` (updateCoverage `:659`, loadNext `:787`, publishPrepared `:871`) and
//! `SurveyScopeClip.cpp`: the lines of contours, roads, buildings, water and boundaries are carried into the
//! work CRS, a line is kept WHOLE when any part of it lies within the buffer around the survey areas (it is
//! not cut), and all kept lines go into one GeoPackage table `topo` with the fields `Layer`, `elevation` and
//! `sheet`. The C++ has two more fields: `Text` (empty on lines) and `z_valid` (here: `elevation` is null
//! when the line has no single height).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use strata_engine::{append_to_table, read_dxf_lines, write_new_table, Cell, ColumnKind, Row, Scope, ScopeTest, ShapeKind, Shapes, TableLayout, Transformer};

#[cfg(test)]
#[path = "merge_tests.rs"]
mod tests;

/// The table of the merged file (`KaTopographicImportDialog.cpp:157`).
pub const TABLE: &str = "topo";
/// Its columns: the CAD layer (the feature code), the height of a contour, the sheet number.
pub const LAYER_COLUMN: &str = "Layer";
pub const ELEVATION_COLUMN: &str = "elevation";
pub const SHEET_COLUMN: &str = "sheet";

/// `KaTopographicImportDialog.cpp:851`.
pub const NOT_OPENED: &str = "받은 수치지형도 레이어를 열지 못했습니다";

/// One placed sheet and the CAD layers of it that go on the map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetLines {
    pub dxf: PathBuf,
    pub number: String,
    /// The belt the sheet is drawn in (`sheet::place`).
    pub epsg: u32,
    pub shown: BTreeSet<String>,
}

/// Why nothing was merged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeStop {
    Cancelled,
    /// The C++ sentence, with the sheet it is about.
    Failed(String),
}

/// Writes the lines of `sheets` that touch `scope` (in the work CRS) into the new file `out`, table `topo`,
/// in the work CRS. Returns how many lines the table holds; with none, no file is written.
///
/// # Errors
/// The user stopped it, a sheet cannot be read or carried over, or the file cannot be written. Nothing is
/// left at `out` then.
pub fn merge(sheets: &[SheetLines], scope: &Scope, work_epsg: u32, out: &Path, cancelled: &dyn Fn() -> bool) -> Result<u64, MergeStop> {
    let merged = merge_into(sheets, scope, work_epsg, out, cancelled);
    if merged.is_err() {
        // Whatever was written before the stop is not a map.
        let _ = std::fs::remove_file(out);
    }
    merged
}

fn merge_into(sheets: &[SheetLines], scope: &Scope, work_epsg: u32, out: &Path, cancelled: &dyn Fn() -> bool) -> Result<u64, MergeStop> {
    let layout = TableLayout { table: TABLE, epsg: work_epsg, kind: ShapeKind::Lines, columns: &[(LAYER_COLUMN, ColumnKind::Text), (ELEVATION_COLUMN, ColumnKind::Real), (SHEET_COLUMN, ColumnKind::Text)] };
    let touch = scope.test().map_err(|e| MergeStop::Failed(e.0))?;
    let mut stored = 0;
    for sheet in sheets {
        if cancelled() {
            return Err(MergeStop::Cancelled);
        }
        let rows = kept_lines(sheet, &touch, work_epsg, cancelled)?;
        if rows.is_empty() {
            continue;
        }
        let not_written = |_| MergeStop::Failed("수치지형도 레이어를 지도에 추가하지 못했습니다".to_owned());
        stored = if stored == 0 { write_new_table(out, &layout, &mut rows.into_iter()).map_err(not_written)? } else { append_to_table(out, TABLE, &mut rows.into_iter()).map_err(not_written)? };
    }
    Ok(stored)
}

/// The lines of one sheet that go on the map: of a shown layer, carried into the work CRS, touching the scope.
fn kept_lines(sheet: &SheetLines, touch: &ScopeTest<'_>, work_epsg: u32, cancelled: &dyn Fn() -> bool) -> Result<Vec<Row>, MergeStop> {
    let not_opened = || MergeStop::Failed(format!("{NOT_OPENED}: {}. 기존 지도와 받은 원본은 유지됩니다.", sheet.number));
    let carry = if sheet.epsg == work_epsg { None } else { Some(Transformer::new(sheet.epsg, work_epsg).map_err(|_| not_opened())?) };
    let (mut rows, mut stop, mut seen) = (Vec::new(), None, 0_u64);
    let read = read_dxf_lines(&sheet.dxf, &mut |line| {
        seen += 1;
        if seen % 1024 == 0 && cancelled() {
            stop = Some(MergeStop::Cancelled);
            return false;
        }
        if line.points.len() < 2 || !sheet.shown.contains(&line.layer) {
            return true;
        }
        let carried = match &carry {
            None => Ok(line.points),
            Some(carry) => carry.transform_all(&line.points),
        };
        // A line that cannot be carried or tested would be a hole in the map nobody knows of: the sheet fails.
        let (Ok(points), true) = (&carried, carried.as_ref().is_ok_and(|points| points.iter().flatten().all(|v| v.is_finite()))) else {
            stop = Some(not_opened());
            return false;
        };
        match touch.touches_line(points) {
            Ok(true) => rows.push(Row { shapes: Shapes::Lines(vec![points.clone()]), cells: vec![Cell::Text(line.layer), line.elevation.map_or(Cell::Null, Cell::Real), Cell::Text(sheet.number.clone())] }),
            Ok(false) => {}
            Err(_) => {
                stop = Some(not_opened());
                return false;
            }
        }
        true
    });
    match (stop, read) {
        (Some(stop), _) => Err(stop),
        (None, Err(_)) => Err(not_opened()),
        (None, Ok(_)) if cancelled() => Err(MergeStop::Cancelled),
        (None, Ok(_)) => Ok(rows),
    }
}
