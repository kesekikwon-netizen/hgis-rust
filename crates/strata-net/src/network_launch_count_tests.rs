use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use super::{allow_launch_urls, allowed_launches, probe_dry_run};

#[test]
fn denied_remote_launch_does_not_move_allowed_launches_while_another_thread_allows_local() {
    let stop = AtomicBool::new(false);
    let hits = AtomicUsize::new(0);
    thread::scope(|scope| {
        scope.spawn(|| {
            while !stop.load(Ordering::Relaxed) {
                if allow_launch_urls(["http://127.0.0.1/other-thread"]) {
                    hits.fetch_add(1, Ordering::Relaxed);
                }
            }
        });
        let started = Instant::now();
        while hits.load(Ordering::Relaxed) == 0 {
            if started.elapsed() > Duration::from_secs(2) {
                stop.store(true, Ordering::Relaxed);
                panic!("other thread never recorded an allowed launch");
            }
            thread::yield_now();
        }
        let mut moved = None;
        for _ in 0..200 {
            let after = allowed_launches();
            let (recorded, _) = probe_dry_run(false, || {
                assert!(!allow_launch_urls([
                    "https://api.vworld.kr/req/wmts/1.0.0/FAKE/1/2.jpeg",
                ]));
            });
            assert_eq!(recorded, 0);
            let now = allowed_launches();
            if now != after {
                moved = Some((after, now));
                break;
            }
        }
        stop.store(true, Ordering::Relaxed);
        assert!(hits.load(Ordering::Relaxed) > 0);
        assert!(
            moved.is_none(),
            "another thread changed allowed_launches across a denied remote launch: {moved:?}"
        );
    });
}
