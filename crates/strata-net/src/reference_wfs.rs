//! 토양도(흙토람)·수계도(VWorld) — 공개 WFS 에서 면을 GeoJSON 으로 받는다.
//!
//! C++ `SoilMapService.cpp`·`RiverMapService.cpp`:
//! - 토양도: `https://gis.naas.go.kr/geoserver/soilmap/ows`, 레이어 `soilmap:SOIL_1`~`SOIL_3`,
//!   속성 `soil_type_geo`(분포지형 코드), 좌표 EPSG:5186, 키 없음
//! - 수계도: `https://api.vworld.kr/req/wfs`, 레이어 `lt_c_wkmstrm`, 속성 `riv_level`·`riv_nm`,
//!   좌표 EPSG:4326, VWorld 키 필요, `Referer: https://localhost`, `DOMAIN` 은 보내지 않는다
//! - bbox 는 둘 다 **위도 먼저** + `urn:ogc:def:crs:EPSG::4326`

use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ka-hgis/0.3";
pub const SOIL_LAYER_TITLE: &str = "토양도(흙토람)";
pub const RIVER_LAYER_TITLE: &str = "수계도(하천망)";
/// 토양도는 화면 범위 한 변 80 km 이하, 수계도는 160 km 이하(C++ 상한).
pub const SOIL_MAX_SPAN_M: f64 = 80_000.0;
pub const RIVER_MAX_SPAN_M: f64 = 160_000.0;
const RIVER_PAGE_SIZE: usize = 1000;
const RIVER_MAX_PAGES: usize = 10;

/// 받은 면 하나. 좌표계는 호출한 쪽이 안다(토양 5186, 수계 4326).
#[derive(Debug, Clone, PartialEq)]
pub struct WfsPolygon {
    pub rings: Vec<Vec<[f64; 2]>>,
    /// 요청한 속성 이름 순서대로의 값(없으면 빈 글자).
    pub props: Vec<String>,
}

fn rings_of(polygon: &serde_json::Value) -> Vec<Vec<[f64; 2]>> {
    polygon
        .as_array()
        .map(|rings| {
            rings
                .iter()
                .filter_map(|ring| {
                    let pts: Vec<[f64; 2]> = ring
                        .as_array()?
                        .iter()
                        .filter_map(|p| Some([p.get(0)?.as_f64()?, p.get(1)?.as_f64()?]))
                        .collect();
                    (pts.len() >= 4).then_some(pts)
                })
                .collect()
        })
        .unwrap_or_default()
}

/// GeoJSON FeatureCollection 의 면을 푼다. `props` 는 읽을 속성 이름.
pub fn parse_wfs_polygons(bytes: &[u8], props: &[&str]) -> Result<Vec<WfsPolygon>, String> {
    let bad = || "지도 서버 응답을 읽지 못했습니다.".to_owned();
    let root: serde_json::Value = serde_json::from_slice(bytes).map_err(|_| bad())?;
    let features = root.get("features").and_then(serde_json::Value::as_array).ok_or_else(bad)?;
    let mut out = Vec::new();
    for feature in features {
        let values: Vec<String> = props
            .iter()
            .map(|name| match feature["properties"].get(*name) {
                Some(serde_json::Value::String(s)) => s.trim().to_owned(),
                Some(serde_json::Value::Number(n)) => n.to_string(),
                _ => String::new(),
            })
            .collect();
        let geometry = &feature["geometry"];
        let polygons: Vec<Vec<Vec<[f64; 2]>>> = match geometry.get("type").and_then(serde_json::Value::as_str) {
            Some("Polygon") => vec![rings_of(&geometry["coordinates"])],
            Some("MultiPolygon") => geometry["coordinates"]
                .as_array()
                .map(|polys| polys.iter().map(rings_of).collect())
                .unwrap_or_default(),
            _ => Vec::new(),
        };
        for rings in polygons.into_iter().filter(|r| !r.is_empty()) {
            out.push(WfsPolygon { rings, props: values.clone() });
        }
    }
    Ok(out)
}

/// 주소 하나를 `dest` 에 받는다. 15초 무응답이면 중단, 전체 180초(C++ `PreparedReferenceMap.h:41-52`).
/// `referer` 가 있으면 붙인다. 받는 동안 `cancel` 이 서면 멈춘다.
pub fn download_to(url: &str, dest: &Path, referer: Option<&str>, cancel: &AtomicBool) -> Result<(), String> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("저장 폴더를 만들지 못했습니다: {e}"))?;
    }
    let mut cmd = Command::new("curl");
    cmd.args(["-sS", "-g", "-A", USER_AGENT, "--speed-time", "15", "--speed-limit", "1", "--max-time", "180"]);
    if let Some(r) = referer {
        cmd.args(["-e", r]);
    }
    let mut child = cmd
        .args(["-w", "%{http_code}", "-o"])
        .arg(dest)
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("지도 서버에 연결하지 못했습니다: {e}"))?;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if cancel.load(Ordering::Relaxed) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = std::fs::remove_file(dest);
                return Err("내려받기를 취소했습니다. 기존 지도는 유지됩니다.".to_owned());
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(100)),
            Err(e) => return Err(format!("지도 서버에 연결하지 못했습니다: {e}")),
        }
    };
    let mut code = String::new();
    if let Some(mut out) = child.stdout.take() {
        let _ = std::io::Read::read_to_string(&mut out, &mut code);
    }
    if status.success() && code.trim() == "200" {
        return Ok(());
    }
    let _ = std::fs::remove_file(dest);
    Err(match status.code() {
        Some(28) => "지도 서버의 응답이 늦어 요청을 중단했습니다. 인터넷 연결을 확인한 뒤 다시 내려받으세요.".to_owned(),
        Some(23) => "받은 자료를 저장하지 못했습니다. 저장 공간을 확인하세요.".to_owned(),
        _ => format!("지도 서버에 연결하지 못했습니다(HTTP {}). 인터넷 연결을 확인한 뒤 다시 내려받으세요.", code.trim()),
    })
}

fn bbox_lat_first(bbox: [f64; 4]) -> String {
    format!("{:.8},{:.8},{:.8},{:.8},urn:ogc:def:crs:EPSG::4326", bbox[1], bbox[0], bbox[3], bbox[2])
}

/// 토양도 WFS 주소(C++ `SoilMapService.cpp:88-107`). `table` 은 1~3.
#[must_use]
pub fn soil_wfs_url(table: u8, bbox: [f64; 4]) -> String {
    format!(
        "https://gis.naas.go.kr/geoserver/soilmap/ows?service=WFS&version=2.0.0&request=GetFeature\
         &typeNames=soilmap:SOIL_{table}&srsName=EPSG:5186&count=100000&outputFormat=application/json\
         &propertyName=soil_type_geo,geom&bbox={}",
        bbox_lat_first(bbox)
    )
}

/// 토양도를 받는다(표 3개를 차례로). 돌려주는 면의 좌표는 EPSG:5186, 속성은 `[soil_type_geo]`.
pub fn fetch_soil(bbox: [f64; 4], folder: &Path, cancel: &AtomicBool) -> Result<Vec<WfsPolygon>, String> {
    let mut all = Vec::new();
    for table in 1..=3_u8 {
        let dest = folder.join(format!("토양도_흙토람_{table}.geojson"));
        download_to(&soil_wfs_url(table, bbox), &dest, None, cancel)?;
        let bytes = std::fs::read(&dest).map_err(|e| format!("받은 토양도를 읽지 못했습니다: {e}"))?;
        all.extend(parse_wfs_polygons(&bytes, &["soil_type_geo"])?);
    }
    Ok(all)
}

/// 수계도 WFS 주소(C++ `RiverMapService.cpp:105-128`). `DOMAIN` 은 넣지 않는다(넣으면 키가 거부된다).
#[must_use]
pub fn river_wfs_url(key: &str, bbox: [f64; 4], page: usize) -> String {
    format!(
        "https://api.vworld.kr/req/wfs?SERVICE=WFS&REQUEST=GetFeature&VERSION=2.0.0&TYPENAME=lt_c_wkmstrm\
         &OUTPUT=application/json&SRSNAME=EPSG:4326&MAXFEATURES={RIVER_PAGE_SIZE}&STARTINDEX={}&BBOX={}&KEY={}",
        page * RIVER_PAGE_SIZE,
        bbox_lat_first(bbox),
        key.trim()
    )
}

/// 수계도를 받는다(1000건씩 최대 10쪽). 돌려주는 면의 좌표는 EPSG:4326, 속성은 `[riv_level, riv_nm]`.
pub fn fetch_river(key: &str, bbox: [f64; 4], folder: &Path, cancel: &AtomicBool) -> Result<Vec<WfsPolygon>, String> {
    let mut all = Vec::new();
    for page in 0..RIVER_MAX_PAGES {
        let dest = folder.join(format!("수계도_VWorld_{page}.geojson"));
        download_to(&river_wfs_url(key, bbox, page), &dest, Some("https://localhost"), cancel)?;
        let bytes = std::fs::read(&dest).map_err(|e| format!("받은 수계도를 읽지 못했습니다: {e}"))?;
        // 오류는 HTTP 200 + XML `<ServiceException>` 으로 온다(C++ `:86-90`).
        if bytes.starts_with(b"<") {
            let _ = std::fs::remove_file(&dest);
            let text = String::from_utf8_lossy(&bytes);
            return Err(if text.contains("KEY") {
                "VWorld 인증키가 거부되었습니다. 배경지도 설정에서 키를 확인한 뒤 다시 내려받으세요.".to_owned()
            } else {
                "수계도 서버가 오류를 돌려주었습니다. 잠시 뒤 다시 내려받으세요.".to_owned()
            });
        }
        let rows = parse_wfs_polygons(&bytes, &["riv_level", "riv_nm"])?;
        let short = rows.len() < RIVER_PAGE_SIZE;
        all.extend(rows);
        if short {
            break;
        }
    }
    Ok(all)
}

/// 토양 분포지형 코드 → (이름, 지도에 붙일 짧은 이름, 색). C++ `SoilMapService.cpp:54-78,173-240`.
#[must_use]
pub fn soil_class(code: &str) -> (&'static str, &'static str, [u8; 3]) {
    match code.trim() {
        "01" | "1" => ("산악지", "산악지", [1, 178, 0]),
        "02" | "2" => ("구릉지", "구릉지", [177, 210, 93]),
        "03" | "3" => ("산록경사지", "산록경사", [139, 68, 108]),
        "04" | "4" => ("곡간지/선상지", "곡간·선상", [254, 244, 182]),
        "05" | "5" => ("해성평탄지", "해성평탄", [132, 186, 230]),
        "06" | "6" => ("하성평탄지", "하성평탄", [192, 239, 245]),
        "07" | "7" => ("고원지", "고원지", [78, 78, 78]),
        "08" | "8" => ("홍적대지", "홍적대지", [254, 160, 0]),
        "09" | "9" => ("용암류대지", "용암대지", [192, 151, 155]),
        "10" => ("용암류평탄", "용암평탄", [194, 179, 182]),
        "99" => ("기타", "기타", [255, 255, 255]),
        _ => ("미분류", "", [200, 200, 200]),
    }
}

/// 하천 등급 → 색. C++ `RiverMapService.cpp:39-78`.
#[must_use]
pub fn river_color(level: &str) -> [u8; 3] {
    if level.contains("국가") {
        [0, 126, 212]
    } else if level.contains("1급") {
        [16, 146, 222]
    } else if level.contains("2급") {
        [40, 166, 232]
    } else {
        [120, 196, 236]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_match_the_cpp_urls() {
        let b = [126.94, 36.60, 127.06, 36.70];
        let soil = soil_wfs_url(2, b);
        assert!(soil.starts_with("https://gis.naas.go.kr/geoserver/soilmap/ows?service=WFS&version=2.0.0&request=GetFeature&typeNames=soilmap:SOIL_2&srsName=EPSG:5186"), "{soil}");
        assert!(soil.ends_with("&propertyName=soil_type_geo,geom&bbox=36.60000000,126.94000000,36.70000000,127.06000000,urn:ogc:def:crs:EPSG::4326"), "{soil}");
        let river = river_wfs_url(" KEY1 ", b, 2);
        assert!(river.contains("TYPENAME=lt_c_wkmstrm&OUTPUT=application/json&SRSNAME=EPSG:4326&MAXFEATURES=1000&STARTINDEX=2000&BBOX=36.60000000,"), "{river}");
        assert!(river.ends_with("&KEY=KEY1") && !river.contains("DOMAIN"), "{river}");
    }

    #[test]
    fn classes_and_colours_follow_the_cpp_tables() {
        assert_eq!(soil_class("06"), ("하성평탄지", "하성평탄", [192, 239, 245]));
        assert_eq!(soil_class("08").2, [254, 160, 0]);
        assert_eq!(soil_class("zz").0, "미분류");
        assert_eq!(river_color("국가하천"), [0, 126, 212]);
        assert_eq!(river_color("지방2급하천"), [40, 166, 232]);
        assert_eq!(river_color(""), [120, 196, 236]);
    }

    #[test]
    fn polygons_and_requested_properties_are_read_from_geojson() {
        let json = r#"{"type":"FeatureCollection","features":[{"type":"Feature","properties":{"riv_level":"국가하천","riv_nm":"금강"},
          "geometry":{"type":"Polygon","coordinates":[[[127.0,36.6],[127.1,36.6],[127.1,36.7],[127.0,36.6]]]}}]}"#;
        let polys = parse_wfs_polygons(json.as_bytes(), &["riv_level", "riv_nm", "none"]).expect("해석");
        assert_eq!(polys.len(), 1);
        assert_eq!(polys[0].props, vec!["국가하천".to_owned(), "금강".to_owned(), String::new()]);
        assert!(parse_wfs_polygons(b"<ServiceException/>", &[]).is_err());
    }
}
