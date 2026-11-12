//! 연속지적도 타일을 **화면을 멈추지 않고** 뒤에서 받는다.
//!
//! 예전에는 그리는 도중(UI 스레드)에서 curl 로 받아, 처음 보는 곳마다 화면이 멈췄고
//! 받기에 실패한 칸은 매 프레임 다시 받아 최대 15초씩 멈출 수 있었다.
//! C++ 은 QGIS 가 WMS 를 뒤에서 받는다. 여기서도 받기·풀기(PNG → 그림)는 작업 스레드가 하고,
//! 화면은 끝난 그림만 텍스처로 올린다.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui;

/// 지적 타일 키 `(z, x, y)`.
pub type CadKey = (u8, u32, u32);

/// 동시에 받는 최대 장수. QGIS 네트워크 관리자와 같은 감각(호스트당 6).
pub const MAX_IN_FLIGHT: usize = 6;

/// 실패한 칸을 다시 받기까지 기다리는 시간.
pub const RETRY_AFTER: Duration = Duration::from_secs(30);

#[derive(Default)]
struct FetchState {
    in_flight: HashSet<CadKey>,
    failed: HashMap<CadKey, Instant>,
    done: Vec<(CadKey, egui::ColorImage)>,
}

/// 지적 타일 받기 작업 관리자. 복제하면 같은 상태를 함께 쓴다.
#[derive(Clone, Default)]
pub struct CadastralFetcher {
    inner: Arc<Mutex<FetchState>>,
}

impl std::fmt::Debug for CadastralFetcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CadastralFetcher").finish_non_exhaustive()
    }
}

impl CadastralFetcher {
    /// 칸 하나를 뒤에서 받기 시작한다. 이미 받는 중이거나, 방금 실패했거나,
    /// 동시 상한에 닿았으면 시작하지 않고 `false`.
    pub fn request_with<F>(&self, key: CadKey, ctx: Option<&egui::Context>, job: F) -> bool
    where
        F: FnOnce() -> Option<egui::ColorImage> + Send + 'static,
    {
        let Ok(mut st) = self.inner.lock() else {
            return false;
        };
        if st.in_flight.contains(&key) || st.in_flight.len() >= MAX_IN_FLIGHT {
            return false;
        }
        if let Some(t) = st.failed.get(&key) {
            if t.elapsed() < RETRY_AFTER {
                return false;
            }
            st.failed.remove(&key);
        }
        st.in_flight.insert(key);
        drop(st);

        let inner = Arc::clone(&self.inner);
        let ctx = ctx.cloned();
        let spawned = std::thread::Builder::new()
            .name("cadastral-tile".to_owned())
            .spawn(move || {
                let result = job();
                if let Ok(mut st) = inner.lock() {
                    st.in_flight.remove(&key);
                    match result {
                        Some(img) => st.done.push((key, img)),
                        None => {
                            st.failed.insert(key, Instant::now());
                        }
                    }
                }
                if let Some(ctx) = ctx {
                    ctx.request_repaint();
                }
            });
        if spawned.is_err() {
            if let Ok(mut st) = self.inner.lock() {
                st.in_flight.remove(&key);
            }
            return false;
        }
        true
    }

    /// 다 받은 그림을 꺼낸다(꺼낸 것은 목록에서 빠진다).
    #[must_use]
    pub fn take_done(&self) -> Vec<(CadKey, egui::ColorImage)> {
        self.inner
            .lock()
            .map(|mut st| std::mem::take(&mut st.done))
            .unwrap_or_default()
    }

    /// 지금 받는 중인지.
    #[must_use]
    pub fn is_in_flight(&self, key: CadKey) -> bool {
        self.inner
            .lock()
            .map(|st| st.in_flight.contains(&key))
            .unwrap_or(false)
    }
}

/// 디스크 캐시나 VWorld 에서 칸 하나를 얻어 그림으로 푼다(작업 스레드에서 부른다).
#[must_use]
pub fn load_cadastral_image(key: &str, z: u8, x: u32, y: u32) -> Option<egui::ColorImage> {
    let path = crate::basemap_cadastral::fetch_cadastral_tile(key, z, x, y)?;
    let decoded = image::open(&path).ok().map(|img| img.to_rgba8());
    let Some(rgba) = decoded else {
        // 깨진 캐시 파일은 지우고 다음에 다시 받는다.
        let _ = std::fs::remove_file(&path);
        return None;
    };
    let (w, h) = (rgba.width() as usize, rgba.height() as usize);
    if w == 0 || h == 0 {
        return None;
    }
    Some(egui::ColorImage::from_rgba_unmultiplied(
        [w, h],
        rgba.as_raw(),
    ))
}
