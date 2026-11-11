//! 열기·그리기·저장·다시 열기가 실제 조사 파일로 이어지는지 본다.
//! C++ 쪽 정답 자료 `samples/demo_survey/demo.gpkg` 는 임시 폴더 사본으로만 쓴다.

use std::path::PathBuf;

use strata_canvas2d::map_tools::MapToolKind;
use strata_storage::survey_gpkg_read::read_survey_gpkg;
use strata_types::crs::WorkCrs;
use strata_types::schema::DomainLayerKey as K;

use crate::app_state::StrataAppState;

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("strata_doc_bridge_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn demo_copy(tag: &str) -> PathBuf {
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg");
    let path = temp_dir(tag).join("demo.gpkg");
    std::fs::copy(src, &path).expect("demo.gpkg 사본을 만들 수 있어야 한다");
    path
}

fn has_layer(app: &StrataAppState, key: K) -> bool {
    app.left_layers.entries.iter().any(|e| e.layer_key == key.id())
}

#[test]
fn opening_a_cpp_survey_shows_its_shapes_and_layers() {
    let path = demo_copy("open");
    let mut app = StrataAppState::new_startup();
    app.open_survey_gpkg(&path);

    assert_eq!(app.work_crs, WorkCrs::Epsg5186);
    assert_eq!(app.survey_path.as_deref(), Some(path.as_path()));
    // 지도에 그려질 것: 조사구역 1, 시굴 1, 그리고 문서의 유구 면 2·선 1·단면선 1·기준점 2·유물 1
    assert_eq!(app.home_map.real_survey.survey_rings.len(), 1);
    assert_eq!(app.home_map.real_survey.trenches.len(), 1);
    assert_eq!(app.home_map.real_survey.trenches[0].name, "TR1");
    assert_eq!(app.survey_doc.layer(K::FeaturePoly).features.len(), 2);
    assert_eq!(app.survey_doc.layer(K::ControlPoints).features.len(), 2);
    // 화면이 그 조사 위치로 간다(경주 고정 좌표가 아니다).
    let c = app.home_map.viewport.center_xy;
    // 조사구역은 X 199990~200110, Y 449990~450090 이다. 화면 중심이 그 안에 있어야 한다.
    assert!(c[0] > 199_990.0 && c[0] < 200_110.0 && c[1] > 449_990.0 && c[1] < 450_090.0, "center = {c:?}");
    // 도형이 있는 레이어 7개가 모두 목록에 오른다.
    for key in K::ALL {
        assert!(has_layer(&app, key), "{} 가 목록에 없다", key.name_ko());
    }
    assert!(!app.is_dirty);
}

#[test]
fn drawn_shapes_are_saved_and_come_back_after_reopen() {
    let dir = temp_dir("draw");
    let path = dir.join("현장.gpkg");
    strata_storage::survey_gpkg_write::create_standard_survey(&path, 5187).expect("새 조사");
    let mut app = StrataAppState::new_startup();
    app.start_new_survey_project_with_path("현장", &path, WorkCrs::Epsg5187);
    assert!(!has_layer(&app, K::SurveyArea), "새 조사는 조사 레이어가 비어 있다");

    let sq = |x: f64, y: f64| vec![[x, y], [x + 10.0, y], [x + 10.0, y + 10.0], [x, y + 10.0]];
    app.draw_target = K::SurveyArea;
    assert_eq!(app.commit_drawn_shape(MapToolKind::DrawPolygon, sq(219_700.0, 360_900.0)), Some("조사구역"));
    app.draw_target = K::FeaturePoly;
    assert_eq!(app.commit_drawn_shape(MapToolKind::DrawPolygon, sq(219_702.0, 360_902.0)), Some("유구_면"));
    assert_eq!(
        app.commit_drawn_shape(MapToolKind::DrawLine, vec![[219_701.0, 360_901.0], [219_709.0, 360_909.0]]),
        Some("유구_선")
    );
    assert_eq!(app.commit_drawn_shape(MapToolKind::PlacePoint, vec![[219_705.0, 360_905.0]]), Some("유물"));
    assert_eq!(app.commit_drawn_shape(MapToolKind::DrawLine, vec![[0.0, 0.0]]), None, "점 하나로는 선이 안 된다");
    assert!(app.is_dirty);

    assert!(app.save_survey_project(), "저장이 성공해야 한다");
    assert!(!app.is_dirty);

    // 파일에 실제로 들어갔다.
    let on_disk = read_survey_gpkg(&path).expect("저장한 파일은 표준 조사 파일이다");
    assert_eq!(on_disk.non_empty_keys(), vec![K::SurveyArea, K::FeaturePoly, K::FeatureLine, K::ArtifactPoint]);
    assert_eq!(on_disk.layer(K::SurveyArea).features[0].text("survey_name"), "현장");

    // 다른 창에서 다시 열어도 돌아온다.
    let mut again = StrataAppState::new_startup();
    again.open_survey_gpkg(&path);
    assert_eq!(again.home_map.real_survey.survey_rings.len(), 1);
    assert_eq!(again.survey_doc.layer(K::FeaturePoly).features.len(), 1);
    assert_eq!(again.survey_doc.layer(K::FeatureLine).features.len(), 1);
    assert_eq!(again.survey_doc.layer(K::ArtifactPoint).features[0].text("artifact_no"), "1");
    assert!(has_layer(&again, K::FeaturePoly));
}

#[test]
fn saving_an_opened_cpp_survey_twice_does_not_duplicate_or_lose_shapes() {
    let path = demo_copy("resave");
    let mut app = StrataAppState::new_startup();
    app.open_survey_gpkg(&path);
    assert!(app.save_survey_project());
    assert!(app.save_survey_project());
    let doc = read_survey_gpkg(&path).expect("표준 조사 파일");
    let counts: Vec<usize> = K::ALL.iter().map(|k| doc.layer(*k).features.len()).collect();
    assert_eq!(counts, vec![1, 2, 1, 1, 2, 1, 1]);
    assert_eq!(doc.layer(K::TrialTrench).features[0].text("name"), "TR1");
}

#[test]
fn opening_something_that_is_not_a_survey_changes_nothing() {
    let dir = temp_dir("bad");
    let path = dir.join("아님.gpkg");
    std::fs::write(&path, b"not a geopackage").expect("쓰기");
    let mut app = StrataAppState::new_startup();
    app.project_title = "그대로".to_owned();
    app.open_survey_gpkg(&path);
    assert_eq!(app.project_title, "그대로");
    assert!(app.survey_path.is_none());
    assert!(app
        .home_map
        .draw_guide_message
        .as_deref()
        .unwrap_or("")
        .contains("열 수 없습니다"));
}

#[test]
fn the_submit_package_holds_the_surveys_real_shapes_and_refuses_a_non_empty_folder() {
    let path = demo_copy("submit");
    let mut app = StrataAppState::new_startup();
    app.open_survey_gpkg(&path);
    let out = path.parent().expect("폴더").join("제출");
    let done = app.export_submit_package_to(&out, &[]).expect("꾸러미");
    assert!(done.contains("SHP 7개 레이어·도형 9개"), "{done}");
    for name in ["survey_area.shp", "feature_poly.dbf", "trial_trench.prj", "README_submit.txt", "MANIFEST.sha256"] {
        assert!(out.join(name).is_file(), "{name} 이 없다");
    }
    assert_eq!(
        strata_storage::shp_file_writer::count_shp_records(&out.join("feature_poly.shp")).expect("shp"),
        2
    );
    // 같은 폴더에 다시 만들면 덮지 않고 알린다.
    let again = app.export_submit_package_to(&out, &[]).expect_err("비어 있지 않은 폴더");
    assert!(again.starts_with("제출 폴더가 비어 있지 않습니다."), "{again}");

    // 도형이 없으면 만들지 않는다.
    let mut empty = StrataAppState::new_startup();
    let why = empty
        .export_submit_package_to(&path.parent().expect("폴더").join("빈제출"), &[])
        .expect_err("도형 없음");
    assert!(why.contains("내보낼 조사 도형이 없습니다"), "{why}");
}
