//! Exclusive offer directories (`target/cpp-ref/src/app/KaHeritageBrowser.cpp:1308-1315`).

use std::collections::HashSet;
use std::fs;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Barrier};

use super::{allocate_in, allocate_received_folder};

#[test]
fn colliding_candidates_preserve_both_previous_same_name_offers() {
    let fixture = tempfile::tempdir().expect("tempdir");
    let parent = fixture.path().join("주변유적/경상북도 경주시/원본");
    let first = parent.join(format!("offer-{}-0", std::process::id()));
    let second = parent.join(format!("offer-{}-1", std::process::id()));
    fs::create_dir_all(&first).expect("first previous offer");
    fs::create_dir(&second).expect("second previous offer");
    fs::write(first.join("same.zip"), b"first archive").expect("first file");
    fs::write(second.join("same.zip"), b"second archive").expect("second file");

    let offered = allocate_in(&parent, &AtomicU64::new(0)).expect("new offer");
    fs::write(offered.join("same.zip"), b"third archive").expect("new file");

    assert_eq!(
        fs::read(first.join("same.zip")).expect("first retained"),
        b"first archive"
    );
    assert_eq!(
        fs::read(second.join("same.zip")).expect("second retained"),
        b"second archive"
    );
    assert_ne!(offered, first);
    assert_ne!(offered, second);
    assert_eq!(
        fs::read(offered.join("same.zip")).expect("new archive"),
        b"third archive"
    );
}

#[test]
fn repeated_allocations_create_distinct_directories_in_the_cpp_layout() {
    let fixture = tempfile::tempdir().expect("tempdir");
    let parent = fixture.path().join("주변유적/경상북도 경주시/원본");
    let mut offers = Vec::new();
    for serial in 0_u8..32 {
        let path =
            allocate_received_folder(fixture.path(), "경상북도", "경주시").expect("allocate");
        assert_eq!(path.parent(), Some(parent.as_path()));
        assert!(
            path.is_dir(),
            "allocation must create the returned directory"
        );
        fs::write(path.join("same.zip"), [serial]).expect("write offer");
        offers.push((path, serial));
    }
    assert_eq!(
        offers
            .iter()
            .map(|(path, _)| path)
            .collect::<HashSet<_>>()
            .len(),
        32
    );
    for (path, serial) in offers {
        assert_eq!(
            fs::read(path.join("same.zip")).expect("retained file"),
            [serial]
        );
    }
}

#[test]
fn concurrent_allocators_preserve_each_same_name_offer() {
    let fixture = tempfile::tempdir().expect("tempdir");
    let parent = fixture.path().join("주변유적/경상북도 경주시/원본");
    let ready = Arc::new(Barrier::new(12));
    let mut workers = Vec::new();
    for serial in 0_u8..12 {
        let parent = parent.clone();
        let ready = Arc::clone(&ready);
        workers.push(std::thread::spawn(move || {
            ready.wait();
            let path = allocate_in(&parent, &AtomicU64::new(0)).expect("allocate");
            fs::write(path.join("same.zip"), [serial]).expect("write offer");
            (path, serial)
        }));
    }
    let offers: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().expect("worker"))
        .collect();
    assert_eq!(
        offers
            .iter()
            .map(|(path, _)| path)
            .collect::<HashSet<_>>()
            .len(),
        12
    );
    for (path, serial) in offers {
        assert!(path.is_dir());
        assert_eq!(
            fs::read(path.join("same.zip")).expect("retained file"),
            [serial]
        );
    }
}

#[test]
fn an_obstructing_root_file_returns_an_io_error_without_changing_it() {
    let fixture = tempfile::tempdir().expect("tempdir");
    let root = fixture.path().join("root-file");
    fs::write(&root, b"keep this file").expect("obstructing file");
    allocate_received_folder(&root, "경상북도", "경주시")
        .expect_err("a file cannot be a directory parent");
    assert_eq!(fs::read(&root).expect("original file"), b"keep this file");
}
