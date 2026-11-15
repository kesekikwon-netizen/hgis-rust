//! Elevation layers of the legend: 「DEM」 (heights as colours) and 「지형 음영」 (the light on the ground).
//!
//! The C++ gives QGIS a colour ramp for the DEM (`target/cpp-ref/src/core/DemPresentation.cpp:35-39, 66-100`:
//! sixteen stops, linear between them, heights beyond the ends take the end colours, empty cells stay clear,
//! bilinear when zoomed) and a second layer, 「지형 음영」, that multiplies what lies under it by the light
//! on the ground at 30 % (`BasemapOps.cpp:2064-2086`). Here both are cells of colour laid over the map: a
//! black cell whose cover is the missing light, drawn at 30 %, darkens what lies under it by that product.
//! Heights and light come from GDAL (`strata_engine::read_elevation_grid`).

use std::sync::Arc;

use strata_engine::ElevationGrid;

use crate::ref_vector::data::{LoadSpec, Look, RefVectorData};
use crate::ref_vector::raster::{Raster, RasterView};

#[cfg(test)]
#[path = "ref_vector_elevation_tests.rs"]
mod tests;

/// `kStops` (`DemPresentation.cpp:35-39`), preset 「전국 표준 (0–2000m)」: metres and `0xRRGGBB`.
const STOPS: [(f32, u32); 16] = [
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
    Cells(Arc<Cells>),
}

impl Drawn {
    /// The rows of a table, for the count beside the legend row; cells have none.
    pub(crate) fn rows(&self) -> Option<u64> {
        match self {
            Self::Shapes(shapes) => Some(shapes.rows),
            Self::Cells(_) => None,
        }
    }

    /// The box of what is drawn in the work CRS; `None` when there is nothing.
    // A grid has at most some ten thousand cells on a side: f64 holds that exactly.
    #[allow(clippy::cast_precision_loss)]
    pub(crate) fn extent(&self) -> Option<[f64; 4]> {
        match self {
            Self::Shapes(shapes) => (!shapes.items.is_empty()).then_some(shapes.bbox),
            Self::Cells(cells) => (!cells.rgba.is_empty()).then(|| [cells.left, cells.top - cells.height as f64 * cells.cell, cells.left + cells.width as f64 * cells.cell, cells.top]),
        }
    }
}

fn channels(colour: u32) -> [u8; 3] {
    let [_, red, green, blue] = colour.to_be_bytes();
    [red, green, blue]
}

/// The colour of a height in metres, as QGIS's linear colour ramp gives it (`QgsColorRampShader::shade`):
/// a height on a stop or beyond the ends takes that stop's colour; between two stops every channel is the
/// lower stop's plus the whole part of its share of the difference, in single precision.
pub(crate) fn height_colour(height: f32) -> [u8; 3] {
    let Some(upper) = STOPS.iter().position(|(metres, _)| height <= *metres) else {
        return channels(STOPS[STOPS.len() - 1].1);
    };
    #[allow(clippy::float_cmp)] // the stop itself, exactly
    if upper == 0 || height == STOPS[upper].0 {
        return channels(STOPS[upper].1);
    }
    let ((from, low), (to, high)) = (STOPS[upper - 1], STOPS[upper]);
    let (low, high, share) = (channels(low), channels(high), (height - from) / (to - from));
    // The whole part of a number between -255 and 255, added to a channel: the sum stays within 0..=255.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let mix = |a: u8, b: u8| (i32::from(a) + ((f32::from(b) - f32::from(a)) * share) as i32) as u8;
    [mix(low[0], high[0]), mix(low[1], high[1]), mix(low[2], high[2])]
}

/// The cells of an elevation layer: opaque colours of the heights, or (`Look::Relief`) black that covers as
/// much as light is missing. A cell without height is clear.
pub(crate) fn cells_of(grid: &ElevationGrid, look: &Look) -> Cells {
    let relief = matches!(look, Look::Relief);
    let colour = |(height, light): (&f32, &u8)| match (height.is_nan(), relief) {
        (true, _) => [0; 4],
        (false, true) => [0, 0, 0, 255 - *light],
        (false, false) => {
            let [red, green, blue] = height_colour(*height);
            [red, green, blue, 255]
        }
    };
    Cells { left: grid.left, top: grid.top, cell: grid.cell, width: grid.width, height: grid.height, rgba: grid.heights.iter().zip(&grid.shade).map(colour).collect() }
}

/// Reads what the layer draws.
///
/// # Errors
/// As `data::load` for a table; for an elevation layer, when the DEM folder holds no file GDAL can read
/// into the work CRS. The text is for the log, not for the user.
pub(crate) fn load(spec: &LoadSpec) -> Result<Drawn, String> {
    if !matches!(spec.look, Look::Heights | Look::Relief) {
        return crate::ref_vector::data::load(spec).map(|shapes| Drawn::Shapes(Arc::new(shapes)));
    }
    let files = crate::topo_download::dem::files_of(&spec.file);
    let grid = strata_engine::read_elevation_grid(&files, spec.work_epsg).map_err(|e| e.0)?;
    Ok(Drawn::Cells(Arc::new(cells_of(&grid, &spec.look))))
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

/// The picture of the layer for the view. `None` when the view has no pixels or no scale.
pub(crate) fn render(data: &Drawn, view: RasterView) -> Option<Raster> {
    match data {
        Drawn::Shapes(shapes) => crate::ref_vector::raster::render(shapes, view),
        Drawn::Cells(cells) => laid(cells, view),
    }
}
