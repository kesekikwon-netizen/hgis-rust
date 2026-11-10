//! 배경지도 타일을 실제로 내려받아 디스크에 캐시한다.
//!
//! C++ 은 QGIS 가 타일을 받아 그리지만 Rust 는 직접 받아야 한다.
//! `curl` 을 쓴다(이 저장소의 다른 HTTP 코드와 같은 방식 — `heritage_http` 참조).
//! 받은 파일은 `%LOCALAPPDATA%\ka-hgis\tiles\<종류>\<z>\<x>\<y>.<ext>` 에 캐시한다.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::basemap_tiles::{tile_url, TileKind};

/// 캐시 폴더(없으면 만든다).
#[must_use]
pub fn tile_cache_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    base.join("ka-hgis").join("tiles")
}

/// 현재 VWorld 키(설정에서 읽는다).
#[must_use]
pub fn current_key() -> String {
    strata_secrets::vworld::VworldSettings::load_api_key()
}

/// 텍스처 이름에 쓸 짧은 종류 키.
#[must_use]
pub fn tile_filename_key(kind: TileKind) -> &'static str {
    kind_dir(kind)
}

fn kind_dir(kind: TileKind) -> &'static str {
    match kind {
        TileKind::Satellite => "satellite",
        TileKind::Base => "base",
        TileKind::Cadastral => "cadastral",
    }
}

fn ext_for(kind: TileKind) -> &'static str {
    match kind {
        TileKind::Satellite => "jpeg",
        TileKind::Base => "png",
        TileKind::Cadastral => "png",
    }
}

/// 캐시 경로.
#[must_use]
pub fn tile_cache_path(kind: TileKind, z: u8, x: u32, y: u32) -> PathBuf {
    tile_cache_dir()
        .join(kind_dir(kind))
        .join(z.to_string())
        .join(x.to_string())
        .join(format!("{y}.{}", ext_for(kind)))
}

/// 파일이 진짜 이미지인지 앞 2~4 바이트로 확인한다.
/// VWorld 는 키가 만료되면 HTTP 200 + XML 오류를 주므로 이 검사가 필수다.
#[must_use]
pub fn looks_like_image(bytes: &[u8]) -> bool {
    bytes.len() > 16
        && (bytes.starts_with(&[0xFF, 0xD8])          // JPEG
            || bytes.starts_with(&[0x89, b'P', b'N', b'G']) // PNG
            || bytes.starts_with(b"GIF8")
            || (bytes.starts_with(b"RIFF") && bytes.len() > 12 && &bytes[8..12] == b"WEBP"))
}

/// 캐시에 이미 있으면 그 경로를, 없으면 `None`.
#[must_use]
pub fn cached_tile(kind: TileKind, z: u8, x: u32, y: u32) -> Option<PathBuf> {
    let p = tile_cache_path(kind, z, x, y);
    if p.is_file() {
        // 크기가 0 이거나 XML 오류가 저장돼 있으면 버린다.
        if let Ok(meta) = std::fs::metadata(&p) {
            if meta.len() > 16 {
                return Some(p);
            }
        }
        let _ = std::fs::remove_file(&p);
    }
    None
}

/// 타일 한 장을 내려받아 캐시에 저장하고 경로를 돌려준다.
///
/// 네트워크가 없거나 키가 만료되면 `None`. 화면은 그 타일만 비고 나머지는 그려진다.
pub fn fetch_tile(kind: TileKind, key: &str, z: u8, x: u32, y: u32) -> Option<PathBuf> {
    if !kind.is_xyz() {
        return None;
    }
    if let Some(hit) = cached_tile(kind, z, x, y) {
        return Some(hit);
    }
    let url = tile_url(kind, key, z, x, y);
    if url.is_empty() {
        return None;
    }
    let dest = tile_cache_path(kind, z, x, y);
    if let Some(dir) = dest.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let ok = run_curl(&url, &dest);
    if !ok {
        return None;
    }
    // 내용 검증 — XML 오류를 이미지로 착각하지 않는다.
    match std::fs::read(&dest) {
        Ok(bytes) if looks_like_image(&bytes) => Some(dest),
        _ => {
            let _ = std::fs::remove_file(&dest);
            None
        }
    }
}

/// curl 로 받아 파일로 쓴다. VWorld 는 Referer 를 인증에 쓰므로 반드시 보낸다.
fn run_curl(url: &str, dest: &Path) -> bool {
    let out = Command::new("curl")
        .args([
            "-sS",
            "-L",
            "--max-time",
            "12",
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
