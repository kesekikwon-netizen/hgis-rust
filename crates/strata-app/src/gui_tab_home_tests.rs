//! `gui_tab_home` 시험 — 새 조사가 빈 프로젝트에서 시작하는지, 자기 레이어만 읽는지.
//!
//! 원래 `gui_tab_home.rs` 안 `mod tests_new_survey` 를 그대로 옮겼다.
//! `#[path]` 로 붙인 시험 모듈이라 `super::*` 가 창구를 가리키지 않으므로 명시한다.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use strata_types::schema::DomainLayerKey;

use crate::app_state::{MainTab, StrataAppState};

/// 새 조사는 이전 조사의 경계·시굴·지적·유산·장면을 하나도 물려받지 않는다.
/// C++ `MainWindow::newSurvey` 는 `kaSafeClearQgisProject` 로 프로젝트를 비운 뒤
/// 방금 만든 빈 GPKG 만 읽는다(`MainWindowSession.cpp:703‑726`).
#[test]
fn test_new_survey_starts_empty_and_keeps_no_previous_survey() {
    let mut app = StrataAppState::new_startup();

    // 이전 조사를 흉내 낸다: 경계·시굴·지적·유산·장면·레이어가 모두 있는 상태.
    app.project_title = "이전조사".to_owned();
    app.survey_path = Some(std::path::PathBuf::from("이전조사.gpkg"));
    app.is_dirty = true;
    app.left_layers
        .ensure_domain_layer(DomainLayerKey::SurveyArea);
    app.left_layers
        .ensure_domain_layer(DomainLayerKey::TrialTrench);
    app.home_map.scene.polygons.push(
        strata_canvas2d::painter_scene::WorldPolygonItem {
            layer_key: "survey_area".to_owned(),
            world_pts: vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]],
            fill_rgba: [0, 0, 0, 255],
            stroke_rgba: [0, 0, 0, 255],
        },
    );
    app.home_map.real_survey.survey_rings = vec![vec![
        [219_754.87, 361_410.69],
        [219_712.66, 361_397.50],
        [219_683.64, 360_632.36],
    ]];
    app.home_map.real_survey.trenches.push(
        crate::real_survey::RealTrenchPoly {
            name: "이전 시굴".to_owned(),
            ring: vec![[1.0, 1.0], [2.0, 1.0], [2.0, 2.0]],
            area_m2: 1.0,
            center: [1.5, 1.5],
        },
    );
    app.home_map.real_survey.cadastral_parcels.push(
        crate::real_survey::RealCadastralParcel {
            jibun: "76 전".to_owned(),
            ring: vec![[1.0, 1.0], [2.0, 1.0], [2.0, 2.0]],
            center: [1.5, 1.5],
        },
    );
    app.home_map.real_survey.heritage_polys.push(
        crate::real_survey::RealHeritagePoly {
            layer_name: "국가지정유산".to_owned(),
            site_name: "이전 유산".to_owned(),
            is_green: false,
            ring: vec![[1.0, 1.0], [2.0, 1.0], [2.0, 2.0]],
            center: [1.5, 1.5],
        },
    );
    app.home_map.selected_feature_idx = Some(0);
    app.align.target_layer = Some("이전 도면".to_owned());
    app.ribbon.sub_toolbar_visible = true;

    // 새 조사를 시작한다.
    app.start_new_survey_project("새조사");

    assert_eq!(app.project_title, "새조사");
    assert_eq!(app.active_tab, MainTab::Map);
    assert!(!app.is_dirty, "a brand-new survey has nothing to save yet");
    assert!(
        app.home_map.real_survey.survey_rings.is_empty(),
        "the new survey must not inherit the previous survey boundary"
    );
    assert!(
        app.home_map.real_survey.trenches.is_empty(),
        "the new survey must not inherit the previous trenches"
    );
    assert!(
        app.home_map.real_survey.cadastral_parcels.is_empty(),
        "the new survey must not inherit the previous cadastral parcels"
    );
    assert!(
        app.home_map.real_survey.heritage_polys.is_empty(),
        "the new survey must not inherit the previous heritage polygons"
    );
    assert!(
        app.home_map.scene.polygons.is_empty(),
        "the new survey must not inherit the previous map scene"
    );
    assert_eq!(
        app.home_map.selected_feature_idx, None,
        "the new survey must not keep a selected feature from the old one"
    );
    assert_eq!(
        app.align.target_layer, None,
        "the new survey must not keep the previous align session"
    );
    assert!(
        !app.ribbon.sub_toolbar_visible,
        "the new survey must not keep the previous sub toolbar"
    );
    assert_eq!(
        app.home_map.real_survey.layer_visible.len(),
        0,
        "layer visibility belongs to the previous survey"
    );
}

/// 새 조사는 방금 만든 파일에서 읽은 레이어만 갖는다(이전 레이어를 이어받지 않는다).
#[test]
fn test_new_survey_reads_only_its_own_layers() {
    let mut app = StrataAppState::new_startup();
    app.left_layers
        .ensure_domain_layer(DomainLayerKey::FeatureLine);
    app.left_layers
        .ensure_domain_layer(DomainLayerKey::ArtifactPoint);

    let dir = std::env::temp_dir().join(format!(
        "strata_new_survey_test_{}",
        std::process::id()
    ));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("새조사_레이어.gpkg");
    let _ = std::fs::remove_file(&path);
    strata_storage::gpkg::GpkgRepository::create_new_survey(
        &path,
        strata_types::crs::WorkCrs::Epsg5187,
    )
    .expect("the new survey file must be created");

    app.start_new_survey_project_with_path(
        "새조사_레이어",
        &path,
        strata_types::crs::WorkCrs::Epsg5187,
    );
    for entry in &app.left_layers.entries {
        assert_ne!(
            entry.layer_key, "feature_line",
            "a layer from the previous survey must not survive"
        );
        assert_ne!(
            entry.layer_key, "artifact_point",
            "a layer from the previous survey must not survive"
        );
    }
    assert_eq!(app.survey_path.as_deref(), Some(path.as_path()));
    let _ = std::fs::remove_file(&path);
}
