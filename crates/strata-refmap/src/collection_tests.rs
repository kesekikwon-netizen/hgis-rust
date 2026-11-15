//! Expected values of `target/cpp-ref/tests/test_reference_download.cpp` tiledFetchMergesAndBoundsRetries
//! (:251-262, the two answers and the counts) and the rules of `PreparedReferenceMap.cpp:100-126`.

use serde_json::Value;

use super::*;

const FIRST: &str = r#"{"type":"FeatureCollection","numberMatched":2,"features":[{"type":"Feature","id":"a","properties":{},"geometry":null},{"type":"Feature","properties":{"n":1},"geometry":null}]}"#;
const SECOND: &str = r#"{"type":"FeatureCollection","numberMatched":2,"features":[{"type":"Feature","id":"a","properties":{},"geometry":null},{"type":"Feature","properties":{"n":1},"geometry":null},{"type":"Feature","id":"b","properties":{},"geometry":null}]}"#;

/// tiledFetchMergesAndBoundsRetries :251-262: feature "a" and the feature without an id come from both
/// answers and are kept once; the joined answer counts three and is complete.
#[test]
fn features_returned_by_two_pieces_are_kept_once() {
    let merged = merge_feature_collections(&[FIRST.as_bytes().to_vec(), SECOND.as_bytes().to_vec()]).expect("merged");
    let object: Value = serde_json::from_slice(&merged).expect("json");
    assert_eq!(object["features"].as_array().expect("list").len(), 3);
    assert_eq!(object["numberMatched"], 3);
    assert!(complete_feature_collection(&merged).is_ok());
    let ids: Vec<&Value> = object["features"].as_array().expect("list").iter().map(|f| &f["id"]).collect();
    assert_eq!(ids, [&Value::from("a"), &Value::Null, &Value::from("b")], "in the order received");
}

/// The other members of the first answer stay (the CRS of the coordinates), and only count members that
/// were there are rewritten.
#[test]
fn the_first_answer_gives_the_other_members() {
    let first = br#"{"type":"FeatureCollection","crs":{"type":"name","properties":{"name":"EPSG:5186"}},"totalFeatures":1,"numberReturned":1,"features":[{"type":"Feature","id":1,"properties":{}}]}"#.to_vec();
    let second = br#"{"type":"FeatureCollection","crs":{"type":"name","properties":{"name":"EPSG:4326"}},"features":[{"type":"Feature","id":1,"properties":{}},{"type":"Feature","id":2,"properties":{}}]}"#.to_vec();
    let object: Value = serde_json::from_slice(&merge_feature_collections(&[first, second]).expect("merged")).expect("json");
    assert_eq!(object["crs"]["properties"]["name"], "EPSG:5186");
    assert_eq!((&object["totalFeatures"], &object["numberReturned"]), (&Value::from(2), &Value::from(2)));
    assert!(object.get("numberMatched").is_none());
    assert_eq!(object["features"].as_array().expect("list").len(), 2, "the number id 1 is one feature");
    // The id decides, not the content: the same id with other properties is still that feature, kept as it
    // came first.
    let first = br#"{"type":"FeatureCollection","features":[{"type":"Feature","id":7,"properties":{"n":1}}]}"#.to_vec();
    let second = br#"{"type":"FeatureCollection","features":[{"type":"Feature","id":7,"properties":{"n":2}},{"type":"Feature","id":"7x","properties":{"n":2}}]}"#.to_vec();
    let object: Value = serde_json::from_slice(&merge_feature_collections(&[first, second]).expect("merged")).expect("json");
    let kept: Vec<&Value> = object["features"].as_array().expect("list").iter().map(|f| &f["properties"]["n"]).collect();
    assert_eq!(kept, [&Value::from(1), &Value::from(2)]);
}

/// Features without an id are the same only when all their content is: another value is another feature.
#[test]
fn features_without_an_id_are_compared_by_content() {
    let one = br#"{"type":"FeatureCollection","features":[{"type":"Feature","properties":{"n":1,"m":"x"},"geometry":null}]}"#.to_vec();
    let same_other_order = br#"{"type":"FeatureCollection","features":[{"properties":{"m":"x","n":1},"geometry":null,"type":"Feature"}]}"#.to_vec();
    let other = br#"{"type":"FeatureCollection","features":[{"type":"Feature","properties":{"n":2,"m":"x"},"geometry":null}]}"#.to_vec();
    let empty_id = br#"{"type":"FeatureCollection","features":[{"type":"Feature","id":"","properties":{"n":1,"m":"x"},"geometry":null}]}"#.to_vec();
    let count = |bodies: &[Vec<u8>]| {
        let object: Value = serde_json::from_slice(&merge_feature_collections(bodies).expect("merged")).expect("json");
        object["features"].as_array().expect("list").len()
    };
    assert_eq!(count(&[one.clone(), same_other_order]), 1);
    assert_eq!(count(&[one.clone(), other]), 2);
    assert_eq!(count(&[one, empty_id]), 2, "an empty id is no id, and that member makes the content differ");
}

#[test]
fn an_answer_that_is_no_feature_list_cannot_be_joined() {
    for bad in [&b"<ServiceExceptionReport/>"[..], br#"{"type":"FeatureCollection"}"#, br#"[1,2]"#, br#"{"features":3}"#] {
        assert_eq!(merge_feature_collections(&[FIRST.as_bytes().to_vec(), bad.to_vec()]).expect_err("refused"), NOT_JOINED);
    }
}

/// `validateFeatureCollection` (:100-106).
#[test]
fn only_a_feature_collection_is_map_data() {
    assert_eq!(feature_count(&feature_collection(FIRST.as_bytes()).expect("ok")), 2);
    assert!(feature_collection(br#"{"type":"FeatureCollection","features":[]}"#).is_ok());
    for bad in [&b""[..], b"<ServiceExceptionReport>unavailable</ServiceExceptionReport>", br#"{"type":"Feature","features":[]}"#, br#"{"type":"FeatureCollection","features":{}}"#, br#"{"type":"FeatureCollection"}"#, b"[]"] {
        assert_eq!(feature_collection(bad).expect_err("refused"), NOT_MAP_DATA);
    }
}

/// `validateCompleteFeatureCollection` (:108-126): the server's own counts, as numbers or as digits.
#[test]
fn an_answer_with_fewer_features_than_matched_is_only_a_part() {
    let with = |member: &str| format!(r#"{{"type":"FeatureCollection",{member},"features":[{{"type":"Feature"}}]}}"#).into_bytes();
    assert_eq!(complete_feature_collection(&with(r#""numberMatched":5"#)).expect_err("part"), ONLY_PART);
    assert_eq!(complete_feature_collection(&with(r#""totalFeatures":2"#)).expect_err("part"), ONLY_PART);
    assert_eq!(complete_feature_collection(&with(r#""totalFeatures":"2""#)).expect_err("part"), ONLY_PART);
    assert_eq!(complete_feature_collection(&with(r#""numberMatched":5.0"#)).expect_err("part"), ONLY_PART);
    for whole in [r#""numberMatched":1"#, r#""numberMatched":0"#, r#""numberMatched":"unknown""#, r#""numberMatched":null"#, r#""numberReturned":9"#, r#""numberMatched":1.5"#] {
        assert!(complete_feature_collection(&with(whole)).is_ok(), "{whole}");
    }
    assert_eq!(complete_feature_collection(b"{}").expect_err("no map data"), NOT_MAP_DATA);
}
