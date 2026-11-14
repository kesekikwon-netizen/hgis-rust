//! DEM 타일 한 장 내려받기 (C++ `DemDownloadService.cpp` 의 curl `-o`·`-f` 단계).
//!
//! `dem_copernicus` 본문을 300줄 이하로 유지하기 위해 분리했다. 공개 경로는 그대로다.

use super::*;
use std::io::Read;

use std::path::Path;
use std::path::PathBuf;
use std::process::Child;
use std::process::Command;
use std::process::Stdio;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

pub(super) fn curl_binary() -> &'static str {
    if cfg!(target_os = "windows") {
        "curl.exe"
    } else {
        "curl"
    }
}

/// curl 로 타일 한 장을 파일로 받는다. C++ `GDALOpenEx(/vsicurl/…)` 자리.
#[derive(Debug)]
pub(crate) enum TileFetch {
    Saved(PathBuf),
    Missing,
    Cancelled,
    Failed,
}

pub(crate) fn fetch_tile(tile: DemTile, destination: &Path, progress: &DemProgress<'_>) -> TileFetch {
    if progress.cancelled() {
        return TileFetch::Cancelled;
    }
    if !crate::network::is_allowed() {
        return TileFetch::Failed;
    }
    let mut cmd = Command::new(curl_binary());
    cmd.args([
        "-sS",
        // C++ `ThreadConfiguration`(:35-37) — 연결 5초, 응답 없으면 15초 뒤 중단, 재시도 없음
        "--connect-timeout",
        "5",
        "--speed-time",
        "15",
        "--speed-limit",
        "1",
        // 404 를 실패로 만들어 반쪽 파일을 남기지 않는다 (C++ `isMissingTile` 대응)
        "-f",
        "-A",
        "ka-hgis/0.8 (Korean archaeology HGIS; contact: local)",
        "-o",
        &destination.to_string_lossy(),
        "-w",
        "%{http_code}",
        &tile.url(),
    ])
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());
    strata_types::process::no_window(&mut cmd);
    let spawned = crate::network::run_spawn(&mut cmd);
    let Ok(mut child) = spawned else {
        return TileFetch::Failed;
    };
    // curl.exe 는 100ms 간격으로 취소를 살핀다. 자식 프로세스는 파이프가 막히지 않도록
    // stdout 이 작은 `%{http_code}` 하나뿐이다.
    loop {
        if progress.cancelled() {
            kill_child(&mut child);
            let _ = std::fs::remove_file(destination);
            return TileFetch::Cancelled;
        }
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => std::thread::sleep(Duration::from_millis(100)),
            Err(_) => {
                kill_child(&mut child);
                let _ = std::fs::remove_file(destination);
                return TileFetch::Failed;
            }
        }
    }
    let (status, stdout, stderr) = collect_child(&mut child);
    let http_code = String::from_utf8_lossy(&stdout).trim().to_owned();
    if status {
        return TileFetch::Saved(destination.to_path_buf());
    }
    let _ = std::fs::remove_file(destination);
    let diagnostic = format!("{http_code} {}", String::from_utf8_lossy(&stderr));
    if http_code == "404" || is_missing_tile(&diagnostic) {
        TileFetch::Missing
    } else {
        TileFetch::Failed
    }
}

pub(super) fn kill_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

/// `(성공 여부, stdout, stderr)`
pub(super) fn collect_child(child: &mut Child) -> (bool, Vec<u8>, Vec<u8>) {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    if let Some(mut pipe) = child.stdout.take() {
        let _ = pipe.read_to_end(&mut stdout);
    }
    if let Some(mut pipe) = child.stderr.take() {
        let _ = pipe.read_to_end(&mut stderr);
    }
    let success = child.wait().is_ok_and(|status| status.success());
    (success, stdout, stderr)
}

pub(super) fn random_suffix() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    format!("{:06x}", (nanos ^ std::process::id()) & 0x00ff_ffff)
}

/// C++ `ReferenceMapPreparation::initializeStorage` (PreparedReferenceMap.cpp:17-28) —
/// 저장 폴더 옆에 자동 삭제 임시 폴더를 만든다.
pub(super) fn create_temp_storage(target_base_path: &str) -> Option<PathBuf> {
    let base = Path::new(target_base_path);
    let parent = match base.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => dir.to_path_buf(),
        _ => PathBuf::from("."),
    };
    let storage = parent.join(format!("ka-hgis-reference-{}", random_suffix()));
    std::fs::create_dir_all(&storage).ok()?;
    Some(storage)
}

pub(super) fn remove_dir_quietly(path: &Path) {
    let _ = std::fs::remove_dir_all(path);
}
