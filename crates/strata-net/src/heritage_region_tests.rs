//! Which city a point lies in, as VWorld's address service says it (`HeritageRegionResolver.cpp:102-220`).
//! The answers here are written by hand in the form of the service; no request is sent.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

fn region(sido: &str, city: &str) -> Option<HeritageRegion> {
    Some(HeritageRegion { sido: sido.to_owned(), city: city.to_owned() })
}

/// `buildAddressUrl` (`:102-119`): the point in degrees with eight decimals, longitude first.
#[test]
fn the_question_names_the_point_and_the_key() {
    assert_eq!(
        address_url("KEY", 126.978, 37.5665),
        "https://api.vworld.kr/req/address?service=address&request=getAddress&version=2.0&crs=EPSG:4326&point=126.97800000,37.56650000&format=json&type=both&zipcode=false&simple=false&key=KEY"
    );
}

/// `fromAddressText` (`:121-173`): 「대한민국」 is skipped; the province is one of the first two words, by
/// its full or short name; the city is a name of the list (two words first, so that 「수원시 장안구」 folds
/// into 「수원시」), else the first word ending in 시, 군 or 구; a province with one city gives that city.
#[test]
fn an_address_names_the_province_and_the_city() {
    assert_eq!(region_of_address("대한민국 경상북도 경주시 황남동 1"), region("경상북도", "경주시"));
    assert_eq!(region_of_address("  경북   경주시 "), region("경상북도", "경주시"));
    assert_eq!(region_of_address("경기도 수원시 장안구 조원동"), region("경기도", "수원시"));
    assert_eq!(region_of_address("강원도 춘천시"), region("강원특별자치도", "춘천시"));
    assert_eq!(region_of_address("경상북도 가상군 가상읍"), region("경상북도", "가상군"), "a city the list does not know");
    let sejong = KoreaRegionCatalog::cities_of("세종특별자치시");
    assert_eq!(sejong.len(), 1, "the list has one city for 세종");
    assert_eq!(region_of_address("세종특별자치시 한누리대로"), region("세종특별자치시", &sejong[0]));
    assert_eq!(region_of_address("경상북도"), None, "no city");
    assert_eq!(region_of_address("어딘가 모르는 곳"), None);
    assert_eq!(region_of_address("  "), None);
}

/// `parseAddress` (`:175-220`): the first result that names a province and a city, by its `structure`
/// (level1, level2) or else by its `text`.
#[test]
fn an_answer_gives_the_city() {
    let by_structure = r#"{"response":{"status":"OK","result":[{"text":"x","structure":{"level1":"경상북도","level2":"경주시"}}]}}"#;
    assert_eq!(parse_address(by_structure.as_bytes()), Ok(region("경상북도", "경주시")));
    let by_text = r#"{"response":{"status":"ok","result":[{"structure":{"level1":"","level2":""},"text":"경기도 수원시 장안구 조원동"}]}}"#;
    assert_eq!(parse_address(by_text.as_bytes()), Ok(region("경기도", "수원시")));
    let second = r#"{"response":{"status":"OK","result":[{"text":"바다"},{"structure":{"level1":"제주특별자치도","level2":"제주시"}}]}}"#;
    assert_eq!(parse_address(second.as_bytes()), Ok(region("제주특별자치도", "제주시")));
    let nothing = r#"{"response":{"status":"OK","result":[{"text":"바다"}]}}"#;
    assert_eq!(parse_address(nothing.as_bytes()), Ok(None));
}

/// An answer that is not OK: a refused key is named by its code, never with the server's text or the
/// request (either can hold the key); anything else asks to try again.
#[test]
fn an_answer_that_is_not_ok_says_why() {
    let refused = r#"{"response":{"status":"ERROR","error":{"code":"INVALID_KEY","text":"등록되지 않은 인증키입니다. key=SECRET"}}}"#;
    let said = parse_address(refused.as_bytes()).unwrap_err();
    assert_eq!(said, "VWorld API 키가 서버에서 거절되었습니다 (INVALID_KEY).\n더보기 → API 키 입력에서 유효한 VWorld 키를 저장한 뒤 다시 누르세요.\n이 키는 조사구역의 자동 시·도 판정과 지적도에 함께 사용됩니다.");
    let incorrect = r#"{"response":{"status":"ERROR","error":{"code":"INCORRECT_KEY"}}}"#;
    assert!(parse_address(incorrect.as_bytes()).unwrap_err().contains("(INCORRECT_KEY)"));
    for other in [r#"{"response":{"status":"NOT_FOUND"}}"#, r#"{"response":{"status":"ERROR","error":{"code":"OVER_REQUEST_LIMIT"}}}"#, "<html>busy</html>", ""] {
        assert_eq!(parse_address(other.as_bytes()), Err("VWorld에서 조사구역의 주소를 받지 못했습니다. 잠시 뒤 다시 시도하세요.".to_owned()), "{other}");
    }
}
