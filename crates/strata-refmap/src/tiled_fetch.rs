//! One large request range fetched piece by piece.
//!
//! C++ `ReferenceTiledFetch` (`target/cpp-ref/src/core/ReferenceTiledFetch.h`, `.cpp`): the whole range is
//! tried first; a piece that is too large or too slow is split into four quarters, a dropped connection is
//! tried once more, and nothing is returned unless every piece completed, so that the caller keeps the map
//! it has on any failure.

use std::time::{Duration, Instant};

use crate::collection::merge_feature_collections;

#[cfg(test)]
#[path = "tiled_fetch_tests.rs"]
mod tests;

/// `[min x, min y, max x, max y]`, here longitude and latitude.
pub type Range = [f64; 4];

/// How the request for one piece ended (`ReferenceTiledFetch.h:18-25`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The body holds this piece's complete FeatureCollection.
    Complete,
    /// The user stopped the download.
    Cancelled,
    /// The server answered with something unusable; splitting cannot help.
    Fatal,
    /// More features than one request may carry: split, never repeat.
    TooLarge,
    /// The total deadline was reached while data was still arriving: split, repeat at the finest level.
    Slow,
    /// The connection dropped or went silent: repeat the same piece once.
    Transient,
}

/// The answer for one piece.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Piece {
    pub outcome: Outcome,
    pub body: Vec<u8>,
    pub error: String,
}

/// `ReferenceTiledFetch.h:33-37`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    /// The whole range, then 2 x 2, then 4 x 4 pieces at most.
    pub max_depth: u32,
    /// Extra attempts for a transient or slow failure.
    pub retries_per_piece: u32,
    /// No further split or repeat after this time.
    pub budget: Duration,
}

impl Default for Options {
    fn default() -> Self {
        Self { max_depth: 2, retries_per_piece: 1, budget: Duration::from_millis(300_000) }
    }
}

/// `ReferenceTiledFetch.h:39-46`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Fetched {
    pub ok: bool,
    pub cancelled: bool,
    /// The joined FeatureCollection, or the single original body.
    pub body: Vec<u8>,
    pub error: String,
    pub requests: u32,
    /// Completed pieces that were joined.
    pub pieces: usize,
}

struct State<'a> {
    fetch_piece: &'a mut dyn FnMut(Range) -> Piece,
    cancelled: &'a dyn Fn() -> bool,
    options: Options,
    started: Instant,
    bodies: Vec<Vec<u8>>,
    result: Fetched,
}

impl State<'_> {
    fn within_budget(&self) -> bool {
        self.started.elapsed() < self.options.budget
    }
}

fn splittable(range: &Range) -> bool {
    range.iter().all(|v| v.is_finite()) && range[2] - range[0] > 0.0 && range[3] - range[1] > 0.0
}

/// North-west, north-east, south-west, south-east quarters of the range (`:83-90`).
#[must_use]
pub fn quarters(range: &Range) -> [Range; 4] {
    let (mid_x, mid_y) = ((range[0] + range[2]) / 2.0, (range[1] + range[3]) / 2.0);
    [[range[0], mid_y, mid_x, range[3]], [mid_x, mid_y, range[2], range[3]], [range[0], range[1], mid_x, mid_y], [mid_x, range[1], range[2], mid_y]]
}

/// Returns false once the whole fetch has failed or was cancelled (`fetchRange`, `:32-78`).
fn fetch_range(state: &mut State<'_>, range: &Range, depth: u32) -> bool {
    let mut attempt = 0;
    loop {
        if (state.cancelled)() {
            state.result.cancelled = true;
            return false;
        }
        // The first request always runs; later pieces stop once the budget is spent.
        if state.result.requests > 0 && !state.within_budget() {
            let minutes = (state.options.budget.as_millis() / 60_000).max(1);
            state.result.error = format!("지도 자료를 받는 시간이 {minutes}분을 넘어 멈췄습니다. 기존 지도는 유지됩니다.");
            return false;
        }
        let mut piece = (state.fetch_piece)(*range);
        state.result.requests += 1;
        if (state.cancelled)() {
            piece.outcome = Outcome::Cancelled;
        }
        match piece.outcome {
            Outcome::Complete => {
                state.bodies.push(piece.body);
                return true;
            }
            Outcome::Cancelled => {
                state.result.cancelled = true;
                state.result.error = piece.error;
                return false;
            }
            Outcome::Fatal => {
                state.result.error = piece.error;
                return false;
            }
            Outcome::TooLarge | Outcome::Slow => {
                if depth < state.options.max_depth && state.within_budget() && splittable(range) {
                    return quarters(range).iter().all(|quarter| fetch_range(state, quarter, depth + 1));
                }
                // A too-large answer is deterministic; only a slow one is worth repeating.
                if piece.outcome == Outcome::Slow && attempt < state.options.retries_per_piece && state.within_budget() {
                    attempt += 1;
                    continue;
                }
                state.result.error = piece.error;
                return false;
            }
            Outcome::Transient => {
                if attempt < state.options.retries_per_piece && state.within_budget() {
                    attempt += 1;
                    continue;
                }
                state.result.error = piece.error;
                return false;
            }
        }
    }
}

/// Fetches the range (`fetch`, `:128-158`). `fetch_piece` is asked for one range at a time, `cancelled`
/// before and after each request.
pub fn fetch(range: &Range, fetch_piece: &mut dyn FnMut(Range) -> Piece, cancelled: &dyn Fn() -> bool, options: Options) -> Fetched {
    let mut state = State { fetch_piece, cancelled, options, started: Instant::now(), bodies: Vec::new(), result: Fetched::default() };
    let ok = fetch_range(&mut state, range, 0);
    let State { mut bodies, mut result, .. } = state;
    if !ok {
        if !result.cancelled && result.requests > 1 {
            result.error = format!("지도 자료를 {}번 나눠 요청했지만 끝까지 받지 못했습니다. 기존 지도는 유지됩니다. 연결이 안정된 곳에서 다시 내려받으세요.\n{}", result.requests, result.error);
        }
        return result;
    }
    result.pieces = bodies.len();
    if bodies.len() == 1 {
        result.body = bodies.swap_remove(0);
    } else {
        match merge_feature_collections(&bodies) {
            Ok(body) => result.body = body,
            Err(error) => {
                result.error = error;
                return result;
            }
        }
    }
    result.ok = true;
    result
}
