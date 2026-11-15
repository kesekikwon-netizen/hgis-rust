//! The second half of soilDownload_usesViewExtentAndTerrainFieldOnly
//! (`target/cpp-ref/tests/test_workflow.cpp:1621-1638`): the two forms of the request for Jeju.

use super::*;

const JEJU: Range = [126.45, 33.43, 126.55, 33.50];

#[test]
fn the_feature_request_asks_for_the_terrain_field_as_json() {
    assert_eq!(
        features_url(1, &JEJU, false),
        "https://gis.naas.go.kr/geoserver/soilmap/ows?service=WFS&version=2.0.0&request=GetFeature&typeNames=soilmap:SOIL_1&srsName=EPSG:5186&count=100000&outputFormat=application/json&propertyName=soil_type_geo,geom&bbox=33.43000000,126.45000000,33.50000000,126.55000000,urn:ogc:def:crs:EPSG::4326"
    );
}

/// "hits must stay XML so numberMatched is cheap".
#[test]
fn the_hits_request_asks_for_the_count_only() {
    assert_eq!(
        features_url(2, &JEJU, true),
        "https://gis.naas.go.kr/geoserver/soilmap/ows?service=WFS&version=2.0.0&request=GetFeature&typeNames=soilmap:SOIL_2&srsName=EPSG:5186&count=100000&resultType=hits&bbox=33.43000000,126.45000000,33.50000000,126.55000000,urn:ogc:def:crs:EPSG::4326"
    );
}

/// `numberMatched\s*=\s*"(\d+)"`, the first one of the answer.
#[test]
fn the_count_is_read_from_the_hits_answer() {
    assert_eq!(number_matched(br#"<FeatureCollection numberMatched="1"/>"#), Some(1));
    assert_eq!(number_matched(br#"<wfs:FeatureCollection xmlns:wfs="x" numberMatched = "15234" numberReturned="0"/>"#), Some(15_234));
    assert_eq!(number_matched(b"<x numberMatched\n=\t\"0\" numberMatched=\"7\"/>"), Some(0));
    // The first that has the form counts.
    assert_eq!(number_matched(br#"<x numberMatched="unknown" numberMatched="12"/>"#), Some(12));
    for body in [&b""[..], br#"<x numberMatched=""/>"#, br#"<x numberMatched='3'/>"#, br#"<x numberReturned="3"/>"#, br#"{"numberMatched":3}"#] {
        assert_eq!(number_matched(body), None, "{}", String::from_utf8_lossy(body));
    }
}
