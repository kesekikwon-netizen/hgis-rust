//! `ngii_topographic` 시험 — 도엽 번호·격자·응답 해석·안내 문구.
//!
//! 원래 `ngii_topographic.rs` 안 `mod tests` 를 그대로 옮겼다.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

#[test]
fn test_ngii_bbox_to_sheets_gongju_gongsanseong_returns_deduplicated_367084_367085() {
    // 옛 계산은 C++ 대응이 없어 그대로 남긴다(기존 시험 고정값).
    let sheets =
        NgiiTopographicClient::bbox_to_sheets_5k(10_000.0, 442_500.0, 12_600.0, 443_000.0);
    assert_eq!(sheets, vec!["367084".to_owned(), "367085".to_owned()]);
}

/// C++ `TopographicSheets::select` 경로 — 경위도 BBOX 로 1:25,000 도엽 번호.
#[test]
fn test_ngii_sheets_for_bbox_25k_returns_sorted_unique_official_numbers() {
    // 공주 공산성 일대(127.10~127.16E, 36.44~36.48N)는 36도 37분30초~36도 30분 대의
    // 1:25,000 두 장에 걸친다(C++ `numberForCell` 로 계산한 값).
    let sheets = NgiiTopographicClient::sheets_for_bbox_25k([127.10, 36.44, 127.16, 36.48]);
    assert_eq!(
        sheets,
        vec!["367091".to_owned(), "367092".to_owned()],
        "got {sheets:?}"
    );
    // 각 번호는 C++ `resolve` 가 받아들이는 6자리 격자 번호이고 도곽이 7분 30초다.
    for number in &sheets {
        let sheet = TopographicSheets::resolve(number).expect(number);
        assert!((sheet.east - sheet.west - 7.5 / 60.0).abs() < 1e-12, "{number}");
    }
}

/// C++ `test_topographic_sheets.cpp:26-37` `officialExamplesAndExactGrid`
#[test]
fn test_cpp_topographic_sheets_resolve_matches_official_examples() {
    let example = TopographicSheets::resolve("367154").expect("367154");
    assert_eq!(example.extent(), [127.625, 36.0, 127.75, 36.125]);
    assert_eq!(example.zone, TopographicZone::Central);
    assert_eq!(example.crs_auth_id(false), None);
    assert!((example.extent()[2] - example.extent()[0] - 7.5 / 60.0).abs() < 1e-12);

    let gangneung = TopographicSheets::resolve("378044").expect("378044");
    assert_eq!(gangneung.extent(), [128.875, 37.75, 129.0, 37.875]);
    assert_eq!(gangneung.zone, TopographicZone::East);
    assert_eq!(gangneung.crs_auth_id(true).as_deref(), Some("EPSG:5187"));
}

/// C++ `test_topographic_sheets.cpp:38-43` `rejectsMalformedAndUnsupportedLongitudeCodes`
#[test]
fn test_cpp_topographic_sheets_rejects_malformed_numbers() {
    for bad in [
        "36715", "3671544", "367004", "367174", "367150", "367155", "36715A", " 367154",
        "363154", "362154",
    ] {
        assert!(
            TopographicSheets::resolve(bad).is_none(),
            "{bad} 는 거부되어야 한다"
        );
    }
    assert!(TopographicSheets::resolve("３６７１５４").is_none());
}

/// C++ `test_topographic_sheets.cpp:56-62` — 원점대 4개
#[test]
fn test_cpp_topographic_sheets_zone_codes_5185_to_5188() {
    for (number, zone, epsg) in [
        ("364011", TopographicZone::West, 5185_u32),
        ("366011", TopographicZone::Central, 5186),
        ("368011", TopographicZone::East, 5187),
        ("370011", TopographicZone::EastSea, 5188),
    ] {
        let sheet = TopographicSheets::resolve(number).expect(number);
        assert_eq!(sheet.zone, zone, "{number}");
        assert_eq!(sheet.zone.epsg(), epsg, "{number}");
        assert!(!sheet.zone.name_ko().is_empty());
    }
}

/// C++ `test_topographic_sheets.cpp:79-89` — 격자 경계 4장이 함께 나온다
#[test]
fn test_cpp_topographic_sheets_boundary_includes_four_cells() {
    // 경도 128.0(격자 x=1023/1024), 위도 36.875(격자 y=294/295) 네 칸 경계
    let ids = TopographicSheets::numbers_in_geographic_box([127.999, 36.874, 128.001, 36.876]);
    assert_eq!(ids.len(), 4, "got {ids:?}");
    for want in ["367042", "367044", "368011", "368013"] {
        assert!(ids.contains(&want.to_owned()), "{want} missing in {ids:?}");
    }
}

#[test]
fn test_cpp_topographic_sheets_number_for_cell_round_trips() {
    for y in 288..=288 {
        for x in 1020..=1027 {
            let number = TopographicSheets::number_for_cell(x, y);
            assert_eq!(number.len(), 6, "{number}");
            let sheet = TopographicSheets::resolve(&number).expect(&number);
            let west_idx = (sheet.west * 8.0).floor() as i32;
            assert_eq!(west_idx, x, "{number} 서쪽 격자");
        }
    }
}

#[test]
fn test_ngii_combine_buffer_and_suchi_bodies_match_cpp_provider() {
    assert_eq!(
        NgiiTopographicClient::combine_buffer_query_url(),
        "https://map.ngii.go.kr/ms/map/combineBufferQuery.do"
    );
    assert_eq!(
        NgiiTopographicClient::suchi_query_url(),
        "https://map.ngii.go.kr/ms/map/suchiQuery.do"
    );
    // `ngii-topographic.js:43` {x,y,buffer}
    assert_eq!(
        NgiiTopographicClient::combine_buffer_body(200_000.0, 500_000.0, 5000.0),
        "x=200000&y=500000&buffer=5000"
    );
    let area = SheetScope {
        wkt: "POLYGON((1 2,3 4,5 6,1 2))".to_owned(),
        minx: 195_000.0,
        miny: 495_000.0,
        maxx: 205_000.0,
        maxy: 505_000.0,
    };
    // `ngii-topographic.js:63-64` — index/type/scale 순서와 scale=25000
    assert_eq!(
        NgiiTopographicClient::suchi_body(&area),
        "index=&type=0&scale=25000&wkt=POLYGON%28%281%202%2C3%204%2C5%206%2C1%202%29%29&minx=195000&miny=495000&maxx=205000&maxy=505000"
    );
}

#[test]
fn test_ngii_parse_combine_buffer_accepts_scope_and_rejects_mismatched_box() {
    let ok = br#"{"items":[{"wkt":"POLYGON((195000 495000,205000 495000,205000 505000,195000 505000,195000 495000))","minx":195000,"miny":495000,"maxx":205000,"maxy":505000}]}"#;
    let area = NgiiTopographicClient::parse_combine_buffer(ok, 200_000.0, 500_000.0, 5000.0)
        .expect("scope");
    assert_eq!(area.maxx, 205_000.0);
    // 범위가 x±radius 와 1m 넘게 다르면 C++ 은 'scope' 오류로 멈춘다.
    let bad = br#"{"items":[{"wkt":"POLYGON((1 2,3 4,5 6,1 2))","minx":190000,"miny":495000,"maxx":205000,"maxy":505000}]}"#;
    assert_eq!(
        NgiiTopographicClient::parse_combine_buffer(bad, 200_000.0, 500_000.0, 5000.0)
            .expect_err("1m 넘는 범위 차이는 거부"),
        NgiiTopographicClient::scope_error()
    );
    // 여러 개를 돌려주면 거부
    let many = br#"{"items":[{"wkt":"POLYGON((1 2))","minx":195000,"miny":495000,"maxx":205000,"maxy":505000},{"wkt":"POLYGON((1 2))","minx":195000,"miny":495000,"maxx":205000,"maxy":505000}]}"#;
    assert!(
        NgiiTopographicClient::parse_combine_buffer(many, 200_000.0, 500_000.0, 5000.0)
            .is_err()
    );
}

#[test]
fn test_ngii_parse_suchi_accepts_dxf_25k_and_reports_unavailable() {
    let area = SheetScope {
        wkt: "POLYGON((1 2,3 4,5 6,1 2))".to_owned(),
        minx: 195_000.0,
        miny: 495_000.0,
        maxx: 205_000.0,
        maxy: 505_000.0,
    };
    let body = br#"{"items":[
        {"num":"367154","name":"gongju","scale":"25000","fileExt":"DXF","historyNo":"1234","projection":"EPSG:5186",
         "minx":198000,"miny":498000,"maxx":202000,"maxy":502000},
        {"num":"367155","name":"gongju2","scale":"25000","fileExt":"dxf","historyNo":"1235","projection":"EPSG:5186",
         "minx":198000,"miny":498000,"maxx":202000,"maxy":502000}]}"#;
    let wanted = vec!["367154".to_owned(), "367999".to_owned()];
    let query = NgiiTopographicClient::parse_suchi(body, &area, &wanted).expect("suchi");
    assert_eq!(query.products.len(), 2);
    assert_eq!(query.products[0].num, "367154");
    assert_eq!(query.unavailable, vec!["367999".to_owned()]);
    // C++ `KaTopographicBrowser.cpp:870` 신청 목록 문구
    let text = query.order_sheets_text();
    assert!(text.starts_with("공식 신청할 도엽 2장 · 367154 gongju, 367155 gongju2"));
    assert!(text.ends_with("중지하려면 「취소」를 누르세요."));
}

#[test]
fn test_ngii_parse_suchi_rejects_wrong_scale_or_extension() {
    let area = SheetScope {
        wkt: "POLYGON((1 2,3 4,5 6,1 2))".to_owned(),
        minx: 195_000.0,
        miny: 495_000.0,
        maxx: 205_000.0,
        maxy: 505_000.0,
    };
    for bad_row in [
        r#"{"num":"367154","name":"공주","scale":"50000","fileExt":"dxf","historyNo":"1","projection":"EPSG:5186","minx":198000,"miny":498000,"maxx":202000,"maxy":502000}"#,
        r#"{"num":"367154","name":"공주","scale":"25000","fileExt":"shp","historyNo":"1","projection":"EPSG:5186","minx":198000,"miny":498000,"maxx":202000,"maxy":502000}"#,
        r#"{"num":"36715","name":"공주","scale":"25000","fileExt":"dxf","historyNo":"1","projection":"EPSG:5186","minx":198000,"miny":498000,"maxx":202000,"maxy":502000}"#,
        r#"{"num":"367154","name":"","scale":"25000","fileExt":"dxf","historyNo":"1","projection":"EPSG:5186","minx":198000,"miny":498000,"maxx":202000,"maxy":502000}"#,
        r#"{"num":"367154","name":"공주","scale":"25000","fileExt":"dxf","historyNo":"1","projection":"EPSG:5186","minx":202000,"miny":498000,"maxx":198000,"maxy":502000}"#,
        r#"{"num":"367154","name":"공주","scale":"25000","fileExt":"dxf","historyNo":"1","projection":"EPSG:5186","minx":900000,"miny":498000,"maxx":902000,"maxy":502000}"#,
    ] {
        let body = format!("{{\"items\":[{bad_row}]}}");
        assert_eq!(
            NgiiTopographicClient::parse_suchi(body.as_bytes(), &area, &[])
                .expect_err("C++ 규격이 아니면 거부"),
            NgiiTopographicClient::scope_error(),
            "{bad_row}"
        );
    }
}

#[test]
fn test_ngii_stage_messages_match_cpp_strings() {
    assert_eq!(
        NgiiTopographicClient::portal_url(),
        "https://map.ngii.go.kr/ms/map/NlipMap.do"
    );
    assert!(NgiiTopographicClient::home_url().ends_with("tabGb=total"));
    assert_eq!(
        NgiiTopographicClient::browser_hint(),
        "현재 조사 주변의 1:25,000 DXF 도엽을 선택합니다. 로그인·인증이 필요하면 공식 화면의 안내를 따라 주세요."
    );
    assert_eq!(
        NgiiTopographicClient::unavailable_message(),
        "이 범위에서 제공 중인 1:25,000 DXF 도엽이 없습니다."
    );
    assert_eq!(
        NgiiTopographicClient::cached_message(),
        "필요한 지형도를 모두 보관하고 있습니다. 다시 신청하지 않습니다."
    );
    assert_eq!(
        NgiiTopographicClient::transfer_program_message(),
        "자동 전송을 계속할 수 없습니다. 전송 프로그램 연결이 필요합니다. 지도 작업은 계속할 수 있습니다."
    );
    assert!(NgiiTopographicClient::login_stage_blocked_message().contains("받은 자료 불러오기"));
}
