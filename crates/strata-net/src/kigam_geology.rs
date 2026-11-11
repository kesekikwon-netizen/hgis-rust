//! 수치지질도(KIGAM 1:5만) — 공개 WFS 에서 암상 면을 받는다. 키·로그인이 없다.
//!
//! C++ `GeologyMapService.cpp`:
//! - 주소 `https://data.kigam.re.kr/geoserver/ows`, WFS 2.0.0 GetFeature, GeoJSON, 좌표 EPSG:5186
//! - 레이어: 본토 `geoOpen:l_50k_geology_litho_latest`, 제주 `geoOpen:l_jeju_50k_geology_litho_view`
//! - bbox 는 **위도 먼저**, 소수 8자리, `urn:ogc:def:crs:EPSG::4326`
//! - 쓰는 속성: `시대`·`기호`·`지층`. 글자 속의 HTML 태그는 지운다.
//! - 색: ICS 표준 지질시대색(기 단위를 먼저 찾고, 없으면 대 단위)
//!
//! 실제 서버 응답으로 확인했다(2026-11-11): FeatureCollection, MultiPolygon, `crs` 는 EPSG:5186.

use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// 레이어 이름(C++ `GeologyMapService` 의 제목).
pub const GEOLOGY_LAYER_TITLE: &str = "지질도(KIGAM 1:5만)";
/// 받을 수 있는 가장 넓은 범위(한 변, m) — C++ `GeologyMapService.h:74`.
pub const GEOLOGY_MAX_SPAN_M: f64 = 80_000.0;
const OWS_URL: &str = "https://data.kigam.re.kr/geoserver/ows";
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ka-hgis/0.3";
const ERA_UNKNOWN: (&str, [u8; 3]) = ("시대미상", [200, 200, 200]);

/// (찾는 낱말, 분류 이름, 색) — 젊은 시대 → 오래된 시대. C++ `kEras` 그대로.
const ERAS: [(&str, &str, [u8; 3]); 21] = [
    ("제4기", "제4기", [249, 249, 127]),
    ("네오기", "네오기(신제3기)", [255, 230, 25]),
    ("신제3기", "네오기(신제3기)", [255, 230, 25]),
    ("팔레오기", "팔레오기(고제3기)", [253, 154, 82]),
    ("고제3기", "팔레오기(고제3기)", [253, 154, 82]),
    ("백악기", "백악기", [127, 198, 78]),
    ("쥐라기", "쥐라기", [52, 178, 201]),
    ("쥬라기", "쥐라기", [52, 178, 201]),
    ("트라이아스기", "트라이아스기", [129, 43, 146]),
    ("페름기", "페름기", [240, 64, 40]),
    ("석탄기", "석탄기", [103, 165, 153]),
    ("데본기", "데본기", [203, 140, 55]),
    ("실루리아기", "실루리아기", [179, 225, 182]),
    ("오르도비스기", "오르도비스기", [0, 146, 112]),
    ("캄브리아기", "캄브리아기", [127, 160, 86]),
    ("신생대", "신생대", [242, 249, 29]),
    ("중생대", "중생대", [103, 197, 202]),
    ("고생대", "고생대", [153, 192, 141]),
    ("원생누대", "원생누대", [247, 53, 99]),
    ("시생누대", "시생누대", [240, 4, 127]),
    ("선캄브리아", "선캄브리아시대", [247, 67, 112]),
];

/// 시대 글자에서 분류 이름과 기준색을 찾는다. 제주 응답의 「제 4기」처럼 공백이 있어도 맞춘다.
#[must_use]
pub fn era_class(era_text: &str) -> (&'static str, [u8; 3]) {
    let compact: String = era_text.chars().filter(|c| *c != ' ').collect();
    ERAS.iter()
        .find(|(keyword, _, _)| era_text.contains(keyword) || compact.contains(keyword))
        .map_or(ERA_UNKNOWN, |(_, name, rgb)| (name, *rgb))
}

/// 같은 시대의 단위를 구분하려고 기호 글자 합으로 밝기를 조금 바꾼 색
/// (C++ `unitFallbackColor`: -2..+2 단계 × 12 %).
#[must_use]
pub fn unit_color(era_text: &str, symbol: &str) -> [u8; 3] {
    let (_, base) = era_class(era_text);
    let sum: u32 = symbol.bytes().map(u32::from).sum();
    let factor = 1.0 + (f64::from(sum % 5) - 2.0) * 0.12;
    base.map(|c| (f64::from(c) * factor).round().clamp(0.0, 255.0) as u8)
}

/// 범위(경위도 `[min_lon, min_lat, max_lon, max_lat]`)에 맞는 암상 레이어. 본토·제주 밖이면 `None`.
#[must_use]
pub fn litho_type_name(bbox: [f64; 4]) -> Option<&'static str> {
    let overlaps = |b: [f64; 4]| bbox[0] <= b[2] && b[0] <= bbox[2] && bbox[1] <= b[3] && b[1] <= bbox[3];
    if overlaps([124.609_99, 33.968_71, 129.584_73, 38.605_07]) {
        Some("geoOpen:l_50k_geology_litho_latest")
    } else if overlaps([126.10, 33.10, 127.00, 33.62]) {
        Some("geoOpen:l_jeju_50k_geology_litho_view")
    } else {
        None
    }
}

/// WFS GetFeature 주소(C++ `GeologyMapService.cpp:231-241`).
#[must_use]
pub fn geology_wfs_url(type_name: &str, bbox: [f64; 4]) -> String {
    format!(
        "{OWS_URL}?service=WFS&version=2.0.0&request=GetFeature&typeNames={type_name}\
         &outputFormat=application/json&srsName=EPSG:5186&count=100000\
         &bbox={:.8},{:.8},{:.8},{:.8},urn:ogc:def:crs:EPSG::4326",
        bbox[1], bbox[0], bbox[3], bbox[2]
    )
}

/// 지질 단위 면 하나(좌표는 EPSG:5186).
#[derive(Debug, Clone, PartialEq)]
pub struct GeologyPolygon {
    /// 바깥 고리 + 구멍들.
    pub rings: Vec<Vec<[f64; 2]>>,
    pub era: String,
    pub symbol: String,
    pub formation: String,
}

fn strip_tags(text: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for ch in text.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.trim().to_owned()
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

/// GeoJSON 응답을 면 목록으로 푼다. MultiPolygon 은 면마다 따로 낸다.
pub fn parse_geology_geojson(bytes: &[u8]) -> Result<Vec<GeologyPolygon>, String> {
    let bad = || "지질도 서버 응답을 읽지 못했습니다.".to_owned();
    let root: serde_json::Value = serde_json::from_slice(bytes).map_err(|_| bad())?;
    if root.get("type").and_then(serde_json::Value::as_str) != Some("FeatureCollection") {
        return Err(bad());
    }
    let features = root.get("features").and_then(serde_json::Value::as_array).ok_or_else(bad)?;
    let mut out = Vec::new();
    for feature in features {
        let props = &feature["properties"];
        let text = |key: &str| props.get(key).and_then(serde_json::Value::as_str).map(strip_tags).unwrap_or_default();
        let (era, symbol, formation) = (text("시대"), text("기호"), text("지층"));
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
            out.push(GeologyPolygon {
                rings,
                era: era.clone(),
                symbol: symbol.clone(),
                formation: formation.clone(),
            });
        }
    }
    Ok(out)
}

/// 지질도를 받아 `dest` 에 GeoJSON 으로 저장하고 면 목록을 돌려준다.
///
/// 시간 제한은 C++ `PreparedReferenceMap.h:41-52` 와 같다: 15초 무응답이면 중단, 전체 180초.
pub fn fetch_geology(bbox: [f64; 4], dest: &Path, cancel: &AtomicBool) -> Result<Vec<GeologyPolygon>, String> {
    let type_name = litho_type_name(bbox)
        .ok_or_else(|| "이 범위에는 KIGAM 암상 지질도가 없습니다(본토·제주 밖).".to_owned())?;
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("지질도 저장 폴더를 만들지 못했습니다: {e}"))?;
    }
    let mut child = Command::new("curl")
        .args(["-sS", "-g", "-A", USER_AGENT, "--speed-time", "15", "--speed-limit", "1", "--max-time", "180"])
        .args(["-w", "%{http_code}", "-o"])
        .arg(dest)
        .arg(geology_wfs_url(type_name, bbox))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("KIGAM 서버 연결 실패: {e}"))?;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if cancel.load(Ordering::Relaxed) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = std::fs::remove_file(dest);
                return Err("지질도 내려받기를 취소했습니다. 기존 지도는 유지됩니다.".to_owned());
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(100)),
            Err(e) => return Err(format!("KIGAM 서버 연결 실패: {e}")),
        }
    };
    let mut code = String::new();
    if let Some(mut out) = child.stdout.take() {
        let _ = std::io::Read::read_to_string(&mut out, &mut code);
    }
    if !status.success() || code.trim() != "200" {
        let _ = std::fs::remove_file(dest);
        return Err(match status.code() {
            Some(28) => "지도 서버의 응답이 늦어 요청을 중단했습니다. 인터넷 연결을 확인한 뒤 다시 내려받으세요.".to_owned(),
            Some(23) => "지질도를 저장하지 못했습니다. 저장 공간을 확인하세요.".to_owned(),
            _ => format!("KIGAM 서버 연결 실패: HTTP {}", code.trim()),
        });
    }
    let bytes = std::fs::read(dest).map_err(|e| format!("받은 지질도를 읽지 못했습니다: {e}"))?;
    parse_geology_geojson(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eras_get_their_ics_colour_period_before_era_and_jeju_spacing_is_tolerated() {
        assert_eq!(era_class("현생이언 중생대 백악기"), ("백악기", [127, 198, 78]));
        assert_eq!(era_class("현생이언 신생대 제 4기"), ("제4기", [249, 249, 127]));
        assert_eq!(era_class("현생이언 중생대"), ("중생대", [103, 197, 202]));
        assert_eq!(era_class("선캄브리아시대"), ("선캄브리아시대", [247, 67, 112]));
        assert_eq!(era_class("시대 미상"), ("시대미상", [200, 200, 200]));
        // 같은 시대라도 기호가 다르면 밝기가 달라진다.
        assert_ne!(unit_color("백악기", "Kbd"), unit_color("백악기", "Kad"));
    }

    #[test]
    fn the_wfs_request_puts_latitude_first_and_picks_the_mainland_or_jeju_layer() {
        let url = geology_wfs_url("geoOpen:l_50k_geology_litho_latest", [126.94, 36.60, 127.06, 36.70]);
        assert!(url.ends_with("&bbox=36.60000000,126.94000000,36.70000000,127.06000000,urn:ogc:def:crs:EPSG::4326"), "{url}");
        assert!(url.contains("typeNames=geoOpen:l_50k_geology_litho_latest&outputFormat=application/json&srsName=EPSG:5186&count=100000"));
        assert_eq!(litho_type_name([126.94, 36.60, 127.06, 36.70]), Some("geoOpen:l_50k_geology_litho_latest"));
        assert_eq!(litho_type_name([126.30, 33.20, 126.60, 33.50]), Some("geoOpen:l_jeju_50k_geology_litho_view"));
        assert_eq!(litho_type_name([140.0, 40.0, 141.0, 41.0]), None);
    }

    #[test]
    fn geojson_multipolygons_become_separate_polygons_with_clean_attributes() {
        let json = r#"{"type":"FeatureCollection","features":[{"type":"Feature","properties":{"시대":"현생이언 중생대 백악기","기호":"Kbd","지층":"불국사<b>화강암</b>"},
          "geometry":{"type":"MultiPolygon","coordinates":[[[[0,0],[10,0],[10,10],[0,10],[0,0]],[[4,4],[6,4],[6,6],[4,6],[4,4]]],[[[20,0],[30,0],[30,10],[20,0]]]]}}]}"#;
        let polys = parse_geology_geojson(json.as_bytes()).expect("해석");
        assert_eq!(polys.len(), 2);
        assert_eq!((polys[0].rings.len(), polys[1].rings.len()), (2, 1), "첫 면은 구멍이 하나 있다");
        assert_eq!((polys[0].symbol.as_str(), polys[0].formation.as_str()), ("Kbd", "불국사화강암"));
        assert!(parse_geology_geojson(b"<html>error</html>").is_err());
    }
}
