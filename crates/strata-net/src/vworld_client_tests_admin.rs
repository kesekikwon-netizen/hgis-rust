//! `vworld_client` 시험 2/2 — 행정경계·국가유산 후속·타일 분할.
//!
//! 원래 `vworld_client.rs` 안 `mod tests` 를 주제별로 나눴다.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

#[test]
fn test_vworld_select_cadastral_resources_picks_one_zip_per_district() {
    let resources = vec![
        CadastralResource {
            file_no: "1".to_owned(),
            name: "LSMD_CONT_LDREG_경북_경주시.zip".to_owned(),
            size_kb: 100,
            revision: "2026-01-05".to_owned(),
        },
        CadastralResource {
            file_no: "2".to_owned(),
            name: "LSMD_CONT_LDREG_경북_포항시.zip".to_owned(),
            size_kb: 100,
            revision: String::new(),
        },
    ];
    let picked = select_cadastral_resources(
        &resources,
        &[CadastralDistrict {
            code: "47110".to_owned(),
            name: "경상북도 경주시".to_owned(),
        }],
    )
    .expect("select");
    assert_eq!(picked.len(), 1);
    assert_eq!(picked[0].file_no, "1");

    // 둘이 걸리면 C++ `:196` 처럼 오류
    let ambiguous = vec![
        resources[0].clone(),
        CadastralResource {
            file_no: "3".to_owned(),
            name: "LSMD_CONT_LDREG_경상북도_경주시.zip".to_owned(),
            size_kb: 100,
            revision: String::new(),
        },
    ];
    let err = select_cadastral_resources(
        &ambiguous,
        &[CadastralDistrict {
            code: "47110".to_owned(),
            name: "경상북도 경주시".to_owned(),
        }],
    )
    .expect_err("두 ZIP 이 걸리면 C++ 처럼 오류");
    assert!(err.contains("지적도 파일을 정확히 하나로 확인하지 못했습니다"), "{err}");
    assert!(err.starts_with("경상북도 경주시 "));
}

#[test]
fn test_vworld_admin_boundary_url_matches_cpp_getfeature() {
    // C++ `AdminBoundaryService.cpp:50-70`
    let url = VworldClient::admin_boundary_url("KEY123", "경상북도", "경주시", "황남동");
    assert!(url.starts_with("https://api.vworld.kr/req/data?service=data&version=2.0&request=GetFeature"));
    assert!(url.contains("&data=LT_C_ADEMD_INFO"));
    assert!(url.contains("&size=10&page=1&geometry=true&attribute=true&crs=EPSG:3857"));
    assert!(url.contains("&key=KEY123"));
    assert!(url.contains(&format!(
        "&attrFilter={}",
        percent_encode("full_nm:like:경상북도 경주시 황남동")
    )));
    assert_eq!(
        VworldClient::attr_filter("경상북도", "경주시", "황남동"),
        "full_nm:like:경상북도 경주시 황남동"
    );
    // 빈 값은 빠진다.
    assert_eq!(VworldClient::attr_filter("", "경주시", ""), "full_nm:like:경주시");
    assert_eq!(VworldClient::attr_filter("", "", ""), "");
    let bare = VworldClient::admin_boundary_url("K", "", "", "");
    assert!(!bare.contains("attrFilter"));
}

#[test]
fn test_vworld_normalize_crs_name_and_admin_errors_match_cpp() {
    // C++ `AdminBoundaryService.cpp:72-81`
    assert_eq!(VworldClient::normalize_crs_name("urn:ogc:def:crs:EPSG::5186"), "EPSG:5186");
    assert_eq!(VworldClient::normalize_crs_name("EPSG:5187"), "EPSG:5187");
    assert_eq!(VworldClient::normalize_crs_name("EPSG:5179"), "EPSG:5179");
    assert_eq!(VworldClient::normalize_crs_name("urn:x:4326"), "EPSG:4326");
    assert_eq!(VworldClient::normalize_crs_name("EPSG:900913"), "EPSG:3857");
    assert_eq!(VworldClient::normalize_crs_name("EPSG:32652"), "EPSG:32652");
    assert_eq!(VworldClient::normalize_crs_name("nonsense"), "EPSG:3857");
    // C++ `AdminBoundaryService.cpp:133-142`
    assert_eq!(VworldClient::admin_boundary_error("OK", ""), None);
    assert_eq!(
        VworldClient::admin_boundary_error("ERROR", "INVALID_KEY"),
        Some(ADMIN_KEY_FAILED_MESSAGE)
    );
    assert_eq!(
        VworldClient::admin_boundary_error("NOT_FOUND", "NOT_FOUND"),
        Some(ADMIN_NOT_FOUND_MESSAGE)
    );
    assert_eq!(
        VworldClient::admin_boundary_error("ERROR", "SYSTEM"),
        Some(ADMIN_BAD_RESPONSE_MESSAGE)
    );
}

#[test]
fn test_vworld_parse_admin_boundary_builds_wkt_and_crs() {
    let body = r#"{"response":{"status":"OK","result":{"featureCollection":{
      "crs":{"properties":{"name":"urn:ogc:def:crs:EPSG::3857"}},
      "features":[{"properties":{"full_nm":"경상북도 경주시 황남동","emd_cd":"4711025000"},
        "geometry":{"type":"Polygon","coordinates":[[[129.0,35.8],[129.1,35.8],[129.1,35.9],[129.0,35.8]]]}}]}}}}"#;
    let parsed = parse_admin_boundary(body.as_bytes(), "경주시").expect("parse");
    assert_eq!(parsed.title, "경상북도 경주시 황남동");
    assert_eq!(parsed.emd_code, "4711025000");
    assert_eq!(parsed.crs_auth_id, "EPSG:3857");
    assert!(parsed.wkt.starts_with("POLYGON(("));
    assert!(parsed.wkt.contains("129.00000000 35.80000000"));
    // 상태가 OK 가 아니면 C++ 문구
    let failed = br#"{"response":{"status":"ERROR","error":{"code":"INVALID_KEY"}}}"#;
    assert_eq!(
        parse_admin_boundary(failed, "").expect_err("INVALID_KEY 는 오류"),
        ADMIN_KEY_FAILED_MESSAGE
    );
}

#[test]
fn test_heritage_nearby_regions_and_tiled_fetch_and_pledge() {
    // TestHeritageNearbyRegions::testBoundaryBufferExpansion:
    // Gyeongju ("47110") and Pohang Nam-gu ("47111") separated by 150.0m, buffer = 500.0m -> both included (len 2)
    let survey_bbox = [200_000.0, 300_000.0, 200_500.0, 300_500.0];
    let districts = vec![
        (
            "47110".to_owned(),
            [190_000.0, 290_000.0, 200_500.0, 310_000.0],
        ),
        (
            "47111".to_owned(),
            [200_650.0, 290_000.0, 220_000.0, 310_000.0],
        ),
        (
            "47130".to_owned(),
            [250_000.0, 290_000.0, 260_000.0, 310_000.0],
        ),
    ];
    let codes = resolve_adjacent_sigungu_codes(survey_bbox, &districts, 500.0);
    assert_eq!(codes.len(), 2);
    assert!(codes.contains(&"47110".to_owned()));
    assert!(codes.contains(&"47111".to_owned()));

    // TestReferenceTiledFetch::testTileSplitCalculation:
    // 5,000m x 3,000m with 2,000m limit -> 3 x 2 = 6 tiles
    let plan = calculate_tile_splits([0.0, 0.0, 5_000.0, 3_000.0], 2_000.0, 2_000.0);
    assert_eq!(plan.num_tiles_x, 3);
    assert_eq!(plan.num_tiles_y, 2);
    assert_eq!(plan.sub_tile_extents.len(), 6);

    // HeritageFetchPlan::siblingRoot
    let next = HeritageCity {
        sido: "경상북도".to_owned(),
        city: "포항시".to_owned(),
    };
    assert_eq!(
        sibling_heritage_root("/data/survey/경상북도 경주시/원본", &next),
        "/data/survey/경상북도 포항시/원본"
    );

    // HeritagePledge::noticeAfterAgreement
    let notice = heritage_pledge_notice_after_agreement(
        "제1조 본 자료는 지표조사 보고서 작성 목적 외 외부 반출을 금합니다.",
    );
    assert!(notice.contains("서약 요지(원문 그대로):"));
}
