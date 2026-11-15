//! Checks on the GeoJSON answers of the map services, and joining answers fetched in pieces.
//!
//! C++ `ReferenceMapPreparation::validateFeatureCollection` / `validateCompleteFeatureCollection`
//! (`target/cpp-ref/src/core/PreparedReferenceMap.cpp:100-126`) and
//! `ReferenceTiledFetch::mergeFeatureCollections` (`src/core/ReferenceTiledFetch.cpp:92-126`).

use std::collections::HashSet;

use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

#[cfg(test)]
#[path = "collection_tests.rs"]
mod tests;

/// `PreparedReferenceMap.cpp:104`.
pub const NOT_MAP_DATA: &str = "지도 서버가 올바른 지도 데이터를 보내지 않았습니다. 잠시 후 다시 내려받으세요.";
/// `PreparedReferenceMap.cpp:119`.
pub const ONLY_PART: &str = "지도 데이터 일부만 받았습니다. 기존 지도는 유지됩니다. 범위를 좁혀 다시 내려받으세요.";
/// `ReferenceTiledFetch.cpp:103`.
pub const NOT_JOINED: &str = "나눠 받은 지도 자료를 합치지 못했습니다. 기존 지도는 유지됩니다. 다시 내려받으세요.";

/// The answer as a FeatureCollection: an object of that type whose `features` is a list.
///
/// # Errors
/// `NOT_MAP_DATA` for anything else.
pub fn feature_collection(body: &[u8]) -> Result<Map<String, Value>, String> {
    match serde_json::from_slice::<Value>(body) {
        Ok(Value::Object(object)) if object.get("type").and_then(Value::as_str) == Some("FeatureCollection") && object.get("features").is_some_and(Value::is_array) => Ok(object),
        _ => Err(NOT_MAP_DATA.to_owned()),
    }
}

/// How many features an answer holds.
#[must_use]
pub fn feature_count(collection: &Map<String, Value>) -> usize {
    collection.get("features").and_then(Value::as_array).map_or(0, Vec::len)
}

/// A count the server states, as a number or as digits in a text (`value.isDouble() ? … : toLongLong`).
fn stated(value: &Value) -> Option<i64> {
    match value {
        // A number that is no whole number counts as -1, which is never more than what was received
        // (`toInteger(-1)`); 5.0 is the whole number 5.
        #[allow(clippy::cast_possible_truncation)]
        Value::Number(n) => Some(n.as_i64().or_else(|| n.as_f64().filter(|f| f.fract() == 0.0 && f.abs() < 9.0e15).map(|f| f as i64)).unwrap_or(-1)),
        Value::String(text) => text.trim().parse().ok(),
        _ => None,
    }
}

/// The answer, when the server says it holds everything that matched: `numberMatched` and `totalFeatures`
/// must not be larger than the number of features received.
///
/// # Errors
/// `NOT_MAP_DATA`, or `ONLY_PART` when the server matched more than it sent.
pub fn complete_feature_collection(body: &[u8]) -> Result<Map<String, Value>, String> {
    let collection = feature_collection(body)?;
    let received = i64::try_from(feature_count(&collection)).unwrap_or(i64::MAX);
    for key in ["numberMatched", "totalFeatures"] {
        if collection.get(key).and_then(stated).is_some_and(|expected| expected > received) {
            return Err(ONLY_PART.to_owned());
        }
    }
    Ok(collection)
}

/// What makes two features the same one: their `id`, or their whole content when they have none.
fn feature_key(feature: &Value) -> String {
    match feature.get("id") {
        Some(Value::String(id)) if !id.is_empty() => format!("id:{id}"),
        Some(Value::Number(id)) => format!("id:{id}"),
        // serde_json writes an object's members in sorted order: equal content gives equal text.
        _ => format!("json:{:x}", Sha256::digest(feature.to_string().as_bytes())),
    }
}

/// Joins answers fetched for neighbouring pieces. A feature that several pieces returned is kept once; the
/// other members of the first answer (`crs`) are kept, the count members are set to the joined count.
///
/// # Errors
/// `NOT_JOINED` when an answer is not an object with a feature list.
pub fn merge_feature_collections(bodies: &[Vec<u8>]) -> Result<Vec<u8>, String> {
    let mut root: Option<Map<String, Value>> = None;
    let mut features = Vec::new();
    let mut seen = HashSet::new();
    for body in bodies {
        let Ok(Value::Object(mut object)) = serde_json::from_slice::<Value>(body) else {
            return Err(NOT_JOINED.to_owned());
        };
        let Some(Value::Array(list)) = object.remove("features") else {
            return Err(NOT_JOINED.to_owned());
        };
        for feature in list {
            if seen.insert(feature_key(&feature)) {
                features.push(feature);
            }
        }
        root.get_or_insert(object);
    }
    let mut root = root.unwrap_or_default();
    let count = features.len();
    root.insert("type".to_owned(), Value::from("FeatureCollection"));
    root.insert("features".to_owned(), Value::Array(features));
    for key in ["numberMatched", "numberReturned", "totalFeatures"] {
        if root.contains_key(key) {
            root.insert(key.to_owned(), Value::from(count));
        }
    }
    Ok(Value::Object(root).to_string().into_bytes())
}
