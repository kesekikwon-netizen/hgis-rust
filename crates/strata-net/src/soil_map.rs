//! 토양도(흙토람) 지도 받기 — C++ `SoilMapService` 대응.
//!
//! ## 왜 필요한가
//!
//! 2026-11-10: 리본 「토양」 버튼을 눌러도 **레이어만 켜지고 데이터가 안 왔다**.
//! 사용자 눈에는 "눌러도 아무것도 안 나온다"였다.
//!
//! C++ 은 `SoilMapService` 로 흙토람 서버에서 두 가지를 받는다:
//! 1. **그림 지도** — XYZ 타일(`_alllayers/L{z}/R{y}/C{x}.png`)
//! 2. **벡터** — WFS GeoJSON(`soilmap:SOIL_1~3`)
//!
//! ## ArcGIS 캐시 경로 규칙 (C++ `rewriteArcGisCacheUrl`)
//!
//! 흙토람은 ArcGIS 캐시 형식을 쓴다. 경로가 **0으로 채운 16진수**다:
//!
//! ```text
//! L{z, 2자리 10진} / R{y, 8자리 16진} / C{x, 8자리 16진} .png
//! 예) z=13, x=7036, y=3221  →  L13/R00000c95/C00001b7c.png
//! ```
//!
//! 10진수로 요청하면 404 다. 처음에 이걸 몰라 "토양도가 안 온다"로 보였다.

use std::path::{Path, PathBuf};

/// 흙토람 그림 지도 서버.
pub const SOIL_TILE_BASE: &str = "https://gis.naas.go.kr/Geodata/SF/TOP_A_SOIL_T_GEO/Layers/_alllayers";
/// 흙토람 WFS 서버.
pub const SOIL_WFS_BASE: &str = "https://gis.naas.go.kr/geoserver/soilmap/ows";
/// 토양 분류 필드(C++ `kTerrainField`).
pub const SOIL_TERRAIN_FIELD: &str = "soil_type_geo";
/// 레이어 제목(C++ `kLayerTitle`).
pub const SOIL_LAYER_TITLE: &str = "토양도(흙토람)";
/// 그림 레이어 제목(C++ `kPictureTitle`).
pub const SOIL_PICTURE_TITLE: &str = "토양도(흙토람 그림)";
/// WFS 표 번호 범위(C++ `for (tableNo = 1; tableNo <= 3; ++tableNo)`).
pub const SOIL_TABLE_RANGE: std::ops::RangeInclusive<i32> = 1..=3;

/// ArcGIS 캐시 타일 경로를 만든다. C++ `rewriteArcGisCacheUrl` 과 같은 규칙.
///
/// `z` 는 2자리 10진, `y`·`x` 는 8자리 16진(0 채움)이다.
#[must_use]
pub fn arcgis_cache_tile_url(z: u8, x: u32, y: u32) -> String {
    format!(
        "{SOIL_TILE_BASE}/L{z:02}/R{y:08x}/C{x:08x}.png"
    )
}

/// 토양 벡터 WFS URL. C++ `wfsGetFeatureUrl` 과 같다.
///
/// `bbox_4326` 은 `[min_lon, min_lat, max_lon, max_lat]`.
/// C++ 은 `(ymin, xmin, ymax, xmax)` 순서로 넣는다 — WFS 2.0 의 축 순서가
/// 위도·경도이기 때문이다.
#[must_use]
pub fn soil_wfs_url(table_no: i32, bbox_4326: [f64; 4], hits_only: bool) -> String {
    let mut url = format!(
        "{SOIL_WFS_BASE}?service=WFS&version=2.0.0&request=GetFeature\
         &typeNames=soilmap:SOIL_{table_no}&srsName=EPSG:5186&count=100000"
    );
    if hits_only {
        url.push_str("&resultType=hits");
    } else {
        url.push_str("&outputFormat=application/json&propertyName=");
        url.push_str(SOIL_TERRAIN_FIELD);
        url.push_str(",geom");
    }
    url.push_str(&format!(
        "&bbox={:.8},{:.8},{:.8},{:.8},urn:ogc:def:crs:EPSG::4326",
        bbox_4326[1], bbox_4326[0], bbox_4326[3], bbox_4326[2]
    ));
    url
}

/// 타일 캐시 폴더.
#[must_use]
pub fn soil_cache_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    base.join("ka-hgis").join("tiles").join("soil")
}

/// 타일 한 장의 캐시 경로.
#[must_use]
pub fn soil_tile_path(z: u8, x: u32, y: u32) -> PathBuf {
    soil_cache_dir()
        .join(z.to_string())
        .join(x.to_string())
        .join(format!("{y}.png"))
}

/// 타일 한 장을 받아 온다. 캐시에 있으면 그대로 쓴다.
///
/// 실패하면 `None`(그 타일만 비고 나머지 지도는 그대로).
pub fn fetch_soil_tile(z: u8, x: u32, y: u32) -> Option<PathBuf> {
    let dest = soil_tile_path(z, x, y);
    if is_png(&dest) {
        return Some(dest);
    }
    if let Some(dir) = dest.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let url = arcgis_cache_tile_url(z, x, y);
    if !run_curl(&url, &dest) {
        return None;
    }
    if is_png(&dest) {
        Some(dest)
    } else {
        // 흙토람은 없는 타일에 HTML 404 를 준다 — 이미지가 아니면 버린다.
        let _ = std::fs::remove_file(&dest);
        None
    }
}

fn is_png(p: &Path) -> bool {
    match std::fs::read(p) {
        Ok(b) => b.len() > 8 && b.starts_with(&[0x89, b'P', b'N', b'G']),
        Err(_) => false,
    }
}

fn run_curl(url: &str, dest: &Path) -> bool {
    let out = std::process::Command::new("curl")
        .args([
            "-sS",
            "-L",
            "--max-time",
            "20",
            "-A",
            "Mozilla/5.0",
            "-o",
        ])
        .arg(dest)
        .arg(url)
        .output();
    matches!(out, Ok(o) if o.status.success())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **핵심**: ArcGIS 캐시는 16진수 0채움 경로다.
    ///
    /// 10진수로 요청하면 404 라 "토양도가 안 온다"로 보인다(2026-11-10 실제로 그랬다).
    #[test]
    fn arcgis_path_uses_padded_hex() {
        let u = arcgis_cache_tile_url(13, 7036, 3221);
        assert!(u.ends_with("/L13/R00000c95/C00001b7c.png"), "실제 URL: {u}");
        // 10진수 그대로면 안 된다.
        assert!(!u.contains("/R3221/"));
        assert!(!u.contains("/C7036/"));
    }

    #[test]
    fn arcgis_path_pads_small_numbers() {
        let u = arcgis_cache_tile_url(6, 1, 2);
        assert!(u.contains("/L06/"), "z 는 2자리: {u}");
        // y=2, x=1 → 16진으로도 2, 1 이라 앞자리만 0 으로 채운다.
        // x 는 파일 이름 앞이라 뒤에 `/` 가 없다(`C00000001.png`).
        assert!(u.contains("/R00000002/"), "y 8자리 16진: {u}");
        assert!(u.contains("/C00000001.png"), "x 8자리 16진: {u}");
    }

    /// WFS URL 은 축 순서가 위도·경도다(C++ 과 같다).
    #[test]
    fn wfs_url_swaps_axis_order() {
        // bbox = [lon_min, lat_min, lon_max, lat_max]
        let u = soil_wfs_url(1, [129.0, 35.8, 129.1, 35.9], false);
        assert!(u.contains("bbox=35.80000000,129.00000000,35.90000000,129.10000000"), "{u}");
        assert!(u.contains("typeNames=soilmap:SOIL_1"));
        assert!(u.contains("outputFormat=application/json"));
        assert!(u.contains("soil_type_geo"));
    }

    #[test]
    fn wfs_hits_url_has_no_output_format() {
        let u = soil_wfs_url(2, [0.0, 0.0, 1.0, 1.0], true);
        assert!(u.contains("resultType=hits"));
        assert!(!u.contains("outputFormat"));
        assert!(u.contains("SOIL_2"));
    }

    #[test]
    fn cache_path_is_per_tile() {
        let a = soil_tile_path(13, 7036, 3221);
        let b = soil_tile_path(13, 7036, 3222);
        assert_ne!(a, b);
        assert!(a.to_string_lossy().contains("soil"));
    }

    /// 표 번호는 1~3(C++ 과 같다).
    #[test]
    fn table_range_matches_cpp() {
        assert_eq!(SOIL_TABLE_RANGE, 1..=3);
    }
}
