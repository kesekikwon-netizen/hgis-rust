//! C++ `DemDownloadService.cpp` 1:1 이식 — Copernicus DSM 30m(GLO-30) 타일 내려받기.
//!
//! C++ 정본과 대응하는 지점:
//! - `tileUri()` (DemDownloadService.cpp:85-93) → [`tile_name`] / [`tile_url`] / [`DemTile::vsicurl_uri`]
//! - `isMissingTile()` (:96-100) → [`is_missing_tile`]
//! - `gridAligned()` (:102-105) → [`grid_aligned`]
//! - 타일 범위 floor/ceil-1 과 `kMaximumTiles` (:25, :138-147) → [`tile_range`] / [`tiles_for_extent`]
//! - `prepare()` (:109-305) → [`prepare`]
//!
//! HTTP 는 `address_search.rs:758` 의 curl.exe 방식을 따르되, 수십 MB 바이너리라
//! `-o` 파일 저장 + `-f`(404 실패) 로 받는다. 새 크레이트를 쓰지 않는다.
//! 타일 여러 장을 한 장으로 붙이는 단계는 C++ 이 GDAL API(`GDALBuildVRT` /
//! `GDALTranslate`, :213-281)를 부르는 것과 같으므로 같은 GDAL 의 명령줄
//! 도구를 같은 옵션으로 부른다.

use std::fmt;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// C++ `kMaximumTiles` (DemDownloadService.cpp:25) — 한 번에 받는 1° 칸 상한
pub const DEM_MAXIMUM_TILES: usize = 24;

/// C++ `DemDownloadService.cpp:93` — Copernicus GLO-30 공개 버킷
pub const COPERNICUS_DEM_BUCKET: &str = "https://copernicus-dem-30m.s3.amazonaws.com";

/// 저장 파일 이름 (DemDownloadService.cpp:151) — 여러 타일을 합친 한 장
pub const DEM_RASTER_FILE_NAME: &str = "dem.tif";

/// C++ `DemDownloadService.cpp:117` 취소 문구
pub const DEM_CANCELLED_KO: &str = "DEM 내려받기를 취소했습니다. 기존 지도는 유지됩니다.";
/// C++ `DemDownloadService.cpp:130` 범위 문구
pub const DEM_EXTENT_INVALID_KO: &str =
    "DEM을 받을 지도 범위가 올바르지 않습니다. 조사구역으로 이동한 뒤 다시 내려받으세요.";
/// C++ `DemDownloadService.cpp:134` 저장 위치 문구
pub const DEM_NO_STORAGE_KO: &str =
    "DEM 저장 위치가 없습니다. 저장 폴더를 선택하고 다시 내려받으세요.";
/// C++ `DemDownloadService.cpp:172` 연결 실패 문구
pub const DEM_CONNECTION_FAILED_KO: &str = "DEM 서버에 연결하지 못했거나 응답이 늦어 중단했습니다. 인터넷 연결을 확인한 뒤 다시 내려받으세요. 기존 지도는 유지됩니다.";
/// C++ `DemDownloadService.cpp:207` 제공 자료 없음 문구
pub const DEM_NO_TILE_KO: &str =
    "이 범위에는 제공되는 DEM 자료가 없습니다. 바다를 제외한 조사구역으로 이동하고 다시 내려받으세요.";
/// C++ `DemDownloadService.cpp:234` 병합 실패 문구
pub const DEM_MOSAIC_FAILED_KO: &str =
    "DEM 자료를 하나로 연결하지 못했습니다. 기존 지도는 유지됩니다. 다시 내려받으세요.";
/// C++ `DemDownloadService.cpp:275` 저장 실패 문구
pub const DEM_SAVE_FAILED_KO: &str = "DEM을 끝까지 내려받거나 저장하지 못했습니다. 인터넷 연결과 저장 공간을 확인하세요. 기존 지도는 유지됩니다.";
/// C++ `DemDownloadService.cpp:293` 저장 확인 실패 문구
pub const DEM_VERIFY_FAILED_KO: &str = "저장한 DEM을 다시 확인하지 못했습니다. 기존 지도는 유지됩니다. 다른 저장 폴더에서 다시 내려받으세요.";
/// C++ `PreparedReferenceMap.cpp:23` 임시 폴더 문구
pub const DEM_STORAGE_CREATE_FAILED_KO: &str =
    "지도를 저장할 임시 폴더를 만들지 못했습니다. 저장 폴더의 쓰기 권한과 남은 공간을 확인하세요.";

/// C++ `DemDownloadService.cpp:144-145` 범위가 너무 넓을 때 문구 (`%1` = kMaximumTiles)
#[must_use]
pub fn dem_too_wide_ko() -> String {
    format!(
        "DEM을 받을 범위가 너무 넓습니다. 한 번에 최대 {DEM_MAXIMUM_TILES}칸까지 받을 수 있습니다. 조사구역으로 확대하고 다시 내려받으세요."
    )
}

/// C++ `DemDownloadService.cpp:211` 경고 문구 (`%1` = 빠진 칸 수)
#[must_use]
pub fn dem_missing_tiles_warning_ko(missing: usize) -> String {
    format!("요청 범위 중 {missing}칸은 바다이거나 DEM이 제공되지 않아 빈 영역으로 남습니다.")
}

/// C++ `QgsRectangle` — WGS84(경위도) 범위. C++ `wgs84Extent` 인자와 같은 값.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Wgs84Extent {
    pub min_lon: f64,
    pub min_lat: f64,
    pub max_lon: f64,
    pub max_lat: f64,
}

impl Wgs84Extent {
    #[must_use]
    pub const fn new(min_lon: f64, min_lat: f64, max_lon: f64, max_lat: f64) -> Self {
        Self {
            min_lon,
            min_lat,
            max_lon,
            max_lat,
        }
    }

    /// C++ `QgsRectangle::isEmpty()` — 최대값이 최소값보다 작으면 빈 범위
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.max_lon < self.min_lon || self.max_lat < self.min_lat
    }

    /// C++ `QgsRectangle::isFinite()`
    #[must_use]
    pub fn is_finite(self) -> bool {
        self.min_lon.is_finite()
            && self.min_lat.is_finite()
            && self.max_lon.is_finite()
            && self.max_lat.is_finite()
    }

    /// C++ `DemDownloadService.cpp:127-129` 범위 검사 그대로
    #[must_use]
    pub fn is_valid_wgs84(self) -> bool {
        !self.is_empty()
            && self.is_finite()
            && self.min_lon >= -180.0
            && self.max_lon <= 180.0
            && self.min_lat >= -90.0
            && self.max_lat <= 90.0
    }
}

/// Copernicus 1°×1° 타일 한 칸 (남서쪽 모서리의 정수 도 단위)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DemTile {
    pub lat: i32,
    pub lon: i32,
}

impl DemTile {
    #[must_use]
    pub const fn new(lat: i32, lon: i32) -> Self {
        Self { lat, lon }
    }

    /// C++ `tileUri()` 의 파일 이름 부분 (DemDownloadService.cpp:86-92)
    #[must_use]
    pub fn name(self) -> String {
        tile_name(self.lat, self.lon)
    }

    /// curl 로 받을 실제 https 주소 (C++ 의 `/vsicurl/` 접두사 없는 형태)
    #[must_use]
    pub fn url(self) -> String {
        tile_url(self.lat, self.lon)
    }

    /// C++ `tileUri()` 원문 (DemDownloadService.cpp:93) — GDAL 가상 파일 경로
    #[must_use]
    pub fn vsicurl_uri(self) -> String {
        format!("/vsicurl/{}", self.url())
    }
}

/// C++ `tileUri()` (DemDownloadService.cpp:85-93) 의 이름 규칙.
/// 예: lat 35, lon 129 → `Copernicus_DSM_COG_10_N35_00_E129_00_DEM`
#[must_use]
pub fn tile_name(lat: i32, lon: i32) -> String {
    let north = if lat >= 0 { 'N' } else { 'S' };
    let east = if lon >= 0 { 'E' } else { 'W' };
    format!(
        "Copernicus_DSM_COG_10_{north}{:02}_00_{east}{:03}_00_DEM",
        lat.abs(),
        lon.abs()
    )
}

/// C++ `DemDownloadService.cpp:93` 의 버킷 주소. `/vsicurl/` 접두사만 뺀 형태다.
#[must_use]
pub fn tile_url(lat: i32, lon: i32) -> String {
    let name = tile_name(lat, lon);
    format!("{COPERNICUS_DEM_BUCKET}/{name}/{name}.tif")
}

/// C++ `isMissingTile()` (DemDownloadService.cpp:96-100) — 404 만 "타일 없음"으로 본다
#[must_use]
pub fn is_missing_tile(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("http response code: 404")
        || lower.contains("http error code : 404")
        || lower.contains("404 not found")
}

/// C++ `gridAligned()` (DemDownloadService.cpp:102-105)
#[must_use]
pub fn grid_aligned(delta: f64, spacing: f64) -> bool {
    let cells = delta / spacing;
    (cells - cells.round()).abs() < 1e-5
}

/// C++ `DemDownloadService.cpp:138-142` 타일 순회 범위 (1도 격자)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DemTileRange {
    pub lon_from: i32,
    pub lon_to: i32,
    pub lat_from: i32,
    pub lat_to: i32,
}

impl DemTileRange {
    /// C++ `(lonTo-lonFrom+1) * (latTo-latFrom+1)` — 음수면 빈 범위
    #[must_use]
    pub fn tile_count(self) -> i64 {
        let width = i64::from(self.lon_to) - i64::from(self.lon_from) + 1;
        let height = i64::from(self.lat_to) - i64::from(self.lat_from) + 1;
        width * height
    }
}

/// C++ `DemDownloadService.cpp:138-142` — `floor(min)` / `ceil(max) - 1`
#[must_use]
pub fn tile_range(extent: Wgs84Extent) -> DemTileRange {
    DemTileRange {
        lon_from: extent.min_lon.floor() as i32,
        lon_to: extent.max_lon.ceil() as i32 - 1,
        lat_from: extent.min_lat.floor() as i32,
        lat_to: extent.max_lat.ceil() as i32 - 1,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DemTileListError {
    /// C++ `DemDownloadService.cpp:130`
    ExtentInvalid,
    /// C++ `DemDownloadService.cpp:143-147`
    TooWide,
}

impl DemTileListError {
    #[must_use]
    pub fn message_ko(self) -> String {
        match self {
            Self::ExtentInvalid => DEM_EXTENT_INVALID_KO.to_owned(),
            Self::TooWide => dem_too_wide_ko(),
        }
    }
}

/// 범위를 덮는 타일 목록. C++ 순회 순서 그대로 위도 바깥 · 경도 안쪽(:160-161).
///
/// C++ `DemDownloadService.cpp:143` 과 같이 1칸 이상 `kMaximumTiles` 이하여야 한다.
pub fn tiles_for_extent(extent: Wgs84Extent) -> Result<Vec<DemTile>, DemTileListError> {
    if !extent.is_valid_wgs84() {
        return Err(DemTileListError::ExtentInvalid);
    }
    let range = tile_range(extent);
    let count = range.tile_count();
    if count <= 0 || count > i64::try_from(DEM_MAXIMUM_TILES).unwrap_or(i64::MAX) {
        return Err(DemTileListError::TooWide);
    }
    let mut tiles = Vec::with_capacity(usize::try_from(count).unwrap_or(0));
    for lat in range.lat_from..=range.lat_to {
        for lon in range.lon_from..=range.lon_to {
            tiles.push(DemTile::new(lat, lon));
        }
    }
    Ok(tiles)
}

/// C++ `PreparedReferenceMap::Status` (PreparedReferenceMap.h:18)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DemStatus {
    Ready,
    Cancelled,
    Failed,
}

/// C++ `PreparedReferenceMap` 중 DEM 경로가 쓰는 값만 옮긴 형태
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PreparedDem {
    pub status: Option<DemStatus>,
    /// C++ `rasterUri` — `<저장폴더>/ka-hgis-reference-XXXXXX/dem.tif`
    pub raster_uri: Option<PathBuf>,
    /// C++ `storage->path()` — 자동 삭제 임시 폴더. 실패·취소면 지운다.
    pub storage_dir: Option<PathBuf>,
    /// C++ `error` — 상태 문구 원문
    pub error: String,
    /// C++ `warnings` — 404 로 빠진 칸 안내
    pub warnings: Vec<String>,
    /// 바다 등으로 404 가 돌아온 칸 수
    pub missing_tiles: usize,
    /// 실제로 파일을 받은 칸 수
    pub downloaded_tiles: usize,
}

impl PreparedDem {
    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.status == Some(DemStatus::Ready)
    }

    fn failed(message: &str) -> Self {
        Self {
            status: Some(DemStatus::Failed),
            error: message.to_owned(),
            ..Self::default()
        }
    }

    fn cancelled() -> Self {
        Self {
            status: Some(DemStatus::Cancelled),
            error: DEM_CANCELLED_KO.to_owned(),
            ..Self::default()
        }
    }
}

/// C++ `QgsFeedback`(진행률) + `cancellationRequested` 대응
pub struct DemProgress<'a> {
    /// 0.0 ~ 1.0 진행률. C++ `feedback->setProgress(0..100)` 를 100 으로 나눈 값.
    pub on_progress: Option<&'a dyn Fn(f32)>,
    /// C++ `QgsFeedback::isCanceled()` / `cancellationRequested()`
    pub is_cancelled: Option<&'a AtomicBool>,
}

impl DemProgress<'_> {
    #[must_use]
    pub const fn none() -> Self {
        Self {
            on_progress: None,
            is_cancelled: None,
        }
    }

    fn cancelled(&self) -> bool {
        self.is_cancelled
            .is_some_and(|flag| flag.load(Ordering::Relaxed))
    }

    fn report(&self, fraction: f32) {
        if let Some(callback) = self.on_progress {
            callback(fraction.clamp(0.0, 1.0));
        }
    }
}

impl fmt::Debug for DemProgress<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DemProgress")
            .field("has_callback", &self.on_progress.is_some())
            .field("cancelled", &self.cancelled())
            .finish()
    }
}

/// C++ `DemDownloadService.cpp:133-136` — 저장 위치 검사에 쓰는 파일 이름.
/// 경로가 비었거나 폴더 구분자로 끝나면 C++ `QFileInfo::fileName()` 이 비므로 `None`.
#[must_use]
pub fn dem_target_file_name(target_base_path: &str) -> Option<String> {
    let trimmed = target_base_path.trim();
    if trimmed.is_empty() {
        return None;
    }
    let name = match trimmed.rfind(['/', '\\']) {
        Some(index) => &trimmed[index + 1..],
        None => trimmed,
    };
    if name.is_empty() {
        None
    } else {
        Some(name.to_owned())
    }
}

fn curl_binary() -> &'static str {
    if cfg!(target_os = "windows") {
        "curl.exe"
    } else {
        "curl"
    }
}

/// curl 로 타일 한 장을 파일로 받는다. C++ `GDALOpenEx(/vsicurl/…)` 자리.
enum TileFetch {
    Saved(PathBuf),
    Missing,
    Cancelled,
    Failed,
}

fn fetch_tile(tile: DemTile, destination: &Path, progress: &DemProgress<'_>) -> TileFetch {
    if progress.cancelled() {
        return TileFetch::Cancelled;
    }
    let mut cmd = Command::new(curl_binary());
    cmd.args([
        "-sS",
        // C++ `ThreadConfiguration`(:35-37) — 연결 5초, 응답 없으면 15초 뒤 중단, 재시도 없음
        "--connect-timeout",
        "5",
        "--speed-time",
        "15",
        "--speed-limit",
        "1",
        // 404 를 실패로 만들어 반쪽 파일을 남기지 않는다 (C++ `isMissingTile` 대응)
        "-f",
        "-A",
        "ka-hgis/0.8 (Korean archaeology HGIS; contact: local)",
        "-o",
        &destination.to_string_lossy(),
        "-w",
        "%{http_code}",
        &tile.url(),
    ])
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let spawned = cmd.spawn();
    let Ok(mut child) = spawned else {
        return TileFetch::Failed;
    };
    // curl.exe 는 100ms 간격으로 취소를 살핀다. 자식 프로세스는 파이프가 막히지 않도록
    // stdout 이 작은 `%{http_code}` 하나뿐이다.
    loop {
        if progress.cancelled() {
            kill_child(&mut child);
            let _ = std::fs::remove_file(destination);
            return TileFetch::Cancelled;
        }
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => std::thread::sleep(Duration::from_millis(100)),
            Err(_) => {
                kill_child(&mut child);
                let _ = std::fs::remove_file(destination);
                return TileFetch::Failed;
            }
        }
    }
    let (status, stdout, stderr) = collect_child(&mut child);
    let http_code = String::from_utf8_lossy(&stdout).trim().to_owned();
    if status {
        return TileFetch::Saved(destination.to_path_buf());
    }
    let _ = std::fs::remove_file(destination);
    let diagnostic = format!("{http_code} {}", String::from_utf8_lossy(&stderr));
    if http_code == "404" || is_missing_tile(&diagnostic) {
        TileFetch::Missing
    } else {
        TileFetch::Failed
    }
}

fn kill_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

/// `(성공 여부, stdout, stderr)`
fn collect_child(child: &mut Child) -> (bool, Vec<u8>, Vec<u8>) {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    if let Some(mut pipe) = child.stdout.take() {
        let _ = pipe.read_to_end(&mut stdout);
    }
    if let Some(mut pipe) = child.stderr.take() {
        let _ = pipe.read_to_end(&mut stderr);
    }
    let success = child.wait().is_ok_and(|status| status.success());
    (success, stdout, stderr)
}

fn random_suffix() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    format!("{:06x}", (nanos ^ std::process::id()) & 0x00ff_ffff)
}

/// C++ `ReferenceMapPreparation::initializeStorage` (PreparedReferenceMap.cpp:17-28) —
/// 저장 폴더 옆에 자동 삭제 임시 폴더를 만든다.
fn create_temp_storage(target_base_path: &str) -> Option<PathBuf> {
    let base = Path::new(target_base_path);
    let parent = match base.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => dir.to_path_buf(),
        _ => PathBuf::from("."),
    };
    let storage = parent.join(format!("ka-hgis-reference-{}", random_suffix()));
    std::fs::create_dir_all(&storage).ok()?;
    Some(storage)
}

fn remove_dir_quietly(path: &Path) {
    let _ = std::fs::remove_dir_all(path);
}

/// C++ `GDALBuildVRT` + `GDALTranslate`(:213-281) 를 같은 GDAL 명령줄 도구로 부른다.
/// 이 PC 의 GDAL 은 `OSGeo4W\apps\gdal-dev\bin` 에 있고 PATH 에는 없을 수 있다.
fn gdal_tool(name: &str) -> Option<PathBuf> {
    if Command::new(name)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
    {
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
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    // C++ `DemDownloadService.cpp:85-93` tileUri 규칙
    #[test]
    fn test_dem_tile_name_matches_cpp_tile_uri_rule() {
        assert_eq!(
            tile_name(35, 129),
            "Copernicus_DSM_COG_10_N35_00_E129_00_DEM"
        );
        assert_eq!(
            tile_name(35, 128),
            "Copernicus_DSM_COG_10_N35_00_E128_00_DEM"
        );
        // 적도·그리니치·남반구·서경(자릿수 0 채움 포함)
        assert_eq!(tile_name(0, 0), "Copernicus_DSM_COG_10_N00_00_E000_00_DEM");
        assert_eq!(
            tile_name(-5, -73),
            "Copernicus_DSM_COG_10_S05_00_W073_00_DEM"
        );
        assert_eq!(
            tile_name(-1, -9),
            "Copernicus_DSM_COG_10_S01_00_W009_00_DEM"
        );
    }

    #[test]
    fn test_dem_tile_url_points_at_copernicus_bucket_with_plain_https_for_curl() {
        let tile = DemTile::new(35, 129);
        assert_eq!(
            tile.url(),
            "https://copernicus-dem-30m.s3.amazonaws.com/Copernicus_DSM_COG_10_N35_00_E129_00_DEM/Copernicus_DSM_COG_10_N35_00_E129_00_DEM.tif"
        );
        // C++ :93 원문은 /vsicurl/ 접두사가 붙은 GDAL 가상 경로다.
        assert_eq!(tile.vsicurl_uri(), format!("/vsicurl/{}", tile.url()));
    }

    // C++ `DemDownloadService.cpp:138-142` floor(min) / ceil(max)-1
    #[test]
    fn test_dem_tile_range_uses_floor_min_and_ceil_max_minus_one() {
        let range = tile_range(Wgs84Extent::new(128.1, 34.2, 129.9, 35.7));
        assert_eq!(
            range,
            DemTileRange {
                lon_from: 128,
                lon_to: 129,
                lat_from: 34,
                lat_to: 35,
            }
        );
        assert_eq!(range.tile_count(), 4);

        // 정수 경계에 딱 맞으면 그 칸 하나만 덮는다.
        let single = tile_range(Wgs84Extent::new(129.0, 35.0, 130.0, 36.0));
        assert_eq!(
            single,
            DemTileRange {
                lon_from: 129,
                lon_to: 129,
                lat_from: 35,
                lat_to: 35,
            }
        );
        assert_eq!(single.tile_count(), 1);

        // 폭 0 범위는 C++ 에서 칸 수 0 → 실패 경로
        let empty_range = tile_range(Wgs84Extent::new(129.0, 35.0, 129.0, 35.0));
        assert_eq!(empty_range.tile_count(), 0);
    }

    // C++ `DemDownloadService.cpp:160-161` 위도 바깥 · 경도 안쪽 순회
    #[test]
    fn test_dem_tiles_for_extent_orders_lat_outer_lon_inner() {
        let tiles = tiles_for_extent(Wgs84Extent::new(128.1, 34.2, 129.9, 35.7))
            .expect("범위 4칸은 상한 안이다");
        let names: Vec<String> = tiles.iter().map(|t| t.name()).collect();
        assert_eq!(
            names,
            vec![
                "Copernicus_DSM_COG_10_N34_00_E128_00_DEM".to_owned(),
                "Copernicus_DSM_COG_10_N34_00_E129_00_DEM".to_owned(),
                "Copernicus_DSM_COG_10_N35_00_E128_00_DEM".to_owned(),
                "Copernicus_DSM_COG_10_N35_00_E129_00_DEM".to_owned(),
            ]
        );
    }

    // C++ `kMaximumTiles = 24` (DemDownloadService.cpp:25, :143-147)
    #[test]
    fn test_dem_maximum_tiles_is_24_and_over_limit_reports_cpp_message() {
        assert_eq!(DEM_MAXIMUM_TILES, 24);
        // 5x5 = 25칸 → 거절
        assert_eq!(
            tiles_for_extent(Wgs84Extent::new(126.1, 33.1, 130.9, 37.9)),
            Err(DemTileListError::TooWide)
        );
        assert_eq!(dem_too_wide_ko(), "DEM을 받을 범위가 너무 넓습니다. 한 번에 최대 24칸까지 받을 수 있습니다. 조사구역으로 확대하고 다시 내려받으세요.");
        // 4x6 = 24칸 → 허용
        let allowed = tiles_for_extent(Wgs84Extent::new(126.1, 33.1, 129.9, 38.9))
            .expect("24칸은 상한과 같으므로 허용");
        assert_eq!(allowed.len(), 24);
    }

    // C++ `isMissingTile()` (DemDownloadService.cpp:96-100)
    #[test]
    fn test_dem_is_missing_tile_only_treats_404_as_missing() {
        assert!(is_missing_tile("HTTP response code: 404"));
        assert!(is_missing_tile("HTTP error code : 404"));
        assert!(is_missing_tile("curl: (22) 404 Not Found"));
        assert!(is_missing_tile("HTTP RESPONSE CODE: 404"));
        assert!(!is_missing_tile("HTTP response code: 500"));
        assert!(!is_missing_tile("Could not resolve host"));
        assert!(!is_missing_tile("Operation timed out after 15001 ms"));
    }

    // C++ `gridAligned()` (DemDownloadService.cpp:102-105)
    #[test]
    fn test_dem_grid_aligned_tolerates_one_part_in_hundred_thousand() {
        assert!(grid_aligned(3600.0, 1.0 / 3600.0));
        assert!(grid_aligned(1.0, 1.0 / 3600.0));
        assert!(!grid_aligned(0.000_5, 1.0 / 3600.0));
    }

    // C++ `DemDownloadService.cpp:127-132` 범위 검사
    #[test]
    fn test_dem_extent_validation_matches_cpp_limits() {
        assert!(Wgs84Extent::new(128.0, 34.0, 129.0, 35.0).is_valid_wgs84());
        assert!(!Wgs84Extent::new(-181.0, 34.0, 129.0, 35.0).is_valid_wgs84());
        assert!(!Wgs84Extent::new(128.0, 34.0, 181.0, 35.0).is_valid_wgs84());
        assert!(!Wgs84Extent::new(128.0, -91.0, 129.0, 35.0).is_valid_wgs84());
        assert!(!Wgs84Extent::new(128.0, 34.0, 129.0, 91.0).is_valid_wgs84());
        assert!(!Wgs84Extent::new(f64::NAN, 34.0, 129.0, 35.0).is_valid_wgs84());
        // 뒤집힌 범위는 C++ `QgsRectangle::isEmpty()`
        assert!(!Wgs84Extent::new(129.0, 35.0, 128.0, 34.0).is_valid_wgs84());
        assert_eq!(
            tiles_for_extent(Wgs84Extent::new(129.0, 35.0, 128.0, 34.0)),
            Err(DemTileListError::ExtentInvalid)
        );
    }

    // C++ `DemDownloadService.cpp:133-136` 저장 위치 검사 (`QFileInfo::fileName()`)
    #[test]
    fn test_dem_target_file_name_matches_qfileinfo_filename() {
        assert_eq!(
            dem_target_file_name(r"C:\조사\DEM\원본\dem.tif"),
            Some("dem.tif".to_owned())
        );
        assert_eq!(
            dem_target_file_name("/tmp/조사/dem.tif"),
            Some("dem.tif".to_owned())
        );
        assert_eq!(dem_target_file_name("dem.tif"), Some("dem.tif".to_owned()));
        assert_eq!(dem_target_file_name(r"C:\조사\DEM\원본\"), None);
        assert_eq!(dem_target_file_name("/tmp/조사/"), None);
        assert_eq!(dem_target_file_name(""), None);
        assert_eq!(dem_target_file_name("   "), None);
    }

    // 네트워크를 부르지 않는 실패 경로 — C++ :127-147 이 다운로드보다 먼저 끝난다.
    #[test]
    fn test_dem_prepare_rejects_bad_extent_and_missing_folder_without_network() {
        let bad = prepare(
            Wgs84Extent::new(200.0, 34.0, 201.0, 35.0),
            r"C:\조사\DEM\원본\dem.tif",
            &DemProgress::none(),
        );
        assert_eq!(bad.status, Some(DemStatus::Failed));
        assert_eq!(bad.error, DEM_EXTENT_INVALID_KO);
        assert!(bad.raster_uri.is_none());

        let no_storage = prepare(
            Wgs84Extent::new(129.0, 35.0, 130.0, 36.0),
            "   ",
            &DemProgress::none(),
        );
        assert_eq!(no_storage.error, DEM_NO_STORAGE_KO);

        let too_wide = prepare(
            Wgs84Extent::new(126.1, 33.1, 130.9, 37.9),
            r"C:\조사\DEM\원본\dem.tif",
            &DemProgress::none(),
        );
        assert_eq!(too_wide.error, dem_too_wide_ko());
    }

    // 취소는 어떤 호출도 하기 전에 끝난다 (C++ :126)
    #[test]
    fn test_dem_prepare_returns_cancelled_when_flag_is_already_set() {
        let cancel = AtomicBool::new(true);
        let result = prepare(
            Wgs84Extent::new(129.0, 35.0, 130.0, 36.0),
            r"C:\조사\DEM\원본\dem.tif",
            &DemProgress {
                on_progress: None,
                is_cancelled: Some(&cancel),
            },
        );
        assert_eq!(result.status, Some(DemStatus::Cancelled));
        assert_eq!(result.error, DEM_CANCELLED_KO);
    }

    #[test]
    fn test_dem_missing_warning_matches_cpp_text() {
        assert_eq!(
            dem_missing_tiles_warning_ko(2),
            "요청 범위 중 2칸은 바다이거나 DEM이 제공되지 않아 빈 영역으로 남습니다."
        );
    }
}
