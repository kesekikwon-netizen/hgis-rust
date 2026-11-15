//! 「겹친 곳 지우기」 on a new survey in a temp folder — ported from tests/test_polygon_erase.cpp and the
//! messages of MainWindowErase.cpp. Rectangles are given as metres from (200000, 450000) of EPSG:5187.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;

use strata_storage::gpkg_wkt::wkt_to_parts;
use strata_storage::wkb_codec::WkbGeometry;

use crate::app_state::{MainTab, StrataAppState};

fn new_survey(dir: &std::path::Path) -> StrataAppState {
    let path = dir.join("새조사.gpkg");
    let crs = strata_types::crs::WorkCrs::Epsg5187;
    strata_storage::gpkg::GpkgRepository::create_new_survey(&path, crs).expect("create");
    let mut app = StrataAppState::new_startup();
    app.start_new_survey_project_with_path("새조사", &path, crs);
    app.active_tab = MainTab::Map;
    app
}

/// Puts a rectangle into a table as an unsaved shape and makes sure the table has its row in the legend.
fn rect(app: &mut StrataAppState, table: &str, [x0, y0, x1, y1]: [f64; 4], name: &str) -> i64 {
    let (x0, y0, x1, y1) = (200_000.0 + x0, 450_000.0 + y0, 200_000.0 + x1, 450_000.0 + y1);
    let wkt = format!("POLYGON (({x0} {y0}, {x1} {y0}, {x1} {y1}, {x0} {y1}, {x0} {y0}))");
    let key = strata_types::schema::DomainLayerKey::from_id(table).expect("a domain table");
    app.left_layers.ensure_domain_layer(key);
    let t = app.survey_tables.iter_mut().find(|t| t.table == table).expect("table");
    // 「note」 and 「uid」 are columns of every survey-area table (survey_gpkg_ddl.rs).
    let mut attributes = BTreeMap::from([("uid".to_owned(), format!("uid-{name}"))]);
    if table == "survey_area" {
        attributes.insert("note".to_owned(), name.to_owned());
    }
    t.layer.add_unsaved_feature(wkt, attributes)
}

fn shape<'a>(app: &'a StrataAppState, table: &str, fid: i64) -> Option<&'a strata_storage::edit_buffer::FeatureRecord> {
    app.survey_tables.iter().find(|t| t.table == table)?.layer.features.iter().find(|f| f.fid == fid)
}

fn area(app: &StrataAppState, table: &str, fid: i64) -> f64 {
    strata_engine::wkt_area(&shape(app, table, fid).expect("shape").wkt_geom).expect("area")
}

fn holes(app: &StrataAppState, table: &str, fid: i64) -> usize {
    match &wkt_to_parts(&shape(app, table, fid).expect("shape").wkt_geom).expect("wkt")[0] {
        WkbGeometry::Polygon(rings) => rings.len() - 1,
        _ => panic!("a polygon"),
    }
}

fn count(app: &StrataAppState, table: &str) -> usize {
    app.survey_tables.iter().find(|t| t.table == table).map_or(0, |t| t.layer.features.len())
}

/// ported from :87-123 (oneSelectedShapeCutsTheShapeUnderItAndOneUndoRestoresBoth): a small shape drawn on
/// a larger one of the same layer and selected cuts its place out of the larger one and is removed itself,
/// as ONE step: one Ctrl+Z fills the hole and brings the small shape back. The words are those of
/// MainWindowErase.cpp:109-116.
#[test]
fn one_selected_shape_cuts_the_shape_under_it_and_one_undo_restores_both() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = new_survey(dir.path());
    let outer = rect(&mut app, "survey_area", [0.0, 0.0, 100.0, 100.0], "바깥");
    let far = rect(&mut app, "survey_area", [300.0, 0.0, 400.0, 100.0], "멀리");
    let inner = rect(&mut app, "survey_area", [40.0, 40.0, 60.0, 60.0], "안쪽");
    let steps = app.undo_tables.len();
    app.home_map.selected_shape = Some(("survey_area".to_owned(), inner));

    app.erase_overlap_with_shape();
    assert!((area(&app, "survey_area", outer) - 9600.0).abs() < 1e-6);
    assert_eq!(holes(&app, "survey_area", outer), 1);
    assert!(shape(&app, "survey_area", inner).is_none(), "the shape drawn on top is gone");
    assert!((area(&app, "survey_area", far) - 10_000.0).abs() < 1e-6);
    assert_eq!(count(&app, "survey_area"), 2);
    assert_eq!(app.status_left_message(), "「조사구역」 도형 1개에서 겹친 자리를 지웠습니다. 면적 10,000.00 ㎡ → 9,600.00 ㎡. 위에 그린 도형은 없앴습니다. Ctrl+Z로 되돌립니다.");
    assert!(app.is_dirty && app.home_map.selected_shape.is_none());

    assert_eq!(app.undo_tables.len(), steps + 1, "one step");
    app.undo_map_action();
    assert!((area(&app, "survey_area", outer) - 10_000.0).abs() < 1e-6);
    assert!(shape(&app, "survey_area", inner).is_some());
    assert_eq!(count(&app, "survey_area"), 3);
    app.redo_map_action();
    assert_eq!(count(&app, "survey_area"), 2);
}

/// ported from :125-145 (selectingTheBigShapeOrALonelyShapeOnlyExplains): the big shape, a shape with
/// nothing under it, and nothing selected only explain, and nothing is changed.
#[test]
fn the_big_shape_a_lonely_shape_and_no_shape_only_explain() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = new_survey(dir.path());
    let outer = rect(&mut app, "survey_area", [0.0, 0.0, 100.0, 100.0], "바깥");
    rect(&mut app, "survey_area", [40.0, 40.0, 60.0, 60.0], "안쪽");
    let apart = rect(&mut app, "survey_area", [300.0, 0.0, 400.0, 100.0], "멀리");
    let line = {
        app.left_layers.ensure_domain_layer(strata_types::schema::DomainLayerKey::FeatureLine);
        let t = app.survey_tables.iter_mut().find(|t| t.table == "feature_line").expect("table");
        t.layer.add_unsaved_feature("LINESTRING (200000 450000, 200100 450100)".to_owned(), BTreeMap::new())
    };
    let said = |app: &mut StrataAppState, picked: Option<(&str, i64)>| {
        app.home_map.selected_shape = picked.map(|(table, fid)| (table.to_owned(), fid));
        app.home_map.last_drawn_shape = None;
        app.erase_overlap_with_shape();
        app.status_left_message().to_owned()
    };
    assert_eq!(said(&mut app, Some(("survey_area", outer))), "고른 도형이 아래 도형을 모두 덮고 있어 남는 부분이 없습니다. 위에 그린 작은 도형을 고르세요.");
    assert_eq!(said(&mut app, Some(("survey_area", apart))), "고른 도형 아래에 겹친 면이 없습니다.");
    assert_eq!(said(&mut app, None), "위에 그린 도형을 [도형선택]으로 고른 뒤 누르세요.");
    assert_eq!(said(&mut app, Some(("feature_line", line))), "고른 도형으로는 지울 수 없습니다. 조사 데이터의 면 도형을 고르세요.");
    assert_eq!(count(&app, "survey_area"), 3);
    assert!((area(&app, "survey_area", outer) - 10_000.0).abs() < 1e-6);
}

/// ported from :147-168 (anotherLayerIsCutOnlyWhenExactlyOneShapeLiesUnder): with nothing under it in its
/// own layer, the selected shape cuts the one shape under it in another layer, and one Ctrl+Z restores
/// both layers; with two shapes under it the user has to choose.
#[test]
fn another_layer_is_cut_only_when_exactly_one_shape_lies_under() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = new_survey(dir.path());
    let outer = rect(&mut app, "survey_area", [0.0, 0.0, 100.0, 100.0], "바깥");
    let cutter = rect(&mut app, "feature_poly", [40.0, 40.0, 60.0, 60.0], "뺄 자리");
    let pit = rect(&mut app, "trial_trench", [30.0, 30.0, 70.0, 70.0], "수혈");

    app.home_map.selected_shape = Some(("feature_poly".to_owned(), cutter));
    app.erase_overlap_with_shape();
    assert_eq!(app.status_left_message(), "아래에 겹친 도형이 2개입니다. Shift를 누른 채 지울 도형도 함께 고른 뒤 다시 누르세요.");
    assert!((area(&app, "survey_area", outer) - 10_000.0).abs() < 1e-6);

    let trenches = app.survey_tables.iter_mut().find(|t| t.table == "trial_trench").unwrap();
    assert!(trenches.layer.delete_feature(pit));
    app.erase_overlap_with_shape();
    assert!((area(&app, "survey_area", outer) - 9600.0).abs() < 1e-6);
    assert_eq!(count(&app, "feature_poly"), 0);
    app.undo_map_action();
    assert!((area(&app, "survey_area", outer) - 10_000.0).abs() < 1e-6, "one Ctrl+Z restores the cut layer");
    assert_eq!(count(&app, "feature_poly"), 1, "and the layer of the shape on top");
}

/// ported from :227-280 (shapeCutInTwoStaysValidInASinglePolygonFile): a band across a shape leaves two
/// pieces. The layer holds single polygons, so the larger piece stays the shape and the other becomes a
/// shape of its own with the same attributes but without the uid; 저장 writes two valid polygons.
#[test]
fn a_shape_cut_in_two_becomes_two_shapes() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = new_survey(dir.path());
    let outer = rect(&mut app, "survey_area", [0.0, 0.0, 100.0, 100.0], "바깥");
    let band = rect(&mut app, "survey_area", [30.0, -10.0, 50.0, 110.0], "띠");
    app.home_map.selected_shape = Some(("survey_area".to_owned(), band));
    app.erase_overlap_with_shape();

    assert_eq!(count(&app, "survey_area"), 2);
    assert!((area(&app, "survey_area", outer) - 5000.0).abs() < 1e-6, "the larger piece stays the shape");
    let piece = app.survey_tables.iter().find(|t| t.table == "survey_area").unwrap().layer.features.iter().find(|f| f.fid != outer).unwrap();
    assert!((strata_engine::wkt_area(&piece.wkt_geom).unwrap() - 3000.0).abs() < 1e-6);
    assert_eq!((piece.attributes.get("note").map(String::as_str), piece.attributes.get("uid")), (Some("바깥"), None));
    assert_eq!(app.status_left_message(), "「조사구역」 도형 1개에서 겹친 자리를 지웠습니다. 떨어진 조각 1개는 따로 도형이 되었습니다. 위에 그린 도형은 없앴습니다. Ctrl+Z로 되돌립니다.");

    assert!(app.save_survey_project());
    let saved = app.survey_tables.iter().find(|t| t.table == "survey_area").unwrap();
    assert_eq!(saved.layer.features.len(), 2);
    assert!(saved.layer.features.iter().all(|f| f.wkt_geom.starts_with("POLYGON") && strata_engine::wkt_is_valid(&f.wkt_geom).unwrap()));
}

/// With nothing selected the first press only selects the shape drawn last and says so; the second press
/// erases (MainWindowErase.cpp:41-53).
#[test]
fn the_first_press_picks_the_shape_drawn_last() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = new_survey(dir.path());
    let outer = rect(&mut app, "survey_area", [0.0, 0.0, 100.0, 100.0], "바깥");
    let inner = rect(&mut app, "survey_area", [40.0, 40.0, 60.0, 60.0], "안쪽");
    app.home_map.last_drawn_shape = Some(("survey_area".to_owned(), inner));

    app.erase_overlap_with_shape();
    assert_eq!(app.home_map.selected_shape, Some(("survey_area".to_owned(), inner)));
    assert_eq!(app.status_left_message(), "방금 그린 도형을 골랐습니다. 이 모양대로 아래 도형에서 지우려면 [겹친 곳 지우기]를 한 번 더 누르세요.");
    assert!((area(&app, "survey_area", outer) - 10_000.0).abs() < 1e-6);

    app.erase_overlap_with_shape();
    assert!((area(&app, "survey_area", outer) - 9600.0).abs() < 1e-6);
}
