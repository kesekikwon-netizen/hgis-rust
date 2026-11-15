//! The area a reference-map button asks for, and the maps kept in the survey folder (`ref_map.rs`).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use strata_engine::Transformer;

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

/// `kept_map`: only a folder of this app's own naming, with the map file and a note for this radius and
/// this box (to a metre); the newest of several.
#[test]
fn a_kept_map_must_carry_the_note_of_this_area() {
    let dir = tempfile::tempdir().unwrap();
    let bbox = [300_000.0, 250_000.0, 320_100.0, 270_100.0];
    let store = |folder: &str, note: Option<String>| -> PathBuf {
        let at = dir.path().join(folder);
        std::fs::create_dir_all(&at).unwrap();
        std::fs::write(at.join(GEOLOGY.file_name), b"map").unwrap();
        if let Some(note) = note {
            std::fs::write(at.join("ka-hgis-scope.json"), note).unwrap();
        }
        at.join(GEOLOGY.file_name)
    };
    store("ka-hgis-reference-noNote", None);
    store("ka-hgis-reference-radius", Some(scope_note(5_000.0, bbox)));
    store("ka-hgis-reference-moved1", Some(scope_note(10_000.0, [bbox[0] + 1.5, bbox[1], bbox[2] + 1.5, bbox[3]])));
    store("ka-hgis-reference-broken", Some("{not json".to_owned()));
    store("another-folder", Some(scope_note(10_000.0, bbox)));
    assert_eq!(kept_map(dir.path(), &GEOLOGY, bbox), None);

    let first = store("ka-hgis-reference-aaaaaa", Some(scope_note(10_000.0, [bbox[0] + 0.4, bbox[1], bbox[2], bbox[3] - 0.4])));
    assert_eq!(kept_map(dir.path(), &GEOLOGY, bbox), Some(first.clone()));
    std::thread::sleep(std::time::Duration::from_millis(30));
    let second = store("ka-hgis-reference-bbbbbb", Some(scope_note(10_000.0, bbox)));
    assert_eq!(kept_map(dir.path(), &GEOLOGY, bbox), Some(second), "the newest");
    assert_eq!(kept_map(Path::new("Z:/no/such/folder"), &GEOLOGY, bbox), None);
    let _ = first;
}
