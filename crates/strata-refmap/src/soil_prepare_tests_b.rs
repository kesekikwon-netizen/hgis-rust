//! What is stored of the soil map: the units cut to the scope with their colour and label, the answers that
//! stop the fetch, and what a stop leaves behind (`SoilMapService.cpp:255-270`, `:313-345`; the cut and the
//! two columns are this app's, docs/RUST_PLAN.md 2.9).

use std::cell::{Cell as Flag, RefCell};

use serde_json::json;
use strata_engine::{polygons_area, Cell, Scope, Shapes, Transformer};

use super::tests::{answer, folders, hits, is_hits, never, ring, scope_at, stored, table_of, unit};
use super::*;
use crate::transfer::{Request, TransferFailure, CANCELLED};

/// Answers table 1 with `features` and says the other tables hold nothing.
fn only_table_1(features: Vec<serde_json::Value>, crs: Option<&'static str>) -> impl Fn(&Request) -> Result<Vec<u8>, TransferFailure> {
    move |request| match (table_of(request), is_hits(request)) {
        (1, true) => Ok(hits(features.len() as u64)),
        (_, true) => Ok(hits(0)),
        _ => Ok(answer(features.clone(), crs, None)),
    }
}

fn text(value: &str) -> Cell {
    Cell::Text(value.to_owned())
}

/// A unit keeps its code; its colour is the official one of the code and its label the short name, written
/// only on 40 000 m2 or more of the stored shape. A code that is none of the eleven, or none at all, is the
/// grey 「미분류」 without a label. What lies outside the scope is cut off or left out.
#[test]
fn units_are_cut_to_the_scope_coloured_and_labelled() {
    let dir = tempfile::tempdir().expect("tempdir");
    let features = vec![
        unit(200_500.0, 450_500.0, 300.0, json!("01")),
        // 10 000 m2.
        unit(202_000.0, 450_500.0, 100.0, json!("05")),
        // Half of it lies east of the scope, which ends at x = 210 100 beside the survey area.
        unit(209_600.0, 450_000.0, 1_000.0, json!("77")),
        // In the corner of the box, 14 km from the survey area.
        unit(190_100.0, 440_100.0, 200.0, json!("04")),
        unit(203_000.0, 450_500.0, 300.0, json!(null)),
    ];
    let prepared = prepare(&scope_at(200_000.0, 450_000.0), &dir.path().join("soil.gpkg"), &only_table_1(features, Some("EPSG:5186")), &never).expect("prepared");
    let rows = stored(&prepared.gpkg);
    let cells: Vec<&Vec<Cell>> = rows.iter().map(|(_, cells)| cells).collect();
    assert_eq!(cells, [&vec![text("01"), text("#01B200"), text("산악지")], &vec![text("05"), text("#84BAE6"), text("")], &vec![text("77"), text("#C8C8C8"), text("")], &vec![Cell::Null, text("#C8C8C8"), text("")]]);
    let area = |shapes: &Shapes| match shapes {
        Shapes::Polygons(polygons) => polygons_area(polygons).expect("area"),
        other => panic!("{other:?}"),
    };
    assert!((area(&rows[0].0) - 90_000.0).abs() < 1.0 && (area(&rows[1].0) - 10_000.0).abs() < 1.0);
    // 500 m of its width inside the straight edge, a little less where the edge bends away.
    assert!(area(&rows[2].0) > 470_000.0 && area(&rows[2].0) < 500_001.0, "{}", area(&rows[2].0));
    assert_eq!(prepared.rows, 4);
}

/// `:340-343`: nothing in any table, or nothing inside the scope.
#[test]
fn an_area_without_soil_units_is_said() {
    for features in [Vec::new(), vec![unit(190_100.0, 440_100.0, 200.0, json!("04"))]] {
        let dir = tempfile::tempdir().expect("tempdir");
        let stopped = prepare(&scope_at(200_000.0, 450_000.0), &dir.path().join("soil.gpkg"), &only_table_1(features, Some("EPSG:5186")), &never).expect_err("stopped");
        assert_eq!(stopped, Stopped::Failed(Why::NoData, NO_DATA.to_owned()));
        assert_eq!(folders(dir.path()), 0);
    }
}

/// `:327-331`: a unit without a shape that can be read stops the fetch.
#[test]
fn a_unit_without_a_readable_shape_stops_the_fetch() {
    let good = unit(200_500.0, 450_500.0, 300.0, json!("04"));
    let no_shape = json!({"type": "Feature", "properties": {"soil_type_geo": "04"}, "geometry": null});
    let a_line = json!({"type": "Feature", "properties": {"soil_type_geo": "04"}, "geometry": {"type": "LineString", "coordinates": [[200_500.0, 450_500.0], [200_800.0, 450_500.0]]}});
    for bad in [no_shape, a_line] {
        let dir = tempfile::tempdir().expect("tempdir");
        let stopped = prepare(&scope_at(200_000.0, 450_000.0), &dir.path().join("soil.gpkg"), &only_table_1(vec![good.clone(), bad], Some("EPSG:5186")), &never).expect_err("stopped");
        assert_eq!(stopped, Stopped::Failed(Why::Data, UNREADABLE.to_owned()));
        assert_eq!(folders(dir.path()), 0);
    }
}

/// `:255-261`: a scope wider than 80 km is refused before anything is asked.
#[test]
fn a_scope_wider_than_80_km_is_refused_without_a_request() {
    let dir = tempfile::tempdir().expect("tempdir");
    let wide = Scope::around(&["POLYGON ((200000 450000, 200100 450000, 200100 450100, 200000 450100, 200000 450000))".to_owned()], 40_000.0).expect("scope").expect("some");
    let download = |_: &Request| -> Result<Vec<u8>, TransferFailure> { panic!("no request for a scope that wide") };
    let stopped = prepare(&wide, &dir.path().join("soil.gpkg"), &download, &never).expect_err("stopped");
    assert_eq!(stopped, Stopped::Failed(Why::TooWide, TOO_WIDE.to_owned()));
    assert_eq!(folders(dir.path()), 0);
}

/// A stop by the user: before anything, during a request, between the tables. Nothing is left.
#[test]
fn a_stop_leaves_nothing_in_the_survey_folder() {
    let scope = scope_at(200_000.0, 450_000.0);
    let features = || vec![unit(200_500.0, 450_500.0, 300.0, json!("04"))];
    let stop = Stopped::Cancelled(CANCELLED.to_owned());

    let dir = tempfile::tempdir().expect("tempdir");
    let none = |_: &Request| -> Result<Vec<u8>, TransferFailure> { panic!("no request after a stop") };
    assert_eq!(prepare(&scope, &dir.path().join("soil.gpkg"), &none, &|| true).expect_err("stopped"), stop);

    // The user stops while the first features come in: the request ends as cancelled.
    let during = |request: &Request| -> Result<Vec<u8>, TransferFailure> {
        if is_hits(request) {
            Ok(hits(1))
        } else {
            Err(TransferFailure::Cancelled)
        }
    };
    assert_eq!(prepare(&scope, &dir.path().join("soil.gpkg"), &during, &never).expect_err("stopped"), stop);

    // The user stops after the first table: the second is not asked.
    let (asked, stopped) = (RefCell::new(Vec::new()), Flag::new(false));
    let between = |request: &Request| -> Result<Vec<u8>, TransferFailure> {
        asked.borrow_mut().push((table_of(request), is_hits(request)));
        if is_hits(request) {
            return Ok(hits(1));
        }
        stopped.set(true);
        Ok(answer(features(), Some("EPSG:5186"), None))
    };
    assert_eq!(prepare(&scope, &dir.path().join("soil.gpkg"), &between, &|| stopped.get()).expect_err("stopped"), stop);
    assert_eq!(asked.into_inner(), [(1, true), (1, false)]);
    assert_eq!(folders(dir.path()), 0);
}

/// The request asks for EPSG:5186; an answer that says it is in another system is carried there (the C++
/// copies the coordinates as they come).
#[test]
fn an_answer_in_longitude_and_latitude_is_carried_to_the_stored_crs() {
    let dir = tempfile::tempdir().expect("tempdir");
    let to_lonlat = Transformer::new(5186, 4326).expect("crs");
    let corners = to_lonlat.transform_all(&ring(200_500.0, 450_500.0, 300.0)).expect("carried");
    let feature = json!({"type": "Feature", "properties": {"soil_type_geo": "08"}, "geometry": {"type": "MultiPolygon", "coordinates": [[corners]]}});
    let prepared = prepare(&scope_at(200_000.0, 450_000.0), &dir.path().join("soil.gpkg"), &only_table_1(vec![feature], None), &never).expect("prepared");
    let rows = stored(&prepared.gpkg);
    assert_eq!(rows[0].1, [text("08"), text("#FEA000"), text("홍적대지")]);
    let Shapes::Polygons(polygons) = &rows[0].0 else {
        panic!("polygons");
    };
    let (xs, ys): (Vec<f64>, Vec<f64>) = polygons[0][0].iter().map(|p| (p[0], p[1])).unzip();
    let low = |v: &[f64]| v.iter().copied().fold(f64::INFINITY, f64::min);
    let high = |v: &[f64]| v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    for (got, wanted) in [low(&xs), high(&xs), low(&ys), high(&ys)].into_iter().zip([200_500.0, 200_800.0, 450_500.0, 450_800.0]) {
        assert!((got - wanted).abs() < 0.01, "{got} for {wanted}");
    }
}
