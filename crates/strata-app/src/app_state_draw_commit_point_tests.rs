//! Point capture commit, CRS transform, form/card/undo, and GeoPackage round-trip tests
//! (`target/cpp-ref/src/app/KaCaptureMapTool.cpp:273-276,305-307,393-456`,
//! `target/cpp-ref/src/app/MainWindowEditing.cpp:782-920,1168-1174`,
//! `target/cpp-ref/tests/test_save_open.cpp:367-376,3153-3172`,
//! `target/cpp-ref/tests/test_workflow.cpp:3299-3365,5962-5980`,
//! `target/cpp-ref/tests/test_feature_records.cpp:102-116`,
//! `target/cpp-ref/tests/test_feature_card.cpp:121-145`).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use strata_engine::Transformer;
use strata_storage::gpkg_wkt::wkt_to_parts;
use strata_storage::wkb_codec::WkbGeometry;
use strata_types::crs::{CrsCode, WorkCrs};

use super::*;

fn opened_demo(dir: &Path) -> (StrataAppState, PathBuf) {
    let path = dir.join("demo.gpkg");
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg"),
        &path,
    )
    .expect("copy demo");
    let mut app = StrataAppState::new_startup();
    app.open_survey_gpkg(&path);
    app.active_tab = MainTab::Map;
    app.home_map.viewport.meters_per_pixel = 0.05;
    (app, path)
}

fn fresh_survey(dir: &Path, name: &str, crs: WorkCrs) -> (StrataAppState, PathBuf) {
    let path = dir.join(format!("{name}.gpkg"));
    strata_storage::gpkg::GpkgRepository::create_new_survey(&path, crs).expect("create");
    let mut app = StrataAppState::new_startup();
    app.start_new_survey_project_with_path(name, &path, crs);
    app.active_tab = MainTab::Map;
    app.home_map.viewport.meters_per_pixel = 0.05;
    (app, path)
}

fn table_features<'a>(
    app: &'a StrataAppState,
    table: &str,
) -> &'a [strata_storage::edit_buffer::FeatureRecord] {
    &app.survey_tables.iter().find(|t| t.table == table).expect("table").layer.features
}

// Ported from target/cpp-ref/tests/test_save_open.cpp:3153-3172,
// target/cpp-ref/tests/test_workflow.cpp:5962-5980 (artifactKindChoicesOmitPolygonPresets),
// target/cpp-ref/tests/test_feature_records.cpp:102-116 (form_artifactNumbersOverWholeLayer), and
// target/cpp-ref/tests/test_feature_card.cpp:121-145 (card_artifactNumberRunsOverWholeLayer).
#[test]
fn placing_artifact_points_offers_form_without_polygon_presets_updates_card_and_saves_to_gpkg() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut app, path) = fresh_survey(dir.path(), "artifact_roundtrip", WorkCrs::Epsg5187);
    app.start_edit_artifact();

    // 1st click: POINT(219700, 360900) -> offers form with empty kind presets and number "1".
    app.sketch_press([219_700.0, 360_900.0]);
    assert!(app.draw_sketch.points.is_empty(), "KaCaptureMapTool.cpp:451 clears m_points");
    assert_eq!(app.home_map.last_drawn_shape, None, "MainWindowEditing.cpp:845-849 polygon only");
    assert_eq!(table_features(&app, "artifact_point").len(), 1);
    assert_eq!(app.status_left_message(), "도형은 넣었습니다. 이름·번호를 적거나 Esc로 건너뛰세요.");
    {
        let form = app.home_map.feature_form.as_mut().expect("feature form offered");
        assert!(form.kind_choices.is_empty(), "artifact_point omits polygon kind presets: {:?}", form.kind_choices);
        assert_eq!(form.number, "1");
        form.name = "토기".to_owned();
        form.period = "청동기".to_owned();
        form.number = "A-1".to_owned();
    }
    app.save_feature_form();

    let view = app.card_view_now().expect("card set after capture");
    assert_eq!((view.header.as_str(), view.kind.as_str(), view.period.as_str(), view.number.as_str()), ("A-1 토기", "토기", "청동기", "A-1"));
    assert!(view.sub_line.contains("유물 위치"), "{}", view.sub_line);
    assert_eq!(
        app.status_left_message(),
        "도형을 넣었습니다 (유물, 1개). Ctrl+Z로 되돌리기 · 「저장」(Ctrl+S)으로 파일에 씁니다"
    );

    // Undo removes both the point and its form attributes in one step; redo restores both.
    app.undo_map_action();
    assert!(table_features(&app, "artifact_point").is_empty());
    assert!(app.card_view_now().is_none());
    app.redo_map_action();
    assert_eq!(table_features(&app, "artifact_point").len(), 1);
    assert_eq!(table_features(&app, "artifact_point")[0].attributes.get("artifact_no").map(String::as_str), Some("A-1"));

    // Whole-layer numbering: after "1" ("토기") and "2" ("석기"), next form offers "3" and distinct kind choices.
    table_features_mut(&mut app, "artifact_point")[0].attributes.insert("artifact_no".to_owned(), "1".to_owned());
    app.sketch_press([219_705.0, 360_902.0]);
    {
        let form = app.home_map.feature_form.as_mut().expect("second form");
        assert_eq!(form.kind_choices, vec!["토기".to_owned()]);
        assert_eq!(form.number, "2");
        form.name = "석기".to_owned();
    }
    app.save_feature_form();

    app.sketch_press([219_710.0, 360_904.0]);
    {
        let form = app.home_map.feature_form.as_mut().expect("third form");
        assert_eq!(form.number, "3", "artifact numbers run over the whole layer regardless of kind");
        form.name = "청자편".to_owned();
        form.period = "고려".to_owned();
        form.number = "A-1".to_owned();
    }
    app.save_feature_form();

    let stamped = &table_features(&app, "artifact_point")[2].attributes;
    assert_eq!(stamped["uid"].len(), 36);
    assert!(!stamped["created_at"].is_empty());
    assert!(!stamped["updated_at"].is_empty());

    assert!(app.save_survey_project());
    assert!(!app.is_dirty);

    let conn = rusqlite::Connection::open(&path).expect("open");
    let count: i64 = conn.query_row("SELECT count(*) FROM artifact_point", [], |r| r.get(0)).expect("count");
    let rtree: i64 = conn.query_row("SELECT count(*) FROM rtree_artifact_point_geom", [], |r| r.get(0)).expect("rtree");
    assert_eq!((count, rtree), (3, 3));
    drop(conn);

    let mut reopened = StrataAppState::new_startup();
    assert!(reopened.open_survey_gpkg(&path));
    let feats = table_features(&reopened, "artifact_point");
    assert_eq!(feats.len(), 3);
    let parts = wkt_to_parts(&feats[0].wkt_geom).expect("wkt");
    assert_eq!(parts, vec![WkbGeometry::Point([219_700.0, 360_900.0])]);
    assert_eq!(feats[2].attributes.get("kind").map(String::as_str), Some("청자편"));
    assert_eq!(feats[2].attributes.get("period").map(String::as_str), Some("고려"));
    assert_eq!(feats[2].attributes.get("artifact_no").map(String::as_str), Some("A-1"));
}

fn table_features_mut<'a>(
    app: &'a mut StrataAppState,
    table: &str,
) -> &'a mut Vec<strata_storage::edit_buffer::FeatureRecord> {
    &mut app.survey_tables.iter_mut().find(|t| t.table == table).expect("table").layer.features
}

// Ported from target/cpp-ref/src/app/KaCaptureMapTool.cpp:435-450 and
// target/cpp-ref/tests/test_workflow.cpp:3299-3365 across all three CRS pairs (5186/5186, 5187/5187,
// and reprojected 5187 canvas into 5186 demo table).
#[test]
fn placing_point_reprojects_across_all_three_crs_pairs_without_altering_existing_points() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut demo, _) = opened_demo(dir.path());
    assert_eq!(CrsCode::from(demo.work_crs).epsg_number(), 5187);
    let orig_wkt = table_features(&demo, "artifact_point")[0].wkt_geom.clone();
    let want_5186 = [200_140.0, 450_160.0];
    let click_5187 = Transformer::new(5186, 5187).expect("PROJ").transform(want_5186).expect("xf");

    demo.start_edit_artifact();
    demo.sketch_press(click_5187);
    demo.skip_feature_form();

    let feats = table_features(&demo, "artifact_point");
    assert_eq!(feats.len(), 2);
    assert_eq!(feats[0].wkt_geom, orig_wkt, "existing point untouched");
    let WkbGeometry::Point(pt) = wkt_to_parts(&feats[1].wkt_geom).expect("wkt")[0] else { panic!("point") };
    assert!((pt[0] - want_5186[0]).hypot(pt[1] - want_5186[1]) < 1e-6, "{pt:?} vs {want_5186:?}");
    assert_eq!(demo.unsaved_chip_text().as_deref(), Some("저장 안 됨 1건"));

    for (name, canvas_crs, layer_crs) in [
        ("crs_5186", WorkCrs::Epsg5186, WorkCrs::Epsg5186),
        ("crs_5187", WorkCrs::Epsg5187, WorkCrs::Epsg5187),
        ("crs_5186_to_5187", WorkCrs::Epsg5186, WorkCrs::Epsg5187),
    ] {
        let (mut app, _) = fresh_survey(dir.path(), name, layer_crs);
        app.work_crs = canvas_crs;
        app.ribbon.continuous_draw = true;
        app.start_edit_artifact();
        let target_layer_xy = [200_050.0, 450_075.0];
        let click_xy = if canvas_crs == layer_crs {
            target_layer_xy
        } else {
            Transformer::new(
                CrsCode::from(layer_crs).epsg_number(),
                CrsCode::from(canvas_crs).epsg_number(),
            )
            .expect("PROJ")
            .transform(target_layer_xy)
            .expect("xf")
        };
        app.sketch_press(click_xy);
        let WkbGeometry::Point(got) = wkt_to_parts(&table_features(&app, "artifact_point")[0].wkt_geom).unwrap()[0] else {
            panic!("point");
        };
        assert!((got[0] - target_layer_xy[0]).hypot(got[1] - target_layer_xy[1]) < 1e-6, "{name}: {got:?}");
    }
}

// Ported from target/cpp-ref/src/app/KaCaptureMapTool.cpp:263-266,305-307,398-402 and
// target/cpp-ref/src/app/MainWindowEditing.cpp:853-915: continuous draw skips form, empty finish and
// double-click do nothing in Point mode, and control_points map clicks offer "P1" via shared capture.
#[test]
fn continuous_draw_empty_finish_double_click_and_control_points_map_click_match_cpp() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut app, _) = fresh_survey(dir.path(), "point_modes", WorkCrs::Epsg5187);
    app.ribbon.continuous_draw = true;
    app.start_edit_artifact();

    let initial = app.status_left_message().to_owned();
    app.sketch_finish();
    assert_eq!(app.status_left_message(), initial, "empty finish in Point mode is silent");
    assert!(!app.is_dirty);

    app.sketch_press([219_700.0, 360_900.0]);
    assert!(app.home_map.feature_form.is_none());
    assert_eq!(table_features(&app, "artifact_point").len(), 1);
    let card = app.card_view_now().expect("card");
    assert_eq!((card.title, card.header.as_str(), card.layer_kind.as_str()), ("선택한 유물", "이름 없음", "유물 위치"));
    assert_eq!(
        app.status_left_message(),
        "도형을 넣었습니다 (유물, 1개). Ctrl+Z로 되돌리기 · 「저장」(Ctrl+S)으로 파일에 씁니다 · 이름·번호는 나중에 속성에서"
    );

    // Ignore double-click when m_mode == Mode::Point (target/cpp-ref/src/app/KaCaptureMapTool.cpp:306).
    app.sketch_double_click([219_700.0, 360_900.0]);
    assert_eq!(table_features(&app, "artifact_point").len(), 1, "double-click does not add a second point");

    // Shared point capture on control_points offers P1 when continuous_draw is off.
    app.ribbon.continuous_draw = false;
    app.begin_edit_layer("control_points", "기준점");
    app.sketch_press([219_800.0, 360_950.0]);
    assert_eq!(app.home_map.feature_form.as_ref().map(|f| f.number.as_str()), Some("P1"));
    app.save_feature_form();
    assert_eq!(table_features(&app, "control_points")[0].attributes.get("point_id").map(String::as_str), Some("P1"));
}
