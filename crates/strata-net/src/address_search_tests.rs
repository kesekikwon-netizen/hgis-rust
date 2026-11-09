//! `address_search` 시험 — VWorld 응답 해석·지번 질의·좌표 읽기.
//!
//! 원래 `address_search.rs` 안 `mod tests` 를 그대로 옮겼다.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use strata_types::error::StrataResult;

#[test]
fn vworld_address_search_parser_extracts_parcel_coords_and_sido_presets_exist(
) -> StrataResult<()> {
    assert!(!sido_presets().is_empty());
    let sample = r#"{
        "response": {
            "status": "OK",
            "result": {
                "items": [
                    {
                        "title": "경상북도 경주시 인왕동 76",
                        "address": { "parcel": "경상북도 경주시 인왕동 76" },
                        "point": { "x": "129.2247", "y": "35.8347" }
                    }
                ]
            }
        }
    }"#;
    let hits = parse_vworld_search_json(sample.as_bytes())?;
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].title, "경상북도 경주시 인왕동 76");
    assert!((hits[0].lon - 129.2247).abs() < 1e-4);
    assert!((hits[0].lat - 35.8347).abs() < 1e-4);
    Ok(())
}

#[test]
fn test_region_locator_parsing_chungnam_buyeo_ssangbuk_123_extracts_admin_hierarchy_and_jibun()
{
    let parsed = parse_cadastral_query_ko("충남 부여군 부여읍 쌍북리 123")
        .expect("valid cadastral query should parse");
    assert_eq!(parsed.sido, "충남");
    assert_eq!(parsed.sigungu, "부여군");
    assert_eq!(parsed.eup_myeon_dong, "부여읍");
    assert_eq!(parsed.ri_or_detail, "쌍북리");
    assert_eq!(parsed.jibun, "123");
    assert!(format_geocode_error_ko(GeocodeError::NotFound)
        .contains("해당 지번을 찾을 수 없습니다."));
}

/// `read_json_coord` 는 VWorld 응답의 좌표를 숫자·문자열 모두 받아들인다.
/// clippy `question_mark` 정리(동작 불변) 뒤에도 같은 값을 내는지 고정한다.
#[test]
fn read_json_coord_accepts_numbers_and_strings_and_rejects_the_rest() {
    let num = serde_json::json!(129.2247);
    let int = serde_json::json!(35);
    let string_ok = serde_json::json!("123.5");
    let string_bad = serde_json::json!("not-a-number");
    let null = serde_json::Value::Null;
    let boolean = serde_json::json!(true);
    let array = serde_json::json!([1.0]);
    let non_finite_text = serde_json::Value::String("NaN".to_owned());
    let infinity_text = serde_json::Value::String("inf".to_owned());

    assert_eq!(read_json_coord(Some(&num)), Some(129.2247));
    assert_eq!(read_json_coord(Some(&int)), Some(35.0));
    assert_eq!(read_json_coord(Some(&string_ok)), Some(123.5));
    // 파싱 실패·비숫자 타입은 좌표 없음
    assert_eq!(read_json_coord(Some(&string_bad)), None);
    assert_eq!(read_json_coord(Some(&null)), None);
    assert_eq!(read_json_coord(Some(&boolean)), None);
    assert_eq!(read_json_coord(Some(&array)), None);
    // 무한·NaN 문자열은 파싱되더라도 유한하지 않으므로 버린다
    assert_eq!(read_json_coord(Some(&non_finite_text)), None);
    assert_eq!(read_json_coord(Some(&infinity_text)), None);
    // 값 자체가 없으면 좌표 없음
    assert_eq!(read_json_coord(None), None);
}
