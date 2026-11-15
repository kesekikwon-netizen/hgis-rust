//! One received sheet: what its CAD layers hold, where the sheet lies, and which of its layers go on the
//! map (C++ `TopographicCatalog.cpp:181-209`, `:351-364`; `KaTopographicScopePanel.cpp` prepareOne
//! `:215-304`).

use std::collections::BTreeSet;
use std::path::Path;

use strata_engine::{dxf_layer_extents, read_dxf_lines, DxfShape};

use crate::codes::{infer_code, Category};
use crate::source_crs::{resolve, LayerBox, FRAME_LAYER};

#[cfg(test)]
#[path = "sheet_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "test_dxf.rs"]
pub(crate) mod test_dxf;

/// `KaTopographicScopePanel.cpp:270`.
pub const NOTHING_PLACED: &str = "도엽 위치가 확인된 지도 항목이 없습니다.";

/// The entities of one CAD layer and one kind of shape.
#[derive(Debug, Clone, PartialEq)]
pub struct SheetLayer {
    pub cad_layer: String,
    pub shape: DxfShape,
    pub count: u64,
    pub category: Category,
    /// The name of its code in the code book, or 「미분류 · <layer>」 (`TopographicCatalog.cpp:199-201`).
    pub name: String,
    /// `[min x, min y, max x, max y]` in the sheet's own coordinates.
    pub extent: [f64; 4],
}

/// What a sheet holds.
#[derive(Debug, Clone, PartialEq)]
pub struct Scanned {
    /// In the order of CAD layer, then kind of shape.
    pub layers: Vec<SheetLayer>,
    /// The lines on the frame's layer (H0017334).
    pub frame: Vec<Vec<[f64; 2]>>,
}

/// A sheet whose belt was found.
#[derive(Debug, Clone, PartialEq)]
pub struct Placed {
    pub epsg: u32,
    /// What the choice rests on.
    pub evidence: String,
    /// What the user is told: a frame that placed the sheet, layers that were left off.
    pub warnings: Vec<String>,
    /// The CAD layers whose lines go on the map: lines of contours, roads, buildings, water and boundaries
    /// that lie inside the sheet (`visibleAtScale`, `TopographicCatalog.cpp:351-364`).
    pub shown: BTreeSet<String>,
}

/// Reads the layers and the frame of a sheet.
///
/// # Errors
/// The file is no DXF drawing or cannot be read; the text is for the log.
pub fn scan(dxf: &Path) -> Result<Scanned, String> {
    let layers = dxf_layer_extents(dxf).map_err(|e| e.0)?;
    let layers = layers
        .into_iter()
        .map(|found| {
            let code = infer_code(&found.layer);
            let name = code.as_ref().map_or_else(|| format!("{} · {}", Category::Unknown.name(), found.layer), |info| info.name.clone());
            SheetLayer { category: code.map_or(Category::Unknown, |info| info.category), name, cad_layer: found.layer, shape: found.shape, count: found.count, extent: found.extent }
        })
        .collect();
    let mut frame = Vec::new();
    read_dxf_lines(dxf, &mut |line| {
        if line.layer == FRAME_LAYER {
            frame.push(line.points);
        }
        true
    })
    .map_err(|e| e.0)?;
    Ok(Scanned { layers, frame })
}

impl Scanned {
    /// The layers as the belt solver takes them.
    #[must_use]
    pub fn boxes(&self) -> Vec<LayerBox> {
        self.layers.iter().map(|layer| LayerBox { category: layer.category, extent: Some(layer.extent) }).collect()
    }
}

/// Finds the belt of a scanned sheet and the layers that go on the map. `number`, `projection` and
/// `official_5179` are the portal's record of the sheet.
///
/// # Errors
/// The C++ sentence that says why the sheet is not put on the map.
pub fn place(number: &str, projection: &str, scanned: &Scanned, official_5179: Option<[f64; 4]>) -> Result<Placed, String> {
    let resolved = resolve(number, projection, &scanned.boxes(), &scanned.frame, official_5179)?;
    let mut warnings: Vec<String> = resolved.warning.into_iter().collect();
    // A layer that does not lie inside the sheet is kept off the map and named (`:262-266`).
    warnings.extend(resolved.unverified.iter().filter_map(|i| scanned.layers.get(*i)).map(|layer| format!("도곽 밖 또는 범위 미확인 항목: {}", layer.name)));
    let inside: Vec<&SheetLayer> = scanned.layers.iter().enumerate().filter(|(i, _)| !resolved.unverified.contains(i)).map(|(_, layer)| layer).collect();
    if inside.is_empty() {
        return Err(NOTHING_PLACED.to_owned());
    }
    let shown = inside.into_iter().filter(|layer| layer.shape == DxfShape::Line && layer.category.is_shown()).map(|layer| layer.cad_layer.clone()).collect();
    Ok(Placed { epsg: resolved.epsg, evidence: resolved.evidence, warnings, shown })
}
