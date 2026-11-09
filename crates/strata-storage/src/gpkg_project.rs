//! 프로젝트 열기 사전 점검·최근 목록·프로젝트 이름·경도 눈금.
//!
//! `gpkg` 에서 분리했다.

use std::path::Path;

use serde::{Deserialize, Serialize};

use strata_types::crs::WorkCrs;

use crate::gpkg_schema::GPKG_MAGIC_HEADER;

pub const HGIS_MAX_RECENT_PROJECTS: usize = 10;

pub const HGIS_AUTOSAVE_INTERVAL_SEC: u64 = 300;

pub const HGIS_LOCK_FILE_EXT: &str = ".hgis.lock";

pub const HGIS_AUTOSAVE_FILE_EXT: &str = ".hgis.autosave";

/// C++ `KaProjectManager::ProjectOpenResult` 1:1 열거형
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectOpenResult {
    Success,
    FileNotFound,
    LockedByAnotherProcess,
    CrashDetectedNeedsRecovery,
    InvalidMagicBytes,
    FutureVersionBlocked,
}

/// C++ `KaProjectManager::openProject` 사전 검사(`.hgis.lock`, `.hgis.autosave`, SQLite 16바이트 매직 헤더)
#[must_use]
pub fn check_project_open_preflight(project_path: &Path, force_unlock: bool) -> ProjectOpenResult {
    if !project_path.exists() {
        return ProjectOpenResult::FileNotFound;
    }
    let lock_path =
        std::path::PathBuf::from(format!("{}{HGIS_LOCK_FILE_EXT}", project_path.display()));
    if lock_path.exists() && !force_unlock {
        return ProjectOpenResult::LockedByAnotherProcess;
    }
    let autosave_path = std::path::PathBuf::from(format!(
        "{}{HGIS_AUTOSAVE_FILE_EXT}",
        project_path.display()
    ));
    let wal_path = std::path::PathBuf::from(format!("{}.wal", project_path.display()));
    if autosave_path.exists() || wal_path.exists() {
        return ProjectOpenResult::CrashDetectedNeedsRecovery;
    }
    let Ok(bytes) = std::fs::read(project_path) else {
        return ProjectOpenResult::FileNotFound;
    };
    if bytes.len() < GPKG_MAGIC_HEADER.len() || &bytes[..16] != GPKG_MAGIC_HEADER {
        return ProjectOpenResult::InvalidMagicBytes;
    }
    ProjectOpenResult::Success
}

/// C++ `KaProjectManager` 최근 프로젝트 MRU 목록 갱신 (중복 제거 후 최상단 배치, 최대 `HGIS_MAX_RECENT_PROJECTS = 10`개 유지)
pub fn push_recent_project_mru(recent_list: &mut Vec<String>, new_path: &str) {
    recent_list.retain(|p| p != new_path);
    recent_list.insert(0, new_path.to_owned());
    if recent_list.len() > HGIS_MAX_RECENT_PROJECTS {
        recent_list.truncate(HGIS_MAX_RECENT_PROJECTS);
    }
}

/// 중부원점 기준 경도 (128.0° E, C++ `HGIS_CENTRAL_MERIDIAN_LON`)
pub const HGIS_CENTRAL_MERIDIAN_LON: f64 = 128.0;

/// 한국 중부원점 GRS80 기본 EPSG 코드 (C++ `HGIS_DEFAULT_EPSG_CODE`)
pub const HGIS_DEFAULT_EPSG_CODE: u32 = 5186;

/// 프로젝트명 최대 허용 길이 (C++ `HGIS_MAX_PROJECT_NAME_LEN`)
pub const HGIS_MAX_PROJECT_NAME_LEN: usize = 128;

/// C++ `KaLongitudeRuler` 8개 기준 도시 및 경도(`126°~130°`)
pub const KA_LONGITUDE_RULER_CITIES: [(&str, f64); 8] = [
    ("제주", 126.53),
    ("광주", 126.85),
    ("대전", 127.38),
    ("춘천", 127.73),
    ("대구", 128.60),
    ("강릉", 128.90),
    ("부산", 129.08),
    ("울산", 129.31),
];

/// C++ `ProjectCreationError` 1:1 대응
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectCreationError {
    InvalidSurveyName,
    WindowsReservedFileName,
    DirectoryAlreadyExists,
    CrsOutOfRange,
    DiskWritePermissionDenied,
}

/// 윈도우 예약 파일명(`CON`, `PRN`, `AUX`, `NUL`, `COM1..9`, `LPT1..9`) 및 금지 특수문자 검증
/// (C++ `SurveyProjectFactory::isValidProjectName` 1:1 대응)
#[must_use]
pub fn is_valid_project_name(name: &str) -> bool {
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed.chars().count() > HGIS_MAX_PROJECT_NAME_LEN {
        return false;
    }
    if trimmed.chars().all(|c| c == '.' || c.is_whitespace()) {
        return false;
    }
    let stem = trimmed
        .split('.')
        .next()
        .unwrap_or(trimmed)
        .trim()
        .to_ascii_uppercase();
    const RESERVED: [&str; 22] = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    if RESERVED.contains(&stem.as_str()) {
        return false;
    }
    !trimmed
        .chars()
        .any(|c| matches!(c, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|'))
}

/// C++ `KaNewSurveyDialog::nameProblem` 한글 안내 문구 검증 1:1 대응
pub fn validate_survey_name_ko(raw: &str) -> Result<String, &'static str> {
    // C++ `KaNewSurveyDialog.cpp:196-212` `nameProblem` 문구 1:1
    let s = raw.trim();
    if s.is_empty() {
        return Err("조사명을 입력하세요.");
    }
    if s.chars()
        .any(|c| matches!(c, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || (c as u32) < 0x20)
    {
        return Err("조사명에는 \\ / : * ? \" < > | 와 제어 문자를 쓸 수 없습니다.");
    }
    if s.ends_with('.') {
        return Err("조사명은 마침표(.)로 끝날 수 없습니다. Windows가 파일 이름 끝의 점을 지웁니다.");
    }
    let stem = s.trim_end_matches(|c: char| c == '.' || c.is_whitespace());
    if stem.is_empty() {
        return Err("조사명은 마침표(.)로 끝날 수 없습니다. Windows가 파일 이름 끝의 점을 지웁니다.");
    }
    let upper = stem
        .split('.')
        .next()
        .unwrap_or(stem)
        .trim()
        .to_ascii_uppercase();
    const RESERVED: [&str; 22] = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    if RESERVED.contains(&upper.as_str()) {
        // C++ 는 이름을 넣어 보여 준다. 정적 &str 로는 이름을 끼울 수 없어 같은 뜻의 문장을 쓴다.
        return Err("Windows가 장치 이름으로 쓰는 예약 이름이라 조사명으로 쓸 수 없습니다.");
    }
    if s.chars().count() > 80 {
        return Err("조사명은 80자 이하로 줄여 주세요.");
    }
    Ok(s.to_owned())
}

/// 동경 128° 기준 자오선 판별 (C++ `KaLongitudeRuler::isCentralMeridian128`)
#[must_use]
pub const fn is_central_meridian_128(epsg_code: u32) -> bool {
    epsg_code == HGIS_DEFAULT_EPSG_CODE
}

/// 경도에 따른 중부원점(5186) / 동부원점(5187) 추천 좌표계 반환 (`128.0°` 경계)
#[must_use]
pub fn recommend_crs_for_longitude(lon_deg: f64) -> WorkCrs {
    if lon_deg >= HGIS_CENTRAL_MERIDIAN_LON {
        WorkCrs::Epsg5187
    } else {
        WorkCrs::Epsg5186
    }
}

/// 경도 눈금 배열 생성 (C++ `KaLongitudeRuler::calculateTicks`)
#[must_use]
pub fn calculate_longitude_ticks(min_lon: f64, max_lon: f64, step_degree: f64) -> Vec<f64> {
    if !min_lon.is_finite() || !max_lon.is_finite() || step_degree <= 1e-6 || max_lon < min_lon {
        return Vec::new();
    }
    let count = (((max_lon - min_lon) / step_degree).floor() as usize).min(10_000);
    (0..=count)
        .map(|i| min_lon + (i as f64) * step_degree)
        .collect()
}
