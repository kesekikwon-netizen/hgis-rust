use std::path::PathBuf;

use strata_types::crs::WorkCrs;
use strata_types::error::StrataResult;
use strata_types::schema::DomainLayerKey;

use crate::gpkg::GpkgRepository;
use crate::project_actions::{
    enforce_standard_layer_z_order, evaluate_workflow_guide, forced_layer_order_note_ko,
    layer_recovery_backoff_seconds, repair_broken_layer_source, same_rest_on_other_drive,
    LayerRole, RecentProjectManager, WorkspaceFileActions, WorkflowProjectInput,
};

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
    assert!(reopened
        .layers
        .contains_key(DomainLayerKey::SurveyArea.id()));

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
fn test_mru_max_limit_12_to_10_prune_3_missing_to_7_and_verify_gpkg_magic_bytes(
) -> StrataResult<()> {
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

    // RecentSurveys::sameRestOnOtherDrive
    assert!(same_rest_on_other_drive(
        "D:/surveys/gyeongju.gpkg",
        "E:\\surveys\\gyeongju.gpkg"
    ));
    assert!(!same_rest_on_other_drive(
        "D:/surveys/gyeongju.gpkg",
        "D:/surveys/gyeongju.gpkg"
    ));

    // TestWorkflowGuide::testStepEvaluation
    let wf = evaluate_workflow_guide(&WorkflowProjectInput {
        survey_name: "경주 성동동 시굴".to_owned(),
        layer_count: 2,
        survey_area_count: 1,
        feature_poly_count: 0,
        feature_line_count: 0,
        control_points_count: 4,
        has_datum: true,
        has_ellipsoid: true,
        has_projection: true,
        has_visible_reference_layer: true,
        checklist_error_count: 1,
        package_created: false,
    });
    assert!(wf[0].completed); // Step 1: 새 조사
    assert!(wf[1].completed); // Step 2: 배경·지적도
    assert!(wf[2].completed); // Step 3: 조사구역
    assert!(!wf[3].completed); // Step 4: 유구·단면선 미완료
    assert!(wf[4].completed); // Step 5: GPS 기준점 4개 완료

    // TestLayerTreePolicy::testZOrderEnforcement: BaseMapSatellite moved to bottom
    let mut tree = vec![
        ("배경지도".to_owned(), LayerRole::BaseMapSatellite),
        ("유구".to_owned(), LayerRole::FeatureBoundaries),
        ("조사구역".to_owned(), LayerRole::SurveyBoundary),
    ];
    enforce_standard_layer_z_order(&mut tree);
    assert_eq!(tree[0].0, "조사구역");
    assert_eq!(tree.last().map(|x| x.0.as_str()), Some("배경지도"));
    assert!(
        forced_layer_order_note_ko("브이월드 위성", true, false, false)
            .contains("맨 아래에 그립니다")
    );

    // TestLayerTreeRecovery::testSameDirectoryRepair & backoffSeconds
    assert_eq!(layer_recovery_backoff_seconds(1), 15);
    assert_eq!(layer_recovery_backoff_seconds(2), 30);
    assert_eq!(layer_recovery_backoff_seconds(6), 300);

    let moved_file = dir.path().join("features.gpkg");
    std::fs::write(&moved_file, b"SQLite format 3\0")?;
    let broken = PathBuf::from("D:/old_unplugged_usb/features.gpkg");
    let rep = repair_broken_layer_source(&broken, dir.path());
    assert!(rep.is_recovered);
    assert_eq!(rep.new_resolved_path, moved_file);
    assert_eq!(rep.recovery_method, "SameDirectoryFallback");
    Ok(())
}
