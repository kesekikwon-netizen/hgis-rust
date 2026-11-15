//! The four geology cases of `target/cpp-ref/tests/test_reference_download.cpp` with their answers
//! (geologySplitsPartialResponseIntoQuarters :190-249, geologyServerErrorIsNotNoDataFallback :481-488,
//! geologyResponseCrsPreservesSurveyLocation :507-640), and the rules the owner's radius adds (the cut).
//! Every answer is made here; no request leaves the test.

use std::cell::{Cell as Counter, RefCell};
use std::path::Path;

use serde_json::{json, Value};
use strata_engine::{read_table, Cell, Scope, ShapeKind, Shapes, Transformer};

use super::*;
use crate::transfer::{Request, TransferFailure, Why};

pub(super) fn square_wkt(x: f64, y: f64, side: f64) -> String {
    format!("POLYGON (({x} {y}, {} {y}, {} {}, {x} {}, {x} {y}))", x + side, x + side, y + side, y + side)
}

/// The 10 km scope of a 100 m survey area whose south-west corner is (x, y), EPSG:5186.
pub(super) fn scope_at(x: f64, y: f64) -> Scope {
    Scope::around(&[square_wkt(x, y, 100.0)], 10_000.0).expect("scope").expect("some")
}

pub(super) fn unit(id: &str, x: f64, y: f64, side: f64, symbol: &str, era: &str) -> Value {
    json!({"type": "Feature", "id": id, "properties": {"기호": symbol, "지층": "시험 충적층", "시대": era},
           "geometry": {"type": "Polygon", "coordinates": [[[x, y], [x + side, y], [x + side, y + side], [x, y + side], [x, y]]]}})
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

/// A picture of the size the request asks for, filled with one colour (the C++ tests paint 249,249,127).
pub(super) fn picture(url: &str, rgba: [u8; 4]) -> Vec<u8> {
    let side = |name: &str| url.split('&').find_map(|p| p.strip_prefix(name)).and_then(|v| v.parse::<u32>().ok()).expect("size in the request");
    let image = image::RgbaImage::from_pixel(side("width="), side("height="), image::Rgba(rgba));
    let mut bytes = std::io::Cursor::new(Vec::new());
    image.write_to(&mut bytes, image::ImageFormat::Png).expect("png");
    bytes.into_inner()
}

pub(super) fn is_picture(request: &Request) -> bool {
    request.url.contains("request=GetMap")
}

/// Width of the box a feature request asks for, in degrees of longitude (`bbox=lat,lon,lat,lon,urn…`).
pub(super) fn asked_width(url: &str) -> f64 {
    let bbox = url.split('&').find_map(|p| p.strip_prefix("bbox=")).expect("bbox");
    let v: Vec<f64> = bbox.split(',').take(4).map(|n| n.parse().expect("number")).collect();
    v[3] - v[1]
}

pub(super) fn stored(gpkg: &Path, columns: &[&str]) -> Vec<(Shapes, Vec<Cell>)> {
    let mut rows = Vec::new();
    let facts = read_table(gpkg, TABLE, columns, &mut |row| {
        rows.push((row.shapes, row.cells));
        true
    })
    .expect("read");
    assert_eq!((facts.epsg, facts.kind), (Some(5186), ShapeKind::Polygons));
    rows
}

fn never() -> bool {
    false
}

/// geologySplitsPartialResponseIntoQuarters: the whole box answers 1 of 5 matched features, so the four
/// quarters are asked; each repeats the unit on the shared edge and adds its own. Five feature requests, five
/// stored units (the shared one once), and the official colour read from the picture.
#[test]
fn a_partial_answer_is_fetched_again_in_quarters_and_joined() {
    let dir = tempfile::tempdir().expect("tempdir");
    let scope = scope_at(200_000.0, 450_000.0);
    let (feature_requests, quarter) = (Counter::new(0), Counter::new(0));
    let whole_width = RefCell::new(None::<f64>);
    let download = |request: &Request| -> Result<Vec<u8>, TransferFailure> {
        assert_eq!(request.user_agent, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ka-hgis/0.3");
        assert_eq!((request.idle_secs, request.total_secs), (15, 180));
        if is_picture(request) {
            return Ok(picture(&request.url, [249, 249, 127, 255]));
        }
        assert!(request.url.contains("typeNames=geoOpen:l_50k_geology_litho_latest&") && request.url.contains("srsName=EPSG:5186&"));
        feature_requests.set(feature_requests.get() + 1);
        let width = asked_width(&request.url);
        let whole = *whole_width.borrow_mut().get_or_insert(width);
        if width > whole * 0.75 {
            // The server cut the answer for the whole box short.
            return Ok(answer(vec![unit("litho.1", 200_000.0, 450_000.0, 10.0, "Qa", "제4기")], Some("EPSG:5186"), Some(5)));
        }
        quarter.set(quarter.get() + 1);
        let own = unit(&format!("litho.q{}", quarter.get()), 200_000.0 + 20.0 * f64::from(quarter.get()), 450_000.0, 10.0, "Qa", "제4기");
        Ok(answer(vec![unit("litho.1", 200_000.0, 450_000.0, 10.0, "Qa", "제4기"), own], Some("EPSG:5186"), Some(2)))
    };
    let prepared = prepare(&scope, &dir.path().join(FILE_NAME), &download, &never).expect("prepared");
    assert_eq!(feature_requests.get(), 5);
    assert_eq!(prepared.rows, 5, "the unit on the shared edge is stored once");
    assert!(prepared.warnings.is_empty(), "{:?}", prepared.warnings);
    assert_eq!(prepared.gpkg.parent(), Some(prepared.folder.as_path()));
    assert_eq!(prepared.gpkg.file_name().and_then(|n| n.to_str()), Some(FILE_NAME));
    assert!(!dir.path().join(FILE_NAME).exists(), "the requested path itself is not written");
    let rows = stored(&prepared.gpkg, &["기호", "지층", "시대", ERA_COLUMN, COLOUR_COLUMN]);
    assert_eq!(rows.len(), 5);
    for (_, cells) in &rows {
        assert_eq!(cells, &[Cell::Text("Qa".to_owned()), Cell::Text("시험 충적층".to_owned()), Cell::Text("제4기".to_owned()), Cell::Text("제4기".to_owned()), Cell::Text("#F9F97F".to_owned())]);
    }
}

/// geologyServerErrorIsNotNoDataFallback (:481-488): an error page is a failure, said as one, after a single
/// request (a bad answer is neither split nor repeated), and nothing is left in the survey folder.
#[test]
fn an_error_page_is_a_failure_not_an_empty_map() {
    let dir = tempfile::tempdir().expect("tempdir");
    let requests = Counter::new(0);
    let download = |_: &Request| -> Result<Vec<u8>, TransferFailure> {
        requests.set(requests.get() + 1);
        Ok(b"<ServiceExceptionReport>unavailable</ServiceExceptionReport>".to_vec())
    };
    let stopped = prepare(&scope_at(200_000.0, 450_000.0), &dir.path().join(FILE_NAME), &download, &never).expect_err("failed");
    assert_eq!(stopped, Stopped::Failed(Why::Service, "KIGAM 서버 연결 실패: 서버가 GeoJSON 대신 다른 응답을 보냈습니다.".to_owned()));
    assert_eq!(requests.get(), 1);
    assert_eq!(std::fs::read_dir(dir.path()).expect("dir").count(), 0, "the folder of the failed map is gone");
    // An object that is no feature list is said with the other sentence.
    let download = |_: &Request| -> Result<Vec<u8>, TransferFailure> { Ok(br#"{"error":"busy"}"#.to_vec()) };
    let stopped = prepare(&scope_at(200_000.0, 450_000.0), &dir.path().join(FILE_NAME), &download, &never).expect_err("failed");
    assert_eq!(stopped.text(), "KIGAM 서버 연결 실패: 지도 서버가 올바른 지도 데이터를 보내지 않았습니다. 잠시 후 다시 내려받으세요.");
}

/// An answer without any unit, and a box that no layer of rock units covers: no map, said plainly. (The C++
/// then shows the official picture from the live service, `:292-342`; that part is order NET-GEO1b.)
#[test]
fn no_unit_in_the_box_is_said_and_nothing_is_stored() {
    let dir = tempfile::tempdir().expect("tempdir");
    let download = |request: &Request| -> Result<Vec<u8>, TransferFailure> {
        assert!(!is_picture(request), "no colours are asked for when there is no unit");
        Ok(answer(Vec::new(), None, None))
    };
    let stopped = prepare(&scope_at(200_000.0, 450_000.0), &dir.path().join(FILE_NAME), &download, &never).expect_err("failed");
    assert_eq!(stopped, Stopped::Failed(Why::NoData, NO_DATA.to_owned()));
    assert_eq!(std::fs::read_dir(dir.path()).expect("dir").count(), 0);
    // Ulleungdo: east of the mainland layer's box and far from Jeju. Nothing is asked of the service.
    let ulleung = Transformer::new(4326, 5186).expect("transformer").transform([130.86, 37.5]).expect("point");
    let download = |_: &Request| -> Result<Vec<u8>, TransferFailure> { panic!("no request for a box without a layer") };
    let stopped = prepare(&scope_at(ulleung[0], ulleung[1]), &dir.path().join(FILE_NAME), &download, &never).expect_err("failed");
    assert_eq!(stopped.text(), NO_DATA);
}

/// The Jeju layer is asked for a survey on Jeju (`lithoTypeNameForWgs84`, `:652-656`).
#[test]
fn a_survey_on_jeju_asks_the_jeju_layer() {
    let dir = tempfile::tempdir().expect("tempdir");
    let jeju = Transformer::new(4326, 5186).expect("transformer").transform([126.5, 33.4]).expect("point");
    let download = |request: &Request| -> Result<Vec<u8>, TransferFailure> {
        if is_picture(request) {
            return Ok(picture(&request.url, [250, 244, 180, 255]));
        }
        assert!(request.url.contains("typeNames=geoOpen:l_jeju_50k_geology_litho_view&"), "{}", request.url);
        Ok(answer(vec![unit("j.1", jeju[0], jeju[1], 50.0, "Qbmtb", "신생대 제 4기")], Some("EPSG:5186"), None))
    };
    let prepared = prepare(&scope_at(jeju[0], jeju[1]), &dir.path().join(FILE_NAME), &download, &never).expect("prepared");
    let rows = stored(&prepared.gpkg, &[ERA_COLUMN, COLOUR_COLUMN]);
    assert_eq!(rows[0].1, [Cell::Text("제4기".to_owned()), Cell::Text("#FAF4B4".to_owned())], "the Jeju spelling of the era, the colour of the picture");
}
