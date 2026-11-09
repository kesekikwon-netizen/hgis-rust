//! DEM 타일 상한·저장 이름과 한국어 안내 문구 (C++ `DemDownloadService.cpp`).
//!
//! `dem_copernicus` 본문을 300줄 이하로 유지하기 위해 분리했다. 공개 경로는 그대로다.


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
