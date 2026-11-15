//! The form of the river request and the reading of VWorld's XML errors (`RiverMapService.cpp:82-123`).
//! `TEST_KEY` is the key of the C++ tests (`test_reference_download.cpp:158`), not a real one.

use super::*;

#[test]
fn a_page_is_asked_for_by_its_start_index_with_the_key_last() {
    let range: Range = [126.9, 36.95, 127.1, 37.05];
    assert_eq!(
        page_url(&range, 0, "TEST_KEY"),
        "https://api.vworld.kr/req/wfs?SERVICE=WFS&REQUEST=GetFeature&VERSION=2.0.0&TYPENAME=lt_c_wkmstrm&OUTPUT=application/json&SRSNAME=EPSG:4326&MAXFEATURES=1000&STARTINDEX=0&BBOX=36.95000000,126.90000000,37.05000000,127.10000000,urn:ogc:def:crs:EPSG::4326&KEY=TEST_KEY"
    );
    let third = page_url(&range, 2, "TEST_KEY");
    assert!(third.contains("&MAXFEATURES=1000&STARTINDEX=2000&"), "{third}");
    // "DOMAIN 은 보내지 않는다": VWorld refuses the same key with it.
    assert!(!third.to_ascii_uppercase().contains("DOMAIN"));
}

#[test]
fn the_reason_is_read_from_an_xml_error() {
    let refused = r#"<?xml version="1.0"?><ServiceExceptionReport><ServiceException code="INCORRECT_KEY">
        INCORRECT_KEY : 등록되지 않은 인증키입니다.
    </ServiceException></ServiceExceptionReport>"#;
    assert_eq!(service_exception(refused.as_bytes()).as_deref(), Some("INCORRECT_KEY : 등록되지 않은 인증키입니다."));
    assert_eq!(service_exception(b"<ServiceException>busy</ServiceException><ServiceException>later</ServiceException>").as_deref(), Some("busy"));
    for body in [&b""[..], b"<html>502</html>", b"<ServiceException></ServiceException>", b"<ServiceException>cut", br#"{"type":"FeatureCollection"}"#] {
        assert_eq!(service_exception(body), None, "{}", String::from_utf8_lossy(body));
    }
}
