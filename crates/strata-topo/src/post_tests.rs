//! Two small sheets received for a survey: the one that lies where its number says goes on the map, the
//! other is kept for review; a later call without new files puts the library on the map again.

use serde_json::{json, Map, Value};
use strata_engine::{read_table, Scope, Transformer};
use strata_net::ngii_topographic::TopographicSheets;

use super::*;
use crate::library::{Library, SheetState};
use crate::merge::{LAYER_COLUMN, SHEET_COLUMN, TABLE};
use crate::sheet::test_dxf::{drawing, Entity};
use crate::source_crs::NO_BELT_FITS;

/// Sheet 367154 as drawn in EPSG:5186: the point that far across the sheet (0 to 1).
fn at(fx: f64, fy: f64) -> [f64; 2] {
    let e = TopographicSheets::resolve("367154").expect("sheet").extent();
    Transformer::new(4737, 5186).expect("crs").transform([e[0] + (e[2] - e[0]) * fx, e[1] + (e[3] - e[1]) * fy]).expect("point")
}

fn scope() -> Scope {
    let [x, y] = at(0.5, 0.5);
    let square = format!("POLYGON (({x} {y}, {} {y}, {} {}, {x} {}, {x} {y}))", x + 100.0, x + 100.0, y + 100.0, y + 100.0);
    Scope::around(&[square], 5_000.0).expect("scope").expect("some")
}

/// The portal's record of a sheet, with the rectangle of its grid cell in EPSG:5179.
fn record(num: &str, scale: &str) -> Map<String, Value> {
    let official = Transformer::new(4737, 5179).expect("crs").transform_box(TopographicSheets::resolve(num).expect("sheet").extent()).expect("box");
    json!({"source": "NGII suchiQuery.do", "num": num, "name": "시험", "scale": scale, "fileExt": "DXF", "projection": "GRS80",
           "minx": official[0].to_string(), "miny": official[1].to_string(), "maxx": official[2].to_string(), "maxy": official[3].to_string()})
    .as_object()
    .cloned()
    .expect("object")
}

/// A file with a contour and a road beside the survey, drawn in EPSG:5186 where sheet 367154 lies.
fn sheet_file(root: &std::path::Path, folder: &str, name: &str) -> PathBuf {
    let path = root.join("원본").join(folder).join(name);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
    std::fs::write(&path, drawing(&[Entity::Line("F0017111", Some(120.0), &[at(0.45, 0.5), at(0.5, 0.55)]), Entity::Line("A0013110", None, &[at(0.5, 0.5), at(0.6, 0.5)])])).expect("write");
    path
}

fn merged_files(root: &std::path::Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(root.join(MERGED_FOLDER)).map(|dir| dir.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect()).unwrap_or_default();
    names.sort();
    names
}

fn never() -> bool {
    false
}

#[test]
fn received_sheets_are_listed_placed_and_merged_and_the_library_is_put_on_the_map_again() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("지형도");
    let good = sheet_file(&root, "a", "수치지도_367154_2024.dxf");
    // The same drawing under the number of the sheet to the west: its lines do not lie in that sheet.
    let misplaced = sheet_file(&root, "b", "수치지도_367153_2024.dxf");
    let received = [
        Received { path: good, record: record("367154", "25000") },
        Received { path: misplaced, record: record("367153", "25000") },
        // The metadata file that comes with a sheet is not a sheet; a record of another scale is not linked.
        Received { path: root.join("원본/a/수치지도_367154_2024.xml"), record: record("367154", "25000") },
        Received { path: sheet_file(&root, "c", "수치지도_367152_2024.dxf"), record: record("367152", "5000") },
    ];
    let posted = post_process(&root, &received, &[], &scope(), 5186, 1_000, &never).expect("posted");
    assert_eq!(posted.placed, ["367154"]);
    assert_eq!(posted.review, [("367153".to_owned(), NO_BELT_FITS.to_owned())]);
    assert_eq!(posted.warnings, [NOT_LINKED]);
    assert_eq!(posted.lines, 2);
    assert_eq!(posted.merged, Some(root.join(MERGED_FOLDER).join("수치지형도-합침-1000.gpkg")));
    let mut rows = Vec::new();
    read_table(posted.merged.as_deref().expect("file"), TABLE, &[LAYER_COLUMN, SHEET_COLUMN], &mut |row| {
        rows.push(row.cells);
        true
    })
    .expect("read");
    assert_eq!(rows.len(), 2);

    // The list knows both sheets and what became of them; both count as received.
    let library = Library::open(&root);
    let states: Vec<(&str, &SheetState)> = library.sheets.iter().map(|s| (s.num.as_str(), &s.state)).collect();
    assert_eq!(states, [("367154", &SheetState::Placed { epsg: 5186 }), ("367153", &SheetState::Review { reason: NO_BELT_FITS.to_owned() })]);
    assert_eq!(library.sheets[0].dxf, PathBuf::from("원본/a/수치지도_367154_2024.dxf"));
    assert_eq!(library.numbers(), ["367154", "367153"]);

    // Later, nothing new: the library goes on the map again, in a new file, and the older one is taken away.
    let again = post_process(&root, &[], &[], &scope(), 5186, 2_000, &never).expect("posted");
    assert_eq!((again.placed.clone(), again.lines, again.review.len()), (vec!["367154".to_owned()], 2, 1));
    assert_eq!(merged_files(&root), ["수치지형도-합침-2000.gpkg"]);
}

/// A stop leaves the list as it was and writes no map.
#[test]
fn a_stop_writes_no_map() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("지형도");
    let received = [Received { path: sheet_file(&root, "a", "수치지도_367154_2024.dxf"), record: record("367154", "25000") }];
    assert_eq!(post_process(&root, &received, &[], &scope(), 5186, 1_000, &|| true), Err(MergeStop::Cancelled));
    assert!(merged_files(&root).is_empty());
}

/// The portal's list of today corrects the rectangle of a sheet listed earlier: a sheet kept for review
/// because its rectangle was wrong is placed once the list brings the right one.
#[test]
fn the_portal_list_brings_the_rectangle_of_a_listed_sheet_up_to_date() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("지형도");
    let mut wrong = record("367154", "25000");
    // 30 km east of where the sheet is.
    let shifted = wrong["minx"].as_str().expect("minx").parse::<f64>().expect("number") + 30_000.0;
    wrong.insert("minx".to_owned(), json!(shifted.to_string()));
    wrong.insert("maxx".to_owned(), json!((shifted + 11_000.0).to_string()));
    let received = [Received { path: sheet_file(&root, "a", "수치지도_367154_2024.dxf"), record: wrong }];
    let first = post_process(&root, &received, &[], &scope(), 5186, 1_000, &never).expect("posted");
    assert_eq!((first.placed.len(), first.review.len(), first.merged), (0, 1, None));

    let second = post_process(&root, &[], &[record("367154", "25000")], &scope(), 5186, 2_000, &never).expect("posted");
    assert_eq!((second.placed, second.review.len(), second.lines), (vec!["367154".to_owned()], 0, 2));
}
