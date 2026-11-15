//! The CRS forms of `target/cpp-ref/tests/test_reference_download.cpp:507-517` (the six rows of
//! geologyResponseCrsPreservesSurveyLocation) and the reading rules of `GeologyMapService.cpp:795-857`.

use serde_json::{json, Value};
use strata_engine::{Cell, ColumnKind};

use super::*;

fn with_crs(crs: Value) -> serde_json::Map<String, Value> {
    json!({"type": "FeatureCollection", "crs": crs, "features": []}).as_object().expect("object").clone()
}

fn named(name: &str) -> serde_json::Map<String, Value> {
    with_crs(json!({"type": "name", "properties": {"name": name}}))
}

#[test]
fn the_declared_crs_is_read_in_the_forms_the_services_use() {
    assert_eq!(declared_epsg(&named("EPSG:5186")), Some(5186));
    assert_eq!(declared_epsg(&named("EPSG:5187")), Some(5187));
    assert_eq!(declared_epsg(&named("urn:ogc:def:crs:EPSG::4326")), Some(4326));
    assert_eq!(declared_epsg(&named("urn:ogc:def:crs:EPSG::5186")), Some(5186));
    assert_eq!(declared_epsg(&named("urn:ogc:def:crs:EPSG:6.18:5179")), Some(5179));
    assert_eq!(declared_epsg(&named("urn:ogc:def:crs:OGC:1.3:CRS84")), Some(4326));
    assert_eq!(declared_epsg(&named("http://www.opengis.net/def/crs/EPSG/0/5186")), Some(5186));
    assert_eq!(declared_epsg(&with_crs(json!({"type": "EPSG", "properties": {"code": 5186}}))), Some(5186));
    // No crs member: GeoJSON's own longitude and latitude.
    let plain = json!({"type": "FeatureCollection", "features": []}).as_object().expect("object").clone();
    assert_eq!(declared_epsg(&plain), Some(4326));
    // A code PROJ does not know is still a code; the caller finds out when it asks for the transformation.
    assert_eq!(declared_epsg(&named("EPSG:999999")), Some(999_999));
}

#[test]
fn a_crs_named_in_no_known_form_is_not_guessed() {
    for name in ["", "Korea 2000 / Central Belt 2010", "5186", "EPSG:", "EPSG:five", "urn:ogc:def:crs:ESRI::102080"] {
        assert_eq!(declared_epsg(&named(name)), None, "{name}");
    }
    assert_eq!(declared_epsg(&with_crs(json!({"type": "link", "properties": {"href": "http://example.invalid/crs"}}))), None);
    assert_eq!(declared_epsg(&with_crs(json!({"type": "name"}))), None);
    assert_eq!(declared_epsg(&with_crs(json!(null))), None);
    assert_eq!(declared_epsg(&with_crs(json!({"type": "EPSG", "properties": {"code": "5186"}}))), None);
}

#[test]
fn polygons_come_out_ring_by_ring() {
    let polygon = json!({"type": "Polygon", "coordinates": [[[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 0.0]], [[2.0, 1.0], [3.0, 1.0], [3.0, 2.0], [2.0, 1.0]]]});
    let got = polygons_of(&polygon).expect("polygons");
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].len(), 2);
    assert_eq!(got[0][1][1], [3.0, 1.0]);
    let multi = json!({"type": "MultiPolygon", "coordinates": [[[[0, 0], [1, 0], [1, 1], [0, 0]]], [[[5, 5, 12.5], [6, 5, 12.5], [6, 6, 12.5], [5, 5, 12.5]]]]});
    let got = polygons_of(&multi).expect("polygons");
    assert_eq!(got.len(), 2);
    assert_eq!(got[1][0][2], [6.0, 6.0], "a height is left out");
}

#[test]
fn what_is_no_polygon_is_no_polygon() {
    assert_eq!(polygons_of(&Value::Null), None);
    assert_eq!(polygons_of(&json!({"type": "Point", "coordinates": [1.0, 2.0]})), None);
    assert_eq!(polygons_of(&json!({"type": "LineString", "coordinates": [[1.0, 2.0], [3.0, 4.0]]})), None);
    assert_eq!(polygons_of(&json!({"type": "MultiLineString", "coordinates": [[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 0.0]]]})), None, "written like a polygon, but a line");
    assert_eq!(polygons_of(&json!({"type": "Polygon"})), None);
    assert_eq!(polygons_of(&json!({"type": "Polygon", "coordinates": [[[0.0, "x"]]]})), None);
    assert_eq!(polygons_of(&json!({"type": "Polygon", "coordinates": [[[0.0]]]})), None);
    // A polygon without a ring is nothing to draw; the caller sees an empty list.
    assert_eq!(polygons_of(&json!({"type": "MultiPolygon", "coordinates": [[]]})), Some(Vec::new()));
}

#[test]
fn properties_become_columns_feature_after_feature() {
    let features = vec![
        json!({"type": "Feature", "properties": {"시대": "현생누대 신생대 제4기", "기호": "Qa", "symnum": 352_001, "area": 12, "note": null}}),
        json!({"type": "Feature", "properties": {"기호": "Kswt", "area": 3.5, "symnum": 5, "note": null, "mapname": "고창"}}),
        json!({"type": "Feature", "properties": {"symnum": "unknown-at-first-sight", "extra": true}}),
        json!({"type": "Feature"}),
    ];
    assert_eq!(
        columns_of(&features),
        [
            ("area".to_owned(), ColumnKind::Real),
            ("note".to_owned(), ColumnKind::Text),
            ("symnum".to_owned(), ColumnKind::Text),
            ("기호".to_owned(), ColumnKind::Text),
            ("시대".to_owned(), ColumnKind::Text),
            ("mapname".to_owned(), ColumnKind::Text),
            ("extra".to_owned(), ColumnKind::Text),
        ]
    );
    let whole = vec![json!({"properties": {"n": 1}}), json!({"properties": {"n": 2}})];
    assert_eq!(columns_of(&whole), [("n".to_owned(), ColumnKind::Integer)]);
}

#[test]
fn markup_is_taken_out_of_texts() {
    assert_eq!(strip_tags("<a href=\"https://data.kigam.re.kr/x\">선운사 용결응회암</a>"), "선운사 용결응회암");
    assert_eq!(strip_tags("앞<b>가운데</b>뒤"), "앞가운데뒤");
    assert_eq!(strip_tags("a < b"), "a < b", "no closing sign: nothing to take out");
    assert_eq!(strip_tags("a <b and c> d <"), "a  d <");
    assert_eq!(strip_tags(""), "");
    assert_eq!(strip_tags("<>"), "");
}

#[test]
fn a_property_becomes_the_cell_of_its_column() {
    assert_eq!(cell(Some(&json!("<i>Qa</i>")), ColumnKind::Text), Cell::Text("Qa".to_owned()));
    assert_eq!(cell(Some(&json!("Qa")), ColumnKind::Text), Cell::Text("Qa".to_owned()));
    assert_eq!(cell(Some(&json!(352_001)), ColumnKind::Integer), Cell::Integer(352_001));
    assert_eq!(cell(Some(&json!(12)), ColumnKind::Real), Cell::Real(12.0));
    assert_eq!(cell(Some(&json!(3.5)), ColumnKind::Real), Cell::Real(3.5));
    // A number in a column that turned out to be text is written as it reads.
    assert_eq!(cell(Some(&json!(5)), ColumnKind::Text), Cell::Text("5".to_owned()));
    assert_eq!(cell(Some(&json!(true)), ColumnKind::Text), Cell::Text("true".to_owned()));
    assert_eq!(cell(Some(&Value::Null), ColumnKind::Text), Cell::Null);
    assert_eq!(cell(None, ColumnKind::Integer), Cell::Null);
}
