//! The requests the soil map is fetched with (C++ `SoilMapService.cpp:43-44`, `:80-107`).

use crate::tiled_fetch::Range;

#[cfg(test)]
#[path = "soil_request_tests.rs"]
mod tests;

/// The public map service of NAAS (흙토람; no key, no account; `:43`).
pub const SERVICE: &str = "https://gis.naas.go.kr/geoserver/soilmap/ows";
/// What the C++ names itself as to this service (`:82`).
pub const USER_AGENT: &str = "ka-hgis/0.3";
/// The one property that is asked for: the terrain class (`kTerrainField`, `:44`).
pub const TERRAIN_FIELD: &str = "soil_type_geo";
/// `count=100000` (`:90`): an answer of that many features was cut.
pub const MAX_FEATURES: u64 = 100_000;
/// The service keeps the country in three tables, `SOIL_1` to `SOIL_3` (`:275`).
pub const TABLES: std::ops::RangeInclusive<u8> = 1..=3;

/// The request for one table and one box of longitude and latitude (`wfsGetFeatureUrl`, `:87-107`): either
/// the count of what matches (an XML answer) or the features with their terrain class only.
#[must_use]
pub fn features_url(table: u8, range: &Range, hits_only: bool) -> String {
    let form = if hits_only { "&resultType=hits".to_owned() } else { format!("&outputFormat=application/json&propertyName={TERRAIN_FIELD},geom") };
    // The box as latitude then longitude, as the URN form of EPSG:4326 orders them, with eight decimals.
    format!(
        "{SERVICE}?service=WFS&version=2.0.0&request=GetFeature&typeNames=soilmap:SOIL_{table}&srsName=EPSG:5186&count={MAX_FEATURES}{form}&bbox={:.8},{:.8},{:.8},{:.8},urn:ogc:def:crs:EPSG::4326",
        range[1], range[0], range[3], range[2]
    )
}

/// The count an answer to a hits request states: the digits of the first `numberMatched="…"` (`:285-287`).
#[must_use]
pub fn number_matched(body: &[u8]) -> Option<u64> {
    const NAME: &[u8] = b"numberMatched";
    let blank = |rest: &[u8]| rest.len() - rest.trim_ascii_start().len();
    let mut from = 0;
    while let Some(found) = body[from..].windows(NAME.len()).position(|window| window == NAME) {
        from += found + NAME.len();
        // `\s*=\s*"(\d+)"` after the name.
        let rest = &body[from..];
        let Some(rest) = rest[blank(rest)..].strip_prefix(b"=") else {
            continue;
        };
        let Some(rest) = rest[blank(rest)..].strip_prefix(b"\"") else {
            continue;
        };
        let digits = rest.iter().take_while(|byte| byte.is_ascii_digit()).count();
        if digits > 0 && rest.get(digits) == Some(&b'"') {
            return std::str::from_utf8(&rest[..digits]).ok()?.parse().ok();
        }
    }
    None
}
