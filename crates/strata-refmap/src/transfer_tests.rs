//! The storage folder of a fetched map (`PreparedReferenceMap.cpp:17-28`; the C++ test
//! preparedFilesAreOwnedUntilAccepted, `tests/test_reference_download.cpp:104-121`, checks that the
//! requested path itself is not written).

use super::*;

#[test]
fn a_map_is_built_in_a_new_folder_beside_the_requested_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let requested = dir.path().join("지질도_KIGAM.gpkg");
    std::fs::write(&requested, b"previous-reference").expect("old map");

    let (folder, file) = new_storage(&requested).expect("storage");
    assert!(folder.is_dir());
    assert_eq!(folder.parent(), Some(dir.path()));
    let name = folder.file_name().and_then(|n| n.to_str()).expect("name");
    let tail = name.strip_prefix("ka-hgis-reference-").expect("prefix");
    assert!(tail.len() == 6 && tail.bytes().all(|b| b.is_ascii_alphanumeric()), "{name}");
    assert_eq!(file, folder.join("지질도_KIGAM.gpkg"));
    assert!(!file.exists(), "nothing is written yet");
    assert_eq!(std::fs::read(&requested).expect("old map"), b"previous-reference", "the map that was there is untouched");

    // The next map gets its own folder.
    let (second, _) = new_storage(&requested).expect("storage");
    assert_ne!(second, folder);
}

#[test]
fn a_place_that_cannot_hold_a_folder_is_said_with_the_cpp_words() {
    let dir = tempfile::tempdir().expect("tempdir");
    let under_a_file = dir.path().join("a-file");
    std::fs::write(&under_a_file, b"x").expect("file");
    assert_eq!(new_storage(&under_a_file.join("지질도_KIGAM.gpkg")).expect_err("refused"), Stopped::Failed(NO_FOLDER.to_owned()));
    assert_eq!(new_storage(std::path::Path::new("")).expect_err("refused").text(), NO_FOLDER);
}

#[test]
fn a_large_transfer_waits_15_seconds_for_a_byte_and_180_in_all() {
    let request = Request::large("https://example.invalid/x".to_owned(), "agent");
    assert_eq!((request.idle_secs, request.total_secs), (15, 180));
}
