//! The river cases of the C++ tests with their answers (riverPageLimitIsFailure and
//! riverRetriesDroppedPieceOnce, `target/cpp-ref/tests/test_reference_download.cpp:149-188`, with
//! `riverData`, `:77-79`) and what is stored. Every answer is made here; no request leaves the test.
//! `TEST_KEY` is the key of the C++ tests, not a real one.

use std::cell::RefCell;
use std::path::Path;

use serde_json::{json, Value};
use strata_engine::{read_table, Cell, Scope, ShapeKind, Shapes, Transformer};

use super::*;
use crate::river_request::PAGE_SIZE;
use crate::transfer::{Request, TransferFailure};

/// Where the river of the C++ test lies (127 E, 37 N), in EPSG:5186.
pub(super) fn at_the_river() -> [f64; 2] {
    Transformer::new(4326, 5186).expect("crs").transform([127.0, 37.0]).expect("carried")
}

/// The 10 km scope of a 100 m survey area whose south-west corner is the river's.
pub(super) fn scope() -> Scope {
    let [x, y] = at_the_river();
    let square = format!("POLYGON (({x} {y}, {} {y}, {} {}, {x} {}, {x} {y}))", x + 100.0, x + 100.0, y + 100.0, y + 100.0);
    Scope::around(&[square], 10_000.0).expect("scope").expect("some")
}

/// A river of 0.001 degrees each way whose south-west corner is (lon, lat), as `riverData` has it.
pub(super) fn river(id: usize, lon: f64, lat: f64, name: &str, level: Value) -> Value {
    let ring = [[lon, lat], [lon + 0.001, lat], [lon + 0.001, lat + 0.001], [lon, lat + 0.001], [lon, lat]];
    json!({"type": "Feature", "id": format!("lt_c_wkmstrm.{id}"), "properties": {"riv_nm": name, "riv_level": level}, "geometry": {"type": "Polygon", "coordinates": [ring]}})
}

pub(super) fn answer(features: Vec<Value>) -> Vec<u8> {
    json!({"type": "FeatureCollection", "features": features}).to_string().into_bytes()
}

/// `riverData`: one national river at 127 E, 37 N.
pub(super) fn river_data() -> Vec<u8> {
    answer(vec![river(1, 127.0, 37.0, "시험하천", json!("국가하천"))])
}

/// The page a request asks for (`STARTINDEX` in thousands).
pub(super) fn page_of(request: &Request) -> usize {
    request.url.split('&').find_map(|p| p.strip_prefix("STARTINDEX=")).and_then(|n| n.parse::<usize>().ok()).expect("start index") / PAGE_SIZE
}

pub(super) fn stored(gpkg: &Path) -> Vec<(Shapes, Vec<Cell>)> {
    let mut rows = Vec::new();
    let facts = read_table(gpkg, TABLE, &[NAME_COLUMN, LEVEL_COLUMN, COLOUR_COLUMN, LINE_COLUMN], &mut |row| {
        rows.push((row.shapes, row.cells));
        true
    })
    .expect("read");
    assert_eq!((facts.epsg, facts.kind), (Some(5186), ShapeKind::Polygons));
    rows
}

pub(super) fn folders(dir: &Path) -> usize {
    std::fs::read_dir(dir).expect("dir").flatten().filter(|e| e.file_name().to_string_lossy().starts_with("ka-hgis-reference-")).count()
}

pub(super) fn never() -> bool {
    false
}

pub(super) fn text(value: &str) -> Cell {
    Cell::Text(value.to_owned())
}

/// One request for the box of the scope with the user's key, as a program that says what it is, with the
/// address the key is registered for. The rivers are carried from longitude and latitude into EPSG:5186
/// and stored with the water colour of their class.
#[test]
fn the_rivers_of_the_scope_are_fetched_carried_and_coloured() {
    let dir = tempfile::tempdir().expect("tempdir");
    let scope = scope();
    let expected = Transformer::new(5186, 4326).expect("crs").transform_box(scope.bbox().expect("box")).expect("carried");
    let requests = RefCell::new(Vec::new());
    let download = |request: &Request| -> Result<Vec<u8>, TransferFailure> {
        assert_eq!((request.user_agent, request.referer), ("ka-hgis/0.3", Some("https://localhost")));
        assert_eq!((request.idle_secs, request.total_secs), (15, 180));
        requests.borrow_mut().push(request.url.clone());
        Ok(answer(vec![
            river(1, 127.0, 37.0, "시험하천", json!("국가하천")),
            river(2, 127.01, 37.0, "작은내", json!("지방2급하천")),
            river(3, 127.02, 37.0, "이름 없는 물", json!(null)),
            // 0.3 degrees east: 27 km from the survey area.
            river(4, 127.3, 37.0, "먼 강", json!("국가하천")),
        ]))
    };
    let prepared = prepare(&scope, &dir.path().join("river.gpkg"), "TEST_KEY", &download, &never).expect("prepared");

    let requests = requests.into_inner();
    assert_eq!(requests.len(), 1);
    assert!(requests[0].starts_with("https://api.vworld.kr/req/wfs?") && requests[0].ends_with("&KEY=TEST_KEY") && requests[0].contains("&STARTINDEX=0&"), "{}", requests[0]);
    let bbox: Vec<&str> = requests[0].split('&').find_map(|p| p.strip_prefix("BBOX=")).expect("bbox").split(',').collect();
    for (given, wanted) in bbox.iter().zip([expected[1], expected[0], expected[3], expected[2]]) {
        assert!((given.parse::<f64>().expect("number") - wanted).abs() < 1e-7, "{}", requests[0]);
    }

    assert_eq!((prepared.rows, prepared.warnings.len(), folders(dir.path())), (3, 0, 1));
    let rows = stored(&prepared.gpkg);
    let cells: Vec<&Vec<Cell>> = rows.iter().map(|(_, cells)| cells).collect();
    assert_eq!(
        cells,
        [
            &vec![text("시험하천"), text("국가하천"), text("#007ED4"), text("#00548D")],
            &vec![text("작은내"), text("지방2급하천"), text("#28A6E8"), text("#1B6F9B")],
            &vec![text("이름 없는 물"), Cell::Null, text("#78C4EC"), text("#50839D")],
        ]
    );
    // The first river's south-west corner is the survey area's: 0.001 degrees are 89 m east and 111 m north.
    let Shapes::Polygons(polygons) = &rows[0].0 else {
        panic!("polygons");
    };
    let [x, y] = at_the_river();
    let (xs, ys): (Vec<f64>, Vec<f64>) = polygons[0][0].iter().map(|p| (p[0], p[1])).unzip();
    let low = |v: &[f64]| v.iter().copied().fold(f64::INFINITY, f64::min);
    let high = |v: &[f64]| v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    assert!((low(&xs) - x).abs() < 0.5 && (low(&ys) - y).abs() < 0.5, "{xs:?} {ys:?}");
    assert!((high(&xs) - x - 88.9).abs() < 1.0 && (high(&ys) - y - 111.0).abs() < 1.0, "{xs:?} {ys:?}");
}

/// riverPageLimitIsFailure: every answer is a full page. The whole box, then a quarter, then a sixteenth
/// are asked for ten pages each (30 requests) and the fetch fails instead of storing a cut map.
#[test]
fn a_box_that_stays_full_after_two_splits_fails_after_thirty_requests() {
    let dir = tempfile::tempdir().expect("tempdir");
    let full_page = answer(vec![river(1, 127.0, 37.0, "시험하천", json!("국가하천")); PAGE_SIZE]);
    let calls = RefCell::new(0);
    let download = |_: &Request| -> Result<Vec<u8>, TransferFailure> {
        *calls.borrow_mut() += 1;
        Ok(full_page.clone())
    };
    let stopped = prepare(&scope(), &dir.path().join("river.gpkg"), "TEST_KEY", &download, &never).expect_err("stopped");
    assert_eq!(calls.into_inner(), 30);
    // The piece-wise fetch says first that it gave up after splitting, then why (`ReferenceTiledFetch.cpp`).
    let Stopped::Failed(Why::Service, text) = &stopped else {
        panic!("{stopped:?}");
    };
    assert!(text.starts_with("VWorld 서버 연결 실패: ") && text.ends_with(TOO_MANY), "{text}");
    assert_eq!(folders(dir.path()), 0);
}

/// riverRetriesDroppedPieceOnce: a dropped connection is tried once more.
#[test]
fn a_dropped_request_is_sent_once_more() {
    let dir = tempfile::tempdir().expect("tempdir");
    let calls = RefCell::new(0);
    let download = |_: &Request| -> Result<Vec<u8>, TransferFailure> {
        *calls.borrow_mut() += 1;
        if *calls.borrow() == 1 {
            return Err(TransferFailure::Network);
        }
        Ok(river_data())
    };
    let prepared = prepare(&scope(), &dir.path().join("river.gpkg"), "TEST_KEY", &download, &never).expect("prepared");
    assert_eq!((calls.into_inner(), prepared.rows), (2, 1));
}

/// A box with more rivers than one answer holds is asked for page after page until a page is not full.
#[test]
fn a_full_page_is_followed_by_the_next_one() {
    let dir = tempfile::tempdir().expect("tempdir");
    let asked = RefCell::new(Vec::new());
    let download = |request: &Request| -> Result<Vec<u8>, TransferFailure> {
        let page = page_of(request);
        asked.borrow_mut().push(page);
        // Rivers side by side along 37 N, 0.00002 degrees apart, each with its own id.
        let count = if page == 0 { PAGE_SIZE } else { 3 };
        Ok(answer((0..count).map(|i| river(page * PAGE_SIZE + i, 127.0 + 0.000_02 * (page * PAGE_SIZE + i) as f64, 37.0, "긴 강", json!("지방1급하천"))).collect()))
    };
    let prepared = prepare(&scope(), &dir.path().join("river.gpkg"), "TEST_KEY", &download, &never).expect("prepared");
    assert_eq!(asked.into_inner(), [0, 1]);
    assert_eq!(prepared.rows, 1003);
}
