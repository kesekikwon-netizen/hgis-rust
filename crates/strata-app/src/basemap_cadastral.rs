//! VWorld 연속지적도 WMS 타일 — C++ `BasemapOps.cpp:1189-1212,1344-1349` 1:1.
//!
//! XYZ 타일이 아니라 **WMS GetMap** 이다. C++ 과 같은 조합:
//!   VERSION=1.3.0 · CRS=EPSG:3857 · LAYERS=lp_pa_cbnd_bonbun,lp_pa_cbnd_bubun
//!   FORMAT=image/png · TRANSPARENT=true
//!
//! VERSION=1.1.1(SRS)로 보내면 VWorld 가 `INVALID_RANGE` XML 을 준다(실측).
//! 레이어 이름이 틀리면 `LayerNotDefined` 가 온다.

use std::path::PathBuf;

/// VWorld 지적 레이어 이름(C++ 과 동일).
pub const CADASTRAL_LAYERS: &str = "lp_pa_cbnd_bonbun,lp_pa_cbnd_bubun";


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



/// PNG 시그니처 확인. VWorld 는 오류를 XML 로 주는데 상태코드는 200 이다.
#[must_use]
/// 지적 캐시 폴더. 화면 한 장 파일이 여기 쌓인다.
pub fn cadastral_cache_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    base.join("ka-hgis").join("tiles").join("cadastral")
}

pub fn is_png(bytes: &[u8]) -> bool {
    bytes.len() > 16 && bytes.starts_with(&[0x89, b'P', b'N', b'G'])
}

