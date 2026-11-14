//! Background tile fetchers and cached API key for VWorld basemap and cadastral layers
//! (target/cpp-ref/src/app/MainWindow.cpp:514,526 — canvas caching and background preview jobs keep UI responsive).

use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::Hash;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use eframe::egui;

use crate::basemap_tiles::TileKind;

/// Cadastral WMS tile key `(z, x, y)`.
pub type CadKey = (u8, u32, u32);

/// Background XYZ tile key `(kind, z, x, y)`.
pub type TexKey = (TileKind, u8, u32, u32);

/// Shared ceiling of concurrent requests to the map service across cadastral and background tiles.
pub const MAX_IN_FLIGHT: usize = 6;

/// Cooldown before retrying a tile whose download or image decode failed.
pub const RETRY_AFTER: Duration = Duration::from_secs(30);

/// Maximum finished tile images converted into GPU textures per UI frame.
pub const MAX_UPLOADS_PER_FRAME: usize = 8;

/// Minimum interval between re-reading the VWorld API key from key stores.
pub const KEY_REFRESH_INTERVAL: Duration = Duration::from_secs(2);

struct FetchState<K> {
    in_flight: HashSet<K>,
    failed: HashMap<K, Instant>,
    done: VecDeque<(K, egui::ColorImage)>,
}

impl<K> Default for FetchState<K> {
    fn default() -> Self {
        Self {
            in_flight: HashSet::new(),
            failed: HashMap::new(),
            done: VecDeque::new(),
        }
    }
}

/// Background worker queue generic over tile key `K`, sharing an optional `AtomicUsize` ceiling.
#[derive(Clone)]
pub struct TileFetcher<K> {
    inner: Arc<(Mutex<FetchState<K>>, Condvar)>,
    in_flight_total: Arc<AtomicUsize>,
    clock: Arc<dyn Fn() -> Instant + Send + Sync>,
}

pub type CadastralFetcher = TileFetcher<CadKey>;
pub type BasemapFetcher = TileFetcher<TexKey>;

/// Creates paired background and cadastral fetchers sharing one `MAX_IN_FLIGHT` counter.
#[must_use]
pub fn shared_tile_fetchers() -> (BasemapFetcher, CadastralFetcher) {
    let total = Arc::new(AtomicUsize::new(0));
    let clock: Arc<dyn Fn() -> Instant + Send + Sync> = Arc::new(Instant::now);
    (
        TileFetcher::new(Arc::clone(&total), Arc::clone(&clock)),
        TileFetcher::new(total, clock),
    )
}

impl<K> Default for TileFetcher<K> {
    fn default() -> Self {
        Self::new(Arc::new(AtomicUsize::new(0)), Arc::new(Instant::now))
    }
}

impl<K> std::fmt::Debug for TileFetcher<K> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TileFetcher")
            .field("in_flight_total", &self.in_flight_total.load(Ordering::Relaxed))
            .finish_non_exhaustive()
    }
}

impl<K> TileFetcher<K> {
    fn new(
        in_flight_total: Arc<AtomicUsize>,
        clock: Arc<dyn Fn() -> Instant + Send + Sync>,
    ) -> Self {
        Self {
            inner: Arc::new((Mutex::new(FetchState::default()), Condvar::new())),
            in_flight_total,
            clock,
        }
    }

    #[cfg(test)]
    #[must_use]
    pub fn with_clock(clock: Arc<dyn Fn() -> Instant + Send + Sync>) -> Self {
        Self::new(Arc::new(AtomicUsize::new(0)), clock)
    }

    /// Total requests currently in flight across all fetchers sharing this counter.
    #[must_use]
    pub fn in_flight_count(&self) -> usize {
        self.in_flight_total.load(Ordering::Acquire)
    }

    #[cfg(test)]
    pub fn wait_for_idle(&self) {
        let (lock, cvar) = &*self.inner;
        if let Ok(guard) = lock.lock() {
            drop(cvar.wait_while(guard, |st| !st.in_flight.is_empty()));
        }
    }
}

impl<K> TileFetcher<K>
where
    K: Copy + Eq + Hash + Send + 'static,
{
    /// Starts `job` on a worker thread unless `key` is already in flight/queued, within `RETRY_AFTER`,
    /// or the shared `MAX_IN_FLIGHT` ceiling is full.
    pub fn request_with<F>(&self, key: K, ctx: Option<&egui::Context>, job: F) -> bool
    where
        F: FnOnce() -> Option<egui::ColorImage> + Send + 'static,
    {
        let (lock, cvar) = &*self.inner;
        let Ok(mut st) = lock.lock() else {
            return false;
        };
        if st.in_flight.contains(&key) || st.done.iter().any(|(dk, _)| *dk == key) {
            return false;
        }
        let now = (self.clock)();
        if let Some(&failed_at) = st.failed.get(&key) {
            if now.saturating_duration_since(failed_at) < RETRY_AFTER {
                return false;
            }
        }
        // A plain compare-and-swap loop: `fetch_update` is deprecated from rustc 1.100 (CI runs the newest
        // stable with -D warnings) and its replacement `try_update` does not exist on rustc 1.91.
        let mut seen = self.in_flight_total.load(Ordering::Acquire);
        let reserved = loop {
            if seen >= MAX_IN_FLIGHT {
                break false;
            }
            match self.in_flight_total.compare_exchange_weak(seen, seen + 1, Ordering::AcqRel, Ordering::Acquire) {
                Ok(_) => break true,
                Err(now) => seen = now,
            }
        };
        if !reserved {
            return false;
        }
        st.failed.remove(&key);
        st.in_flight.insert(key);
        drop(st);

        let inner = Arc::clone(&self.inner);
        let total = Arc::clone(&self.in_flight_total);
        let clock = Arc::clone(&self.clock);
        let ctx = ctx.cloned();
        let spawned = std::thread::Builder::new()
            .name("map-tile".to_owned())
            .spawn(move || {
                let result = job();
                let (lock, cvar) = &*inner;
                let mut st = lock.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                st.in_flight.remove(&key);
                match result {
                    Some(img) => st.done.push_back((key, img)),
                    None => {
                        st.failed.insert(key, (clock)());
                    }
                }
                total.fetch_sub(1, Ordering::AcqRel);
                cvar.notify_all();
                drop(st);
                if let Some(ctx) = ctx {
                    ctx.request_repaint();
                }
            });
        if spawned.is_err() {
            let mut st = lock.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            st.in_flight.remove(&key);
            self.in_flight_total.fetch_sub(1, Ordering::AcqRel);
            cvar.notify_all();
            return false;
        }
        true
    }

    /// Drains all finished images.
    #[must_use]
    pub fn take_done(&self) -> Vec<(K, egui::ColorImage)> {
        let (lock, _) = &*self.inner;
        lock.lock()
            .map(|mut st| st.done.drain(..).collect())
            .unwrap_or_default()
    }

    /// Drains up to `limit` finished images and reports whether more remain queued.
    #[must_use]
    pub fn take_done_limited(&self, limit: usize) -> (Vec<(K, egui::ColorImage)>, bool) {
        let (lock, _) = &*self.inner;
        let Ok(mut st) = lock.lock() else {
            return (Vec::new(), false);
        };
        let n = st.done.len().min(limit);
        let out = st.done.drain(..n).collect();
        (out, !st.done.is_empty())
    }

    /// Returns whether `key` is currently being fetched.
    #[must_use]
    pub fn is_in_flight(&self, key: K) -> bool {
        let (lock, _) = &*self.inner;
        lock.lock().is_ok_and(|st| st.in_flight.contains(&key))
    }
}

/// Keeps the VWorld API key in app state and re-reads key stores at most every `KEY_REFRESH_INTERVAL`.
#[derive(Clone)]
pub struct CachedApiKey {
    cached: Option<(String, Instant)>,
    reader: Arc<dyn Fn() -> String + Send + Sync>,
    clock: Arc<dyn Fn() -> Instant + Send + Sync>,
}

impl Default for CachedApiKey {
    fn default() -> Self {
        Self {
            cached: None,
            reader: Arc::new(crate::basemap_fetch::current_key),
            clock: Arc::new(Instant::now),
        }
    }
}

impl std::fmt::Debug for CachedApiKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CachedApiKey")
            .field("has_cached", &self.cached.is_some())
            .finish_non_exhaustive()
    }
}

impl CachedApiKey {
    pub fn get(&mut self) -> &str {
        let now = (self.clock)();
        let stale = match &self.cached {
            None => true,
            Some((_, read_at)) => now.saturating_duration_since(*read_at) >= KEY_REFRESH_INTERVAL,
        };
        if stale {
            self.cached = Some(((self.reader)(), now));
        }
        self.cached.as_ref().map_or("", |(k, _)| k.as_str())
    }

    #[cfg(test)]
    #[must_use]
    pub fn with_reader_and_clock(
        reader: Arc<dyn Fn() -> String + Send + Sync>,
        clock: Arc<dyn Fn() -> Instant + Send + Sync>,
    ) -> Self {
        Self { cached: None, reader, clock }
    }
}

/// Fetches and decodes one cadastral WMS tile on a worker thread.
#[must_use]
pub fn load_cadastral_image(key: &str, z: u8, x: u32, y: u32) -> Option<egui::ColorImage> {
    let path = crate::basemap_cadastral::fetch_cadastral_tile(key, z, x, y)?;
    decode_cached_tile_image(&path)
}

/// Fetches and decodes one satellite/base XYZ tile on a worker thread.
#[must_use]
pub fn load_basemap_image(kind: TileKind, key: &str, z: u8, x: u32, y: u32) -> Option<egui::ColorImage> {
    let path = crate::basemap_fetch::fetch_tile(kind, key, z, x, y)?;
    decode_cached_tile_image(&path)
}

fn decode_cached_tile_image(path: &std::path::Path) -> Option<egui::ColorImage> {
    let decoded = image::open(path).ok().map(|img| img.to_rgba8());
    let Some(rgba) = decoded else {
        let _ = std::fs::remove_file(path);
        return None;
    };
    let (w, h) = (rgba.width() as usize, rgba.height() as usize);
    if w == 0 || h == 0 {
        let _ = std::fs::remove_file(path);
        return None;
    }
    Some(egui::ColorImage::from_rgba_unmultiplied([w, h], rgba.as_raw()))
}
