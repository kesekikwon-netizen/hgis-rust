//! What stops the fetch of the river network, in the C++ words, and what a stop leaves behind
//! (`RiverMapService.cpp:131-143`, `:281-295`, `:309-321`, `:355-366`). `TEST_KEY` is not a real key.

use std::cell::RefCell;

use serde_json::json;
use strata_engine::Scope;

use super::tests::{answer, folders, never, river, river_data, scope};
use super::*;
use crate::collection::NOT_MAP_DATA;
use crate::transfer::{Request, TransferFailure, CANCELLED, NOT_REACHED, TOO_SLOW};

fn fails_with(body: &'static [u8]) -> Stopped {
    let dir = tempfile::tempdir().expect("tempdir");
    let download = |_: &Request| -> Result<Vec<u8>, TransferFailure> { Ok(body.to_vec()) };
    let stopped = prepare(&scope(), &dir.path().join("river.gpkg"), "TEST_KEY", &download, &never).expect_err("stopped");
    assert_eq!(folders(dir.path()), 0);
    stopped
}

/// VWorld answers 200 with an XML error: a refused key is told apart from anything else, and the sentence
/// never holds the key.
#[test]
fn an_answer_that_is_not_map_data_says_what_to_do() {
    let refused = fails_with(br#"<ServiceExceptionReport><ServiceException code="INCORRECT_KEY">incorrect_key: TEST_KEY</ServiceException></ServiceExceptionReport>"#);
    assert_eq!(refused, Stopped::Failed(Why::KeyRefused, format!("VWorld 서버 연결 실패: {KEY_REFUSED}")));
    assert!(!refused.text().contains("TEST_KEY"));
    let busy = fails_with(b"<ServiceExceptionReport><ServiceException>server busy</ServiceException></ServiceExceptionReport>");
    assert_eq!(busy, Stopped::Failed(Why::Service, format!("VWorld 서버 연결 실패: {NOT_SENT}")));
    for body in [&b""[..], b"<html>502 Bad Gateway</html>"] {
        assert_eq!(fails_with(body), Stopped::Failed(Why::Service, format!("VWorld 서버 연결 실패: {NOT_GEOJSON}")));
    }
    assert_eq!(fails_with(br#"{"error":"x"}"#), Stopped::Failed(Why::Service, format!("VWorld 서버 연결 실패: {NOT_MAP_DATA}")));
}

/// A service that cannot be reached, after the one further try, and one that is too slow in every piece.
#[test]
fn a_service_that_does_not_answer_is_said() {
    for (failure, sentence) in [(TransferFailure::Network, NOT_REACHED), (TransferFailure::Timeout, TOO_SLOW)] {
        let dir = tempfile::tempdir().expect("tempdir");
        let download = |_: &Request| -> Result<Vec<u8>, TransferFailure> { Err(failure) };
        let stopped = prepare(&scope(), &dir.path().join("river.gpkg"), "TEST_KEY", &download, &never).expect_err("stopped");
        // After its repeats and splits the piece-wise fetch says so first, then why.
        let Stopped::Failed(Why::Service, text) = &stopped else {
            panic!("{stopped:?}");
        };
        assert!(text.starts_with("VWorld 서버 연결 실패: ") && text.ends_with(sentence), "{text}");
        assert_eq!(folders(dir.path()), 0);
    }
}

/// `:281-295`: without a key, or for a scope wider than 160 km, nothing is asked.
#[test]
fn without_a_key_or_for_too_wide_a_scope_nothing_is_asked() {
    let dir = tempfile::tempdir().expect("tempdir");
    let none = |_: &Request| -> Result<Vec<u8>, TransferFailure> { panic!("no request here") };
    for key in ["", "   "] {
        assert_eq!(prepare(&scope(), &dir.path().join("river.gpkg"), key, &none, &never).expect_err("stopped"), Stopped::Failed(Why::NoKey, NO_KEY.to_owned()));
    }
    let wide = Scope::around(&["POLYGON ((200000 450000, 200100 450000, 200100 450100, 200000 450100, 200000 450000))".to_owned()], 80_000.0).expect("scope").expect("some");
    assert_eq!(prepare(&wide, &dir.path().join("river.gpkg"), "TEST_KEY", &none, &never).expect_err("stopped"), Stopped::Failed(Why::TooWide, TOO_WIDE.to_owned()));
    assert_eq!(folders(dir.path()), 0);
}

/// `:316-321`, `:363-366`: no river in the answer, or none inside the scope.
#[test]
fn an_area_without_rivers_is_said() {
    for (features, sentence) in [(Vec::new(), NO_DATA_HERE), (vec![river(1, 127.3, 37.0, "먼 강", json!("국가하천"))], NO_DATA)] {
        let dir = tempfile::tempdir().expect("tempdir");
        let body = answer(features);
        let download = |_: &Request| -> Result<Vec<u8>, TransferFailure> { Ok(body.clone()) };
        let stopped = prepare(&scope(), &dir.path().join("river.gpkg"), "TEST_KEY", &download, &never).expect_err("stopped");
        assert_eq!(stopped, Stopped::Failed(Why::NoData, sentence.to_owned()));
        assert_eq!(folders(dir.path()), 0);
    }
}

/// `:355-358`: a river without a shape that can be read stops the fetch.
#[test]
fn a_river_without_a_readable_shape_stops_the_fetch() {
    let dir = tempfile::tempdir().expect("tempdir");
    let no_shape = json!({"type": "Feature", "id": "lt_c_wkmstrm.9", "properties": {"riv_nm": "빈 강", "riv_level": "국가하천"}, "geometry": null});
    let body = answer(vec![river(1, 127.0, 37.0, "시험하천", json!("국가하천")), no_shape]);
    let download = |_: &Request| -> Result<Vec<u8>, TransferFailure> { Ok(body.clone()) };
    let stopped = prepare(&scope(), &dir.path().join("river.gpkg"), "TEST_KEY", &download, &never).expect_err("stopped");
    assert_eq!(stopped, Stopped::Failed(Why::Data, UNREADABLE.to_owned()));
    assert_eq!(folders(dir.path()), 0);
}

/// A stop by the user: before anything, and while a page comes in. Nothing is left, and the map that is
/// there stays as it is.
#[test]
fn a_stop_leaves_nothing_in_the_survey_folder() {
    let dir = tempfile::tempdir().expect("tempdir");
    let base = dir.path().join("river.gpkg");
    std::fs::write(&base, b"original-reference").expect("write");
    let stop = Stopped::Cancelled(CANCELLED.to_owned());
    let none = |_: &Request| -> Result<Vec<u8>, TransferFailure> { panic!("no request after a stop") };
    assert_eq!(prepare(&scope(), &base, "TEST_KEY", &none, &|| true).expect_err("stopped"), stop);

    let asked = RefCell::new(0);
    let during = |_: &Request| -> Result<Vec<u8>, TransferFailure> {
        *asked.borrow_mut() += 1;
        Err(TransferFailure::Cancelled)
    };
    assert_eq!(prepare(&scope(), &base, "TEST_KEY", &during, &never).expect_err("stopped"), stop);
    assert_eq!(asked.into_inner(), 1, "a cancelled request is not sent again");

    // The stop comes while the answer is worked on.
    let stopped = std::cell::Cell::new(false);
    let then = |_: &Request| -> Result<Vec<u8>, TransferFailure> {
        stopped.set(true);
        Ok(river_data())
    };
    assert_eq!(prepare(&scope(), &base, "TEST_KEY", &then, &|| stopped.get()).expect_err("stopped"), stop);
    assert_eq!(std::fs::read(&base).expect("read"), b"original-reference");
    assert_eq!(folders(dir.path()), 0);
}
