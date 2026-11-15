//! Elevation layers of the legend: 「DEM」 (heights as colours) and 「지형 음영」 (the light on the ground).
//!
//! The C++ gives QGIS a colour ramp for the DEM (`target/cpp-ref/src/core/DemPresentation.cpp:35-39, 66-100`:
//! sixteen stops, linear between them, heights beyond the ends take the end colours, empty cells stay clear,
//! bilinear when zoomed) and a second layer, 「지형 음영」, that multiplies what lies under it by the light
//! on the ground at 30 % (`BasemapOps.cpp:2064-2086`). Here both are cells of colour laid over the map: a
//! black cell whose cover is the missing light, drawn at 30 %, darkens what lies under it by that product.
//! Heights and light come from GDAL (`strata_engine::read_elevation_grid`); the colours are made for every
//! picture, so another colour preset needs no second reading.

use std::sync::Arc;

use strata_engine::ElevationGrid;

use crate::ref_vector::data::{HeightRamp, LoadSpec, Look, RefVectorData};
use crate::ref_vector::raster::{Raster, RasterView};

#[cfg(test)]
#[path = "ref_vector_elevation_tests.rs"]
mod tests;

/// `kStops` (`DemPresentation.cpp:35-39`): metres of preset 「전국 표준 (0–2000m)」 and `0xRRGGBB`.
const STOPS: [(f64, u32); 16] = [
    (0.0, 0x72_c6b5),
    (10.0, 0x81_ceaa),
    (25.0, 0x91_d396),
    (50.0, 0xa5_d680),
    (75.0, 0xba_d975),
    (100.0, 0xd0_dc78),
    (150.0, 0xe0_da7b),
    (200.0, 0xeb_d27a),
    (300.0, 0xed_be70),
    (400.0, 0xe8_a666),
    (600.0, 0xd9_8b5a),
    (800.0, 0xc4_7452),
    (1000.0, 0xad_6650),
    (1300.0, 0xb6_9886),
    (1600.0, 0xd1_c5bb),
    (2000.0, 0xf4_f3ef),
];

/// Colours on a grid of square cells, row by row from the north; premultiplied RGBA.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Cells {
    pub(crate) left: f64,
    pub(crate) top: f64,
    pub(crate) cell: f64,
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) rgba: Vec<[u8; 4]>,
}

/// What a layer draws, once it is read; a copy shares what was read.
#[derive(Clone)]
pub(crate) enum Drawn {
    Shapes(Arc<RefVectorData>),
    Heights(Arc<ElevationGrid>),
}

impl Drawn {
    /// The rows of a table, for the count beside the legend row; an elevation grid has none.
    pub(crate) fn rows(&self) -> Option<u64> {
        match self {
            Self::Shapes(shapes) => Some(shapes.rows),
            Self::Heights(_) => None,
        }
    }

    /// The box of what is drawn in the work CRS; `None` when there is nothing.
    pub(crate) fn extent(&self) -> Option<[f64; 4]> {
        match self {
            Self::Shapes(shapes) => (!shapes.items.is_empty()).then_some(shapes.bbox),
            Self::Heights(grid) => (!grid.heights.is_empty()).then(|| grid.extent()),
        }
    }

    /// The lines and rings of a table that reach into a box of the work CRS (`[min x, min y, max x, max y]`),
    /// as points of the work CRS; nothing for heights. Kept points are single precision around an origin
    /// (ref_vector_data.rs): within a millimetre at the distances a layer covers.
    pub(crate) fn runs_within(&self, within: [f64; 4]) -> Vec<Vec<[f64; 2]>> {
        let Self::Shapes(shapes) = self else {
            return Vec::new();
        };
        let [x, y] = shapes.origin;
        let reaches = |b: &[f32; 4]| x + f64::from(b[0]) <= within[2] && x + f64::from(b[2]) >= within[0] && y + f64::from(b[1]) <= within[3] && y + f64::from(b[3]) >= within[1];
        let runs = shapes.items.iter().filter(|item| reaches(&item.bbox)).flat_map(|item| &shapes.runs[item.first_run as usize..(item.first_run + item.run_count) as usize]);
        runs.map(|run| shapes.coords[run.start as usize..(run.start + run.len) as usize].iter().map(|p| [x + f64::from(p[0]), y + f64::from(p[1])]).collect()).collect()
    }

    /// The lowest and the highest height among the cells whose centres lie in a box of the work CRS
    /// (`[min x, min y, max x, max y]`); `None` for a table and where the box holds no height.
    pub(crate) fn height_range(&self, within: [f64; 4]) -> Option<(f32, f32)> {
        let Self::Heights(grid) = self else {
            return None;
        };
        // The cells whose centres lie between two places, both counted in cells from the grid's edge.
        let span = |from: f64, to: f64, count: usize| {
            #[allow(clippy::cast_precision_loss)]
            let (first, last) = ((from - 0.5).ceil().max(0.0), (to - 0.5).floor().min(count as f64 - 1.0));
            // Whole numbers between 0 and the number of cells.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            (count > 0 && first <= last).then_some(first as usize..=last as usize)
        };
        let columns = span((within[0] - grid.left) / grid.cell, (within[2] - grid.left) / grid.cell, grid.width)?;
        let rows = span((grid.top - within[3]) / grid.cell, (grid.top - within[1]) / grid.cell, grid.height)?;
        let heights = rows.flat_map(|row| grid.heights[row * grid.width..][columns.clone()].iter().copied()).filter(|height| !height.is_nan());
        heights.fold(None, |range: Option<(f32, f32)>, height| Some(range.map_or((height, height), |(low, high)| (low.min(height), high.max(height)))))
    }
}

fn channels(colour: u32) -> [u8; 3] {
    let [_, red, green, blue] = colour.to_be_bytes();
    [red, green, blue]
}

/// The height of a stop under a preset (`DemPresentation.cpp:68-75`).
fn stop_height(index: usize, ramp: HeightRamp) -> f64 {
    let metres = STOPS[index].0;
    match ramp {
        HeightRamp::National => metres,
        // Half of the colours describe 0 to 100 m; the top stays at 2000 m.
        HeightRamp::Lowland if metres <= 200.0 => metres * 0.5,
        HeightRamp::Lowland => 100.0 + (metres - 200.0) * (1900.0 / 1800.0),
        // Sixteen stops spread evenly; the index is at most 15.
        #[allow(clippy::cast_precision_loss)]
        HeightRamp::Range { low, high } => f64::from(low) + (f64::from(high) - f64::from(low)) * index as f64 / (STOPS.len() - 1) as f64,
    }
}

/// The colour of a height in metres, as QGIS's linear colour ramp gives it (`QgsColorRampShader::shade`):
/// a height on a stop or beyond the ends takes that stop's colour; between two stops every channel is the
/// lower stop's plus the whole part of its share of the difference, the share in single precision.
pub(crate) fn height_colour(height: f32, ramp: HeightRamp) -> [u8; 3] {
    let height = f64::from(height);
    let Some(upper) = (0..STOPS.len()).find(|index| height <= stop_height(*index, ramp)) else {
        return channels(STOPS[STOPS.len() - 1].1);
    };
    #[allow(clippy::float_cmp)] // the stop itself, exactly
    if upper == 0 || height == stop_height(upper, ramp) {
        return channels(STOPS[upper].1);
    }
    let (from, to) = (stop_height(upper - 1, ramp), stop_height(upper, ramp));
    // QGIS narrows both differences to single precision before it divides.
    #[allow(clippy::cast_possible_truncation)]
    let share = (height - from) as f32 / (to - from) as f32;
    let (low, high) = (channels(STOPS[upper - 1].1), channels(STOPS[upper].1));
    // The whole part of a number between -255 and 255, added to a channel: the sum stays within 0..=255.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let mix = |a: u8, b: u8| (i32::from(a) + ((f32::from(b) - f32::from(a)) * share) as i32) as u8;
    [mix(low[0], high[0]), mix(low[1], high[1]), mix(low[2], high[2])]
}

/// The cells of an elevation layer: opaque colours of the heights (`Look::Heights`), or black that covers
/// as much as light is missing (`Look::Relief`). A cell without height is clear.
pub(crate) fn cells_of(grid: &ElevationGrid, look: &Look) -> Cells {
    let colour = |(height, light): (&f32, &u8)| match (height.is_nan(), look) {
        (true, _) => [0; 4],
        (false, Look::Heights { ramp }) => {
            let [red, green, blue] = height_colour(*height, *ramp);
            [red, green, blue, 255]
        }
        (false, _) => [0, 0, 0, 255 - *light],
    };
    Cells { left: grid.left, top: grid.top, cell: grid.cell, width: grid.width, height: grid.height, rgba: grid.heights.iter().zip(&grid.shade).map(colour).collect() }
}

/// Reads what the layer draws.
///
/// # Errors
/// As `data::load` for a table; for an elevation layer, when the DEM folder holds no file GDAL can read
/// into the work CRS. The text is for the log, not for the user.
pub(crate) fn load(spec: &LoadSpec) -> Result<Drawn, String> {
    // The light is made when the files are read: the relief names how many times the heights count for it.
    let z_factor = match spec.look {
        Look::Heights { .. } => 1.0,
        Look::Relief { z_factor } => f64::from(z_factor),
        Look::Lines { .. } | Look::Polygons { .. } => return crate::ref_vector::data::load(spec).map(|shapes| Drawn::Shapes(Arc::new(shapes))),
    };
    // One DEM file of the user, or the files the list of a survey's DEM folder names.
    let files = if spec.file.is_file() { vec![spec.file.clone()] } else { crate::topo_download::dem::files_of(&spec.file) };
    strata_engine::read_elevation_grid(&files, spec.work_epsg, z_factor).map(|grid| Drawn::Heights(Arc::new(grid))).map_err(|e| e.0)
}

/// Where the centres of `count` picture pixels lie among the centres of `cells` cells: for each pixel the
/// two cells it lies between and the share of the second; `None` beside the grid. `first` is the place of
/// the first pixel's centre and `step` the way to the next, both in cells from the grid's edge.
fn between(first: f64, step: f64, count: u32, cells: usize) -> Vec<Option<(usize, usize, f32)>> {
    let last = cells.saturating_sub(1);
    let place = |pixel: u32| {
        let at = first + f64::from(pixel) * step - 0.5;
        // Half a cell beyond the outermost centres is still the outermost cell.
        #[allow(clippy::cast_precision_loss)]
        let inside = cells > 0 && at >= -0.5 && at <= last as f64 + 0.5;
        // Inside the grid `at` is between -0.5 and the cell count: its floor is a small whole number.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        inside.then(|| {
            let below = at.floor();
            let near = if below < 0.0 { 0 } else { (below as usize).min(last) };
            (near, if below < 0.0 { 0 } else { (near + 1).min(last) }, (at - below) as f32)
        })
    };
    (0..count).map(place).collect()
}

/// Lays the cells into the picture of a view: bilinear between the centres of the cells.
fn laid(cells: &Cells, view: RasterView) -> Option<Raster> {
    if !view.metres_per_pixel.is_finite() || view.metres_per_pixel <= 0.0 || view.width == 0 || view.height == 0 {
        return None;
    }
    let step = view.metres_per_pixel / cells.cell;
    let across = between((view.left - cells.left) / cells.cell + step / 2.0, step, view.width, cells.width);
    let down = between((cells.top - view.top) / cells.cell + step / 2.0, step, view.height, cells.height);
    let mut rgba = vec![0_u8; view.width as usize * view.height as usize * 4];
    let mix = |a: u8, b: u8, share: f32| f32::from(a) + (f32::from(b) - f32::from(a)) * share;
    for (row, out) in down.iter().zip(rgba.chunks_exact_mut(view.width as usize * 4)) {
        let Some((north, south, share_down)) = *row else {
            continue;
        };
        let (upper, lower) = (&cells.rgba[north * cells.width..][..cells.width], &cells.rgba[south * cells.width..][..cells.width]);
        for (column, pixel) in across.iter().zip(out.chunks_exact_mut(4)) {
            let Some((west, east, share_across)) = *column else {
                continue;
            };
            let corners = [upper[west], upper[east], lower[west], lower[east]];
            if corners.iter().all(|corner| corner[3] == 0) {
                continue;
            }
            for (channel, value) in pixel.iter_mut().enumerate() {
                let (above, below) = (mix(corners[0][channel], corners[1][channel], share_across), mix(corners[2][channel], corners[3][channel], share_across));
                // A mean of bytes, rounded: between 0 and 255.
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let mean = (above + (below - above) * share_down).round() as u8;
                *value = mean;
            }
        }
    }
    Some(Raster { view, rgba })
}

/// The picture of the layer for the view, in the look it has now. `None` when the view has no pixels or no
/// scale.
pub(crate) fn render(data: &Drawn, look: &Look, view: RasterView) -> Option<Raster> {
    match data {
        Drawn::Shapes(shapes) => crate::ref_vector::raster::render(shapes, view),
        Drawn::Heights(grid) => laid(&cells_of(grid, look), view),
    }
}
