//! Expected values of `target/cpp-ref/tests/test_reference_download.cpp` tiledFetchMergesAndBoundsRetries
//! (:264-287) and the rules of `ReferenceTiledFetch.cpp:32-90`, each with its request count.

use std::cell::{Cell, RefCell};
use std::time::Duration;

use super::*;

const UNIT: Range = [0.0, 0.0, 1.0, 1.0];

fn piece(outcome: Outcome, error: &str) -> Piece {
    Piece { outcome, body: Vec::new(), error: error.to_owned() }
}

fn collection(ids: &[&str]) -> Vec<u8> {
    let features: Vec<String> = ids.iter().map(|id| format!(r#"{{"type":"Feature","id":"{id}","properties":{{}},"geometry":null}}"#)).collect();
    format!(r#"{{"type":"FeatureCollection","features":[{}]}}"#, features.join(",")).into_bytes()
}

fn never() -> bool {
    false
}

/// :264-271 — a dropped connection is tried once more, never in an endless loop.
#[test]
fn a_dropped_piece_is_tried_once_more() {
    let mut requests = 0;
    let dropped = fetch(&UNIT, &mut |_| {
        requests += 1;
        piece(Outcome::Transient, "끊김")
    }, &never, Options::default());
    assert!(!dropped.ok);
    assert_eq!(requests, 2);
    assert!(dropped.error.contains("끊김"));
    assert_eq!(dropped.error, "지도 자료를 2번 나눠 요청했지만 끝까지 받지 못했습니다. 기존 지도는 유지됩니다. 연결이 안정된 곳에서 다시 내려받으세요.\n끊김");
    assert!(dropped.body.is_empty());
}

/// :273-279 — a bad server answer is neither split nor repeated.
#[test]
fn an_unusable_answer_ends_the_fetch_at_once() {
    let mut requests = 0;
    let fatal = fetch(&UNIT, &mut |_| {
        requests += 1;
        piece(Outcome::Fatal, "잘못된 응답")
    }, &never, Options::default());
    assert!(!fatal.ok && !fatal.cancelled);
    assert_eq!(requests, 1);
    assert_eq!(fatal.error, "잘못된 응답", "one request: the plain reason");
}

/// :281-287 — no time left: the first request runs, nothing more.
#[test]
fn a_spent_budget_allows_the_first_request_only() {
    let mut requests = 0;
    let spent = Options { budget: Duration::ZERO, ..Options::default() };
    let over = fetch(&UNIT, &mut |_| {
        requests += 1;
        piece(Outcome::TooLarge, "많음")
    }, &never, spent);
    assert!(!over.ok);
    assert_eq!(requests, 1);
    assert_eq!(over.error, "많음");
}

/// A whole range that is too large is fetched as its four quarters, north-west first; the answers are
/// joined and a feature on a shared edge is kept once.
#[test]
fn a_range_too_large_is_fetched_in_quarters() {
    let asked = RefCell::new(Vec::new());
    let fetched = fetch(&[126.0, 35.0, 128.0, 37.0], &mut |range| {
        asked.borrow_mut().push(range);
        match asked.borrow().len() {
            1 => piece(Outcome::TooLarge, "많음"),
            n => Piece { outcome: Outcome::Complete, body: collection(&["edge", &format!("q{n}")]), error: String::new() },
        }
    }, &never, Options::default());
    assert!(fetched.ok, "{}", fetched.error);
    assert_eq!((fetched.requests, fetched.pieces), (5, 4));
    assert_eq!(asked.borrow()[1..], [[126.0, 36.0, 127.0, 37.0], [127.0, 36.0, 128.0, 37.0], [126.0, 35.0, 127.0, 36.0], [127.0, 35.0, 128.0, 36.0]]);
    assert_eq!(quarters(&[126.0, 35.0, 128.0, 37.0]).to_vec(), asked.borrow()[1..].to_vec());
    let joined: serde_json::Value = serde_json::from_slice(&fetched.body).expect("json");
    let ids: Vec<&str> = joined["features"].as_array().expect("list").iter().filter_map(|f| f["id"].as_str()).collect();
    assert_eq!(ids, ["edge", "q2", "q3", "q4", "q5"]);
}

/// One complete answer is handed on untouched, byte for byte.
#[test]
fn a_single_complete_answer_is_returned_as_it_came() {
    let body = br#"{ "type" : "FeatureCollection", "features" : [] , "x": 1}"#.to_vec();
    let answer = body.clone();
    let fetched = fetch(&UNIT, &mut |_| Piece { outcome: Outcome::Complete, body: answer.clone(), error: String::new() }, &never, Options::default());
    assert!(fetched.ok);
    assert_eq!((fetched.requests, fetched.pieces), (1, 1));
    assert_eq!(fetched.body, body);
}

/// Splitting stops at 4 x 4 pieces: 1 + 4 requests on the way down, then the first of the 16 fails for good.
#[test]
fn splitting_goes_two_levels_deep_and_no_further() {
    let mut requests = 0;
    let fetched = fetch(&UNIT, &mut |_| {
        requests += 1;
        piece(Outcome::TooLarge, "많음")
    }, &never, Options::default());
    assert!(!fetched.ok);
    // The whole, its first quarter, that quarter's first quarter: too large at the finest level is not repeated.
    assert_eq!(requests, 3);
    assert!(fetched.error.starts_with("지도 자료를 3번 나눠 요청했지만"));
    // With every piece of the finest level complete: 1 + 4 + 16.
    let mut requests = 0;
    let depth_of = |range: Range| ((1.0 / (range[2] - range[0])).log2().round()) as u32;
    let fetched = fetch(&UNIT, &mut |range| {
        requests += 1;
        if depth_of(range) < 2 { piece(Outcome::TooLarge, "많음") } else { Piece { outcome: Outcome::Complete, body: collection(&[&format!("{range:?}")]), error: String::new() } }
    }, &never, Options::default());
    assert!(fetched.ok);
    assert_eq!((requests, fetched.pieces), (21, 16));
}

/// A slow piece is split while it can be; at the finest level it is tried once more, then given up.
#[test]
fn a_slow_piece_is_split_and_at_the_finest_level_repeated_once() {
    let mut requests = 0;
    let fetched = fetch(&UNIT, &mut |_| {
        requests += 1;
        piece(Outcome::Slow, "늦음")
    }, &never, Options::default());
    assert!(!fetched.ok);
    assert_eq!(requests, 4, "whole, quarter, sixteenth, the sixteenth again");
    // A range that cannot be split (no width) is repeated at once.
    let mut requests = 0;
    fetch(&[1.0, 0.0, 1.0, 1.0], &mut |_| {
        requests += 1;
        piece(Outcome::Slow, "늦음")
    }, &never, Options::default());
    assert_eq!(requests, 2);
    // A slow piece that answers at the second try completes the fetch.
    let mut requests = 0;
    let fetched = fetch(&[1.0, 0.0, 1.0, 1.0], &mut |_| {
        requests += 1;
        if requests == 1 { piece(Outcome::Slow, "늦음") } else { Piece { outcome: Outcome::Complete, body: collection(&["a"]), error: String::new() } }
    }, &never, Options::default());
    assert!(fetched.ok && fetched.requests == 2);
}

/// The user's stop: before a request nothing is asked; during a request its answer is thrown away. No
/// "tried n times" sentence is added.
#[test]
fn a_stop_ends_the_fetch_without_a_failure_sentence() {
    let mut requests = 0;
    let stopped = fetch(&UNIT, &mut |_| {
        requests += 1;
        piece(Outcome::Complete, "")
    }, &|| true, Options::default());
    assert!(stopped.cancelled && !stopped.ok);
    assert_eq!(requests, 0);

    let stop = Cell::new(false);
    let mut requests = 0;
    let stopped = fetch(&UNIT, &mut |_| {
        requests += 1;
        if requests == 2 {
            stop.set(true);
        }
        if requests == 1 { piece(Outcome::TooLarge, "많음") } else { Piece { outcome: Outcome::Complete, body: collection(&["a"]), error: "취소".to_owned() } }
    }, &|| stop.get(), Options::default());
    assert!(stopped.cancelled && !stopped.ok && stopped.body.is_empty());
    assert_eq!(requests, 2);
    assert_eq!(stopped.error, "취소");
}

/// After the budget no further piece is asked for, and the sentence names the minutes.
#[test]
fn the_budget_ends_a_fetch_in_pieces() {
    let mut requests = 0;
    let short = Options { budget: Duration::from_millis(30), ..Options::default() };
    let fetched = fetch(&UNIT, &mut |_| {
        requests += 1;
        if requests == 2 {
            std::thread::sleep(Duration::from_millis(60));
        }
        if requests == 1 { piece(Outcome::TooLarge, "많음") } else { Piece { outcome: Outcome::Complete, body: collection(&["a"]), error: String::new() } }
    }, &never, short);
    assert!(!fetched.ok);
    assert_eq!(requests, 2, "the first quarter outlived the budget: the second is not asked for");
    assert_eq!(fetched.error, "지도 자료를 2번 나눠 요청했지만 끝까지 받지 못했습니다. 기존 지도는 유지됩니다. 연결이 안정된 곳에서 다시 내려받으세요.\n지도 자료를 받는 시간이 1분을 넘어 멈췄습니다. 기존 지도는 유지됩니다.");
    assert_eq!(Options::default().budget, Duration::from_secs(300));
}
