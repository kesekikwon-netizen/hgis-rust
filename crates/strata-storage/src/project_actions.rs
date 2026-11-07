use crate::cad_csv_io::{
    extract_dxf_elevation_points, parse_control_points_csv, ControlPointRow,
};
use crate::gpkg::{GpkgRepository, SurveyWorkspace};
use crate::session_save::persist_workspace;
use std::path::Path;
use strata_types::error::StrataResult;

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
}
