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

/// 지적 타일 격자 한 칸의 EPSG:3857 미터 크기.
///
/// 위성은 XYZ 타일(줌별 격자)을 쓰지만 지적 WMS 는 줌 개념이 없어 경계를
/// 직접 준다. 화면이 흔들려도 같은 칸을 재사용하도록 **고정 격자**를 쓴다.
/// 512px 짜리 한 장이 1km 를 덮게 하면, 확대해도 선명하고 이동해도 캐시가 산다.
pub const CADASTRAL_CELL_M: f64 = 1000.0;

/// 화면 영역이 걸치는 지적 타일 격자 범위 `(x0, y0, x1, y1)` (칸 번호).
#[must_use]
pub fn cadastral_grid_range(bbox: [f64; 4]) -> (i64, i64, i64, i64) {
    (
        (bbox[0] / CADASTRAL_CELL_M).floor() as i64,
        (bbox[1] / CADASTRAL_CELL_M).floor() as i64,
        (bbox[2] / CADASTRAL_CELL_M).floor() as i64,
        (bbox[3] / CADASTRAL_CELL_M).floor() as i64,
    )
}

/// 칸 번호 → EPSG:3857 미터 경계.
#[must_use]
pub fn cadastral_cell_bbox(cx: i64, cy: i64) -> [f64; 4] {
    [
        cx as f64 * CADASTRAL_CELL_M,
        cy as f64 * CADASTRAL_CELL_M,
        (cx + 1) as f64 * CADASTRAL_CELL_M,
        (cy + 1) as f64 * CADASTRAL_CELL_M,
    ]
}

/// 한 칸의 캐시 경로.
#[must_use]
pub fn cadastral_cell_path(cx: i64, cy: i64) -> PathBuf {
    cadastral_cache_dir().join(format!("{cx}_{cy}.png"))
}

/// 이미 받아 둔 칸이 있으면 그 경로(PNG 인지 확인한다).
#[must_use]
pub fn cached_cadastral_cell(cx: i64, cy: i64) -> Option<PathBuf> {
    let dest = cadastral_cell_path(cx, cy);
    std::fs::read(&dest).ok().filter(|b| is_png(b)).map(|_| dest)
}

/// 한 칸을 받아 캐시 경로를 돌려준다.
pub fn fetch_cadastral_cell(key: &str, cx: i64, cy: i64) -> Option<PathBuf> {
    if key.trim().is_empty() {
        return None;
    }
    let dest = cadastral_cell_path(cx, cy);
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
    let bbox = cadastral_cell_bbox(cx, cy);
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

fn run_curl(url: &str, dest: &Path) -> bool {
    let out = Command::new("curl")
        .args([
            "-sS",
            "-L",
            "--max-time",
            "15",
            "-H",
            "Referer: https://localhost",
            "-H",
            "User-Agent: Mozilla/5.0",
            "-o",
        ])
        .arg(dest)
        .arg(url)
        .output();
    matches!(out, Ok(o) if o.status.success())
}
