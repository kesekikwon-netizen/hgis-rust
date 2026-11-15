//! A reference table of the survey folder, read for drawing.
//!
//! C++ hands such a file to QGIS as a layer (`target/cpp-ref/src/core/GeologyMapService.cpp:924`,
//! `src/app/KaTopographicImportDialog.cpp:186`), which reprojects it on the fly. Here every point is carried
//! into the work CRS once, through PROJ, and kept as a single-precision offset from an origin near the data:
//! half the memory of f64, and at most 2 mm off over 30 km (one unit in the last place of an f32 below 32768).

use std::collections::HashMap;
use std::path::PathBuf;

use strata_engine::{read_table, table_facts, Cell, ShapeKind, Shapes, Transformer};

#[cfg(test)]
#[path = "ref_vector_data_tests.rs"]
mod tests;

/// How a shape is drawn. A fill with alpha 0 is no fill.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PaintOf {
    pub(crate) fill: [u8; 4],
    pub(crate) stroke: [u8; 4],
    pub(crate) width_mm: f32,
}

/// Where the fill of a polygon comes from.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PolygonFill {
    None,
    /// A text column holding `#RRGGBB`; `alpha` is the layer's fill opacity, `missing` the colour of a row
    /// without a readable one.
    Column { name: &'static str, alpha: u8, missing: [u8; 3] },
}

/// How a layer is drawn.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Look {
    Lines { stroke: [u8; 4], width_mm: f32 },
    Polygons { stroke: [u8; 4], width_mm: f32, fill: PolygonFill },
}

/// Which table to read, into which CRS, and how it will be drawn.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LoadSpec {
    pub(crate) file: PathBuf,
    pub(crate) table: String,
    pub(crate) work_epsg: u32,
    pub(crate) look: Look,
}

/// A run of points in `RefVectorData::coords`: one line, or one ring of a polygon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Run {
    pub(crate) start: u32,
    pub(crate) len: u32,
}

/// One shape: a line (one run) or a polygon (its rings, outer first).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Item {
    pub(crate) first_run: u32,
    pub(crate) run_count: u32,
    /// `[min x, min y, max x, max y]` as offsets from the origin.
    pub(crate) bbox: [f32; 4],
    pub(crate) paint: u16,
}

/// A layer ready to draw. Coordinates are metres of the work CRS: `origin + offset`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RefVectorData {
    pub(crate) origin: [f64; 2],
    pub(crate) bbox: [f64; 4],
    pub(crate) coords: Vec<[f32; 2]>,
    pub(crate) runs: Vec<Run>,
    pub(crate) items: Vec<Item>,
    pub(crate) paints: Vec<PaintOf>,
    /// Polygons (runs are rings) or lines.
    pub(crate) closed: bool,
    /// Rows of the table.
    pub(crate) rows: u64,
}

/// `#RRGGBB`.
fn colour(text: &str) -> Option<[u8; 3]> {
    let hex = text.strip_prefix('#').filter(|h| h.len() == 6 && h.is_ascii())?;
    let part = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    Some([part(0)?, part(2)?, part(4)?])
}

struct Build {
    data: RefVectorData,
    carry: Option<Transformer>,
    paint_of: HashMap<([u8; 4], [u8; 4], u32), u16>,
}

impl Build {
    fn paint(&mut self, paint: PaintOf) -> Result<u16, String> {
        let key = (paint.fill, paint.stroke, paint.width_mm.to_bits());
        if let Some(index) = self.paint_of.get(&key) {
            return Ok(*index);
        }
        let index = u16::try_from(self.data.paints.len()).map_err(|_| "more looks than a layer can hold".to_owned())?;
        self.data.paints.push(paint);
        self.paint_of.insert(key, index);
        Ok(index)
    }

    /// Adds one shape made of these runs; a run too short to draw is left out.
    fn item(&mut self, runs: &[Vec<[f64; 2]>], least: usize, paint: PaintOf) -> Result<(), String> {
        let first_run = u32::try_from(self.data.runs.len()).map_err(|_| "too many lines".to_owned())?;
        let mut bbox = [f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY];
        for run in runs.iter().filter(|r| r.len() >= least) {
            let points = match &self.carry {
                Some(carry) => carry.transform_all(run).map_err(|e| e.0)?,
                None => run.clone(),
            };
            if self.data.coords.is_empty() {
                self.data.origin = [(points[0][0] / 1000.0).floor() * 1000.0, (points[0][1] / 1000.0).floor() * 1000.0];
            }
            let start = u32::try_from(self.data.coords.len()).map_err(|_| "too many points".to_owned())?;
            let len = u32::try_from(points.len()).map_err(|_| "too many points".to_owned())?;
            for p in &points {
                let b = &mut self.data.bbox;
                *b = [b[0].min(p[0]), b[1].min(p[1]), b[2].max(p[0]), b[3].max(p[1])];
                // Deliberately narrowed: see the note on precision at the top of this file.
                #[allow(clippy::cast_possible_truncation)]
                let offset = [(p[0] - self.data.origin[0]) as f32, (p[1] - self.data.origin[1]) as f32];
                bbox = [bbox[0].min(offset[0]), bbox[1].min(offset[1]), bbox[2].max(offset[0]), bbox[3].max(offset[1])];
                self.data.coords.push(offset);
            }
            self.data.runs.push(Run { start, len });
        }
        let run_count = u32::try_from(self.data.runs.len()).map_err(|_| "too many lines".to_owned())? - first_run;
        if run_count > 0 {
            let paint = self.paint(paint)?;
            self.data.items.push(Item { first_run, run_count, bbox, paint });
        }
        Ok(())
    }
}

/// Reads the table for drawing.
///
/// # Errors
/// The file or table is missing, holds the other kind of shape or no known CRS, lacks the colour column, or
/// PROJ cannot carry its points into the work CRS. The text is for the log, not for the user.
pub(crate) fn load(spec: &LoadSpec) -> Result<RefVectorData, String> {
    let facts = table_facts(&spec.file, &spec.table).map_err(|e| e.0)?;
    let (wanted, columns): (ShapeKind, Vec<&str>) = match &spec.look {
        Look::Lines { .. } => (ShapeKind::Lines, Vec::new()),
        Look::Polygons { fill: PolygonFill::Column { name, .. }, .. } => (ShapeKind::Polygons, vec![name]),
        Look::Polygons { fill: PolygonFill::None, .. } => (ShapeKind::Polygons, Vec::new()),
    };
    if facts.kind != wanted {
        return Err(format!("table {} holds another kind of shape than its layer draws", spec.table));
    }
    let epsg = facts.epsg.ok_or_else(|| format!("table {} names no CRS", spec.table))?;
    // The work CRS is checked even when the table is in it already: an unknown CRS is an error, never a pass-through.
    let carry = Some(Transformer::new(epsg, spec.work_epsg).map_err(|e| e.0)?).filter(|_| epsg != spec.work_epsg);
    let empty = RefVectorData { origin: [0.0; 2], bbox: [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY], coords: Vec::new(), runs: Vec::new(), items: Vec::new(), paints: Vec::new(), closed: wanted == ShapeKind::Polygons, rows: facts.rows };
    let mut build = Build { data: empty, carry, paint_of: HashMap::new() };
    let mut failure = None;
    let mut add = |row: strata_engine::Row| -> Result<(), String> {
        match (&spec.look, row.shapes) {
            (Look::Lines { stroke, width_mm }, Shapes::Lines(lines)) => {
                let paint = PaintOf { fill: [0; 4], stroke: *stroke, width_mm: *width_mm };
                lines.into_iter().try_for_each(|line| build.item(&[line], 2, paint))
            }
            (Look::Polygons { stroke, width_mm, fill }, Shapes::Polygons(polygons)) => {
                let fill = match fill {
                    PolygonFill::None => [0; 4],
                    PolygonFill::Column { alpha, missing, .. } => {
                        let [r, g, b] = match row.cells.first() {
                            Some(Cell::Text(text)) => colour(text).unwrap_or(*missing),
                            _ => *missing,
                        };
                        [r, g, b, *alpha]
                    }
                };
                let paint = PaintOf { fill, stroke: *stroke, width_mm: *width_mm };
                polygons.iter().try_for_each(|rings| build.item(rings, 4, paint))
            }
            _ => Err("a row holds another kind of shape than its table".to_owned()),
        }
    };
    read_table(&spec.file, &spec.table, &columns, &mut |row| match add(row) {
        Ok(()) => true,
        Err(e) => {
            failure = Some(e);
            false
        }
    })
    .map_err(|e| e.0)?;
    match failure {
        Some(e) => Err(e),
        None => Ok(build.data),
    }
}
