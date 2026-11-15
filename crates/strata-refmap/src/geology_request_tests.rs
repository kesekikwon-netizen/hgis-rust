//! The request forms letter by letter (`GeologyMapService.cpp:195-203`, `:230-241`) and the layer choice with
//! the boxes of the C++ test (`target/cpp-ref/tests/test_workflow.cpp`, geology region cases: a Jeju box uses
//! the Jeju layer, a Gangneung box the mainland layer).

use super::*;

#[test]
fn the_layer_follows_where_the_box_lies() {
    assert_eq!(layer_for(&[126.40, 33.40, 126.60, 33.55]), Some(JEJU_LAYER), "Jeju");
    assert_eq!(layer_for(&[128.80, 37.70, 128.95, 37.80]), Some(MAINLAND_LAYER), "Gangneung");
    assert_eq!(layer_for(&[129.10, 35.70, 129.35, 35.95]), Some(MAINLAND_LAYER), "Gyeongju");
    // A box that reaches both: the mainland layer is asked first.
    assert_eq!(layer_for(&[126.0, 33.5, 126.9, 34.0]), Some(MAINLAND_LAYER));
    // Dokdo and Ulleungdo lie east of both boxes: no layer of rock units.
    assert_eq!(layer_for(&[130.80, 37.40, 130.95, 37.55]), None);
    // Touching the edge of the Jeju box counts (the C++ `intersects` includes the edge).
    assert_eq!(layer_for(&[127.00, 33.20, 127.10, 33.30]), Some(JEJU_LAYER));
    assert_eq!(layer_for(&[127.01, 33.20, 127.10, 33.30]), None);
    assert_eq!(layer_for(&[f64::NAN, 33.2, 127.1, 33.3]), None);
    assert_eq!(layer_for(&[127.0, 36.0, f64::INFINITY, 37.0]), None);
}

#[test]
fn the_feature_request_is_the_one_of_the_cpp() {
    let url = features_url(MAINLAND_LAYER, &[129.123_456_789, 35.7, 129.35, 35.987_654_321]);
    assert_eq!(
        url,
        "https://data.kigam.re.kr/geoserver/ows?service=WFS&version=2.0.0&request=GetFeature&typeNames=geoOpen:l_50k_geology_litho_latest&outputFormat=application/json&srsName=EPSG:5186&count=100000&bbox=35.70000000,129.12345679,35.98765432,129.35000000,urn:ogc:def:crs:EPSG::4326"
    );
    assert!(features_url(JEJU_LAYER, &[126.4, 33.4, 126.6, 33.55]).contains("typeNames=geoOpen:l_jeju_50k_geology_litho_view&"));
}

#[test]
fn the_picture_request_is_the_one_of_the_cpp() {
    // Wider than high: 1400 wide, the height in proportion (0.25 by 0.2 degrees: 1120).
    let wide = [129.1, 35.7, 129.35, 35.9];
    assert_eq!(picture_size(&wide), (1400, 1120));
    assert_eq!(
        picture_url(&wide),
        "https://data.kigam.re.kr/geoserver/ows?service=WMS&version=1.1.1&request=GetMap&layers=geoOpen:L_50K_Geology_Map&styles=&srs=EPSG:4326&bbox=129.1000000,35.7000000,129.3500000,35.9000000&width=1400&height=1120&format=image/png&transparent=true"
    );
    // Higher than wide: 1400 high.
    assert_eq!(picture_size(&[129.1, 35.7, 129.2, 35.9]), (700, 1400));
    // A very flat box keeps 64 pixels of height; a box without size does not divide by zero.
    assert_eq!(picture_size(&[129.0, 35.7, 130.0, 35.71]), (1400, 64));
    assert_eq!(picture_size(&[129.0, 35.7, 129.0, 35.7]), (1400, 1400));
}

/// `px = round((x - xmin) / w * (W - 1))`, `py = round((ymax - y) / h * (H - 1))` (`:214-215`).
#[test]
fn a_point_lands_on_its_pixel() {
    let range = [129.1, 35.7, 129.35, 35.9];
    assert_eq!(picture_pixel(&range, [129.1, 35.9]), (0, 0), "north-west corner");
    assert_eq!(picture_pixel(&range, [129.35, 35.7]), (1399, 1119), "south-east corner");
    // 0.1 / 0.25 * 1399 = 559.6 and 0.05 / 0.2 * 1119 = 279.75: rounded to the nearest pixel.
    assert_eq!(picture_pixel(&range, [129.2, 35.85]), (560, 280));
    assert_eq!(picture_pixel(&range, [129.05, 35.95]), (-280, -280), "outside: outside the picture");
}
