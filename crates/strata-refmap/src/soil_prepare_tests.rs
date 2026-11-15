//! The soil cases of the C++ tests with their answers (soilDownload_usesViewExtentAndTerrainFieldOnly,
//! `target/cpp-ref/tests/test_workflow.cpp:1573-1620`; partialSoilFailurePreservesOriginal,
//! `test_reference_download.cpp:126-146`) and the counting rules of `SoilMapService.cpp:275-312`.
//! Every answer is made here; no request leaves the test.

use std::cell::RefCell;
use std::path::Path;

use serde_json::{json, Value};
use strata_engine::{read_table, Cell, Scope, ShapeKind, Shapes, Transformer};

use super::*;
use crate::collection::{NOT_MAP_DATA, ONLY_PART};
use crate::transfer::{Request, TransferFailure, NOT_REACHED, TOO_SLOW};

/// The 10 km scope of a 100 m survey area whose south-west corner is (x, y), EPSG:5186.
pub(super) fn scope_at(x: f64, y: f64) -> Scope {
    let square = format!("POLYGON (({x} {y}, {} {y}, {} {}, {x} {}, {x} {y}))", x + 100.0, x + 100.0, y + 100.0, y + 100.0);
    Scope::around(&[square], 10_000.0).expect("scope").expect("some")
}

pub(super) fn ring(x: f64, y: f64, side: f64) -> Vec<[f64; 2]> {
    vec![[x, y], [x + side, y], [x + side, y + side], [x, y + side], [x, y]]
}

/// A square unit of a terrain class.
pub(super) fn unit(x: f64, y: f64, side: f64, code: Value) -> Value {
    json!({"type": "Feature", "properties": {"soil_type_geo": code}, "geometry": {"type": "Polygon", "coordinates": [ring(x, y, side)]}})
}

pub(super) fn answer(features: Vec<Value>, crs: Option<&str>, matched: Option<u64>) -> Vec<u8> {
    let mut collection = json!({"type": "FeatureCollection", "features": features});
    if let Some(name) = crs {
        collection["crs"] = json!({"type": "name", "properties": {"name": name}});
    }
    if let Some(n) = matched {
        collection["numberMatched"] = json!(n);
    }
    collection.to_string().into_bytes()
}

/// What the service answers to a hits request (the C++ test's `<FeatureCollection numberMatched="1"/>`).
pub(super) fn hits(n: u64) -> Vec<u8> {
    format!("<FeatureCollection numberMatched=\"{n}\"/>").into_bytes()
}

pub(super) fn is_hits(request: &Request) -> bool {
    request.url.contains("&resultType=hits&")
}

/// The number of the table a request asks (`typeNames=soilmap:SOIL_<n>`).
pub(super) fn table_of(request: &Request) -> u8 {
    request.url.split('&').find_map(|p| p.strip_prefix("typeNames=soilmap:SOIL_")).and_then(|n| n.parse().ok()).expect("table")
}

/// The stored rows: shape, terrain code, colour and label.
pub(super) fn stored(gpkg: &Path) -> Vec<(Shapes, Vec<Cell>)> {
    let mut rows = Vec::new();
    let facts = read_table(gpkg, TABLE, &["soil_type_geo", COLOUR_COLUMN, LABEL_COLUMN], &mut |row| {
        rows.push((row.shapes, row.cells));
        true
    })
    .expect("read");
    assert_eq!((facts.epsg, facts.kind), (Some(5186), ShapeKind::Polygons));
    rows
}

/// How many reference folders stand in the survey folder.
pub(super) fn folders(dir: &Path) -> usize {
    std::fs::read_dir(dir).expect("dir").flatten().filter(|e| e.file_name().to_string_lossy().starts_with("ka-hgis-reference-")).count()
}

pub(super) fn never() -> bool {
    false
}

fn text(value: &str) -> Cell {
    Cell::Text(value.to_owned())
}

/// soilDownload_usesViewExtentAndTerrainFieldOnly, with the owner's box: a count and a feature request for
/// each of the three tables, all for the box of the scope, the features with the terrain class only.
#[test]
fn six_requests_for_the_box_of_the_scope_with_the_terrain_field_only() {
    let dir = tempfile::tempdir().expect("tempdir");
    let scope = scope_at(200_000.0, 450_000.0);
    let expected = Transformer::new(5186, 4326).expect("crs").transform_box(scope.bbox().expect("box")).expect("carried");
    let requests = RefCell::new(Vec::new());
    let download = |request: &Request| -> Result<Vec<u8>, TransferFailure> {
        assert_eq!(request.user_agent, "ka-hgis/0.3");
        requests.borrow_mut().push(request.url.clone());
        if is_hits(request) {
            // A count is a small answer: the plain limits (`ReferenceTransferLimits`, 15 s and 15 s).
            assert_eq!((request.idle_secs, request.total_secs), (15, 15));
            return Ok(hits(1));
        }
        // "Polygon pages can be large: a progressing transfer is not cut at 15 seconds."
        assert_eq!((request.idle_secs, request.total_secs), (15, 180));
        Ok(answer(vec![unit(200_000.0 + f64::from(table_of(request)) * 1_000.0, 450_000.0, 300.0, json!("04"))], Some("EPSG:5186"), None))
    };
    let prepared = prepare(&scope, &dir.path().join("soil.gpkg"), &download, &never).expect("prepared");

    let requests = requests.into_inner();
    let order: Vec<(bool, bool)> = requests.iter().map(|url| (url.contains("typeNames=soilmap:SOIL_1&"), url.contains("&resultType=hits&"))).collect();
    assert_eq!(order, [(true, true), (true, false), (false, true), (false, false), (false, true), (false, false)]);
    for url in &requests {
        let bbox: Vec<&str> = url.split('&').find_map(|p| p.strip_prefix("bbox=")).expect("bbox").split(',').collect();
        assert_eq!((bbox.len(), bbox[4]), (5, "urn:ogc:def:crs:EPSG::4326"));
        for (given, wanted) in bbox.iter().zip([expected[1], expected[0], expected[3], expected[2]]) {
            assert!((given.parse::<f64>().expect("number") - wanted).abs() < 1e-7, "{url}");
        }
        if !url.contains("&resultType=hits&") {
            assert!(url.contains("&propertyName=soil_type_geo,geom&") && url.contains("&outputFormat=application/json&"), "{url}");
        }
    }
    // The scope reaches 10 km from the survey area: about 20.1 km each way.
    assert!((expected[2] - expected[0] - 0.228).abs() < 0.01 && (expected[3] - expected[1] - 0.181).abs() < 0.01, "{expected:?}");

    assert_eq!((prepared.rows, prepared.warnings.len()), (3, 0));
    assert!(prepared.gpkg.ends_with("soil.gpkg") && prepared.gpkg.parent() == Some(prepared.folder.as_path()) && folders(dir.path()) == 1);
    let rows = stored(&prepared.gpkg);
    assert_eq!(rows.len(), 3);
    for (_, cells) in rows {
        assert_eq!(cells, [text("04"), text("#FEF4B6"), text("곡간·선상")]);
    }
}

/// partialSoilFailurePreservesOriginal: the second table cannot be had. The map that is there stays as it
/// is and nothing new is left beside it.
#[test]
fn a_table_that_cannot_be_had_leaves_the_kept_map_and_nothing_else() {
    for (failure, sentence) in [(TransferFailure::Network, NOT_REACHED), (TransferFailure::Timeout, TOO_SLOW)] {
        let dir = tempfile::tempdir().expect("tempdir");
        let base = dir.path().join("soil.gpkg");
        std::fs::write(&base, b"original-reference").expect("write");
        let download = |request: &Request| -> Result<Vec<u8>, TransferFailure> {
            match (table_of(request), is_hits(request)) {
                (2, _) => Err(failure),
                (_, true) => Ok(hits(1)),
                _ => Ok(answer(vec![unit(200_500.0, 450_500.0, 300.0, json!("04"))], Some("EPSG:5186"), None)),
            }
        };
        let stopped = prepare(&scope_at(200_000.0, 450_000.0), &base, &download, &never).expect_err("stopped");
        assert_eq!(stopped, Stopped::Failed(Why::Service, sentence.to_owned()));
        assert_eq!(std::fs::read(&base).expect("read"), b"original-reference");
        assert_eq!(folders(dir.path()), 0);
    }
}

/// `:288`: a table that matches nothing is not asked for its features.
#[test]
fn a_table_without_matches_is_not_asked_for_features() {
    let dir = tempfile::tempdir().expect("tempdir");
    let asked = RefCell::new(Vec::new());
    let download = |request: &Request| -> Result<Vec<u8>, TransferFailure> {
        asked.borrow_mut().push((table_of(request), is_hits(request)));
        match (table_of(request), is_hits(request)) {
            (3, true) => Ok(hits(1)),
            (_, true) => Ok(hits(0)),
            _ => Ok(answer(vec![unit(200_500.0, 450_500.0, 300.0, json!("01"))], Some("EPSG:5186"), None)),
        }
    };
    let prepared = prepare(&scope_at(200_000.0, 450_000.0), &dir.path().join("soil.gpkg"), &download, &never).expect("prepared");
    assert_eq!(asked.into_inner(), [(1, true), (2, true), (3, true), (3, false)]);
    assert_eq!(prepared.rows, 1);
}

/// `:283`: a count that cannot be had (or read) is gone without; the features are asked for all the same.
#[test]
fn a_count_that_cannot_be_had_is_gone_without() {
    let dir = tempfile::tempdir().expect("tempdir");
    let asked = RefCell::new(0);
    let download = |request: &Request| -> Result<Vec<u8>, TransferFailure> {
        *asked.borrow_mut() += 1;
        match (table_of(request), is_hits(request)) {
            (1, true) => Err(TransferFailure::Network),
            (2, true) => Err(TransferFailure::Timeout),
            (_, true) => Ok(b"<ows:ExceptionReport/>".to_vec()),
            (table, _) => Ok(answer(vec![unit(200_000.0 + f64::from(table) * 1_000.0, 450_500.0, 300.0, json!("02"))], Some("EPSG:5186"), None)),
        }
    };
    let prepared = prepare(&scope_at(200_000.0, 450_000.0), &dir.path().join("soil.gpkg"), &download, &never).expect("prepared");
    assert_eq!((asked.into_inner(), prepared.rows), (6, 3));
}

/// `:289-293`: more matches than one answer holds stops everything before a feature is asked for.
#[test]
fn more_matches_than_one_answer_holds_stop_the_fetch() {
    let dir = tempfile::tempdir().expect("tempdir");
    let asked = RefCell::new(0);
    let download = |request: &Request| -> Result<Vec<u8>, TransferFailure> {
        *asked.borrow_mut() += 1;
        assert!(is_hits(request), "no feature request before or after such a count");
        Ok(hits(if table_of(request) == 1 { 0 } else { 100_001 }))
    };
    let stopped = prepare(&scope_at(200_000.0, 450_000.0), &dir.path().join("soil.gpkg"), &download, &never).expect_err("stopped");
    assert_eq!(stopped, Stopped::Failed(Why::TooWide, TOO_MANY.to_owned()));
    assert_eq!((asked.into_inner(), folders(dir.path())), (2, 0));
}

/// `:306-311`: fewer features than were counted, or as many as one answer can hold, is not the whole table.
#[test]
fn an_answer_that_is_not_the_whole_table_is_refused() {
    let one = || vec![unit(200_500.0, 450_500.0, 300.0, json!("04"))];
    let bare = r#"{"type":"Feature","properties":{},"geometry":null}"#;
    let full = format!(r#"{{"type":"FeatureCollection","features":[{}]}}"#, vec![bare; 100_000].join(",")).into_bytes();
    let cases: [(Option<u64>, Vec<u8>, &str); 5] = [
        (Some(2), answer(one(), Some("EPSG:5186"), None), NOT_WHOLE),
        // A count of exactly 100 000 is let through (`hits > 100000`); the answer of that size is not.
        (Some(100_000), full, NOT_WHOLE),
        // `validateCompleteFeatureCollection`: the service itself says it matched more than it sent.
        (Some(1), answer(one(), Some("EPSG:5186"), Some(5)), ONLY_PART),
        (None, b"<html>busy</html>".to_vec(), NOT_MAP_DATA),
        // An answer whose coordinates are in a system nobody knows cannot be placed: the part is not read.
        (Some(1), answer(one(), Some("EPSG:banana"), None), NOT_WHOLE),
    ];
    for (count, body, sentence) in cases {
        let dir = tempfile::tempdir().expect("tempdir");
        let download = |request: &Request| -> Result<Vec<u8>, TransferFailure> {
            match (is_hits(request), count) {
                (true, Some(n)) => Ok(hits(n)),
                (true, None) => Err(TransferFailure::Network),
                _ => Ok(body.clone()),
            }
        };
        let stopped = prepare(&scope_at(200_000.0, 450_000.0), &dir.path().join("soil.gpkg"), &download, &never).expect_err("stopped");
        assert_eq!(stopped, Stopped::Failed(Why::Service, sentence.to_owned()));
        assert_eq!(folders(dir.path()), 0);
    }
}
