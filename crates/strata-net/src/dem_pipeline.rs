//! DEM 타일 병합·준비 오케스트레이션 (C++ `DemDownloadService.cpp` 의 GDAL 단계).
//!
//! `dem_copernicus` 본문을 300줄 이하로 유지하기 위해 분리했다. 공개 경로는 그대로다.

use super::*;
use super::dem_fetch::{TileFetch, create_temp_storage, fetch_tile, remove_dir_quietly};

use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;

/// C++ `GDALBuildVRT` + `GDALTranslate`(:213-281) 를 같은 GDAL 명령줄 도구로 부른다.
/// 이 PC 의 GDAL 은 `OSGeo4W\apps\gdal-dev\bin` 에 있고 PATH 에는 없을 수 있다.
fn gdal_tool(name: &str) -> Option<PathBuf> {
    let mut cmd = Command::new(name);
    strata_types::process::no_window(&mut cmd)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if cmd.status().is_ok_and(|status| status.success()) {
        return Some(PathBuf::from(name));
    }
    for root in [r"C:\OSGeo4W\apps\gdal-dev\bin", r"C:\OSGeo4W\bin"] {
        let candidate = Path::new(root).join(format!("{name}.exe"));
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn run_quiet(tool: &Path, args: &[&std::ffi::OsStr]) -> bool {
    let mut cmd = Command::new(tool);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    strata_types::process::no_window(&mut cmd);
    cmd.status().is_ok_and(|status| status.success())
}

/// 받은 타일들을 `dem.tif` 한 장으로 붙인다. C++ :213-281 의 VRT+translate 대응.
fn mosaic_tiles(tiles: &[PathBuf], storage: &Path, raster_path: &Path) -> bool {
    let Some(build) = gdal_tool("gdalbuildvrt") else {
        return false;
    };
    let Some(translate) = gdal_tool("gdal_translate") else {
        return false;
    };
    let vrt_path = storage.join("dem.vrt");
    let mut vrt_args: Vec<std::ffi::OsString> = vec![
        "-resolution".into(),
        "highest".into(),
        "-vrtnodata".into(),
        "nan".into(),
        "-strict".into(),
        vrt_path.clone().into_os_string(),
    ];
    vrt_args.extend(tiles.iter().map(|p| p.clone().into_os_string()));
    let refs: Vec<&std::ffi::OsStr> = vrt_args.iter().map(|s| s.as_os_str()).collect();
    if !run_quiet(&build, &refs) {
        let _ = std::fs::remove_file(&vrt_path);
        return false;
    }
    // C++ :256-258 과 같은 생성 옵션. `-srcwin` 자르기는 열어 본 데이터셋의
    // 지오트랜스폼이 있어야 하는데, 여기서는 GDAL API 대신 명령줄을 쓰므로
    // 원본 격자 그대로(타일 전체) 옮긴다.
    let translate_args: Vec<&std::ffi::OsStr> = vec![
        "-of".as_ref(),
        "GTiff".as_ref(),
        "-co".as_ref(),
        "TILED=YES".as_ref(),
        "-co".as_ref(),
        "COMPRESS=DEFLATE".as_ref(),
        "-co".as_ref(),
        "PREDICTOR=3".as_ref(),
        "-co".as_ref(),
        "BIGTIFF=IF_SAFER".as_ref(),
        "--config".as_ref(),
        "GDAL_CACHEMAX".as_ref(),
        "256".as_ref(),
        vrt_path.as_os_str(),
        raster_path.as_os_str(),
    ];
    let ok = run_quiet(&translate, &translate_args);
    let _ = std::fs::remove_file(&vrt_path);
    ok
}

/// C++ `DemDownloadService::prepare` (DemDownloadService.cpp:109-305) 1:1.
///
/// `target_base_path` 는 C++ 호출부가 주는 저장 파일 경로(예: `…/DEM/원본/dem.tif`)이고,
/// 실제 파일은 그 옆의 자동 삭제 임시 폴더 안 `dem.tif` 에 놓인다(C++ :148-151).
#[must_use]
pub fn prepare(
    wgs84_extent: Wgs84Extent,
    target_base_path: &str,
    progress: &DemProgress<'_>,
) -> PreparedDem {
    if progress.cancelled() {
        return PreparedDem::cancelled();
    }
    if !wgs84_extent.is_valid_wgs84() {
        return PreparedDem::failed(DEM_EXTENT_INVALID_KO);
    }
    if dem_target_file_name(target_base_path).is_none() {
        return PreparedDem::failed(DEM_NO_STORAGE_KO);
    }
    let tiles = match tiles_for_extent(wgs84_extent) {
        Ok(tiles) => tiles,
        Err(error) => return PreparedDem::failed(&error.message_ko()),
    };
    let Some(storage) = create_temp_storage(target_base_path) else {
        return PreparedDem::failed(DEM_STORAGE_CREATE_FAILED_KO);
    };
    let result = download_and_merge(&tiles, &storage, progress);
    if result.status == Some(DemStatus::Failed) || result.status == Some(DemStatus::Cancelled) {
        remove_dir_quietly(&storage);
        return PreparedDem {
            storage_dir: None,
            ..result
        };
    }
    result
}

fn download_and_merge(
    tiles: &[DemTile],
    storage: &Path,
    progress: &DemProgress<'_>,
) -> PreparedDem {
    let tile_dir = storage.join("타일");
    if std::fs::create_dir_all(&tile_dir).is_err() {
        return PreparedDem::failed(DEM_STORAGE_CREATE_FAILED_KO);
    }
    let total = tiles.len().max(1);
    let mut saved: Vec<PathBuf> = Vec::new();
    let mut missing = 0_usize;
    let mut opened = 0_usize;
    for tile in tiles {
        if progress.cancelled() {
            return PreparedDem::cancelled();
        }
        let destination = tile_dir.join(format!("{}.tif", tile.name()));
        match fetch_tile(*tile, &destination, progress) {
            TileFetch::Saved(path) => saved.push(path),
            TileFetch::Missing => missing += 1,
            TileFetch::Cancelled => return PreparedDem::cancelled(),
            TileFetch::Failed => return PreparedDem::failed(DEM_CONNECTION_FAILED_KO),
        }
        // C++ :202-203 — 타일을 열어 본 수(openedTiles)로 20% 까지 채운다.
        opened += 1;
        let fraction = 0.20_f32 * (opened as f32) / (total as f32);
        progress.report(fraction);
    }
    if saved.is_empty() {
        return PreparedDem::failed(DEM_NO_TILE_KO);
    }
    let raster_path = storage.join(DEM_RASTER_FILE_NAME);
    if progress.cancelled() {
        return PreparedDem::cancelled();
    }
    progress.report(0.25);
    let merged = if saved.len() == 1 {
        // 타일 한 장이면 그 자체가 합친 결과다.
        std::fs::copy(&saved[0], &raster_path).is_ok()
    } else {
        mosaic_tiles(&saved, storage, &raster_path)
    };
    if progress.cancelled() {
        return PreparedDem::cancelled();
    }
    if !merged {
        return PreparedDem::failed(DEM_MOSAIC_FAILED_KO);
    }
    progress.report(0.99);
    // C++ :285-295 — 저장한 파일을 다시 확인한다.
    let verified = std::fs::metadata(&raster_path).is_ok_and(|meta| meta.len() > 0);
    if !verified {
        return PreparedDem::failed(DEM_VERIFY_FAILED_KO);
    }
    remove_dir_quietly(&tile_dir);
    progress.report(1.0);
    let mut warnings = Vec::new();
    if missing > 0 {
        warnings.push(dem_missing_tiles_warning_ko(missing));
    }
    PreparedDem {
        status: Some(DemStatus::Ready),
        raster_uri: Some(raster_path),
        storage_dir: Some(storage.to_path_buf()),
        error: String::new(),
        warnings,
        missing_tiles: missing,
        downloaded_tiles: saved.len(),
    }
}
