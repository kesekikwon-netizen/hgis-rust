//! Elevation rasters (DEM): the files of a survey brought to one grid in the work CRS, with its hillshade.
//!
//! Every step is GDAL's. The files are read through its drivers (HFA for the NGII `.img`, as the C++ app's
//! `QgsRasterLayer(path, "DEM", "gdal")`, `target/cpp-ref/src/core/BasemapOps.cpp:2007`), brought to the
//! target CRS by its warper (`GDALReprojectImage`, bilinear, exact transformer, cells the file marks as
//! empty left out), and shaded by `gdaldem hillshade` (multidirectional, the sun 45 degrees high: the
//! settings of the C++ relief layer, `BasemapOps.cpp:2072-2076`).

use std::path::{Path, PathBuf};

use gdal::raster::processing::dem::{hillshade, HillshadeOptions, ShadingMode};
use gdal::raster::reproject;
use gdal::spatial_ref::{AxisMappingStrategy, CoordTransform, SpatialRef};
use gdal::{Dataset, DriverManager};

use crate::EngineError;

/// The value that stands for "no height" in the grid while GDAL works on it (the NGII files use the same).
const NO_DATA: f64 = -9999.0;
/// More cells than this on a side is no DEM of a survey's surroundings.
const MAX_SIDE: f64 = 20_000.0;
/// Points GDAL puts on each edge of a file's box when it carries the box into the target CRS.
const EDGE_POINTS: i32 = 21;

/// Heights and light on one grid of square cells, row by row from the north.
#[derive(Debug, Clone, PartialEq)]
pub struct ElevationGrid {
    /// The west edge of the grid, metres of the target CRS.
    pub left: f64,
    /// The north edge of the grid.
    pub top: f64,
    /// The side of a cell, metres.
    pub cell: f64,
    pub width: usize,
    pub height: usize,
    /// Metres; `NaN` where there is no height.
    pub heights: Vec<f32>,
    /// Light on the ground, 0 (none) to 255 (full); 0 where there is no height.
    pub shade: Vec<u8>,
}

impl ElevationGrid {
    /// `[min x, min y, max x, max y]` of the grid.
    #[must_use]
    // A grid has at most MAX_SIDE cells on a side: f64 holds that exactly.
    #[allow(clippy::cast_precision_loss)]
    pub fn extent(&self) -> [f64; 4] {
        [self.left, self.top - self.height as f64 * self.cell, self.left + self.width as f64 * self.cell, self.top]
    }
}

fn named(file: &Path, e: &dyn std::fmt::Display) -> EngineError {
    EngineError(format!("{}: {e}", file.display()))
}

/// A file GDAL opens as a north-up raster with a band and a CRS: the raster, where its cells lie, its CRS.
fn north_up(file: &Path) -> Result<(Dataset, [f64; 6], SpatialRef), EngineError> {
    let source = Dataset::open(file).map_err(|e| named(file, &e))?;
    let gt = source.geo_transform().map_err(|e| named(file, &e))?;
    if source.raster_count() == 0 || gt[2] != 0.0 || gt[4] != 0.0 || gt[1] <= 0.0 || gt[5] >= 0.0 {
        return Err(named(file, &"no north-up elevation raster"));
    }
    let mut crs = source.spatial_ref().map_err(|e| named(file, &e))?;
    // A file without a CRS answers with an empty one: it cannot be brought to the work CRS.
    if crs.to_wkt().map_or(true, |wkt| wkt.is_empty()) {
        return Err(named(file, &"no coordinate reference system"));
    }
    crs.set_axis_mapping_strategy(AxisMappingStrategy::TraditionalGisOrder);
    Ok((source, gt, crs))
}

/// A file with its box in the target CRS and the side of its cells.
fn opened(file: &Path, target: &SpatialRef) -> Result<(Dataset, [f64; 4], f64), EngineError> {
    let (source, gt, crs) = north_up(file)?;
    let (across, down) = source.raster_size();
    // Sizes of a raster file are far below 2^53.
    #[allow(clippy::cast_precision_loss)]
    let own = [gt[0], gt[3] + gt[5] * down as f64, gt[0] + gt[1] * across as f64, gt[3]];
    let carried = CoordTransform::new(&crs, target).and_then(|to| to.transform_bounds(&own, EDGE_POINTS)).map_err(|e| named(file, &e))?;
    Ok((source, carried, gt[1].min(-gt[5])))
}

/// Whether a file can be the DEM of the map: GDAL opens it as a north-up raster with a band and a CRS.
/// Its heights are not read.
///
/// # Errors
/// What is wrong with the file, in GDAL's words or these.
pub fn check_elevation_file(file: &Path) -> Result<(), EngineError> {
    north_up(file).map(drop)
}

/// Reads elevation files into one grid of `EPSG:{target_epsg}`; where files overlap, a later one lies over
/// an earlier one. `z_factor` is how many times the heights count for the light (「수직과장」, the C++
/// keeps it between 0.1 and 5, `BasemapOps.cpp:2073-2075`); the heights of the grid are not changed by it.
///
/// # Errors
/// No file is given; a file is no raster GDAL can read, has no CRS or stands askew; the grid would be
/// larger than any DEM of a survey's surroundings.
pub fn read_elevation_grid(files: &[PathBuf], target_epsg: u32, z_factor: f64) -> Result<ElevationGrid, EngineError> {
    let mut target = SpatialRef::from_epsg(target_epsg)?;
    target.set_axis_mapping_strategy(AxisMappingStrategy::TraditionalGisOrder);
    let mut sources = Vec::with_capacity(files.len());
    let (mut bounds, mut cell) = ([f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY], f64::INFINITY);
    for file in files {
        let (source, carried, side) = opened(file, &target)?;
        bounds = [bounds[0].min(carried[0]), bounds[1].min(carried[1]), bounds[2].max(carried[2]), bounds[3].max(carried[3])];
        cell = cell.min(side);
        sources.push(source);
    }
    // Whole cells from a corner that is a multiple of the cell: two readings of the same files agree.
    let (left, top) = ((bounds[0] / cell).floor() * cell, (bounds[3] / cell).ceil() * cell);
    let (across, down) = (((bounds[2] - left) / cell).ceil(), ((top - bounds[1]) / cell).ceil());
    if sources.is_empty() || !(1.0..=MAX_SIDE).contains(&across) || !(1.0..=MAX_SIDE).contains(&down) {
        return Err(EngineError(format!("no elevation grid for {} file(s): {across} x {down} cells", files.len())));
    }
    // Checked just above: whole numbers between 1 and MAX_SIDE.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let (width, height) = (across as usize, down as usize);

    let mut grid = DriverManager::get_driver_by_name("MEM")?.create_with_band_type::<f32, _>("", width, height, 1)?;
    grid.set_geo_transform(&[left, cell, 0.0, top, 0.0, -cell])?;
    grid.set_spatial_ref(&target)?;
    let mut band = grid.rasterband(1)?;
    band.set_no_data_value(Some(NO_DATA))?;
    band.fill(NO_DATA, None)?;
    for source in &sources {
        reproject(source, &grid)?;
    }

    let mut light = HillshadeOptions::new();
    light.with_output_format("MEM").with_compute_edges(true).with_shading_mode(ShadingMode::Multidirectional).with_altitude(45.0).with_z_factor(z_factor).with_scale(1.0);
    let lit = hillshade(&grid, Path::new(""), &light)?;
    if lit.c_dataset().is_null() {
        return Err(EngineError("GDAL made no hillshade of the elevation grid".to_owned()));
    }
    let (_, heights) = grid.rasterband(1)?.read_band_as::<f32>()?.into_shape_and_vec();
    let (_, shade) = lit.rasterband(1)?.read_band_as::<u8>()?.into_shape_and_vec();
    // The value GDAL was told to keep empty cells at comes back exactly.
    #[allow(clippy::float_cmp, clippy::cast_possible_truncation)]
    let heights: Vec<f32> = heights.into_iter().map(|h| if h == NO_DATA as f32 || !h.is_finite() { f32::NAN } else { h }).collect();
    let shade = shade.into_iter().zip(&heights).map(|(light, h)| if h.is_nan() { 0 } else { light }).collect();
    Ok(ElevationGrid { left, top, cell, width, height, heights, shade })
}

#[cfg(test)]
#[path = "elevation_tests.rs"]
mod tests;
