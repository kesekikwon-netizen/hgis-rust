//! The list of received sheets: what is written is read back, a sheet whose file is gone is ordered again,
//! a damaged list is an empty one.

use serde_json::json;

use super::*;

fn entry(num: &str, dxf: &str, state: SheetState) -> SheetEntry {
    SheetEntry { num: num.to_owned(), name: "경주".to_owned(), projection: "GRS80".to_owned(), official: Some([1_100_000.5, 1_760_000.25, 1_111_500.0, 1_774_000.0]), dxf: dxf.into(), state }
}

#[test]
fn the_list_is_read_back_as_it_was_written() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut library = Library::open(dir.path());
    assert!(library.sheets.is_empty() && library.numbers().is_empty());
    library.upsert(entry("359012", "원본/a/359012.dxf", SheetState::Placed { epsg: 5187 }));
    library.upsert(entry("359023", "원본/b/359023.dxf", SheetState::Review { reason: "원본 좌표가 공식 도엽 위치와 일치하지 않습니다.".to_owned() }));
    library.upsert(SheetEntry { official: None, ..entry("359021", "원본/c/359021.dxf", SheetState::Received) });
    library.save().expect("save");
    assert!(dir.path().join("strata-sheets.json").is_file());

    let again = Library::open(dir.path());
    assert_eq!(again, library);
    assert_eq!(again.dxf_of(&again.sheets[0]), dir.path().join("원본/a/359012.dxf"));
}

/// A sheet of the same number takes the place of the older entry; the numbers that count as received are
/// those whose file is there.
#[test]
fn a_sheet_is_listed_once_and_counts_while_its_file_is_there() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join("원본/a")).expect("folder");
    std::fs::write(dir.path().join("원본/a/359012.dxf"), b"x").expect("file");
    let mut library = Library::open(dir.path());
    library.upsert(entry("359012", "원본/old/359012.dxf", SheetState::Received));
    library.upsert(entry("359012", "원본/a/359012.dxf", SheetState::Placed { epsg: 5187 }));
    library.upsert(entry("359014", "원본/gone/359014.dxf", SheetState::Placed { epsg: 5187 }));
    assert_eq!(library.sheets.len(), 2);
    assert_eq!(library.sheets[0].dxf, std::path::PathBuf::from("원본/a/359012.dxf"));
    assert_eq!(library.numbers(), ["359012"]);
}

#[test]
fn a_list_that_cannot_be_read_is_an_empty_one() {
    let dir = tempfile::tempdir().expect("tempdir");
    for text in ["not json", r#"{"schema": 2, "sheets": [{"num": "359012"}]}"#, r#"{"schema": 1, "sheets": "none"}"#] {
        std::fs::write(dir.path().join(FILE), text).expect("write");
        assert!(Library::open(dir.path()).sheets.is_empty(), "{text}");
    }
    // An entry without a number or a file is left out; the others stay.
    let mixed = json!({"schema": 1, "sheets": [{"num": "359012", "dxf": "원본/a/359012.dxf", "state": "placed", "epsg": 5187}, {"num": "", "dxf": "x.dxf"}, {"num": "359014"}]});
    std::fs::write(dir.path().join(FILE), mixed.to_string()).expect("write");
    let library = Library::open(dir.path());
    assert_eq!(library.sheets.len(), 1);
    assert_eq!((library.sheets[0].num.as_str(), &library.sheets[0].state, library.sheets[0].official), ("359012", &SheetState::Placed { epsg: 5187 }, None));
}

/// The portal gives the rectangle as numbers or as texts of numbers; anything that is not a rectangle is none.
#[test]
fn the_official_rectangle_of_a_portal_record() {
    let record = |minx: serde_json::Value, maxx: serde_json::Value| json!({"num": "359012", "minx": minx, "miny": "1760000.25", "maxx": maxx, "maxy": 1_774_000}).as_object().cloned().expect("object");
    assert_eq!(official_bounds(&record(json!("1100000.5"), json!(1_111_500.0))), Some([1_100_000.5, 1_760_000.25, 1_111_500.0, 1_774_000.0]));
    for (minx, maxx) in [(json!("east"), json!(1.0)), (json!(5.0), json!(5.0)), (json!(9.0), json!(1.0)), (json!(null), json!(1.0)), (json!("inf"), json!(1.0))] {
        assert_eq!(official_bounds(&record(minx.clone(), maxx.clone())), None, "{minx} {maxx}");
    }
    assert_eq!(official_bounds(&serde_json::Map::new()), None);
}
