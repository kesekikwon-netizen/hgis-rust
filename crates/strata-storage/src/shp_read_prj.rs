//! `.prj`(ESRI WKT) 에서 EPSG 번호를 추정한다.
//!
//! `shp_reader` 에서 분리했다(300줄 규칙).
//!
//! C++ 은 QGIS `QgsCoordinateReferenceSystem::fromWkt()` 로 WKT 를 해석한 뒤
//! `authid()` 를 봤다(`D:\hgis\src\core\CadCrsHints.cpp` `CadCrsHints::fromPrj`).
//! Rust 에는 그 WKT 해석기가 없으므로, 한국에서 실제로 배포되는 `.prj` 글에서
//! **투영 이름·단위·원점·거짓 동거리·거짓 북거리**를 읽어 EPSG 를 고른다.
//!
//! 전역 좌표변환을 다시 만드는 것이 아니라, 파일 옆에 적힌 글을 읽는 일만 한다.
//! 실제 변환은 `strata-geo` 의 `proj4rs` 가 한다.
//!
//! ## 알아보는 것
//!
//! - `AUTHORITY["EPSG","5179"]` 처럼 권위 번호가 그대로 적힌 경우 → 그 값
//! - `GEOGCS` 만 있는 지리좌표계 → 4326(GRS80/WGS84 계열) 또는 4166/4162 계열
//! - `PROJCS` + 한국 원점 → 5187/5186/5185/5188(GRS80 2010)·5183/5181/5180/5184(2002)·
//!   5176/5174/5173/5177(베셀 보정)·5179(UTM-K)·32651/32652(UTM)
//!
//! 못 알아보면 `None`. C++ 도 `crs.isValid()` 가 아니면 빈 문자열을 돌려줬다.

use std::path::Path;

use crate::shp_reader::find_sibling;

/// `.prj` 를 읽어 EPSG 를 추정한다. 파일이 없거나 못 알아보면 `None`.
#[must_use]
pub fn read_shp_epsg(shp_path: &Path) -> Option<u32> {
    let prj = find_sibling(shp_path, "prj")?;
    let bytes = std::fs::read(&prj).ok()?;
    // .prj 는 8비트 ASCII 또는 UTF-8 이다. 해독 실패 글자는 버린다.
    let text = String::from_utf8_lossy(&bytes[..bytes.len().min(65_536)]).into_owned();
    epsg_from_wkt(&text)
}

/// ESRI/OGC WKT 글에서 EPSG 를 추정한다.
#[must_use]
pub fn epsg_from_wkt(text: &str) -> Option<u32> {
    if let Some(code) = authority_epsg(text) {
        return Some(code);
    }
    let lower = text.to_ascii_lowercase();
    if !lower.contains("projcs") {
        // 지리좌표계만 있는 경우. 한국 측지계는 4326 계열로 본다.
        if lower.contains("geogcs") || lower.contains("geodcrs") {
            return geographic_epsg(&lower);
        }
        return None;
    }
    projected_epsg(&lower)
}

/// `AUTHORITY["EPSG","5179"]` · `ID["EPSG",5179]` 중 **가장 바깥** 번호.
///
/// ESRI 글은 `PROJCS` 안의 `GEOGCS` 에 측지계 AUTHORITY(예: 4162)를 먼저 쓴다.
/// 파일 전체의 좌표계는 가장 바깥(괄호 깊이가 가장 얕은) AUTHORITY 이므로,
/// 깊이를 세어 그것을 고른다. `PROJCS` 가 없으면 첫 AUTHORITY.
fn authority_epsg(text: &str) -> Option<u32> {
    let upper = text.to_ascii_uppercase();
    let bytes = upper.as_bytes();
    let mut depth = 0_usize;
    let mut best: Option<(usize, u32)> = None;
    let mut i = 0_usize;
    while i < bytes.len() {
        match bytes[i] {
            b'[' => depth += 1,
            b']' => depth = depth.saturating_sub(1),
            _ => {}
        }
        if let Some(code) = epsg_number_at(&upper[i..]) {
            let keep = best.is_none_or(|(d, _)| depth < d);
            if keep {
                best = Some((depth, code));
            }
        }
        i += 1;
    }
    best.map(|(_, code)| code)
}

/// 이 조각이 `AUTHORITY[...EPSG...N]`/`ID[...EPSG...N]` 이면 N.
fn epsg_number_at(rest: &str) -> Option<u32> {
    if !rest.starts_with("AUTHORITY") && !rest.starts_with("ID") {
        return None;
    }
    let open = rest.find('[')?;
    let close = rest[open..].find(']')? + open;
    let inner = &rest[open + 1..close];
    let pos = inner.find("EPSG")?;
    let digits: String = inner[pos + 4..]
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(char::is_ascii_digit)
        .collect();
    let code = digits.parse::<u32>().ok()?;
    (1..=99_999).contains(&code).then_some(code)
}

fn geographic_epsg(lower: &str) -> Option<u32> {
    let bessel = lower.contains("bessel") || lower.contains("korea 1985");
    if bessel {
        return Some(4162);
    }
    Some(4326)
}

/// `PROJCS` 글의 투영 이름·원점·거짓 북거리로 한국 EPSG 를 고른다.
///
/// 한국에서 배포되는 `.prj` 는 대개 ESRI 글이다. 예:
/// `PROJCS["Korea_2000_Korea_East_Belt_2010",GEOGCS["GCS_Korea_2000",
/// DATUM["D_Korea_2000",SPHEROID["GRS_1980",...]]],PROJECTION["Transverse_Mercator"],
/// PARAMETER["False_Easting",200000],PARAMETER["False_Northing",600000],
/// PARAMETER["Central_Meridian",129]]`
fn projected_epsg(lower: &str) -> Option<u32> {
    if lower.contains("unified") || lower.contains("utmk") || lower.contains("utm-k") {
        return Some(5179);
    }
    // 위도 38° UTM(52N: 중부·동부, 51N: 서부)
    if lower.contains("utm zone 52") || lower.contains("utm_zone_52") {
        return Some(32652);
    }
    if lower.contains("utm zone 51") || lower.contains("utm_zone_51") {
        return Some(32651);
    }
    let central = number_after(lower, "central_meridian")?;
    // 베셀 측지계(구 지적·동경측지계)인가.
    let bessel = lower.contains("bessel") || lower.contains("korea 1985") || lower.contains("tokyo");
    let (b2002, b2010, bbessel) = belt_of_central_meridian(central);
    if bessel {
        return Some(bbessel);
    }
    // 2010 원점은 거짓 북거리 600 km, 2002 원점은 500 km. 적혀 있지 않으면 2010(현행)으로 본다.
    let false_northing = number_after(lower, "false_northing").unwrap_or(600_000.0);
    let is_2002 = (false_northing - 500_000.0).abs() < (false_northing - 600_000.0).abs();
    Some(if is_2002 { b2002 } else { b2010 })
}

/// (2002 원점, 2010 원점, 베셀 보정) 묶음을 중앙 자오선으로 고른다.
///
/// 한국 국가기본도 원점 4개: 서부 125°, 중부 127°, 동부 129°, 동해 131°.
const fn belt_of_central_meridian(central: f64) -> (u32, u32, u32) {
    if central < 126.0 {
        (5180, 5185, 5173) // 서부원점
    } else if central < 128.0 {
        (5181, 5186, 5174) // 중부원점
    } else if central < 130.0 {
        (5183, 5187, 5176) // 동부원점
    } else {
        (5184, 5188, 5177) // 동해원점
    }
}

/// `PARAMETER["central_meridian",129.0]` 같은 줄에서 숫자만 뽑는다.
fn number_after(lower: &str, key: &str) -> Option<f64> {
    let idx = lower.find(key)?;
    let rest = &lower[idx + key.len()..];
    let mut out = String::new();
    let mut started = false;
    for ch in rest.chars() {
        // 숫자·소수점, 그리고 맨 앞의 부호만 받는다. 그 밖의 글자가 나오면 끝.
        if ch.is_ascii_digit() || ch == '.' || (ch == '-' && !started) {
            out.push(ch);
            started = true;
        } else if started {
            break;
        }
    }
    out.parse::<f64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authority_wins_and_uses_top_level_value() {
        let wkt = concat!(
            "PROJCS[\"Korea 2000 / Unified CS\",",
            "GEOGCS[\"Korea 2000\",AUTHORITY[\"EPSG\",\"4162\"]],",
            "AUTHORITY[\"EPSG\",\"5179\"]]"
        );
        assert_eq!(epsg_from_wkt(wkt), Some(5179));
    }

    #[test]
    fn crate_wkt_for_5179_is_recognised() {
        assert_eq!(
            Some(crate::shp_writer::EPSG_5179_PRJ_WKT),
            Some(crate::shp_writer::EPSG_5179_PRJ_WKT)
        );
        let epsg = epsg_from_wkt(crate::shp_writer::EPSG_5179_PRJ_WKT);
        assert_eq!(epsg, Some(5179));
    }

    #[test]
    fn central_meridian_picks_belt() {
        // 실무 4원점: 서부 125°, 중부 127°, 동부 129°, 동해 131°.
        let east = "PROJCS[\"x\",PARAMETER[\"central_meridian\",129],\
                    PARAMETER[\"false_northing\",600000]]";
        assert_eq!(epsg_from_wkt(east), Some(5187));
        let central = "PROJCS[\"x\",PARAMETER[\"central_meridian\",127],\
                       PARAMETER[\"false_northing\",600000]]";
        assert_eq!(epsg_from_wkt(central), Some(5186));
        let west = "PROJCS[\"x\",PARAMETER[\"central_meridian\",125],\
                    PARAMETER[\"false_northing\",600000]]";
        assert_eq!(epsg_from_wkt(west), Some(5185));
        let donghae = "PROJCS[\"x\",PARAMETER[\"central_meridian\",131],\
                       PARAMETER[\"false_northing\",600000]]";
        assert_eq!(epsg_from_wkt(donghae), Some(5188));
    }

    #[test]
    fn false_northing_splits_2002_and_2010_belts() {
        // 2002 원점은 거짓 북거리 500 km, 2010 원점은 600 km.
        let y2002 = "PROJCS[\"x\",PARAMETER[\"central_meridian\",129],\
                     PARAMETER[\"false_northing\",500000]]";
        assert_eq!(epsg_from_wkt(y2002), Some(5183));
        let y2010 = "PROJCS[\"x\",PARAMETER[\"central_meridian\",129],\
                     PARAMETER[\"false_northing\",600000]]";
        assert_eq!(epsg_from_wkt(y2010), Some(5187));
    }

    #[test]
    fn bessel_wkt_maps_to_corrected_belt() {
        let bessel = "PROJCS[\"Korea\",GEOGCS[\"Korea 1985\",DATUM[\"Bessel\"]],\
                      PARAMETER[\"central_meridian\",129]]";
        assert_eq!(epsg_from_wkt(bessel), Some(5176));
    }

    #[test]
    fn esri_style_east_belt_name_is_recognised() {
        let wkt = concat!(
            "PROJCS[\"Korea_2000_Korea_East_Belt_2010\",",
            "GEOGCS[\"GCS_Korea_2000\",DATUM[\"D_Korea_2000\",",
            "SPHEROID[\"GRS_1980\",6378137,298.257222101]]],",
            "PROJECTION[\"Transverse_Mercator\"],",
            "PARAMETER[\"False_Easting\",200000],",
            "PARAMETER[\"False_Northing\",600000],",
            "PARAMETER[\"Central_Meridian\",129],",
            "UNIT[\"Meter\",1.0]]"
        );
        assert_eq!(epsg_from_wkt(wkt), Some(5187));
    }

    #[test]
    fn geographic_wkt_and_garbage() {
        assert_eq!(epsg_from_wkt("GEOGCS[\"WGS 84\"]"), Some(4326));
        assert_eq!(epsg_from_wkt("GEOGCS[\"Korea 1985\",DATUM[\"Bessel\"]]"), Some(4162));
        assert_eq!(epsg_from_wkt(""), None);
        assert_eq!(epsg_from_wkt("not a wkt"), None);
    }

    #[test]
    fn missing_prj_file_is_none() {
        assert_eq!(read_shp_epsg(Path::new("no-such-dir/none.shp")), None);
    }
}
