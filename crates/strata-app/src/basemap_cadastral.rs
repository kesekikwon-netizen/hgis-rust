//! VWorld 연속지적도 WMS 타일 — C++ `BasemapOps.cpp:1189-1212,1344-1349` 1:1.
//!
//! XYZ 타일이 아니라 **WMS GetMap** 이다. C++ 과 같은 조합:
//!   VERSION=1.3.0 · CRS=EPSG:3857 · LAYERS=lp_pa_cbnd_bonbun,lp_pa_cbnd_bubun
//!   FORMAT=image/png · TRANSPARENT=true
//!
//! VERSION=1.1.1(SRS)로 보내면 VWorld 가 `INVALID_RANGE` XML 을 준다(실측).
//! 레이어 이름이 틀리면 `LayerNotDefined` 가 온다.

use std::path::{Path, PathBuf};
use std::process::Command;

/// VWorld 지적 레이어 이름(C++ 과 동일).
pub const CADASTRAL_LAYERS: &str = "lp_pa_cbnd_bonbun,lp_pa_cbnd_bubun";

/// 한 장의 픽셀 크기. C++ `stepWidth/stepHeight=512`.
pub const CADASTRAL_TILE_PX: u32 = 512;

/// 지적도를 그리는 최대 축척 분모. C++ `BasemapOps.cpp:1383`
/// `if (s > 80000.0 || s < 200.0)` — 1:80,000 보다 넓게 보면 지적선을 그리지 않는다.
///
/// 이유: 지적 필지는 1:5,000 안팎에서 읽는 것이고, 넓게 보면 선이 뭉개져
/// 오히려 방해가 된다. 사용자 제안(2026-11-10)도 같은 방향이다.
pub const CADASTRAL_MAX_SCALE_DENOM: f64 = 80_000.0;

/// 지적 레이어를 켤 때 맞추는 축척. C++ `BasemapOps.cpp:1384` `zoomScale(25000.0)`.
#[cfg(test)]
pub const CADASTRAL_DEFAULT_SCALE_DENOM: f64 = 25_000.0;

/// 너무 확대했을 때의 하한. C++ 같은 줄의 `s < 200.0`.
pub const CADASTRAL_MIN_SCALE_DENOM: f64 = 200.0;

/// 지금 축척에서 지적선을 그려야 하는가.
///
/// `scale_denom` 은 화면에 표시되는 축척 분모(1:5000 이면 5000).
/// C++ `addVworldCadastralMap` 과 같은 범위를 쓴다 — 1:200 ~ 1:80,000.
#[must_use]
pub fn cadastral_visible_at_scale(scale_denom: f64) -> bool {
    scale_denom.is_finite()
        && (CADASTRAL_MIN_SCALE_DENOM..=CADASTRAL_MAX_SCALE_DENOM).contains(&scale_denom)
}

/// 지적 레이어를 켤 때 맞출 축척 분모(C++ `zoomScale(25000.0)`).
/// 시험에서만 쓴다 — 앱은 `cadastral_visible_at_scale` 로 그리기만 막는다.
#[cfg(test)]
#[must_use]
pub fn cadastral_target_scale(scale_denom: f64) -> Option<f64> {
    if cadastral_visible_at_scale(scale_denom) {
        None
    } else {
        Some(CADASTRAL_DEFAULT_SCALE_DENOM)
    }
}

/// 지적은 위성과 같은 **웹 메르카토르 z/x/y 타일 격자**로 받는다. 한 장은 512px.
///
/// C++ 은 QGIS 가 WMS 를 화면 해상도(물리 픽셀) 그대로 받아 선·글자가 부드럽다.
/// 예전 Rust 는 1 km 를 512px 한 장으로 고정해, 가까이 볼수록 그 그림을 몇 배로
/// 늘려 붙였다(선이 굵고 흐림). 여기서는 화면 해상도에 가장 가까운 줌을 고른다.
///
/// VWorld 는 요청 1픽셀이 약 2 m(EPSG:3857)보다 거칠면 **빈 그림**을 준다
/// (2026-11-12 실측: 1 km/512px = 1.95 m 는 필지선, z15/512px = 2.39 m 는 6.7 KB 빈 PNG).
/// C++ 은 QGIS 가 화면 해상도로 요청하므로 그보다 넓게 보면 지적이 비어 보인다. 같게 맞춘다.
pub const CADASTRAL_COARSEST_REQUEST_M: f64 = 2.0;

/// 가장 거친 줌. z16/512px = 1.19 m 로 서버가 늘 그린다.
pub const CADASTRAL_MIN_ZOOM: u8 = 16;

/// 가장 가까이 볼 때의 줌(1:200 에서도 그림이 늘어나지 않는 수준).
pub const CADASTRAL_MAX_ZOOM: u8 = 21;

/// 한 화면에 그리는 지적 타일 상한(안전장치). 200% 배율 화면에서
/// 지적이 보이는 가장 넓은 축척일 때 약 85장이 든다.
pub const CADASTRAL_MAX_TILES: usize = 96;

/// 지구 둘레(EPSG:3857 의 x 범위 전체, m).
const WEB_MERCATOR_SPAN: f64 = 40_075_016.685_578_49;

/// 줌 `z` 에서 512px 지적 그림 1픽셀이 덮는 지상 거리(m).
#[must_use]
pub fn cadastral_image_ground_mpp(z: u8, lat_deg: f64) -> f64 {
    WEB_MERCATOR_SPAN * lat_deg.to_radians().cos()
        / (f64::from(CADASTRAL_TILE_PX) * f64::from(1_u32 << z))
}

/// 화면 물리 픽셀 하나가 덮는 지상 거리(m)에 맞는 지적 줌.
/// VWorld 가 빈 그림을 줄 만큼 넓게 보고 있으면 `None`(C++ 에서도 지적이 비어 보인다).
#[must_use]
pub fn cadastral_zoom_for(ground_m_per_px: f64, lat_deg: f64) -> Option<u8> {
    let cos_lat = lat_deg.to_radians().cos();
    if !ground_m_per_px.is_finite() || ground_m_per_px <= 0.0 || cos_lat.is_nan() || cos_lat <= 0.0
    {
        return None;
    }
    if ground_m_per_px / cos_lat > CADASTRAL_COARSEST_REQUEST_M {
        return None;
    }
    let z = (cadastral_image_ground_mpp(0, lat_deg) / ground_m_per_px)
        .log2()
        .round();
    if !z.is_finite() {
        return None;
    }
    Some(z.clamp(f64::from(CADASTRAL_MIN_ZOOM), f64::from(CADASTRAL_MAX_ZOOM)) as u8)
}

/// 타일 `(z, x, y)` 의 EPSG:3857 미터 경계 `(minx, miny, maxx, maxy)`.
/// 이웃 타일과 같은 식으로 경계를 계산하므로 맞닿은 변이 정확히 같다.
#[must_use]
pub fn cadastral_tile_bbox_3857(z: u8, x: u32, y: u32) -> [f64; 4] {
    let half = WEB_MERCATOR_SPAN / 2.0;
    let size = WEB_MERCATOR_SPAN / f64::from(1_u32 << z);
    let east = |i: u32| -half + f64::from(i) * size;
    let north = |j: u32| half - f64::from(j) * size;
    [east(x), north(y + 1), east(x + 1), north(y)]
}

/// 타일 하나의 캐시 경로 `cadastral/<z>/<x>/<y>.png`.
#[must_use]
pub fn cadastral_tile_path(z: u8, x: u32, y: u32) -> PathBuf {
    cadastral_cache_dir()
        .join(z.to_string())
        .join(x.to_string())
        .join(format!("{y}.png"))
}

/// 타일 하나를 받아 캐시 경로를 돌려준다(작업 스레드에서 부른다).
pub fn fetch_cadastral_tile(key: &str, z: u8, x: u32, y: u32) -> Option<PathBuf> {
    if key.trim().is_empty() {
        return None;
    }
    let dest = cadastral_tile_path(z, x, y);
    if dest.is_file() {
        if let Ok(b) = std::fs::read(&dest) {
            if is_png(&b) {
                return Some(dest);
            }
        }
        let _ = std::fs::remove_file(&dest);
    }
    if let Some(dir) = dest.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let bbox = cadastral_tile_bbox_3857(z, x, y);
    let url = cadastral_wms_url(key, bbox, CADASTRAL_TILE_PX);
    if !run_curl(&url, &dest) {
        return None;
    }
    match std::fs::read(&dest) {
        Ok(b) if is_png(&b) => Some(dest),
        _ => {
            let _ = std::fs::remove_file(&dest);
            None
        }
    }
}

/// WMS GetMap URL 을 만든다. `bbox` 는 EPSG:3857 미터 `(minx, miny, maxx, maxy)`.
#[must_use]
pub fn cadastral_wms_url(key: &str, bbox: [f64; 4], size_px: u32) -> String {
    let key = key.trim();
    format!(
        "https://api.vworld.kr/req/wms?KEY={key}\
         &SERVICE=WMS&REQUEST=GetMap&VERSION=1.3.0\
         &LAYERS={CADASTRAL_LAYERS}&STYLES=&FORMAT=image/png\
         &TRANSPARENT=true&CRS=EPSG:3857\
         &WIDTH={size_px}&HEIGHT={size_px}\
         &BBOX={},{},{},{}",
        bbox[0], bbox[1], bbox[2], bbox[3]
    )
}

/// 캐시 폴더.
#[must_use]
pub fn cadastral_cache_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    base.join("ka-hgis").join("tiles").join("cadastral")
}


/// PNG 시그니처 확인. VWorld 는 오류를 XML 로 주는데 상태코드는 200 이다.
#[must_use]
pub fn is_png(bytes: &[u8]) -> bool {
    bytes.len() > 16 && bytes.starts_with(&[0x89, b'P', b'N', b'G'])
}

/// 지적 WMS 받기 curl 명령(주소 없이).
pub(crate) fn cadastral_curl(dest: &Path) -> Command {
    let mut cmd = Command::new("curl");
    cmd.args(["-sS", "-L", "--max-time", "15", "-H", "Referer: https://localhost", "-H", "User-Agent: Mozilla/5.0", "-o"]).arg(dest);
    cmd
}

/// 키(KEY=)가 든 GetMap 주소는 명령줄이 아니라 표준입력으로 준다(SEC1).
fn run_curl(url: &str, dest: &Path) -> bool {
    let out = strata_net::curl_stdin::output_with_url_on_stdin(cadastral_curl(dest), url);
    matches!(out, Ok(o) if o.status.success())
}
