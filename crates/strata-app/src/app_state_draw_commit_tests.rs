//! 그린 면을 편집 레이어에 넣기 — C++ onGeometryCaptured(MainWindowEditing.cpp:782-920). demo.gpkg 는 임시 사본만.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use strata_engine::Transformer;
use strata_storage::gpkg_wkt::wkt_to_parts;
use strata_storage::wkb_codec::WkbGeometry;
use strata_types::crs::CrsCode;

use super::*;

fn opened_demo(dir: &Path) -> (StrataAppState, PathBuf) {
    let path = dir.join("demo.gpkg");
    std::fs::copy(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg"), &path).expect("copy demo");
    let mut app = StrataAppState::new_startup();
    app.open_survey_gpkg(&path);
    app.active_tab = MainTab::Map;
    app.home_map.viewport.meters_per_pixel = 0.05;
    (app, path)
}

/// demo(표 5186) 가운데 근처의 작업 좌표계 점 셋.
fn work_points(app: &StrataAppState) -> Vec<[f64; 2]> {
    let to_work = Transformer::new(5186, CrsCode::from(app.work_crs).epsg_number()).expect("PROJ");
    [[200_100.0, 450_100.0], [200_130.0, 450_100.0], [200_130.0, 450_140.0]]
        .iter()
        .map(|p| to_work.transform(*p).expect("transform"))
        .collect()
}

fn poly_features(app: &StrataAppState) -> &[strata_storage::edit_buffer::FeatureRecord] {
    &app.survey_tables.iter().find(|t| t.table == "feature_poly").expect("feature_poly").layer.features
}

/// 유구 그리기 → 세 점 → 마치기: 편집 레이어에 음수 fid 로 들어가고, 좌표는 PROJ 로 표 좌표계(5186)에
/// 돌아오고, 「*」·범례 수·상태줄 글이 C++ 과 같다.
#[test]
fn finished_polygon_goes_into_the_edit_layer_in_the_table_crs() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut app, _) = opened_demo(dir.path());
    app.start_edit_feature_poly();
    let pts = work_points(&app);
    for p in &pts {
        app.sketch_press(*p);
    }
    assert!(!app.is_dirty, "'*' only after a finished feature");
    app.sketch_finish();

    let feats = poly_features(&app);
    assert_eq!(feats.len(), 3);
    let new = feats.last().unwrap();
    assert!(new.fid < 0);
    let parts = wkt_to_parts(&new.wkt_geom).expect("wkt");
    let WkbGeometry::Polygon(rings) = &parts[0] else { panic!("polygon") };
    let want = [[200_100.0, 450_100.0], [200_130.0, 450_100.0], [200_130.0, 450_140.0], [200_100.0, 450_100.0]];
    assert_eq!(rings[0].len(), 4);
    for (g, w) in rings[0].iter().zip(want) {
        assert!((g[0] - w[0]).hypot(g[1] - w[1]) < 1e-6, "{g:?} vs {w:?}");
    }
    assert!(app.is_dirty);
    assert!(app.draw_sketch.points.is_empty());
    let entry = app.left_layers.entries.iter().find(|e| e.layer_key == "feature_poly").unwrap();
    assert_eq!(entry.feature_count, 3);
    let name = entry.label_ko.clone();
    let msg = app.home_map.draw_guide_message.clone().unwrap();
    assert!(
        msg.starts_with(&format!("도형을 넣었습니다 ({name}, 3개). Ctrl+Z로 되돌리기 · 「저장」(Ctrl+S)으로 파일에 씁니다")),
        "{msg}"
    );
    assert!(msg.contains(" · 종류·시대 빈 유구 "), "the new shape has no kind/period: {msg}");
}

/// 그린 뒤 「저장」 → 다시 열면 유구 면 3개, 공간 색인(rtree)도 3개.
#[test]
fn drawn_polygon_is_written_by_save_and_reopens() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut app, path) = opened_demo(dir.path());
    app.start_edit_feature_poly();
    for p in work_points(&app) {
        app.sketch_press(p);
    }
    app.sketch_finish();
    assert!(app.save_survey_project());
    assert!(!app.is_dirty);

    let conn = rusqlite::Connection::open(&path).expect("open");
    let count: i64 = conn.query_row("SELECT count(*) FROM feature_poly", [], |r| r.get(0)).expect("count");
    let rtree: i64 = conn.query_row("SELECT count(*) FROM rtree_feature_poly_geom", [], |r| r.get(0)).expect("rtree");
    assert_eq!((count, rtree), (3, 3));
    drop(conn);
    let mut again = StrataAppState::new_startup();
    again.open_survey_gpkg(&path);
    assert_eq!(poly_features(&again).len(), 3);
}

/// 점이 모자라면 넣지 않고 C++ 글을 띄우고 점은 남긴다. 「*」도 켜지 않는다.
#[test]
fn too_few_points_keep_the_sketch_and_say_so() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut app, _) = opened_demo(dir.path());
    app.start_edit_feature_poly();
    let pts = work_points(&app);
    app.sketch_press(pts[0]);
    app.sketch_press(pts[1]);
    app.sketch_finish();
    assert_eq!(app.home_map.draw_guide_message.as_deref(), Some(TOO_FEW_TEXT));
    assert_eq!(app.draw_sketch.points.len(), 2);
    assert_eq!(poly_features(&app).len(), 2);
    assert!(!app.is_dirty);
}

/// 더블클릭으로 마치면 찍은 꼭짓점 그대로(겹친 점 없음).
#[test]
fn double_click_finish_adds_no_duplicate_vertex() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut app, _) = opened_demo(dir.path());
    app.start_edit_feature_poly();
    let pts = work_points(&app);
    for p in &pts {
        app.sketch_press(*p);
    }
    app.sketch_press(pts[2]);
    app.sketch_double_click(pts[2]);
    let parts = wkt_to_parts(&poly_features(&app).last().unwrap().wkt_geom).expect("wkt");
    let WkbGeometry::Polygon(rings) = &parts[0] else { panic!("polygon") };
    assert_eq!(rings[0].len(), 4);
}

/// 되돌리기는 점 하나, 취소는 그리던 도형만(C++ 글), 둘 다 「*」를 켜지 않는다.
#[test]
fn undo_vertex_and_cancel_touch_only_the_sketch() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut app, _) = opened_demo(dir.path());
    app.start_edit_feature_poly();
    let pts = work_points(&app);
    for p in &pts {
        app.sketch_press(*p);
    }
    app.undo_map_action();
    assert_eq!(app.draw_sketch.points.len(), 2);
    assert_eq!(app.home_map.draw_guide_message.as_deref(), Some("꼭짓점 하나를 되돌렸습니다."));
    app.sketch_cancel();
    assert!(app.draw_sketch.points.is_empty());
    assert_eq!(app.home_map.draw_guide_message.as_deref(), Some("그리던 도형을 지웠습니다."));
    assert_eq!(app.active_tool, MapToolKind::DrawPolygon, "Esc keeps the tool");
    assert!(!app.is_dirty);
}

/// 그린 뒤 Ctrl+Z 는 넣은 도형을 빼고, Ctrl+Y 는 다시 넣는다(범례 수도 따라간다).
#[test]
fn undo_and_redo_after_drawing_follow_the_layer_buffer() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut app, _) = opened_demo(dir.path());
    app.start_edit_feature_poly();
    for p in work_points(&app) {
        app.sketch_press(p);
    }
    app.sketch_finish();
    app.undo_map_action();
    assert_eq!(poly_features(&app).len(), 2);
    assert_eq!(app.home_map.draw_guide_message.as_deref(), Some("이전 상태로 되돌렸습니다."));
    assert_eq!(app.left_layers.entries.iter().find(|e| e.layer_key == "feature_poly").unwrap().feature_count, 2);
    app.redo_map_action();
    assert_eq!(poly_features(&app).len(), 3);
    assert_eq!(app.home_map.draw_guide_message.as_deref(), Some("다시 실행했습니다."));
    app.redo_map_action();
    assert_eq!(app.home_map.draw_guide_message.as_deref(), Some("다시 실행할 것이 없습니다."));
}

/// stampNew — uid(UUID v4)·created_at·updated_at 칸이 있을 때만, 시각은 초까지 + 시간대.
#[test]
fn stamps_fill_only_existing_columns() {
    let cols = |names: &[&str]| names.iter().map(|n| ((*n).to_owned(), "TEXT".to_owned())).collect::<Vec<_>>();
    assert!(stamped_attributes(&cols(&["kind", "period"])).is_empty());
    let a = stamped_attributes(&cols(&["uid", "created_at", "updated_at"]));
    let uid = &a["uid"];
    assert_eq!(uid.len(), 36);
    assert_eq!(uid.as_bytes()[14], b'4', "v4: {uid}");
    assert_eq!(a["created_at"], a["updated_at"]);
    let t = &a["created_at"];
    assert!(t.len() == 25 || t.ends_with('Z'), "{t}");
    assert_eq!(&t[10..11], "T");
}

/// 「저장 안 됨 n건」(KaStatusBar.cpp:152-160, EditBufferSummary.cpp:14-27) — 레이어마다 더한·지운·바꾼 도형 id 수의 합.
/// 되돌려 0건이 되어도 「*」가 남아 있으면 「저장 안 됨」.
#[test]
fn unsaved_chip_counts_edited_features() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut app, _) = opened_demo(dir.path());
    assert_eq!(app.unsaved_chip_text(), None);
    app.start_edit_feature_poly();
    for p in work_points(&app) {
        app.sketch_press(p);
    }
    app.sketch_finish();
    assert_eq!(app.unsaved_chip_text().as_deref(), Some("저장 안 됨 1건"));
    let area = app.survey_tables.iter_mut().find(|t| t.table == "survey_area").unwrap();
    area.layer.features[0].attributes.insert("name".to_owned(), "바꾼 이름".to_owned());
    let trench = app.survey_tables.iter_mut().find(|t| t.table == "trial_trench").unwrap();
    trench.layer.features.clear();
    assert_eq!(app.unsaved_chip_text().as_deref(), Some("저장 안 됨 3건"));
    let area = app.survey_tables.iter_mut().find(|t| t.table == "survey_area").unwrap();
    area.layer.features = area.baseline.clone();
    let trench = app.survey_tables.iter_mut().find(|t| t.table == "trial_trench").unwrap();
    trench.layer.features = trench.baseline.clone();
    app.undo_map_action();
    assert_eq!(app.unsaved_chip_text().as_deref(), Some("저장 안 됨"), "'*' stays after undo");
}

/// 새 조사(임시 폴더, 작업 좌표계 = 표 좌표계): 유구 면을 그려 저장하면 좌표가 그대로 쓰이고 uid·created_at·
/// updated_at 칸이 채워진다. 다시 열면 그 도형이 있다.
#[test]
fn new_survey_drawing_is_saved_with_stamps() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("새조사.gpkg");
    let crs = strata_types::crs::WorkCrs::Epsg5187;
    strata_storage::gpkg::GpkgRepository::create_new_survey(&path, crs).expect("create");
    let mut app = StrataAppState::new_startup();
    app.start_new_survey_project_with_path("새조사", &path, crs);
    app.active_tab = MainTab::Map;
    app.home_map.viewport.meters_per_pixel = 0.05;
    app.start_edit_feature_poly();
    let pts = [[21_100.0, 451_800.0], [21_130.0, 451_800.0], [21_130.0, 451_840.0]];
    for p in pts {
        app.sketch_press(p);
    }
    app.sketch_finish();
    assert_eq!(poly_features(&app).len(), 1, "{:?}", app.home_map.draw_guide_message);
    assert!(app.save_survey_project());

    let conn = rusqlite::Connection::open(&path).expect("open");
    let (uid, created, updated): (String, String, String) = conn
        .query_row("SELECT uid, created_at, updated_at FROM feature_poly", [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .expect("stamps");
    assert_eq!(uid.len(), 36);
    assert!(!created.is_empty() && created == updated);
    drop(conn);
    let mut again = StrataAppState::new_startup();
    again.open_survey_gpkg(&path);
    let parts = wkt_to_parts(&poly_features(&again)[0].wkt_geom).expect("wkt");
    let WkbGeometry::Polygon(rings) = &parts[0] else { panic!("polygon") };
    for (g, w) in rings[0].iter().zip(pts) {
        assert!((g[0] - w[0]).abs() < 1e-6 && (g[1] - w[1]).abs() < 1e-6, "{g:?} vs {w:?}");
    }
}

/// 조사구역 그리기(기존 조사구역 레이어에 이어 그리기) — demo 사본의 survey_area 에 들어간다.
#[test]
fn survey_area_drawing_goes_into_survey_area() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut app, path) = opened_demo(dir.path());
    app.start_edit_survey_area();
    assert_eq!(app.edit_layer_key.as_deref(), Some("survey_area"));
    for p in work_points(&app) {
        app.sketch_press(p);
    }
    app.sketch_finish();
    let area = app.survey_tables.iter().find(|t| t.table == "survey_area").unwrap();
    assert_eq!(area.layer.features.len(), 2);
    assert!(app.save_survey_project());
    let conn = rusqlite::Connection::open(&path).expect("open");
    let n: i64 = conn.query_row("SELECT count(*) FROM survey_area", [], |r| r.get(0)).expect("count");
    assert_eq!(n, 2);
}
