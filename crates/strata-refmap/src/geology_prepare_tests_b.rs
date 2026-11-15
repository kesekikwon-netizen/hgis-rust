//! geologyResponseCrsPreservesSurveyLocation (`target/cpp-ref/tests/test_reference_download.cpp:507-640`),
//! the cut to the scope, the colours and the stops.

use std::cell::Cell as Counter;

use serde_json::json;
use strata_engine::{polygons_area, Cell, Scope, Shapes, Transformer};

use super::tests::{answer, is_picture, picture, scope_at, square_wkt, stored, unit};
use super::*;
use crate::transfer::{Request, TransferFailure, Why, CANCELLED};

fn never() -> bool {
    false
}

/// The six rows of the C++ test: a 100 m square at a survey in Daegu (EPSG:5187 135803.52, 412903.06),
/// answered in the CRS of the first column and declared as the second. Ready: the stored unit lies where
/// the survey is, to a centimetre, after one feature request and one picture request. Not ready: a failure
/// with words, no picture request, nothing stored, and the map that was there untouched.
#[test]
fn the_crs_the_answer_declares_decides_where_the_unit_lies() {
    let centre_5187 = [135_803.523_437_5, 412_903.062_5];
    let to_stored = Transformer::new(5187, 5186).expect("transformer");
    let centre = to_stored.transform(centre_5187).expect("centre");
    for (coordinate_epsg, declared, ready) in [
        (5186, Some("EPSG:5186"), true),
        (5187, Some("EPSG:5187"), true),
        (4326, Some("urn:ogc:def:crs:EPSG::4326"), true),
        (4326, None, true),
        (5187, Some("EPSG:999999"), false),
        (5187, Some("EPSG:4326"), false),
    ] {
        let dir = tempfile::tempdir().expect("tempdir");
        let requested = dir.path().join(FILE_NAME);
        std::fs::write(&requested, b"previous-reference").expect("old map");
        let to_answer = Transformer::new(5187, coordinate_epsg).expect("transformer");
        let corner = |dx: f64, dy: f64| to_answer.transform([centre_5187[0] + dx, centre_5187[1] + dy]).expect("corner");
        let ring: Vec<[f64; 2]> = vec![corner(-50.0, -50.0), corner(50.0, -50.0), corner(50.0, 50.0), corner(-50.0, 50.0), corner(-50.0, -50.0)];
        let feature = json!({"type": "Feature", "properties": {"기호": "Qa", "지층": "시험 충적층", "시대": "제4기"}, "geometry": {"type": "Polygon", "coordinates": [ring]}});
        let body = answer(vec![feature], declared, None);
        let (feature_requests, picture_requests) = (Counter::new(0), Counter::new(0));
        let download = |request: &Request| -> Result<Vec<u8>, TransferFailure> {
            if is_picture(request) {
                picture_requests.set(picture_requests.get() + 1);
                return Ok(picture(&request.url, [249, 249, 127, 255]));
            }
            feature_requests.set(feature_requests.get() + 1);
            Ok(body.clone())
        };
        let scope = Scope::around(&[square_wkt(centre[0] - 50.0, centre[1] - 50.0, 100.0)], 10_000.0).expect("scope").expect("some");
        let result = prepare(&scope, &requested, &download, &never);
        let row = format!("coordinates in EPSG:{coordinate_epsg}, declared {declared:?}");
        assert_eq!(feature_requests.get(), 1, "{row}");
        assert_eq!(std::fs::read(&requested).expect("old map"), b"previous-reference", "{row}");
        if !ready {
            let stopped = result.expect_err(&row);
            assert!(matches!(&stopped, Stopped::Failed(Why::Data, text) if !text.is_empty()), "{row}: {stopped:?}");
            assert_eq!(picture_requests.get(), 0, "{row}");
            assert_eq!(std::fs::read_dir(dir.path()).expect("dir").count(), 1, "{row}: only the old map is in the folder");
            continue;
        }
        let prepared = result.unwrap_or_else(|stopped| panic!("{row}: {stopped:?}"));
        assert_eq!(picture_requests.get(), 1, "{row}");
        let rows = stored(&prepared.gpkg, &[COLOUR_COLUMN]);
        assert_eq!(rows[0].1, [Cell::Text("#F9F97F".to_owned())], "{row}: the official colour of Qa");
        let Shapes::Polygons(polygons) = &rows[0].0 else { panic!("{row}: polygons") };
        let outer = &polygons[0][0];
        let n = (outer.len() - 1) as f64;
        let mean = outer[..outer.len() - 1].iter().fold([0.0, 0.0], |m, p| [m[0] + p[0] / n, m[1] + p[1] / n]);
        assert!((mean[0] - centre[0]).abs() < 0.01 && (mean[1] - centre[1]).abs() < 0.01, "{row}: stored at {mean:?}, survey at {centre:?}");
    }
    // The two failures say different things: a CRS nobody knows, and coordinates that are no degrees.
    assert_eq!(UNKNOWN_CRS, "받은 지질도의 좌표계를 확인할 수 없습니다. 기존 지도는 유지됩니다.");
    assert_eq!(NOT_CARRIED, "지질도 좌표를 변환하지 못했습니다. 기존 지도는 유지됩니다.");
}

/// A unit that reaches beyond the 10 km scope is stored only where it lies inside; one outside is not
/// stored; when all lie outside there is no map.
#[test]
fn units_are_cut_to_the_scope_around_the_survey() {
    let dir = tempfile::tempdir().expect("tempdir");
    let scope = scope_at(200_000.0, 450_000.0);
    let features = vec![
        // 200 km wide: covers the whole scope.
        unit("huge", 100_000.0, 350_000.0, 200_000.0, "Kgr", "현생누대 중생대 백악기"),
        // 1 km square 30 km east of the survey: outside.
        unit("far", 230_000.0, 450_000.0, 1_000.0, "Qa", "제4기"),
        // 100 m square beside the survey: inside, kept as it is.
        unit("near", 200_200.0, 450_200.0, 100.0, "Qa", "제4기"),
    ];
    let download = |request: &Request| -> Result<Vec<u8>, TransferFailure> { if is_picture(request) { Err(TransferFailure::Network) } else { Ok(answer(features.clone(), Some("EPSG:5186"), None)) } };
    let prepared = prepare(&scope, &dir.path().join(FILE_NAME), &download, &never).expect("prepared");
    assert_eq!(prepared.rows, 2);
    let rows = stored(&prepared.gpkg, &[SYMBOL_COLUMN]);
    let area = |shapes: &Shapes| match shapes {
        Shapes::Polygons(polygons) => polygons_area(polygons).expect("area"),
        Shapes::Lines(_) => panic!("polygons"),
    };
    assert_eq!(rows[0].1, [Cell::Text("Kgr".to_owned())]);
    assert!((area(&rows[0].0) / scope.area().expect("area") - 1.0).abs() < 1e-9, "the huge unit is the scope's shape now");
    assert_eq!(rows[1].1, [Cell::Text("Qa".to_owned())]);
    assert!((area(&rows[1].0) - 10_000.0).abs() < 1e-6);

    let only_far = vec![unit("far", 230_000.0, 450_000.0, 1_000.0, "Qa", "제4기")];
    let download = |_: &Request| -> Result<Vec<u8>, TransferFailure> { Ok(answer(only_far.clone(), Some("EPSG:5186"), None)) };
    assert_eq!(prepare(&scope, &dir.path().join(FILE_NAME), &download, &never).expect_err("no map").text(), NO_DATA);
}

/// Without the picture of the official map the units are still stored, in the colours of their eras shaded
/// by their symbols, and the user is told so (`:905-907`). A unit without a symbol is grey. Texts lose their
/// markup. A symbol whose point falls on a see-through part of the picture gets the era colour as well.
#[test]
fn without_official_colours_the_era_colours_are_used_and_said() {
    let dir = tempfile::tempdir().expect("tempdir");
    let scope = scope_at(200_000.0, 450_000.0);
    let mut linked = unit("u1", 200_000.0, 450_200.0, 100.0, "Qa", "현생누대 신생대 제4기");
    linked["properties"]["지층"] = json!("<a href=\"https://data.kigam.re.kr/x\">충적층</a>");
    let features = vec![linked, unit("u2", 200_200.0, 450_200.0, 100.0, "Kswt", "현생누대 중생대 백악기"), unit("u3", 200_400.0, 450_200.0, 100.0, "", "")];
    let download = |request: &Request| -> Result<Vec<u8>, TransferFailure> { if is_picture(request) { Err(TransferFailure::Timeout) } else { Ok(answer(features.clone(), Some("EPSG:5186"), None)) } };
    let prepared = prepare(&scope, &dir.path().join(FILE_NAME), &download, &never).expect("prepared");
    assert_eq!(prepared.warnings, [NO_OFFICIAL_COLOUR]);
    let rows = stored(&prepared.gpkg, &["지층", ERA_COLUMN, COLOUR_COLUMN]);
    let text = |s: &str| Cell::Text(s.to_owned());
    // Qa: 제4기 (249,249,127) one step lighter; Kswt: 백악기 (127,198,78) two steps darker (geology_era_tests.rs).
    assert_eq!(rows[0].1, [text("충적층"), text("제4기"), text("#FFFF9A")]);
    assert_eq!(rows[1].1, [text("시험 충적층"), text("백악기"), text("#61973C")]);
    assert_eq!(rows[2].1, [text("시험 충적층"), text("시대미상"), text("#C8C8C8")]);

    // A picture that is see-through everywhere: no colour can be read, the same warning.
    let download = |request: &Request| -> Result<Vec<u8>, TransferFailure> { if is_picture(request) { Ok(picture(&request.url, [249, 249, 127, 0])) } else { Ok(answer(features.clone(), Some("EPSG:5186"), None)) } };
    let prepared = prepare(&scope, &dir.path().join(FILE_NAME), &download, &never).expect("prepared");
    assert_eq!(prepared.warnings, [NO_OFFICIAL_COLOUR]);
    // What is no picture at all is treated the same way.
    let download = |request: &Request| -> Result<Vec<u8>, TransferFailure> { if is_picture(request) { Ok(b"<html>busy</html>".to_vec()) } else { Ok(answer(features.clone(), Some("EPSG:5186"), None)) } };
    assert_eq!(prepare(&scope, &dir.path().join(FILE_NAME), &download, &never).expect("prepared").warnings, [NO_OFFICIAL_COLOUR]);
}

/// The stops: the user's stop before and during the fetch, a connection that drops twice, an answer that
/// stays slow, a shape that is no polygon, a scope wider than the C++ limit. Nothing is left behind.
#[test]
fn every_stop_says_its_cpp_sentence_and_leaves_nothing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let scope = scope_at(200_000.0, 450_000.0);
    let requested = dir.path().join(FILE_NAME);
    let left = || std::fs::read_dir(dir.path()).expect("dir").count();

    let download = |_: &Request| -> Result<Vec<u8>, TransferFailure> { panic!("stopped before any request") };
    assert_eq!(prepare(&scope, &requested, &download, &|| true).expect_err("stopped"), Stopped::Cancelled(CANCELLED.to_owned()));

    let stop = Counter::new(false);
    let download = |_: &Request| -> Result<Vec<u8>, TransferFailure> {
        stop.set(true);
        Err(TransferFailure::Cancelled)
    };
    assert_eq!(prepare(&scope, &requested, &download, &|| stop.get()).expect_err("stopped"), Stopped::Cancelled(CANCELLED.to_owned()));

    let requests = Counter::new(0);
    let download = |_: &Request| -> Result<Vec<u8>, TransferFailure> {
        requests.set(requests.get() + 1);
        Err(TransferFailure::Network)
    };
    let stopped = prepare(&scope, &requested, &download, &never).expect_err("failed");
    assert_eq!(requests.get(), 2, "a dropped connection is tried once more");
    assert_eq!(stopped.text(), "KIGAM 서버 연결 실패: 지도 자료를 2번 나눠 요청했지만 끝까지 받지 못했습니다. 기존 지도는 유지됩니다. 연결이 안정된 곳에서 다시 내려받으세요.\n지도 서버에 연결하지 못했습니다. 인터넷 연결을 확인한 뒤 다시 내려받으세요.");

    let requests = Counter::new(0);
    let download = |_: &Request| -> Result<Vec<u8>, TransferFailure> {
        requests.set(requests.get() + 1);
        Err(TransferFailure::Timeout)
    };
    let stopped = prepare(&scope, &requested, &download, &never).expect_err("failed");
    assert_eq!(requests.get(), 4, "a slow box is split twice, then its smallest piece tried once more");
    assert!(stopped.text().ends_with("\n지도 서버의 응답이 늦어 요청을 중단했습니다. 인터넷 연결을 확인한 뒤 다시 내려받으세요."), "{}", stopped.text());

    let point = json!({"type": "Feature", "properties": {"기호": "Qa"}, "geometry": {"type": "Point", "coordinates": [200_050.0, 450_050.0]}});
    for bad in [point, json!({"type": "Feature", "properties": {"기호": "Qa"}, "geometry": null}), json!({"type": "Feature", "properties": {}, "geometry": {"type": "MultiPolygon", "coordinates": []}})] {
        let download = |request: &Request| -> Result<Vec<u8>, TransferFailure> {
            assert!(!is_picture(request));
            Ok(answer(vec![unit("ok", 200_200.0, 450_200.0, 100.0, "Qa", "제4기"), bad.clone()], Some("EPSG:5186"), None))
        };
        assert_eq!(prepare(&scope, &requested, &download, &never).expect_err("failed").text(), "지질도에 읽을 수 없는 구역이 있습니다. 기존 지도는 유지됩니다.");
    }

    // A survey area 70 km wide: with 10 km on each side the box is 90 km, past the C++ limit of 80.
    let wide = Scope::around(&["POLYGON ((200000 450000, 270000 450000, 270000 450100, 200000 450100, 200000 450000))".to_owned()], 10_000.0).expect("scope").expect("some");
    let download = |_: &Request| -> Result<Vec<u8>, TransferFailure> { panic!("no request for a box that wide") };
    assert_eq!(prepare(&wide, &requested, &download, &never).expect_err("failed"), Stopped::Failed(Why::TooWide, "범위가 너무 넓습니다. 지도를 조사구역(한 변 80km 이하)으로 확대한 뒤 다시 내려받으세요.".to_owned()));
    assert_eq!(left(), 0, "no folder of a stopped map is left");
}

/// An answer that names its CRS in words nobody can read is refused as an unknown CRS (`:806-810`); it is not
/// taken for longitude and latitude, which would end in the other sentence.
#[test]
fn a_crs_named_in_words_is_an_unknown_crs() {
    let dir = tempfile::tempdir().expect("tempdir");
    let features = vec![unit("u1", 200_200.0, 450_200.0, 100.0, "Qa", "제4기")];
    let download = |request: &Request| -> Result<Vec<u8>, TransferFailure> {
        assert!(!is_picture(request));
        Ok(answer(features.clone(), Some("Korea 2000 / Central Belt 2010"), None))
    };
    let stopped = prepare(&scope_at(200_000.0, 450_000.0), &dir.path().join(FILE_NAME), &download, &never).expect_err("failed");
    assert_eq!(stopped, Stopped::Failed(Why::Data, UNKNOWN_CRS.to_owned()));
}

/// The official colour of a symbol is read inside its largest shape (`:877-887`). Here the map picture is
/// one colour west of the survey and another east of it; the symbol has a small shape in the west, which
/// comes first in the answer, and a large one in the east: every shape of the symbol gets the eastern colour.
#[test]
fn the_colour_of_a_symbol_is_read_in_its_largest_shape() {
    let dir = tempfile::tempdir().expect("tempdir");
    let features = vec![unit("small", 195_000.0, 450_000.0, 100.0, "Qa", "제4기"), unit("large", 203_000.0, 449_500.0, 1_000.0, "Qa", "제4기")];
    let download = |request: &Request| -> Result<Vec<u8>, TransferFailure> {
        if !is_picture(request) {
            return Ok(answer(features.clone(), Some("EPSG:5186"), None));
        }
        let mut two_colours = image::load_from_memory(&picture(&request.url, [200, 150, 100, 255])).expect("png").to_rgba8();
        let middle = two_colours.width() / 2;
        for (x, _, pixel) in two_colours.enumerate_pixels_mut() {
            if x >= middle {
                *pixel = image::Rgba([120, 180, 240, 255]);
            }
        }
        let mut bytes = std::io::Cursor::new(Vec::new());
        two_colours.write_to(&mut bytes, image::ImageFormat::Png).expect("png");
        Ok(bytes.into_inner())
    };
    let prepared = prepare(&scope_at(200_000.0, 450_000.0), &dir.path().join(FILE_NAME), &download, &never).expect("prepared");
    let rows = stored(&prepared.gpkg, &[COLOUR_COLUMN]);
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|(_, cells)| cells == &[Cell::Text("#78B4F0".to_owned())]), "{:?}", rows.iter().map(|r| &r.1).collect::<Vec<_>>());
}
