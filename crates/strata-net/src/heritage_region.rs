//! Which city the survey area lies in (`target/cpp-ref/src/core/HeritageRegionResolver.cpp`): the heritage
//! intranet gives its data by city, so one point of the survey area is asked of VWorld's address service
//! with the user's key. This file builds the question and reads the answer; it sends nothing.

pub use crate::address_search::KoreaRegionCatalog;

#[cfg(test)]
#[path = "heritage_region_tests.rs"]
mod tests;

/// Why no city could be given (`:296-345`).
pub const NO_KEY: &str = "VWorld 키가 없어 시·군을 판정하지 못했습니다. 더보기에서 키를 넣거나 시·군을 직접 고르세요.";
pub const NOT_REACHED: &str = "행정구역 서버에 연결하지 못했습니다. 인터넷 연결을 확인한 뒤 다시 누르세요.";
pub const NOT_FOUND: &str = "좌표로 시·군을 찾지 못했습니다. 시·군을 직접 고르세요.";
/// The service is given this long (`:56`).
pub const TIMEOUT_SECS: u64 = 15;

/// A province and a city of it, by the names of the list the dialog offers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeritageRegion {
    pub sido: String,
    pub city: String,
}

/// The question for a point in degrees (WGS 84).
#[must_use]
pub fn address_url(key: &str, lon: f64, lat: f64) -> String {
    format!("https://api.vworld.kr/req/address?service=address&request=getAddress&version=2.0&crs=EPSG:4326&point={lon:.8},{lat:.8}&format=json&type=both&zipcode=false&simple=false&key={key}")
}

/// The province and the city an address names.
#[must_use]
pub fn region_of_address(text: &str) -> Option<HeritageRegion> {
    let mut words: Vec<&str> = text.split_whitespace().collect();
    if words.first() == Some(&"대한민국") {
        words.remove(0);
    }
    // The province is one of the first two words (`:131-140`).
    let known = |name: &str| KoreaRegionCatalog::all_sido().iter().any(|sido| sido.name == name);
    let (at, sido) = words.iter().take(2).enumerate().map(|(at, word)| (at, KoreaRegionCatalog::canonical_sido(word))).find(|(_, sido)| known(sido))?;
    let cities = KoreaRegionCatalog::cities_of(&sido);
    let rest = &words[at + 1..];
    // Two words first: 「수원시 장안구」 folds into the 「수원시」 of the list (`:145-159`).
    let listed = [2, 1].into_iter().filter(|take| rest.len() >= *take).find_map(|take| {
        let joined = rest[..take].join(" ");
        cities.iter().find(|city| **city == joined).or_else(|| cities.iter().find(|city| joined.starts_with(&format!("{city} ")))).cloned()
    });
    // A name the list does not know (a new or renamed city): the first word ending in 시, 군 or 구.
    let unlisted = || rest.iter().find(|word| ["시", "군", "구"].iter().any(|end| word.ends_with(end))).map(|word| (*word).to_owned());
    // A province with one city, as 세종.
    let only = || (cities.len() == 1).then(|| cities[0].clone());
    let city = listed.or_else(unlisted).or_else(only)?;
    Some(HeritageRegion { sido, city })
}

/// Reads the answer of the service: the city, or `None` when the answer names none.
///
/// # Errors
/// The answer is not OK; the words are for the user and never hold the server's text.
pub fn parse_address(body: &[u8]) -> Result<Option<HeritageRegion>, String> {
    let root: serde_json::Value = serde_json::from_slice(body).unwrap_or_default();
    let response = &root["response"];
    if !response["status"].as_str().is_some_and(|status| status.eq_ignore_ascii_case("OK")) {
        // Neither the server's text nor the request is repeated: either can hold the key.
        return Err(match response["error"]["code"].as_str() {
            Some(code @ ("INVALID_KEY" | "INCORRECT_KEY")) => format!("VWorld API 키가 서버에서 거절되었습니다 ({code}).\n더보기 → API 키 입력에서 유효한 VWorld 키를 저장한 뒤 다시 누르세요.\n이 키는 조사구역의 자동 시·도 판정과 지적도에 함께 사용됩니다."),
            _ => "VWorld에서 조사구역의 주소를 받지 못했습니다. 잠시 뒤 다시 시도하세요.".to_owned(),
        });
    }
    let text = |value: &serde_json::Value| value.as_str().map(str::trim).unwrap_or_default().to_owned();
    for item in response["result"].as_array().into_iter().flatten() {
        let (level1, level2) = (text(&item["structure"]["level1"]), text(&item["structure"]["level2"]));
        let by_structure = (!level1.is_empty()).then(|| region_of_address(&format!("{level1} {level2}"))).flatten();
        if let Some(region) = by_structure.or_else(|| region_of_address(&text(&item["text"]))) {
            return Ok(Some(region));
        }
    }
    Ok(None)
}
