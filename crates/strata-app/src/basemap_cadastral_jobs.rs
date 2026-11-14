//! Background tile fetchers for VWorld basemap and cadastral layers
//! (target/cpp-ref/src/app/MainWindow.cpp:514,526 — canvas caching and background preview jobs keep UI responsive).

use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::Hash;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use eframe::egui;

use crate::basemap_paint::TileView;
use crate::basemap_tile_key::{is_map_drag_in_progress, ViewRestState, ViewSignature};
use crate::basemap_tiles::TileKind;

pub(crate) use crate::basemap_tile_key::RestDecision;
pub use crate::basemap_tile_key::{load_basemap_image, load_cadastral_image, CachedApiKey};

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

struct FetchState<K> {
    in_flight: HashSet<K>,
    failed: HashMap<K, Instant>,
    done: VecDeque<(K, egui::ColorImage)>,
    generation: u64,
}

impl<K> Default for FetchState<K> {
    fn default() -> Self {
        Self {
            in_flight: HashSet::new(),
            failed: HashMap::new(),
            done: VecDeque::new(),
            generation: 0,
        }
    }
}

/// Completes worker bookkeeping in one place whether the closure returns an image, returns `None`,
/// panics during execution, or fails to spawn.
struct WorkerGuard<K: Copy + Eq + Hash> {
    inner: Arc<(Mutex<FetchState<K>>, Condvar)>,
    total: Arc<AtomicUsize>,
    clock: Arc<dyn Fn() -> Instant + Send + Sync>,
    ctx: Option<egui::Context>,
    key: K,
    generation: u64,
    outcome: Option<egui::ColorImage>,
}

impl<K: Copy + Eq + Hash> Drop for WorkerGuard<K> {
    fn drop(&mut self) {
        let (lock, cvar) = &*self.inner;
        let mut st = lock.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        st.in_flight.remove(&self.key);
        match self.outcome.take() {
            Some(img) => st.done.push_back((self.key, img)),
            None if self.generation == st.generation => {
                st.failed.insert(self.key, (self.clock)());
            }
            None => {}
        }
        self.total.fetch_sub(1, Ordering::AcqRel);
        cvar.notify_all();
        drop(st);
        if let Some(ctx) = &self.ctx {
            ctx.request_repaint();
        }
    }
}

/// Background worker queue generic over tile key `K`, sharing an `AtomicUsize` ceiling.
#[derive(Clone)]
pub struct TileFetcher<K> {
    inner: Arc<(Mutex<FetchState<K>>, Condvar)>,
    in_flight_total: Arc<AtomicUsize>,
    requests_started: Arc<AtomicUsize>,
    view_rest: Arc<Mutex<ViewRestState>>,
    clock: Arc<dyn Fn() -> Instant + Send + Sync>,
}

pub type CadastralFetcher = TileFetcher<CadKey>;
pub type BasemapFetcher = TileFetcher<TexKey>;

/// Creates paired background and cadastral fetchers sharing `MAX_IN_FLIGHT` and view-rest state.
#[must_use]
pub fn shared_tile_fetchers() -> (BasemapFetcher, CadastralFetcher) {
    let total = Arc::new(AtomicUsize::new(0));
    let rest = Arc::new(Mutex::new(ViewRestState::default()));
    let clock: Arc<dyn Fn() -> Instant + Send + Sync> = Arc::new(Instant::now);
    (TileFetcher::new(Arc::clone(&total), Arc::clone(&rest), Arc::clone(&clock)), TileFetcher::new(total, rest, clock))
}

#[cfg(test)]
#[must_use]
pub fn shared_tile_fetchers_with_clock(
    clock: Arc<dyn Fn() -> Instant + Send + Sync>,
) -> (BasemapFetcher, CadastralFetcher) {
    let total = Arc::new(AtomicUsize::new(0));
    let rest = Arc::new(Mutex::new(ViewRestState::default()));
    (TileFetcher::new(Arc::clone(&total), Arc::clone(&rest), Arc::clone(&clock)), TileFetcher::new(total, rest, clock))
}

impl<K> Default for TileFetcher<K> {
    fn default() -> Self {
        Self::new(Arc::new(AtomicUsize::new(0)), Arc::new(Mutex::new(ViewRestState::default())), Arc::new(Instant::now))
    }
}

impl<K> std::fmt::Debug for TileFetcher<K> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TileFetcher").field("in_flight_total", &self.in_flight_total.load(Ordering::Relaxed)).finish_non_exhaustive()
    }
}

impl<K> TileFetcher<K> {
    fn new(
        in_flight_total: Arc<AtomicUsize>,
        view_rest: Arc<Mutex<ViewRestState>>,
        clock: Arc<dyn Fn() -> Instant + Send + Sync>,
    ) -> Self {
        Self {
            inner: Arc::new((Mutex::new(FetchState::default()), Condvar::new())),
            in_flight_total,
            requests_started: Arc::new(AtomicUsize::new(0)),
            view_rest,
            clock,
        }
    }

    #[cfg(test)]
    #[must_use]
    pub fn with_clock(clock: Arc<dyn Fn() -> Instant + Send + Sync>) -> Self {
        Self::new(Arc::new(AtomicUsize::new(0)), Arc::new(Mutex::new(ViewRestState::default())), clock)
    }

    pub(crate) fn observe_view(&self, ctx: &egui::Context, view: &TileView, rect: egui::Rect) -> RestDecision {
        let sig = ViewSignature::from_tile_view(view, rect);
        let dragging = is_map_drag_in_progress(ctx, rect);
        let mut st = self.view_rest.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        st.observe(ctx, sig, dragging, (self.clock)())
    }

    /// Total requests currently in flight across all fetchers sharing this counter.
    #[must_use]
    pub fn in_flight_count(&self) -> usize {
        self.in_flight_total.load(Ordering::Acquire)
    }

    /// Number of tile worker requests started on this fetcher in this run.
    #[must_use]
    pub fn requests_started(&self) -> usize {
        self.requests_started.load(Ordering::Acquire)
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
        let (lock, _) = &*self.inner;
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
        let generation = st.generation;
        st.failed.remove(&key);
        st.in_flight.insert(key);
        drop(st);

        let guard = WorkerGuard {
            inner: Arc::clone(&self.inner),
            total: Arc::clone(&self.in_flight_total),
            clock: Arc::clone(&self.clock),
            ctx: ctx.cloned(),
            key,
            generation,
            outcome: None,
        };
        let spawned = std::thread::Builder::new()
            .name("map-tile".to_owned())
            .spawn(move || {
                // The guard's Drop does the bookkeeping for every way the worker ends. It is moved in and
                // dropped by name: rustc 1.100 rejects a captured value that is only assigned to.
                let mut guard = guard;
                guard.outcome = job();
                drop(guard);
            })
            .is_ok();
        if spawned {
            self.requests_started.fetch_add(1, Ordering::AcqRel);
        }
        spawned
    }

    /// Clears all failed marks and increments `generation` so older in-flight jobs that fail later
    /// do not put their tile back into `RETRY_AFTER`.
    pub fn clear_failed(&self) {
        let (lock, _) = &*self.inner;
        if let Ok(mut st) = lock.lock() {
            st.failed.clear();
            st.generation = st.generation.wrapping_add(1);
        }
    }

    /// If any of `keys` is waiting out `RETRY_AFTER`, returns the shortest remaining wait (`> 0`).
    #[must_use]
    pub fn earliest_retry_delay(&self, keys: impl IntoIterator<Item = K>) -> Option<Duration> {
        let (lock, _) = &*self.inner;
        let st = lock.lock().ok()?;
        if st.failed.is_empty() {
            return None;
        }
        let now = (self.clock)();
        keys.into_iter()
            .filter_map(|k| {
                let &failed_at = st.failed.get(&k)?;
                let elapsed = now.saturating_duration_since(failed_at);
                (elapsed < RETRY_AFTER).then(|| RETRY_AFTER - elapsed)
            })
            .min()
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
