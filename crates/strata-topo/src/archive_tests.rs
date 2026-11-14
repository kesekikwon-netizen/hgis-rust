//! The expectations of the C++ archive tests (`target/cpp-ref/tests/test_topographic_archive.cpp`), one test
//! function each, with the same inputs.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::Cell;
use std::fs;
use std::path::Path;

use super::test_zip::{le16, le32, put_le32, zip_deflated, zip_raw, RawEntry};
use super::{prepare, Limits, Prepared};

fn run(source: &Path, library: &Path) -> Prepared {
    prepare(source, library, &Limits::default(), &|| false)
}

fn find(bytes: &[u8], signature: [u8; 4]) -> usize {
    bytes.windows(4).position(|w| w == signature).expect("signature in the ZIP")
}

/// `nestedArchiveIsReadOnlyAndReusable` (:33-65)
#[test]
fn a_nested_archive_is_read_only_and_reusable() {
    let temp = tempfile::tempdir().unwrap();
    let inner = temp.path().join("sheet.zip");
    zip_deflated(&inner, &[("도엽/F0010000.dxf", b"contours".to_vec())]);
    let outer = temp.path().join("download.zip");
    zip_deflated(&outer, &[("nested/sheet.zip", fs::read(&inner).unwrap()), ("license.txt", b"license".to_vec())]);
    let original = fs::read(&outer).unwrap();
    let library = temp.path().join("library");

    let first = run(&outer, &library);
    assert_eq!(first.error, "");
    assert!(!first.canceled && !first.invalid_archive && !first.reused);
    assert_eq!(first.files.len(), 2);
    assert!(first.directory.as_ref().is_some_and(|d| d.is_dir()));
    let dxf: Vec<_> = first.files.iter().filter(|f| f.extension().is_some_and(|e| e == "dxf")).collect();
    assert_eq!(dxf.len(), 1);
    assert_eq!(fs::read(dxf[0]).unwrap(), b"contours");
    assert!(dxf[0].ends_with("nested/sheet.zip.contents/도엽/F0010000.dxf"), "{}", dxf[0].display());
    assert_eq!(fs::read(&outer).unwrap(), original, "the source is never changed");

    let again = run(&outer, &library);
    assert_eq!(again.error, "");
    assert!(again.reused && !again.invalid_archive);
    assert_eq!((again.directory.clone(), again.files.clone()), (first.directory.clone(), first.files.clone()));

    fs::write(&first.files[0], b"corrupted").unwrap();
    let repaired = run(&outer, &library);
    assert_eq!(repaired.error, "");
    assert!(!repaired.reused, "a changed file is not trusted: the archive is unpacked again");
    assert_eq!(fs::read(&outer).unwrap(), original);
    assert!(run(&outer, &library).reused);
}

/// `rejectsUnsafePaths` (:66-90). The first group is what GDAL's ZIP reader leaves out, so the archive has
/// nothing readable and counts as a damaged download; the second group reaches the path rule.
#[test]
fn unsafe_paths_are_refused_and_nothing_is_written() {
    let cases: [(&str, bool); 9] = [
        ("../escape.dxf", true),
        ("safe/../../escape.dxf", true),
        ("/escape.dxf", true),
        ("safe\\..\\escape.dxf", true),
        ("C:/escape.dxf", false),
        ("NUL.dxf", false),
        ("safe/CON/file.dxf", false),
        ("safe/file.dxf:stream", false),
        ("safe./file.dxf", false),
    ];
    for (name, invalid_archive) in cases {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("unsafe.zip");
        zip_raw(&source, &[RawEntry::file(name, b"data")]);
        let result = run(&source, &temp.path().join("library"));
        assert!(!result.error.is_empty(), "{name}");
        assert!(result.directory.is_none() && result.files.is_empty(), "{name}");
        assert_eq!(result.invalid_archive, invalid_archive, "{name}: {}", result.error);
        assert!(!temp.path().join("escape.dxf").exists(), "{name}");
    }
}

/// `rejectsResourceLimitsAndCancellation` (:91-109)
#[test]
fn size_limits_and_cancelling_stop_the_work() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("large.zip");
    zip_deflated(&source, &[("a.dxf", vec![b'a'; 32768]), ("b.dxf", vec![b'b'; 32768])]);
    let library = temp.path().join("library");
    let limited = prepare(&source, &library, &Limits { max_file_bytes: 100, ..Limits::default() }, &|| false);
    assert!(!limited.error.is_empty() && !limited.invalid_archive);
    assert!(!prepare(&source, &library, &Limits { max_total_bytes: 40000, ..Limits::default() }, &|| false).error.is_empty());
    assert!(!prepare(&source, &library, &Limits { max_entries: 1, ..Limits::default() }, &|| false).error.is_empty());

    let checks = Cell::new(0);
    let canceled = prepare(&source, &library, &Limits::default(), &|| {
        checks.set(checks.get() + 1);
        checks.get() > 3
    });
    assert!(canceled.canceled && canceled.directory.is_none() && canceled.files.is_empty() && !canceled.invalid_archive);
    let retry = run(&source, &library);
    assert_eq!(retry.error, "");
    let late = prepare(&source, &library, &Limits::default(), &|| true);
    assert!(late.canceled);
    assert!(retry.directory.as_ref().is_some_and(|d| d.exists()), "a cancelled later call leaves the kept copy alone");
}

/// `rawVectorReturnsOriginalAndUnsupportedFails` (:110-117)
#[test]
fn a_plain_map_file_is_used_where_it_lies() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("map.dxf");
    fs::write(&source, b"test").unwrap();
    let result = run(&source, &temp.path().join("library"));
    assert_eq!(result.error, "");
    assert_eq!(result.files, vec![source.clone()]);
    assert_eq!(result.directory.as_deref(), Some(temp.path()));
    assert_eq!(fs::read(&source).unwrap(), b"test");
    assert!(!run(&temp.path().join("absent.zip"), temp.path()).error.is_empty());
    let other = temp.path().join("notes.txt");
    fs::write(&other, b"x").unwrap();
    assert_eq!(run(&other, temp.path()).error, "수치지형도 ZIP·SHP·DXF·GPKG·NGI 자료를 선택하세요.");
}

/// `damagedArchiveCanBeDownloadedAgain` (:118-157): four kinds of damage, each marks the download itself
/// as bad (so that the caller may fetch it again) and leaves no output.
#[test]
fn a_damaged_archive_is_reported_as_a_bad_download() {
    for damage in ["central", "crc", "deflate", "empty"] {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("damaged.zip");
        let payload: Vec<u8> = (0..74000).map(|i| (i % 251) as u8).collect();
        zip_deflated(&source, &[("map.dxf", payload)]);
        let mut bytes = fs::read(&source).unwrap();
        let local = find(&bytes, [0x50, 0x4b, 0x03, 0x04]);
        let central = find(&bytes, [0x50, 0x4b, 0x01, 0x02]);
        let end = find(&bytes, [0x50, 0x4b, 0x05, 0x06]);
        assert!(central > local && end > central);
        match damage {
            "central" => bytes.truncate(central),
            "empty" => bytes = vec![0x50, 0x4b, 0x05, 0x06, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            _ => {
                assert_eq!(le16(&bytes, local + 8), 8, "the fixture is deflated");
                if damage == "crc" {
                    let wrong = le32(&bytes, central + 16) ^ 1;
                    put_le32(&mut bytes, local + 14, wrong);
                    put_le32(&mut bytes, central + 16, wrong);
                } else {
                    // Keep the directory intact but advertise only half the DEFLATE stream.
                    let compressed = le32(&bytes, central + 20);
                    assert!(compressed > 4);
                    put_le32(&mut bytes, local + 18, compressed / 2);
                    put_le32(&mut bytes, central + 20, compressed / 2);
                }
            }
        }
        fs::write(&source, &bytes).unwrap();
        let result = run(&source, &temp.path().join("library"));
        assert!(!result.error.is_empty(), "{damage}");
        assert!(result.invalid_archive, "{damage}: {}", result.error);
        assert!(result.files.is_empty() && result.directory.is_none(), "{damage}");
        assert_eq!(fs::read(&source).unwrap(), bytes, "{damage}");
    }
}

/// `storageFailureDoesNotRequestAnotherDownload` (:158-170)
#[test]
fn a_storage_problem_is_not_blamed_on_the_download() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("valid.zip");
    zip_deflated(&source, &[("map.dxf", b"map".to_vec())]);
    let original = fs::read(&source).unwrap();
    let library = temp.path().join("library");
    fs::write(&library, b"keep").unwrap();
    let result = run(&source, &library);
    assert!(!result.error.is_empty() && !result.invalid_archive);
    assert!(result.files.is_empty() && result.directory.is_none());
    assert_eq!(fs::read(&source).unwrap(), original);
    assert_eq!(fs::read(&library).unwrap(), b"keep");
}

/// `rejectsNestedDepthAndCaseCollisions` (:171-186)
#[test]
fn nesting_too_deep_and_names_that_differ_only_in_case_are_refused() {
    let temp = tempfile::tempdir().unwrap();
    let inner = temp.path().join("inner.zip");
    let outer = temp.path().join("outer.zip");
    zip_deflated(&inner, &[("map.dxf", b"map".to_vec())]);
    zip_deflated(&outer, &[("inner.zip", fs::read(&inner).unwrap())]);
    let shallow = Limits { max_archive_depth: 1, ..Limits::default() };
    assert!(!prepare(&outer, &temp.path().join("library"), &shallow, &|| false).error.is_empty());
    let collision = temp.path().join("collision.zip");
    zip_raw(&collision, &[RawEntry::file("Map.dxf", b"a"), RawEntry::file("map.dxf", b"b")]);
    let result = run(&collision, &temp.path().join("library"));
    assert!(!result.error.is_empty());
    assert!(result.directory.is_none());
}

/// `archiveLinkNeverCreatesFilesystemLink` (:187-209): an entry marked as a symbolic link in the ZIP is
/// refused or written as an ordinary file; it never becomes a link and never reaches outside.
#[test]
fn a_link_entry_never_becomes_a_link() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("link.zip");
    zip_raw(&source, &[RawEntry::file("link.dxf", b"../../outside.dxf").unix_mode(0o120_777)]);
    let result = run(&source, &temp.path().join("library"));
    if result.error.is_empty() {
        assert_eq!(result.files.len(), 1);
        assert!(!fs::symlink_metadata(&result.files[0]).unwrap().file_type().is_symlink());
        assert_eq!(fs::read(&result.files[0]).unwrap(), b"../../outside.dxf");
    } else {
        assert!(result.directory.is_none());
    }
    assert!(!temp.path().join("outside.dxf").exists());
}
