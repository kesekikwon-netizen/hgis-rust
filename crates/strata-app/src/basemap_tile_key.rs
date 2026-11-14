//! Cached VWorld API key and worker-side tile image decoders
//! (target/cpp-ref/src/app/MainWindow.cpp:514,526).

use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui;

use crate::basemap_paint::TileView;
use crate::basemap_tiles::TileKind;

/// Minimum interval between re-reading the VWorld API key from key stores.
pub const KEY_REFRESH_INTERVAL: Duration = Duration::from_secs(2);

/// How long the map view must remain unchanged (and not dragged) before missing tiles are requested.
/// This 120 ms rest time is this Rust app's own number, not a C++ Strata constant
/// (C++ delegates drag/render timing to the map canvas widget; target/cpp-ref/src/app/MainWindow.cpp:528).
pub const VIEW_REST_TIME: Duration = Duration::from_millis(120);

/// Snapshot of the map view geometry compared between frames to detect pan, zoom, or resize.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ViewSignature {
    world_origin: [f64; 2],
    meters_per_px: f64,
    size_px: [f32; 2],
}

impl ViewSignature {
    pub(crate) fn from_tile_view(view: &TileView, rect: egui::Rect) -> Self {
        Self {
            world_origin: view.world_origin,
            meters_per_px: view.meters_per_px,
            size_px: [rect.width(), rect.height()],
        }
    }
}

/// Whether the map view is still changing/settling or ready to start missing tile requests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RestDecision {
    Dragging,
    Waiting(Duration),
    AtRest,
}

/// Tracks across frames whether the map view has been still and undragged for `VIEW_REST_TIME`.
#[derive(Default)]
pub(crate) struct ViewRestState {
    last_ctx_pass: Option<(egui::Context, u64)>,
    last_decision: Option<RestDecision>,
    last_view: Option<ViewSignature>,
    settled_since: Option<Instant>,
}

impl ViewRestState {
    pub(crate) fn observe(
        &mut self,
        ctx: &egui::Context,
        view: ViewSignature,
        dragging: bool,
        now: Instant,
    ) -> RestDecision {
        let pass_nr = ctx.cumulative_pass_nr();
        if let Some((prev_ctx, prev_pass)) = &self.last_ctx_pass {
            if prev_ctx == ctx
                && *prev_pass == pass_nr
                && self.last_view == Some(view)
                && (self.last_decision == Some(RestDecision::Dragging)) == dragging
            {
                if let Some(decision) = self.last_decision {
                    return decision;
                }
            }
        }
        let decision = if dragging {
            self.last_view = Some(view);
            self.settled_since = None;
            RestDecision::Dragging
        } else {
            if self.last_view != Some(view) || self.settled_since.is_none() {
                self.last_view = Some(view);
                self.settled_since = Some(now);
            }
            let since = self.settled_since.unwrap_or(now);
            let elapsed = now.saturating_duration_since(since);
            if elapsed >= VIEW_REST_TIME {
                RestDecision::AtRest
            } else {
                RestDecision::Waiting(VIEW_REST_TIME - elapsed)
            }
        };
        self.last_ctx_pass = Some((ctx.clone(), pass_nr));
        self.last_decision = Some(decision);
        decision
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PressDragLatch {
    origin: egui::Pos2,
    start_time: Option<f64>,
    moved_past_threshold: bool,
    aborted: bool,
}

/// True while a pointer press that started inside `rect` has moved past `InputOptions::max_click_dist`
/// and is still held down (and not aborted with `Escape`).
///
/// `PointerState::could_any_button_be_click` and `is_decidedly_dragging` (`egui-0.31.1/src/input_state/mod.rs:25,33,43-44,1264-1296`)
/// flip after `max_click_duration` (`0.8 s`, for touch long-press menus) even with zero movement,
/// while `has_moved_too_much_for_a_click` (`mod.rs:848,930-933,958-960`) is `pub(crate)`. Latching
/// movement past `max_click_dist` per `(press_origin, press_start_time)` (`mod.rs:838-845,1085-1117,1157-1159,1201-1203`,
/// `src/interaction.rs:137-141`, `src/util/id_type_map.rs:369,385`) ignores stationary holds of any length.
pub(crate) fn is_map_drag_in_progress(ctx: &egui::Context, rect: egui::Rect) -> bool {
    let latch_id = egui::Id::new("ka_map_press_drag_latch");
    let max_click_dist = ctx.options(|o| o.input_options.max_click_dist);
    let prev = ctx.data(|d| d.get_temp::<Option<PressDragLatch>>(latch_id)).flatten();
    let next = ctx.input(|i| {
        if !i.pointer.any_down() {
            return None;
        }
        let origin = i.pointer.press_origin()?;
        let start_time = i.pointer.press_start_time();
        let fresh_press = i.pointer.any_pressed()
            || prev.is_none_or(|p| p.origin != origin || p.start_time != start_time);
        let (mut moved, mut aborted) = match prev {
            Some(p) if !fresh_press => (p.moved_past_threshold, p.aborted),
            _ => (false, false),
        };
        if i.key_pressed(egui::Key::Escape) {
            aborted = true;
        }
        if !moved {
            for ev in i.events.iter().rev() {
                match ev {
                    egui::Event::PointerButton { pressed: true, .. } if fresh_press => break,
                    egui::Event::PointerMoved(pos) if origin.distance(*pos) > max_click_dist => {
                        moved = true;
                        break;
                    }
                    _ => {}
                }
            }
            if !moved {
                if let Some(pos) = i.pointer.interact_pos().or_else(|| i.pointer.latest_pos()) {
                    moved = origin.distance(pos) > max_click_dist;
                }
            }
        }
        Some(PressDragLatch { origin, start_time, moved_past_threshold: moved, aborted })
    });
    if next != prev {
        ctx.data_mut(|d| d.insert_temp(latch_id, next));
    }
    next.is_some_and(|l| rect.contains(l.origin) && l.moved_past_threshold && !l.aborted)
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
    /// Returns the cached VWorld API key (refreshing after `KEY_REFRESH_INTERVAL`) and whether
    /// the key handed out differs from the previous handout.
    pub fn get_with_change(&mut self) -> (&str, bool) {
        let now = (self.clock)();
        let stale = match &self.cached {
            None => true,
            Some((_, read_at)) => now.saturating_duration_since(*read_at) >= KEY_REFRESH_INTERVAL,
        };
        let mut changed = false;
        if stale {
            let next = (self.reader)();
            changed = self.cached.as_ref().is_some_and(|(prev, _)| *prev != next);
            self.cached = Some((next, now));
        }
        (self.cached.as_ref().map_or("", |(k, _)| k.as_str()), changed)
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
