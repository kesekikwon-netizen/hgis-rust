//! Unit tests for `StrataAppState` startup, studio tabs, scale, and save-as target checks
//! (split from `app_state.rs` under the 300-line file rule).

use super::*;
use strata_types::schema::DomainLayerKey;

/// R3 — 저장이 실제 .gpkg 파일을 만들고, 다시 읽으면 좌표계·제목이 살아 있는지.
#[test]
fn test_r3_save_project_writes_real_gpkg_and_reopens() {
    let mut app = StrataAppState::new_startup();
    app.project_title = "병산동".to_owned();
    app.work_crs = WorkCrs::Epsg5186;
    app.left_layers
        .ensure_domain_layer(DomainLayerKey::SurveyArea);
    app.is_dirty = true;

    // 시험마다 다른 폴더를 쓴다. 고정 이름은 병렬 실행이나 동시 빌드에서
    // 다른 프로세스와 부딪혀 간헐적으로 실패한다(실제로 한 번 그렇게 실패했다).
    let dir = std::env::temp_dir().join(format!(
        "strata_r3_save_test_{}",
        std::process::id()
    ));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("병산동.gpkg");
    let _ = std::fs::remove_file(&path);

    assert!(app.write_survey_to(&path), "save should succeed");
    assert!(path.exists(), "the .gpkg must exist on disk");
    assert!(!app.is_dirty, "a successful save clears the dirty flag");

    // 다시 읽어 좌표계와 레이어가 살아 있는지 — 파일은 C++(GDAL) 조사 형식이다.
    let tables = strata_storage::survey_gpkg_read::read_survey_tables(&path)
        .expect("the file must be a readable survey GeoPackage");
    assert!(tables.iter().any(|t| t.table == "survey_area"));
    let srs: i64 = rusqlite::Connection::open(&path)
        .and_then(|c| {
            c.query_row(
                "SELECT srs_id FROM gpkg_geometry_columns WHERE table_name = 'survey_area'",
                [],
                |r| r.get(0),
            )
        })
        .expect("geometry column");
    assert_eq!(srs, 5186);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn startup_opens_home_tab_and_drawing_ribbon_tool_switches_to_map_tab() {
    let mut app = StrataAppState::new_startup();
    assert_eq!(app.active_tab, MainTab::Home);
    assert_eq!(app.work_crs, WorkCrs::Epsg5187);
    // C++ `KaSurveyBadge.cpp:40-44` windowTitleFor — 조사가 없으면 「Strata」.
    assert_eq!(app.window_title(), "Strata");
    assert!(app.left_layers.entries.is_empty());

    app.active_tab = MainTab::Drawing;
    app.trigger_map_ribbon_tool(MapToolKind::TrenchPlace);
    assert_eq!(app.active_tab, MainTab::Map);
    assert_eq!(app.active_tool, MapToolKind::TrenchPlace);

    app.is_dirty = true;
    assert_eq!(app.window_title(), "Strata *");
    app.survey_path = Some(std::path::PathBuf::from("병산동.gpkg"));
    assert_eq!(app.window_title(), "병산동 * - Strata");
    app.is_dirty = false;
    assert_eq!(app.window_title(), "병산동 - Strata");
    app.survey_path = None;

    app.open_studio_tab(MainTab::Drawing);
    assert_eq!(app.active_tab, MainTab::Drawing);
    assert!(app.open_studio_tabs.contains(&MainTab::Drawing));

    app.active_tab = MainTab::Map;
    assert!(app.open_studio_tabs.contains(&MainTab::Drawing));

    app.open_studio_tab(MainTab::Drawing);
    app.close_studio_tab(MainTab::Drawing);
    assert_eq!(app.active_tab, MainTab::Map);
    assert!(!app.open_studio_tabs.contains(&MainTab::Drawing));
}

#[test]
fn test_project_lifecycle_creates_and_reopens_4_default_layers_and_epsg_5186() {
    let tmp_path =
        std::env::temp_dir().join(format!("strata_test_project_{}.qgs", std::process::id()));
    let mut creator = StrataAppState::new_startup();
    let config = ProjectTemplateConfig::default();
    let created = creator.create_new_project(&tmp_path, &config);
    assert!(created.is_some());

    let mut reader = StrataAppState::new_startup();
    assert_eq!(reader.active_tab, MainTab::Home);
    assert!(reader.open_project(&tmp_path));
    assert_eq!(reader.active_tab, MainTab::Map);
    assert_eq!(reader.work_crs.auth_id(), "EPSG:5186");
    assert_eq!(reader.left_layers.entries.len(), 4);

    let _ = std::fs::remove_file(&tmp_path);
}

#[test]
fn test_status_bar_scale_1000_from_0264583333m_and_dirty_label_and_studio_tab_close() {
    let mpp = 0.264_583_333_333_333_3_f64;
    let scale =
        StrataAppState::compute_representative_scale(mpp, StrataAppState::HGIS_STANDARD_DPI)
            .expect("valid scale");
    assert!((scale - 1000.0).abs() < 1e-4);
    assert_eq!(
        StrataAppState::format_representative_scale(mpp, StrataAppState::HGIS_STANDARD_DPI),
        "1:1,000"
    );
    assert_eq!(
        StrataAppState::format_representative_scale(-1.0, StrataAppState::HGIS_STANDARD_DPI),
        "1: ---"
    );
    assert_eq!(StrataAppState::dirty_status_label_ko(true), "● 저장 필요");
    assert_eq!(StrataAppState::dirty_status_label_ko(false), "✓ 저장됨");

    let mut app = StrataAppState::new_startup();
    app.open_studio_tab(MainTab::Drawing);
    app.open_studio_tab(MainTab::Paleo);
    app.active_tab = MainTab::Drawing;
    assert_eq!(app.open_studio_tabs.len(), 2);
    app.close_studio_tab(MainTab::Drawing);
    assert_eq!(app.open_studio_tabs.len(), 1);
    assert_eq!(app.active_tab, MainTab::Map);
}

/// C++ `MainWindowSession.cpp:1438-1462` — 다른 이름으로 저장 대상 검사. 있는 .gpkg·같은
/// 이름 .qgz 는 덮어쓰지 않고, 열린 조사와 같은 파일(대소문자 무시)은 허용한다.
/// .gpkg 로 끝나지 않는 이름은 확장자를 바꾸지 않고 뒤에 붙인다.
#[test]
fn save_as_target_refuses_existing_survey_files_but_allows_the_open_one() {
    use super::app_state_projects::resolve_save_as_target;
    const REFUSED: &str = "같은 이름의 조사 파일 또는 작업공간 사본이 있습니다. 기존 자료를 보존하기 위해 덮어쓰지 않았습니다. 사용하지 않은 이름으로 저장하세요.";
    let dir = tempfile::tempdir().expect("temp dir");
    let d = dir.path();
    for name in ["기존.gpkg", "원본.qgz", "Open.gpkg"] {
        std::fs::write(d.join(name), b"x").expect("seed file");
    }
    let open = d.join("Open.gpkg");

    assert_eq!(resolve_save_as_target(&d.join("기존.gpkg"), Some(&open)), Err(REFUSED));
    // .qgz 를 고르면 같은 폴더 같은 이름 .gpkg 가 대상 — .qgz 가 있으니 거절
    assert_eq!(resolve_save_as_target(&d.join("원본.qgz"), None), Err(REFUSED));
    // .gpkg 를 골라도 같은 이름 .qgz 가 있으면 거절
    assert_eq!(resolve_save_as_target(&d.join("원본.gpkg"), None), Err(REFUSED));
    // 열린 조사와 같은 파일은 허용(대소문자 무시)
    assert_eq!(
        resolve_save_as_target(&d.join("OPEN.GPKG"), Some(&open)),
        Ok(d.join("OPEN.GPKG"))
    );
    // 열린 조사가 없으면 같은 이름 파일은 거절
    assert_eq!(resolve_save_as_target(&open, None), Err(REFUSED));
    // 확장자를 바꾸지 않고 뒤에 붙인다: 「조사.2차」 → 「조사.2차.gpkg」
    assert_eq!(
        resolve_save_as_target(&d.join("조사.2차"), Some(&open)),
        Ok(d.join("조사.2차.gpkg"))
    );
    assert_eq!(
        resolve_save_as_target(&d.join("새이름.qgz"), None),
        Ok(d.join("새이름.gpkg"))
    );
}
