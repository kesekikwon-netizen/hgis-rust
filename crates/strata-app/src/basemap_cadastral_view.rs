//! 지적도 WMS를 **화면 한 장으로** 받는다. C++ 과 같은 방식.
//!
//! ## 왜 고쳤나
//!
//! 예전에는 화면을 격자 칸(수백 m)으로 나눠 칸마다 WMS를 따로 받았다.
//! 그러면 1) 칸 경계에서 필지선이 잘리고 2) 칸마다 선 굵기가 달라 이어지지 않으며
//! 3) 인접 칸이 겹쳐 선이 두꺼워진다. 화면에서 "지적도가 격자처럼 보이는" 원인이었다.
//!
//! C++ `BasemapOps.cpp` `addVworldCadastralMap` 은 QGIS XYZ/WMS 레이어 하나로
//! **현재 화면 영역 전체**를 한 번에 그린다. 여기서도 그렇게 한다.

use std::path::{Path, PathBuf};



/// 화면 한 장의 캐시 파일 경로. bbox·크기를 키로 쓴다.
///
/// 같은 영역을 다시 볼 때는 파일을 재사용하고, 조금이라도 움직이면 새로 받는다
/// (C++ 도 화면이 바뀌면 다시 그린다).
#[must_use]
pub fn cadastral_view_cache_path(bbox: [f64; 4], size_px: u32) -> PathBuf {
    // 좌표를 1m 단위로 반올림해 키를 만든다 — 1px 흔들림마다 새로 받지 않게.
    let k = format!(
        "{}_{}_{}_{}_{}",
        bbox[0].round() as i64,
        bbox[1].round() as i64,
        bbox[2].round() as i64,
        bbox[3].round() as i64,
        size_px
    );
    crate::basemap_cadastral::cadastral_cache_dir()
        .join("view")
        .join(format!("{k}.png"))
}

/// 화면 영역 한 장을 받아 온다. 캐시에 있으면 그대로 쓴다.
///
/// 실패하면 `None`(지적선만 안 그려지고 나머지 지도는 그대로).
pub fn fetch_cadastral_view(key: &str, bbox: [f64; 4], size_px: u32) -> Option<PathBuf> {
    if key.trim().is_empty() || size_px == 0 {
        return None;
    }
    if bbox[2] <= bbox[0] || bbox[3] <= bbox[1] {
        return None;
    }
    let dest = cadastral_view_cache_path(bbox, size_px);
    if is_valid_png(&dest) {
        return Some(dest);
    }
    if let Some(dir) = dest.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let url = crate::basemap_cadastral::cadastral_wms_url(key, bbox, size_px);
    if !run_curl(&url, &dest) {
        return None;
    }
    if is_valid_png(&dest) {
        Some(dest)
    } else {
        let _ = std::fs::remove_file(&dest);
        None
    }
}

/// 캐시 파일이 쓸 만한 PNG 인지.
///
/// **완전히 투명한 PNG 는 버린다.** VWorld 는 데이터가 없는 영역에도
/// HTTP 200 + 빈(투명) PNG 를 주는데, 그걸 "성공"으로 캐시하면 그 영역은
/// 영원히 빈 채로 남는다. 2026-11-10 화면에서 지적선이 조사구역 안쪽에만
/// 보이던 원인이 이것이었다(먼저 받은 빈 이미지가 재사용됨).
fn is_valid_png(p: &Path) -> bool {
    let Ok(b) = std::fs::read(p) else {
        return false;
    };
    if !crate::basemap_cadastral::is_png(&b) {
        return false;
    }
    !is_fully_transparent(&b)
}

/// PNG 가 **거의 투명한지** 검사한다.
///
/// VWorld 는 데이터가 없는 영역에도 HTTP 200 + 거의 투명한 PNG 를 준다.
/// 그걸 "성공"으로 캐시하면 그 영역은 영원히 빈 채로 남는다.
///
/// 판정: 불투명(알파 80 초과) 픽셀이 전체의 **0.05% 미만**이면 버린다.
/// 실제 지적도는 띠·선이 촘촘해 그보다 훨씬 많다(보통 1% 이상).
fn is_fully_transparent(bytes: &[u8]) -> bool {
    // 60KB 이상이면 지적선이 있다고 본다(디코드 비용을 아낀다).
    const SMALL: usize = 60 * 1024;
    if bytes.len() >= SMALL {
        return false;
    }
    let Ok(img) = image::load_from_memory_with_format(bytes, image::ImageFormat::Png) else {
        return false; // 디코드 실패는 is_png 가 이미 걸렀다
    };
    let rgba = img.to_rgba8();
    let total = rgba.pixels().len();
    if total == 0 {
        return true;
    }
    let opaque = rgba.pixels().filter(|p| p.0[3] > 80).count();
    // 0.05% 미만이면 사실상 빈 이미지.
    (opaque as f64) / (total as f64) < 0.0005
}

/// curl 로 받아 파일로 쓴다.
///
/// `Referer` 는 **필수**이고 `DOMAIN` 은 넣지 않는다
/// (C++ `BasemapOps.cpp:1197-1200` 주석: DOMAIN 을 넣으면 INCORRECT_KEY).
fn run_curl(url: &str, dest: &Path) -> bool {
    let out = std::process::Command::new("curl")
        .args([
            "-sS",
            "-L",
            "--max-time",
            "20",
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

/// 화면 크기에 맞는 WMS 이미지 크기를 고른다.
///
/// 너무 크면 느리고, 작으면 선이 뭉갠다. 512~2048 사이로 맞춘다.
#[must_use]
pub fn cadastral_view_size(rect_w: f32, rect_h: f32) -> u32 {
    let longest = rect_w.max(rect_h).max(1.0);
    // 2의 거듭제곱으로 올림해 캐시 키를 안정시킨다.
    let mut n = 512u32;
    while (n as f32) < longest && n < 2048 {
        n *= 2;
    }
    n
}


/// 화면 한 장 지적 텍스처 캐시.
///
/// `TextureHandle` 은 `Debug` 를 구현하지 않으므로 직접 구현한다
/// (기존 `CadastralCellCache` 와 같은 방식).
#[derive(Default, Clone)]
pub struct CadastralViewCache(pub std::collections::HashMap<String, eframe::egui::TextureHandle>);

impl std::fmt::Debug for CadastralViewCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CadastralViewCache")
            .field("len", &self.0.len())
            .finish()
    }
}

impl CadastralViewCache {
    /// 캐시에서 꺼낸다.
    #[must_use]
    pub fn get(&self, k: &str) -> Option<&eframe::egui::TextureHandle> {
        self.0.get(k)
    }

    /// 넣는다.
    pub fn insert(&mut self, k: String, v: eframe::egui::TextureHandle) {
        self.0.insert(k, v);
    }

    /// 비운다(지도를 크게 움직였을 때 오래된 것 정리).
    pub fn clear(&mut self) {
        self.0.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn view_cache_path_is_stable_for_same_bbox() {
        let b = [14_385_000.0, 4_274_000.0, 14_386_000.0, 4_275_000.0];
        assert_eq!(
            cadastral_view_cache_path(b, 1024),
            cadastral_view_cache_path(b, 1024),
            "같은 영역은 같은 파일"
        );
    }

    #[test]
    fn view_cache_path_changes_with_bbox() {
        let a = [14_385_000.0, 4_274_000.0, 14_386_000.0, 4_275_000.0];
        let b = [14_385_500.0, 4_274_000.0, 14_386_500.0, 4_275_000.0];
        assert_ne!(
            cadastral_view_cache_path(a, 1024),
            cadastral_view_cache_path(b, 1024),
            "다른 영역은 다른 파일"
        );
    }

    /// 1m 미만 흔들림은 같은 키로 본다 — 지도를 조금 움직일 때마다 다시 받지 않게.
    #[test]
    fn sub_meter_jitter_keeps_the_same_key() {
        let a = [14_385_000.4, 4_274_000.4, 14_386_000.4, 4_275_000.4];
        let b = [14_385_000.0, 4_274_000.0, 14_386_000.0, 4_275_000.0];
        assert_eq!(cadastral_view_cache_path(a, 512), cadastral_view_cache_path(b, 512));
    }

    #[test]
    fn size_is_power_of_two_and_bounded() {
        assert_eq!(cadastral_view_size(100.0, 100.0), 512, "작으면 512");
        assert_eq!(cadastral_view_size(600.0, 300.0), 1024, "600 넘으면 1024");
        assert_eq!(cadastral_view_size(1200.0, 800.0), 2048, "1200 넘으면 2048");
        assert_eq!(cadastral_view_size(5000.0, 5000.0), 2048, "상한 2048");
    }

    #[test]
    fn empty_key_or_zero_size_returns_none() {
        let b = [0.0, 0.0, 100.0, 100.0];
        assert!(fetch_cadastral_view("", b, 512).is_none());
        assert!(fetch_cadastral_view("k", b, 0).is_none());
        // 뒤집힌 bbox 도 거른다
        assert!(fetch_cadastral_view("k", [100.0, 100.0, 0.0, 0.0], 512).is_none());
    }
}
