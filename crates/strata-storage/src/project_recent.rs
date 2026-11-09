//! 理쒓렐 ?꾨줈?앺듃 紐⑸줉怨??뚰겕?ㅽ럹?댁뒪 ?뚯씪 ?숈옉.
//!
//! `project_actions` ?먯꽌 遺꾨━?덈떎.

use std::io::Read;
use std::path::{Path, PathBuf};

use strata_types::error::StrataResult;

use crate::cad_csv_io::{extract_dxf_elevation_points, parse_control_points_csv, ControlPointRow};
use crate::gpkg::{GpkgRepository, SurveyWorkspace};
use crate::session_save::persist_workspace;

pub const HGIS_MAX_RECENT_PROJECTS: usize = 10;

pub const HGIS_SQLITE_MAGIC: &[u8; 16] = b"SQLite format 3\0";

#[derive(Debug, Clone, Default)]
pub struct RecentProjectManager {
    pub recent_paths: Vec<PathBuf>,
}

impl RecentProjectManager {
    #[must_use]
    pub fn new() -> Self {
        Self {
            recent_paths: Vec::new(),
        }
    }

    /// C++ `KaProjectManager::addRecentProject` — 중복 제거 후 최상단(0번 인덱스) 삽입 및 최대 10개 상한 유지
    pub fn add_recent_project(&mut self, project_path: PathBuf) {
        self.recent_paths.retain(|p| p != &project_path);
        self.recent_paths.insert(0, project_path);
        if self.recent_paths.len() > HGIS_MAX_RECENT_PROJECTS {
            self.recent_paths.truncate(HGIS_MAX_RECENT_PROJECTS);
        }
    }

    /// C++ `KaProjectManager::pruneMissingRecentProjects` — 삭제·이동된 경로 자동 제거
    pub fn prune_missing_recent_projects(&mut self) -> Vec<PathBuf> {
        self.recent_paths.retain(|p| p.exists());
        self.recent_paths.clone()
    }

    /// C++ `KaProjectManager::verifyGpkgMagicBytes` — SQLite3 16바이트 헤더 매직(`"SQLite format 3\0"`) 사전 검사
    #[must_use]
    pub fn verify_gpkg_magic_bytes(file_path: &Path) -> bool {
        let Ok(mut file) = std::fs::File::open(file_path) else {
            return false;
        };
        let mut header = [0u8; 16];
        if file.read_exact(&mut header).is_err() {
            return false;
        }
        &header == HGIS_SQLITE_MAGIC
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WorkspaceFileActions;

impl WorkspaceFileActions {
    pub fn save_and_reopen_gpkg(
        path: &Path,
        ws: &mut SurveyWorkspace,
    ) -> StrataResult<SurveyWorkspace> {
        persist_workspace(ws, path)?;
        GpkgRepository::open_read_only(path)
    }

    pub fn import_dxf_contour_points(dxf_path: &Path) -> StrataResult<Vec<[f64; 3]>> {
        extract_dxf_elevation_points(dxf_path)
    }

    pub fn import_control_points_csv(csv_path: &Path) -> StrataResult<Vec<ControlPointRow>> {
        let bytes = std::fs::read(csv_path)?;
        parse_control_points_csv(&bytes)
    }
}

/// 외장 USB 드라이브 문자가 변경되어 마운트된 경우(예: `"D:/surveys/site.gpkg"` vs `"E:/surveys/site.gpkg"`) 드라이브 문자를 제외한 나머지 경로가 동일한지 판정
/// (C++ `RecentSurveys::sameRestOnOtherDrive` 1:1 대응)
#[must_use]
pub fn same_rest_on_other_drive(path_a: &str, path_b: &str) -> bool {
    let clean = |s: &str| s.trim().replace('\\', "/");
    let x = clean(path_a);
    let y = clean(path_b);
    let has_drive = |s: &str| {
        let b = s.as_bytes();
        b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && b[2] == b'/'
    };
    if !has_drive(&x) || !has_drive(&y) {
        return false;
    }
    if x.as_bytes()[0].eq_ignore_ascii_case(&y.as_bytes()[0]) {
        return false;
    }
    x[2..].eq_ignore_ascii_case(&y[2..])
}
