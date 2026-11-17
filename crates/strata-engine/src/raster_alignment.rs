//! Protected-source persistence, target/cpp-ref/src/core/GeorefService.cpp:208-341 and GeorefBackup.cpp:190-277.
use crate::{EngineError, PictureInfo};
use gdal::{spatial_ref::SpatialRef, Dataset, DatasetOptions, GdalOpenFlags};
use std::path::{Path, PathBuf};
#[path = "raster_alignment_io.rs"]
mod transaction;

#[derive(Debug, Clone)]
pub struct AlignedRaster {
    source: PathBuf,
    file: PathBuf,
    backup: PathBuf,
}

/// Create a uniquely owned aligned copy, retaining the original image and sidecars.
/// # Errors
/// Invalid transform/CRS, unreadable source or failed output write.
pub fn create_aligned_raster(source: &Path, root: &Path, transform: [f64; 6], epsg: u32) -> Result<AlignedRaster, EngineError> {
    validate(transform, epsg)?;
    let info = crate::picture_info(source)?;
    if info.width < 2 || info.height < 2 {
        return Err(EngineError("그림이 너무 작습니다".to_owned()));
    }
    let source = std::path::absolute(source).map_err(error)?;
    let name = source.file_name().ok_or_else(|| error("그림 파일을 찾지 못했습니다"))?;
    std::fs::create_dir_all(root).map_err(error)?;
    let folder = unique_folder(root)?;
    let result = AlignedRaster { source: source.clone(), file: folder.join(name), backup: folder.join("before") };
    let written = (|| {
        std::fs::create_dir(&result.backup).map_err(error)?;
        // The untouched source remains the image backup; sidecars are also snapshotted (GeorefBackup.cpp:210-223).
        std::fs::write(folder.join("source.txt"), format!("Strata alignment v1\nsource={}\n", source.display())).map_err(error)?;
        for path in sidecars(&source) {
            if path.is_file() {
                let name = path.file_name().ok_or_else(|| error("Invalid sidecar name"))?;
                std::fs::copy(&path, result.backup.join(name)).map_err(error)?;
            }
        }
        result.apply(transform, epsg)?;
        Ok(())
    })();
    if let Err(why) = written {
        drop(result.restore());
        return Err(why);
    }
    Ok(result)
}

impl AlignedRaster {
    #[must_use]
    pub fn source(&self) -> &Path {
        &self.source
    }
    #[must_use]
    pub fn file(&self) -> &Path {
        &self.file
    }
    #[must_use]
    pub fn backup(&self) -> &Path {
        &self.backup
    }
    /// Apply another absolute fit to this session's own copy.
    /// # Errors
    /// Invalid transform/CRS or failed output write/read-back.
    pub fn apply(&self, transform: [f64; 6], epsg: u32) -> Result<PictureInfo, EngineError> {
        let crs = validate(transform, epsg)?;
        transaction::replace(&self.source, &self.file, |staged| write_fit(staged, transform, epsg, &crs))
    }
    /// Remove only this result's known output files, retaining its source and backup.
    /// # Errors
    /// An owned output cannot be removed.
    pub fn restore(&self) -> Result<(), EngineError> {
        for path in sidecars(&self.file).into_iter().chain(std::iter::once(self.file.clone())) {
            match std::fs::remove_file(path) {
                Ok(()) => {}
                Err(why) if why.kind() == std::io::ErrorKind::NotFound => {}
                Err(why) => return Err(error(why)),
            }
        }
        Ok(())
    }
}

fn write_fit(file: &Path, transform: [f64; 6], epsg: u32, crs: &SpatialRef) -> Result<PictureInfo, EngineError> {
    let [c, a, b, f, d, e] = transform;
    // Corner -> pixel centre, GeorefService.cpp:235-248. No resampling of source pixels.
    let values = [a, d, b, e, c + 0.5 * (a + b), f + 0.5 * (d + e)];
    let world = values.into_iter().map(|v| format!("{v}\r\n")).collect::<String>();
    std::fs::write(world_file(file), world).map_err(error)?;
    std::fs::write(file.with_extension("prj"), crs.to_wkt().map_err(error)?).map_err(error)?;
    let ext = file.extension().and_then(|s| s.to_str()).unwrap_or_default().to_ascii_lowercase();
    if matches!(ext.as_str(), "tif" | "tiff" | "gtiff") {
        let mut dataset =
            Dataset::open_ex(file, DatasetOptions { open_flags: GdalOpenFlags::GDAL_OF_UPDATE | GdalOpenFlags::GDAL_OF_RASTER, ..Default::default() }).map_err(error)?;
        if !dataset.gcps().is_empty() {
            dataset.set_gcps(Vec::new(), crs).map_err(error)?;
        }
        dataset.set_geo_transform(&transform).map_err(error)?;
        dataset.set_spatial_ref(crs).map_err(error)?;
        dataset.flush_cache().map_err(error)?;
    }
    let info = crate::picture_info(file)?;
    let got = info.transform.ok_or_else(|| error("그림을 지도 좌표로 붙이지 못했습니다. 점을 다시 찍고 이동하세요."))?;
    if info.epsg != Some(epsg) || got.into_iter().zip(transform).any(|(actual, wanted)| (actual - wanted).abs() > 1e-9 * wanted.abs().max(1.0)) {
        return Err(error("맞춰진 그림을 다시 열지 못했습니다"));
    }
    Ok(info)
}

fn error(why: impl std::fmt::Display) -> EngineError {
    EngineError(why.to_string())
}

fn validate(transform: [f64; 6], epsg: u32) -> Result<SpatialRef, EngineError> {
    if !matches!(epsg, 5186 | 5187) {
        return Err(error("좌표계가 없습니다"));
    }
    let determinant = transform[1] * transform[5] - transform[2] * transform[4];
    if transform.iter().any(|v| !v.is_finite()) || !determinant.is_finite() || determinant.abs() < 1e-18 {
        return Err(error("변환이 없습니다"));
    }
    SpatialRef::from_epsg(epsg).map_err(error)
}

fn unique_folder(root: &Path) -> Result<PathBuf, EngineError> {
    for index in 1..=10000 {
        let folder = root.join(format!("alignment-{index:04}"));
        match std::fs::create_dir(&folder) {
            Ok(()) => return Ok(folder),
            Err(why) if why.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(why) => return Err(error(why)),
        }
    }
    Err(error("백업 폴더를 만들지 못했습니다"))
}

fn world_file(image: &Path) -> PathBuf {
    let ext = image.extension().and_then(|s| s.to_str()).unwrap_or_default().to_ascii_lowercase();
    match ext.as_str() {
        "jpg" | "jpeg" => image.with_extension("jgw"),
        "png" => image.with_extension("pgw"),
        "tif" | "tiff" => image.with_extension("tfw"),
        _ => {
            let mut name = image.as_os_str().to_owned();
            name.push(".wld");
            PathBuf::from(name)
        }
    }
}

fn sidecars(image: &Path) -> Vec<PathBuf> {
    let mut full = image.as_os_str().to_owned();
    full.push(".aux.xml");
    vec![world_file(image), image.with_extension("prj"), PathBuf::from(full), image.with_extension("aux.xml")]
}

#[cfg(test)]
#[path = "raster_alignment_tests.rs"]
mod tests;
