//! Pictures of the user — a scanned plan, a photo, a GeoTIFF — opened through GDAL as the C++ opens them
//! (`QgsRasterLayer(path, title, "gdal")`, `target/cpp-ref/src/app/MainWindow.cpp:2887`), world files included:
//! what the file is, and the part one view of the map needs, read at the size it is shown at. QGIS reads the
//! visible part of a raster at screen resolution; a scanned plan of some ten thousand pixels a side does not fit
//! one texture.

use std::path::Path;

use gdal::raster::ResampleAlg;
use gdal::Dataset;

use crate::EngineError;

#[cfg(test)]
#[path = "picture_tests.rs"]
mod tests;

/// What a picture file is.
#[derive(Debug, Clone, PartialEq)]
pub struct PictureInfo {
    pub width: usize,
    pub height: usize,
    /// Bands of colour: 1 grey (or the colours of its table), 2 grey and alpha, 3 RGB, 4 RGBA.
    pub bands: usize,
    /// Where its pixels lie, GDAL's order: `x = t[0] + col·t[1] + row·t[2]`, `y = t[3] + col·t[4] + row·t[5]`, from the
    /// file or its world file; none when it has neither.
    pub transform: Option<[f64; 6]>,
    /// The CRS the file names by an EPSG code.
    pub epsg: Option<u32>,
}

fn named(file: &Path, e: &dyn std::fmt::Display) -> EngineError {
    EngineError(format!("{}: {e}", file.display()))
}

/// What GDAL knows of the picture.
///
/// # Errors
/// GDAL cannot open the file as a raster, or it has no band.
pub fn picture_info(path: &Path) -> Result<PictureInfo, EngineError> {
    let dataset = Dataset::open(path).map_err(|e| named(path, &e))?;
    let bands = dataset.raster_count();
    if bands == 0 {
        return Err(named(path, &"the file holds no band"));
    }
    let (width, height) = dataset.raster_size();
    let epsg = dataset.spatial_ref().ok().filter(|s| s.auth_name().as_deref() == Some("EPSG")).and_then(|s| s.auth_code().ok()).and_then(|code| u32::try_from(code).ok());
    Ok(PictureInfo { width, height, bands, transform: dataset.geo_transform().ok(), epsg })
}

/// The pixels of `window` (`[col, row, cols, rows]` of the file) read at `size` (`[width, height]`), nearest
/// neighbour as QGIS draws a raster by default: straight RGBA, rows from the top.
///
/// # Errors
/// GDAL cannot open the file or read the window.
pub fn read_picture(path: &Path, window: [usize; 4], size: [usize; 2]) -> Result<Vec<u8>, EngineError> {
    let dataset = Dataset::open(path).map_err(|e| named(path, &e))?;
    let offset = (isize::try_from(window[0]).map_err(|e| named(path, &e))?, isize::try_from(window[1]).map_err(|e| named(path, &e))?);
    let read = |index: usize| -> Result<Vec<u8>, EngineError> {
        let band = dataset.rasterband(index).map_err(|e| named(path, &e))?;
        let buffer = band.read_as::<u8>(offset, (window[2], window[3]), (size[0], size[1]), Some(ResampleAlg::NearestNeighbour)).map_err(|e| named(path, &e))?;
        Ok(buffer.into_shape_and_vec().1)
    };
    let bands = dataset.raster_count();
    let mut rgba = vec![255_u8; size[0] * size[1] * 4];
    if bands >= 3 {
        let (red, green, blue) = (read(1)?, read(2)?, read(3)?);
        let alpha = if bands >= 4 { Some(read(4)?) } else { None };
        for (i, pixel) in rgba.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            pixel[..3].copy_from_slice(&[red[i], green[i], blue[i]]);
            pixel[3] = alpha.as_ref().map_or(255, |a| a[i]);
        }
    } else {
        let grey = read(1)?;
        let alpha = if bands == 2 { Some(read(2)?) } else { None };
        // A band of indexes is drawn in the colours of its table.
        let table = dataset.rasterband(1).ok().and_then(|band| {
            let colours = band.color_table()?;
            Some((0..256).map(|i| colours.entry_as_rgb(i).map_or([0, 0, 0, 255], |c| [c.r, c.g, c.b, c.a].map(|v| u8::try_from(v.clamp(0, 255)).unwrap_or(255)))).collect::<Vec<[u8; 4]>>())
        });
        for (i, pixel) in rgba.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            let value = grey[i];
            let colour = table.as_ref().map_or([value, value, value, 255], |t| t[usize::from(value)]);
            pixel.copy_from_slice(&colour);
            if let Some(a) = &alpha {
                pixel[3] = a[i];
            }
        }
    }
    Ok(rgba)
}
