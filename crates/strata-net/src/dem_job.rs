//! DEM 내려받기 작업 상태·진행 알림과 저장 파일 이름.
//!
//! `dem_copernicus` 본문을 300줄 이하로 유지하기 위해 분리했다. 공개 경로는 그대로다.

use super::*;

use std::fmt;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

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

    pub(super) fn failed(message: &str) -> Self {
        Self {
            status: Some(DemStatus::Failed),
            error: message.to_owned(),
            ..Self::default()
        }
    }

    pub(super) fn cancelled() -> Self {
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

    pub(super) fn cancelled(&self) -> bool {
        self.is_cancelled
            .is_some_and(|flag| flag.load(Ordering::Relaxed))
    }

    pub(super) fn report(&self, fraction: f32) {
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
