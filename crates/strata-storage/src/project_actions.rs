use crate::cad_csv_io::{
    extract_dxf_elevation_points, parse_control_points_csv, ControlPointRow,
};
use crate::gpkg::{GpkgRepository, SurveyWorkspace};
use crate::session_save::persist_workspace;
use std::io::Read;
use std::path::{Path, PathBuf};
use strata_types::error::StrataResult;

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

#[cfg(test)]
mod tests {
    use super::*;
    use strata_types::crs::WorkCrs;
    use strata_types::schema::DomainLayerKey;

    #[test]
    fn workspace_file_actions_save_reopen_and_import_dxf_csv_roundtrip() -> StrataResult<()> {
        let dir = tempfile::tempdir()?;
        let gpkg_path = dir.path().join("action_test.gpkg");
        let mut ws = GpkgRepository::create_new_survey(&gpkg_path, WorkCrs::Epsg5187)?;
        let buf = ws.ensure_domain_layer(DomainLayerKey::SurveyArea);
        buf.add_unsaved_feature(
            String::from("POLYGON((218500 359500, 218550 359500, 218550 359550, 218500 359500))"),
            std::collections::BTreeMap::new(),
        );

        let reopened = WorkspaceFileActions::save_and_reopen_gpkg(&gpkg_path, &mut ws)?;
        assert_eq!(reopened.schema_version, 3);
        assert!(reopened.layers.contains_key(DomainLayerKey::SurveyArea.id()));

        let dxf_path = dir.path().join("sample_contour.dxf");
        std::fs::write(
            &dxf_path,
            "0\nSECTION\n2\nENTITIES\n0\nPOINT\n10\n218500.0\n20\n359500.0\n30\n48.2\n0\nENDSEC\n0\nEOF\n",
        )?;
        let pts = WorkspaceFileActions::import_dxf_contour_points(&dxf_path)?;
        assert_eq!(pts.len(), 1);
        assert!((pts[0][2] - 48.2).abs() < 1e-6);

        let csv_path = dir.path().join("cp.csv");
        std::fs::write(&csv_path, "CP-01,218510.0,359510.0,45.5\n")?;
        let cps = WorkspaceFileActions::import_control_points_csv(&csv_path)?;
        assert_eq!(cps.len(), 1);
        assert_eq!(cps[0].point_id, "CP-01");
        Ok(())
    }

    #[test]
    fn test_mru_max_limit_12_to_10_prune_3_missing_to_7_and_verify_gpkg_magic_bytes() -> StrataResult<()> {
        let dir = tempfile::tempdir()?;
        let mut mgr = RecentProjectManager::new();
        for i in 1..=12 {
            let p = dir.path().join(format!("proj{i}.gpkg"));
            GpkgRepository::create_new_survey(&p, WorkCrs::Epsg5186)?;
            mgr.add_recent_project(p);
        }
        assert_eq!(mgr.recent_paths.len(), 10);
        assert!(mgr.recent_paths[0].ends_with("proj12.gpkg"));

        for del_idx in [11, 9, 7] {
            std::fs::remove_file(dir.path().join(format!("proj{del_idx}.gpkg")))?;
        }
        let valid = mgr.prune_missing_recent_projects();
        assert_eq!(valid.len(), 7);

        let txt_path = dir.path().join("corrupt.gpkg");
        std::fs::write(&txt_path, b"Hello World")?;
        assert!(!RecentProjectManager::verify_gpkg_magic_bytes(&txt_path));
        assert!(RecentProjectManager::verify_gpkg_magic_bytes(&valid[0]));
        Ok(())
    }
}
