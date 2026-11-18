//! `surface.tif` and `contours.gpkg` for one survey contour run.
//!
//! Paths and names follow `SurveyContourWrite.cpp` and `MainWindowSurveyContour.cpp:122-128`:
//! Float32 north-up GeoTIFF, NoData −9999, and a GeoPackage with `survey_points` and
//! `contour_lines` in the work CRS. Colour bands stay at zero until polygonize is ported.

use std::path::Path;

use gdal::raster::{Buffer, RasterCreationOptions};
use gdal::spatial_ref::SpatialRef;
use gdal::vector::{
    Feature, FieldDefn, Geometry, LayerAccess, LayerOptions, OGRFieldType, OGRwkbGeometryType,
};
use gdal::DriverManager;
use strata_geo::contour_surface::{surface_contours, HeightGrid, SurfaceContour};

use crate::EngineError;

/// What was written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContourOutput {
    pub lines: usize,
    pub points: usize,
}

/// Writes `surface.tif` and `contours.gpkg` into `dir`, which must already exist and be empty.
///
/// # Errors
/// The points cannot make a surface, or either file cannot be written. A half-written file is removed.
pub fn write_contour_package(
    dir: &Path,
    epsg: u32,
    points: &[[f64; 3]],
    interval_m: f64,
    cell_m: f64,
) -> Result<ContourOutput, EngineError> {
    let (grid, lines) = surface_contours(points, interval_m, cell_m)
        .map_err(|text| EngineError(text.to_owned()))?;
    let tif = dir.join("surface.tif");
    let gpkg = dir.join("contours.gpkg");
    if let Err(e) = write_tif(&tif, epsg, &grid)
        .and_then(|()| write_gpkg(&gpkg, epsg, points, interval_m, &lines))
    {
        let _ = std::fs::remove_file(&tif);
        let _ = std::fs::remove_file(&gpkg);
        return Err(e);
    }
    Ok(ContourOutput {
        lines: lines.len(),
        points: points.len(),
    })
}

fn write_tif(path: &Path, epsg: u32, grid: &HeightGrid) -> Result<(), EngineError> {
    let driver =
        DriverManager::get_driver_by_name("GTiff").map_err(|e| EngineError(e.to_string()))?;
    let options =
        RasterCreationOptions::from_iter(["COMPRESS=DEFLATE", "TILED=YES", "PREDICTOR=3"]);
    let mut file = driver
        .create_with_band_type_with_options::<f32, _>(path, grid.width, grid.height, 1, &options)
        .map_err(|e| EngineError(e.to_string()))?;
    file.set_geo_transform(&[grid.west, grid.cell, 0.0, grid.north, 0.0, -grid.cell])?;
    file.set_spatial_ref(&SpatialRef::from_epsg(epsg)?)?;
    let mut band = file.rasterband(1)?;
    band.set_no_data_value(Some(-9999.0))?;
    band.write(
        (0, 0),
        (grid.width, grid.height),
        &mut Buffer::new((grid.width, grid.height), grid.values.clone()),
    )?;
    Ok(())
}

fn write_gpkg(
    path: &Path,
    epsg: u32,
    points: &[[f64; 3]],
    interval_m: f64,
    lines: &[SurfaceContour],
) -> Result<(), EngineError> {
    let srs = SpatialRef::from_epsg(epsg)?;
    let driver =
        DriverManager::get_driver_by_name("GPKG").map_err(|e| EngineError(e.to_string()))?;
    let mut dataset = driver
        .create_vector_only(path)
        .map_err(|e| EngineError(e.to_string()))?;
    let mut work = dataset.start_transaction()?;
    {
        let points_layer = work.create_layer(LayerOptions {
            name: "survey_points",
            srs: Some(&srs),
            ty: OGRwkbGeometryType::wkbPoint,
            options: None,
        })?;
        FieldDefn::new("name", OGRFieldType::OFTString)?.add_to_layer(&points_layer)?;
        FieldDefn::new("elev", OGRFieldType::OFTReal)?.add_to_layer(&points_layer)?;
        FieldDefn::new("elev_cm", OGRFieldType::OFTInteger)?.add_to_layer(&points_layer)?;
        for (i, p) in points.iter().enumerate() {
            let mut feature = Feature::new(points_layer.defn())?;
            feature.set_field_string(0, &i.to_string())?;
            feature.set_field_double(1, p[2])?;
            feature.set_field_integer(2, (p[2] * 100.0).round() as i32)?;
            let mut geom = Geometry::empty(OGRwkbGeometryType::wkbPoint)?;
            geom.add_point_2d((p[0], p[1]));
            feature.set_geometry(geom)?;
            feature.create(&points_layer)?;
        }
        let lines_layer = work.create_layer(LayerOptions {
            name: "contour_lines",
            srs: Some(&srs),
            ty: OGRwkbGeometryType::wkbLineString,
            options: None,
        })?;
        FieldDefn::new("elev", OGRFieldType::OFTReal)?.add_to_layer(&lines_layer)?;
        FieldDefn::new("elev_cm", OGRFieldType::OFTInteger)?.add_to_layer(&lines_layer)?;
        FieldDefn::new("elev_label", OGRFieldType::OFTString)?.add_to_layer(&lines_layer)?;
        FieldDefn::new("index_line", OGRFieldType::OFTInteger)?.add_to_layer(&lines_layer)?;
        for line in lines {
            let mut feature = Feature::new(lines_layer.defn())?;
            feature.set_field_double(0, line.elevation_m)?;
            feature.set_field_integer(1, (line.elevation_m * 100.0).round() as i32)?;
            feature.set_field_string(2, &elev_label(line.elevation_m, interval_m))?;
            feature.set_field_integer(3, i32::from(line.index_line))?;
            let mut geom = Geometry::empty(OGRwkbGeometryType::wkbLineString)?;
            for p in &line.xy {
                geom.add_point_2d((p[0], p[1]));
            }
            feature.set_geometry(geom)?;
            feature.create(&lines_layer)?;
        }
    }
    work.commit()?;
    dataset.close()?;
    Ok(())
}

fn elev_label(level: f64, interval_m: f64) -> String {
    if interval_m < 1.0 {
        format!("{level:.1}")
    } else {
        format!("{}", level.round() as i64)
    }
}

#[cfg(test)]
#[path = "contour_files_tests.rs"]
mod tests;
