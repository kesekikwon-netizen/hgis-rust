//! The area a reference-map button asks for, and the maps kept in the survey folder (`ref_map.rs`).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use strata_engine::{write_new_table, Cell, ColumnKind, PolygonRings, Row, ShapeKind, Shapes, TableLayout, Transformer};

use super::tests::{draw_survey_area, CENTRE};
use super::*;

/// The scope is the drawn survey area grown by the radius, in the work CRS, also when the survey file keeps
/// its survey area in another CRS.
#[test]
fn the_scope_is_the_survey_area_grown_by_the_radius() {
    let mut app = StrataAppState::new_startup();
    assert!(survey_scope(&app, 10_000.0).unwrap().is_none(), "no survey area drawn");
    draw_survey_area(&mut app, CENTRE, 5187);
    let scope = survey_scope(&app, 10_000.0).unwrap().unwrap();
    assert_eq!(scope.bbox().unwrap(), [CENTRE[0] - 10_050.0, CENTRE[1] - 10_050.0, CENTRE[0] + 10_050.0, CENTRE[1] + 10_050.0]);

    // The same place kept in EPSG:5186 by the survey file.
    let in_5186 = Transformer::new(5187, 5186).unwrap().transform(CENTRE).unwrap();
    draw_survey_area(&mut app, in_5186, 5186);
    let carried = survey_scope(&app, 10_000.0).unwrap().unwrap().bbox().unwrap();
    for (got, want) in carried.iter().zip(scope.bbox().unwrap()) {
        // The square is turned by a degree between the belts: its box grows by about a metre.
        assert!((got - want).abs() < 2.0, "{carried:?}");
    }
    // A table without a CRS: said, not guessed.
    draw_survey_area(&mut app, CENTRE, 0);
    assert_eq!(survey_scope(&app, 10_000.0).unwrap_err(), "조사구역 좌표계를 확인하지 못했습니다.");
}

/// An L-shaped scope in the stored CRS; `inner` moves its inner corner, which leaves its box as it is.
fn l_scope(inner: f64) -> Vec<PolygonRings> {
    let (x, y, side) = (300_000.0, 250_000.0, 20_100.0);
    let middle = side / 2.0 + inner;
    vec![vec![vec![[x, y], [x + side, y], [x + side, y + middle], [x + middle, y + middle], [x + middle, y + side], [x, y + side], [x, y]]]]
}

/// A map file of this kind with one unit.
fn store_map(file: &Path) {
    let layout = TableLayout { table: GEOLOGY.table, epsg: GEOLOGY.stored_epsg, kind: ShapeKind::Polygons, columns: &[("기호", ColumnKind::Text), ("ka_color", ColumnKind::Text)] };
    let ring = vec![[300_100.0, 250_100.0], [300_200.0, 250_100.0], [300_200.0, 250_200.0], [300_100.0, 250_200.0], [300_100.0, 250_100.0]];
    let unit = Row { shapes: Shapes::Polygons(vec![vec![ring]]), cells: vec![Cell::Text("Qa".to_owned()), Cell::Text("#FAF4B4".to_owned())] };
    write_new_table(file, &layout, &mut vec![unit].into_iter()).unwrap();
}

/// `kept_map`: only a folder of this app's own naming, with a map file that opens and a note saying that
/// the map is whole and was fetched for this radius and this very shape of the area (every point to a
/// metre); the newest of several. The box alone does not say that: a survey area added between two others,
/// or a moved inner corner, changes what was cut out and leaves the box as it is.
#[test]
fn a_kept_map_must_carry_the_note_of_this_area() {
    let dir = tempfile::tempdir().unwrap();
    let scope = l_scope(0.0);
    let store = |folder: &str, note: Option<String>| -> PathBuf {
        let at = dir.path().join(folder);
        std::fs::create_dir_all(&at).unwrap();
        store_map(&at.join(GEOLOGY.file_name));
        if let Some(note) = note {
            std::fs::write(at.join("ka-hgis-scope.json"), note).unwrap();
        }
        at.join(GEOLOGY.file_name)
    };
    store("ka-hgis-reference-noNote", None);
    store("ka-hgis-reference-radius", Some(scope_note(5_000.0, &scope, true)));
    store("ka-hgis-reference-corner", Some(scope_note(10_000.0, &l_scope(1.5), true)));
    // The same shape a metre and a half to the east, and to the north: each way alone is another area.
    let moved = |dx: f64, dy: f64| -> Vec<PolygonRings> { scope.iter().map(|rings| rings.iter().map(|ring| ring.iter().map(|p| [p[0] + dx, p[1] + dy]).collect()).collect()).collect() };
    store("ka-hgis-reference-toEast", Some(scope_note(10_000.0, &moved(1.5, 0.0), true)));
    store("ka-hgis-reference-toNort", Some(scope_note(10_000.0, &moved(0.0, 1.5), true)));
    let two_parts = vec![scope[0].clone(), scope[0].clone()];
    store("ka-hgis-reference-twoPar", Some(scope_note(10_000.0, &two_parts, true)));
    let (mut longer, mut holed) = (scope.clone(), scope.clone());
    longer[0][0].push([300_000.0, 250_000.0]);
    holed[0].push(vec![[300_010.0, 250_010.0], [300_020.0, 250_010.0], [300_020.0, 250_020.0], [300_010.0, 250_010.0]]);
    store("ka-hgis-reference-longer", Some(scope_note(10_000.0, &longer, true)));
    store("ka-hgis-reference-aHole1", Some(scope_note(10_000.0, &holed, true)));
    store("ka-hgis-reference-broken", Some("{not json".to_owned()));
    store("ka-hgis-reference-oldBox", Some(r#"{"radius_m":10000.0,"bbox":[300000.0,250000.0,320100.0,270100.0]}"#.to_owned()));
    store("another-folder", Some(scope_note(10_000.0, &scope, true)));
    // Stored with a warning (for example without the official colours): to be fetched again.
    store("ka-hgis-reference-partly", Some(scope_note(10_000.0, &scope, false)));
    // The right note beside a file that does not open.
    let unreadable = store("ka-hgis-reference-noFile", Some(scope_note(10_000.0, &scope, true)));
    std::fs::write(&unreadable, b"not a map").unwrap();
    assert_eq!(kept_map(dir.path(), &GEOLOGY, &scope), None);

    let first = store("ka-hgis-reference-aaaaaa", Some(scope_note(10_000.0, &l_scope(0.4), true)));
    assert_eq!(kept_map(dir.path(), &GEOLOGY, &scope), Some(first));
    std::thread::sleep(std::time::Duration::from_millis(30));
    let second = store("ka-hgis-reference-bbbbbb", Some(scope_note(10_000.0, &scope, true)));
    assert_eq!(kept_map(dir.path(), &GEOLOGY, &scope), Some(second), "the newest");
    assert_eq!(kept_map(Path::new("Z:/no/such/folder"), &GEOLOGY, &scope), None);
}
