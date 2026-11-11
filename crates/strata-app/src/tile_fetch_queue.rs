//! 배경 타일을 화면 그리기 밖(뒤의 스레드)에서 받는다.
//!
//! 예전에는 지도를 그리는 도중에 타일 한 장씩 `curl` 이 끝나기를 기다렸다. 새 타일이
//! 필요할 때마다, 인터넷이 느리면 장당 최대 12~15초씩 화면이 멈췄다. C++ 은 QGIS 가
//! 타일을 비동기로 받고 준비된 것만 그린다. 여기서도 같게 한다:
//! - 캐시에 있으면 바로 그린다.
//! - 없으면 뒤에서 받기 시작하고 이번 프레임은 그 칸을 비워 둔다. 다 받으면 다시 그린다.
//! - 동시에 6개까지만 받는다(C++ 타일 동시 요청 수와 같다).
//! - 실패한 칸은 30초 동안 다시 시도하지 않는다(매 프레임 재시도 방지).

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use eframe::egui;

/// 동시에 받는 수.
const MAX_IN_FLIGHT: usize = 6;
/// 실패한 칸을 다시 시도하기까지.
const RETRY_AFTER: Duration = Duration::from_secs(30);

/// 타일 한 칸의 상태.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TileState {
    /// 받아 둔 파일이 있다.
    Ready(PathBuf),
    /// 받는 중이거나 순서를 기다린다.
    Loading,
    /// 받지 못했다(잠시 뒤 다시 시도).
    Failed,
}

#[derive(Default)]
struct Jobs {
    in_flight: HashSet<String>,
    failed: HashMap<String, Instant>,
}

fn jobs() -> &'static Mutex<Jobs> {
    static JOBS: OnceLock<Mutex<Jobs>> = OnceLock::new();
    JOBS.get_or_init(|| Mutex::new(Jobs::default()))
}

/// 타일(또는 지적 그림 칸) 하나를 요청한다.
///
/// `cached` 는 이미 받아 둔 파일을 찾는 함수, `fetch` 는 실제로 받는(느린) 함수다.
/// `fetch` 는 뒤의 스레드에서 돈다. 시험 중에는 네트워크를 쓰지 않는다.
pub fn request_tile(
    ctx: &egui::Context,
    id: String,
    cached: impl FnOnce() -> Option<PathBuf>,
    fetch: impl FnOnce() -> Option<PathBuf> + Send + 'static,
) -> TileState {
    if let Some(path) = cached() {
        return TileState::Ready(path);
    }
    if cfg!(test) {
        return TileState::Failed;
    }
    let Ok(mut state) = jobs().lock() else {
        return TileState::Failed;
    };
    if let Some(when) = state.failed.get(&id) {
        if when.elapsed() < RETRY_AFTER {
            return TileState::Failed;
        }
        state.failed.remove(&id);
    }
    if state.in_flight.contains(&id) || state.in_flight.len() >= MAX_IN_FLIGHT {
        // 자리가 나면 다음 프레임에 시작한다.
        ctx.request_repaint_after(Duration::from_millis(150));
        return TileState::Loading;
    }
    state.in_flight.insert(id.clone());
    drop(state);
    let ctx = ctx.clone();
    std::thread::spawn(move || {
        let ok = fetch().is_some();
        if let Ok(mut state) = jobs().lock() {
            state.in_flight.remove(&id);
            if !ok {
                state.failed.insert(id, Instant::now());
            }
        }
        ctx.request_repaint();
    });
    TileState::Loading
}

/// 지금 받고 있는 수(시험용).
#[cfg(test)]
fn in_flight_count() -> usize {
    jobs().lock().map_or(0, |s| s.in_flight.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cached_tile_is_ready_at_once_and_a_missing_one_never_blocks() {
        let ctx = egui::Context::default();
        let path = PathBuf::from("cached.png");
        let hit = request_tile(&ctx, "a".to_owned(), || Some(path.clone()), || None);
        assert_eq!(hit, TileState::Ready(path));
        // 캐시에 없으면 받기를 기다리지 않고 바로 돌아온다(시험 중에는 네트워크를 쓰지 않는다).
        let started = Instant::now();
        let miss = request_tile(&ctx, "b".to_owned(), || None, || {
            std::thread::sleep(Duration::from_secs(5));
            None
        });
        assert_eq!(miss, TileState::Failed);
        assert!(started.elapsed() < Duration::from_secs(1));
        assert_eq!(in_flight_count(), 0);
    }
}
