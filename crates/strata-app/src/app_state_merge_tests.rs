//! Shift selection and 「폴리곤 묶기」 on a new survey in a temp folder — ported from tests/test_layer_edits.cpp:42-106
//! and the selection words of KaFeatureSelectPick.cpp:188-209.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;

use super::{draw_order, merge_conflict_summary, merge_conflicts};
use crate::app_state::{MainTab, StrataAppState};

fn new_survey(dir: &std::path::Path) -> StrataAppState {
    let path = dir.join("새조사.gpkg");
    let crs = strata_types::crs::WorkCrs::Epsg5187;
    strata_storage::gpkg::GpkgRepository::create_new_survey(&path, crs).unwrap();
    let mut app = StrataAppState::new_startup();
    app.start_new_survey_project_with_path("새조사", &path, crs);
    app.active_tab = MainTab::Map;
    app.home_map.viewport.meters_per_pixel = 0.1;
    app
}

fn square(x: f64, y: f64, side: f64) -> String {
    format!("POLYGON (({x} {y}, {} {y}, {} {}, {x} {}, {x} {y}))", x + side, x + side, y + side, y + side)
}

fn add(app: &mut StrataAppState, table: &str, wkt: &str, attributes: &[(&str, &str)]) -> i64 {
    app.left_layers.ensure_domain_layer(strata_types::schema::DomainLayerKey::from_id(table).unwrap());
    let values: BTreeMap<String, String> = attributes.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect();
    app.survey_tables.iter_mut().find(|t| t.table == table).unwrap().layer.add_unsaved_feature(wkt.to_owned(), values)
}

fn count(app: &StrataAppState, table: &str) -> usize {
    app.survey_tables.iter().find(|t| t.table == table).unwrap().layer.features.len()
}

/// Shift adds a shape to the selection and takes a selected one off again; a click without Shift starts
/// over. With two shapes the C++ words name their layers, with three it counts them, and a selection of more
/// than one shape shows no vertex handles.
#[test]
fn shift_adds_and_takes_off_and_a_plain_click_starts_over() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = new_survey(dir.path());
    let a = add(&mut app, "feature_poly", &square(200_000.0, 450_000.0, 10.0), &[]);
    let b = add(&mut app, "feature_poly", &square(200_020.0, 450_000.0, 10.0), &[]);
    let c = add(&mut app, "feature_poly", &square(200_040.0, 450_000.0, 10.0), &[]);
    let at = |x: f64| [x, 450_005.0];
    app.select_at_with(at(200_005.0), [100.0, 100.0], false);
    app.select_at_with(at(200_025.0), [200.0, 100.0], true);
    let fp = |fid: i64| ("feature_poly".to_owned(), fid);
    assert_eq!(app.all_selected(), [fp(a), fp(b)]);
    let label = app.left_layers.entries.iter().find(|e| e.layer_key == "feature_poly").unwrap().label_ko.clone();
    assert_eq!(app.status_left_message(), format!("도형 2개 선택됨 ({label}, {label}) — [겹친 곳 지우기](우클릭)는 큰 도형에서 작은 도형 자리를 지우고, [폴리곤 나누기]는 겹친 자리를 새 도형으로 나눕니다."));
    assert!(app.vertex_target().is_none(), "no handles on two shapes");
    app.select_at_with(at(200_045.0), [300.0, 100.0], true);
    assert_eq!((app.all_selected().len(), app.status_left_message()), (3, "도형 3개 선택됨"));
    app.select_at_with(at(200_025.0), [200.0, 100.0], true);
    assert_eq!(app.all_selected(), [fp(a), fp(c)], "Shift on a selected shape takes it off");
    app.select_at_with(at(200_005.0), [100.0, 100.0], false);
    assert_eq!(app.all_selected(), [fp(a)], "a plain click starts over");
}

/// The first drawn keeps its values: saved shapes by number first, then unsaved ones -1, -2 … (`mergeKeeperId`).
#[test]
fn the_first_drawn_shape_keeps_its_values() {
    assert_eq!(draw_order(&[7, 3, -2, -1]), [3, 7, -1, -2]);
    assert_eq!(draw_order(&[-3, -1]), [-1, -3]);
}

/// Values that differ from the first drawn shape are named, with the C++ words (`mergeConflicts`,
/// `mergeConflictSummary`); the area label and empty values do not count.
#[test]
fn differing_values_are_named_before_merging() {
    let row = |pairs: &[(&str, &str)]| pairs.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect::<BTreeMap<String, String>>();
    let rows = [row(&[("feature_no", "1호 주거지"), ("period", "청동기"), ("area_m2", "100")]), row(&[("feature_no", "2호 수혈"), ("period", ""), ("area_m2", "50")])];
    let conflicts = merge_conflicts(&rows);
    assert_eq!(conflicts.len(), 1, "{conflicts:?}");
    assert_eq!(conflicts[0].1, ["1호 주거지", "2호 수혈"]);
    let summary = merge_conflict_summary(&conflicts);
    assert!(summary.starts_with("고른 도형의 속성이 서로 다릅니다. 가장 먼저 그린 도형의 값만 남습니다.\n· "));
    assert!(summary.ends_with(": 1호 주거지, 2호 수혈\n\n묶은 뒤에도 Ctrl+Z로 되돌릴 수 있습니다."), "{summary}");
}

/// Two touching squares become one shape that keeps the first drawn's values, as one step that Ctrl+Z takes
/// back; fewer than two shapes or shapes apart in a layer of single polygons are refused with the C++ words.
#[test]
fn touching_shapes_merge_and_shapes_apart_are_refused() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = new_survey(dir.path());
    let first = add(&mut app, "feature_poly", &square(200_000.0, 450_000.0, 10.0), &[("feature_no", "1호")]);
    let second = add(&mut app, "feature_poly", &square(200_010.0, 450_000.0, 10.0), &[("feature_no", "1호")]);
    let far = add(&mut app, "feature_poly", &square(200_100.0, 450_100.0, 10.0), &[("feature_no", "1호")]);
    let fp = |fid: i64| ("feature_poly".to_owned(), fid);

    app.home_map.selected_shape = Some(fp(first));
    app.merge_selected_polygons();
    assert!(app.status_left_message().starts_with("묶을 면을 2개 이상 고른 뒤 누르세요."));
    assert_eq!(count(&app, "feature_poly"), 3);

    (app.home_map.also_selected, app.home_map.selected_shape) = (vec![fp(first)], Some(fp(far)));
    app.merge_selected_polygons();
    assert_eq!(app.status_left_message(), "떨어진 구역은 하나로 묶을 수 없습니다. 두 구역은 따로 저장되고 제출에도 함께 들어갑니다. 맞닿거나 겹친 구역만 하나로 묶습니다.");
    assert_eq!(count(&app, "feature_poly"), 3);

    let steps = app.undo_tables.len();
    (app.home_map.also_selected, app.home_map.selected_shape) = (vec![fp(second)], Some(fp(first)));
    app.merge_selected_polygons();
    let label = app.left_layers.entries.iter().find(|e| e.layer_key == "feature_poly").unwrap().label_ko.clone();
    assert_eq!(app.status_left_message(), format!("「{label}」의 폴리곤 2개를 1개로 묶었습니다. Ctrl+Z로 되돌릴 수 있습니다."));
    assert_eq!((count(&app, "feature_poly"), app.undo_tables.len()), (2, steps + 1));
    let kept = app.survey_tables.iter().find(|t| t.table == "feature_poly").unwrap().layer.features.iter().find(|f| f.fid == first).unwrap().clone();
    assert!((strata_engine::wkt_area(&kept.wkt_geom).unwrap().abs() - 200.0).abs() < 1e-6, "{}", kept.wkt_geom);
    assert!(app.all_selected().is_empty(), "the merged shape is not selected");
    app.undo_map_action();
    assert_eq!(count(&app, "feature_poly"), 3);
}

/// Values that differ ask first; 「아니요」 merges nothing and 「예」 merges.
#[test]
fn differing_values_ask_first() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = new_survey(dir.path());
    let first = add(&mut app, "feature_poly", &square(200_000.0, 450_000.0, 10.0), &[("feature_no", "1호 주거지")]);
    let second = add(&mut app, "feature_poly", &square(200_010.0, 450_000.0, 10.0), &[("feature_no", "2호 수혈")]);
    let fp = |fid: i64| ("feature_poly".to_owned(), fid);
    (app.home_map.also_selected, app.home_map.selected_shape) = (vec![fp(second)], Some(fp(first)));
    app.merge_selected_polygons();
    let asked = app.home_map.merge_ask.clone().expect("asks first");
    assert!(asked.contains("1호 주거지") && asked.ends_with("묶을까요?"), "{asked}");
    assert_eq!(count(&app, "feature_poly"), 2, "nothing merged before the answer");
    app.home_map.merge_ask = None;
    assert_eq!(count(&app, "feature_poly"), 2);
    app.merge_selected_polygons_confirmed();
    assert_eq!(count(&app, "feature_poly"), 1);
}
