//! The numbers of the heritage sites on a sheet, from the rows checked in the legend
//! (`HeritageLayoutNumbers.cpp:194-217, 625-960`). The sites are made up; their copies are written here.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use strata_engine::{write_new_table, Cell, ColumnKind, Row, ShapeKind, Shapes, TableLayout};
use strata_refmap::heritage::{CODE_COLUMN, DATASETS, NAME_COLUMN, TABLE};
use strata_refmap::heritage_load::Loaded;

use super::*;
use crate::app_state::StrataAppState;
use crate::ref_vector::map::tests::{draw_survey_area, CENTRE};

/// The copy of a layer: squares of `side` metres, each `east` and `north` of the survey area's middle, in
/// the work CRS of the tests (EPSG:5187).
fn copy_of(dir: &Path, title: &str, sites: &[(&str, f64, f64, f64)]) -> Loaded {
    let file: PathBuf = dir.join(format!("{title}.gpkg"));
    let layout = TableLayout { table: TABLE, epsg: 5187, kind: ShapeKind::Polygons, columns: &[(NAME_COLUMN, ColumnKind::Text), (CODE_COLUMN, ColumnKind::Text)] };
    let rows: Vec<Row> = sites
        .iter()
        .map(|(name, east, north, side)| {
            let (x, y, half) = (CENTRE[0] + east, CENTRE[1] + north, side / 2.0);
            let ring = vec![[x - half, y - half], [x + half, y - half], [x + half, y + half], [x - half, y + half], [x - half, y - half]];
            Row { shapes: Shapes::Polygons(vec![vec![ring]]), cells: vec![Cell::Text((*name).to_owned()), Cell::Null] }
        })
        .collect();
    write_new_table(&file, &layout, &mut rows.into_iter()).unwrap();
    Loaded { title: title.to_owned(), file, sites: sites.len() as u64, kind: ShapeKind::Polygons, named: true }
}

fn wide_sheet() -> [f64; 4] {
    [CENTRE[0] - 1000.0, CENTRE[1] - 1000.0, CENTRE[0] + 1000.0, CENTRE[1] + 1000.0]
}

fn listed(badges: &[Badge]) -> Vec<(&str, &str, u32)> {
    badges.iter().map(|badge| (badge.kind, badge.name.as_str(), badge.number)).collect()
}

/// The rows of the 「유산」 button that are checked are numbered in the order of the legend: every kind
/// counts through its rows, only within 500 m of the survey area and on the map of the sheet; a badge has
/// the colour of its kind. A row that is unchecked takes its sites out of the count.
#[test]
fn the_checked_heritage_rows_are_numbered_for_the_sheet() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = StrataAppState::new_startup();
    draw_survey_area(&mut app, CENTRE, 5187);
    app.show_heritage_layers(&DATASETS[0], &[copy_of(dir.path(), "국가지정유산", &[("나 고분", 0.0, 0.0, 60.0), ("가 산성", 300.0, 0.0, 40.0), ("먼 사지", 900.0, 0.0, 40.0)]), copy_of(dir.path(), "시도지정유산", &[("다 요지", 0.0, 200.0, 40.0)])]);
    app.show_heritage_layers(&DATASETS[2], &[copy_of(dir.path(), "매장유산유존지역", &[("가 유물산포지", 100.0, -200.0, 40.0)])]);

    let badges = app.sheet_heritage_badges(wide_sheet());
    assert_eq!(listed(&badges), [("지정유산", "가 산성", 1), ("지정유산", "나 고분", 2), ("지정유산", "다 요지", 3), ("매장유산유존지역", "가 유물산포지", 1)]);
    assert_eq!((badges[0].layer_key.as_str(), badges[0].colour), ("ref_heritage:designated_heritage:국가지정유산", [0x8E, 0x44, 0xAD]));
    assert_eq!(badges[3].colour, [0x2E, 0x86, 0xC1]);
    assert!((badges[1].at[0] - CENTRE[0]).abs() <= 30.0 && (badges[1].at[1] - CENTRE[1]).abs() <= 30.0, "on its site: {:?}", badges[1].at);
    assert_eq!(app.sheet_heritage_badges(wide_sheet()), badges, "the same view gives the same numbers");

    app.home_map.real_survey.layer_visible.insert("국가지정유산".to_owned(), false);
    assert_eq!(listed(&app.sheet_heritage_badges(wide_sheet())), [("지정유산", "다 요지", 1), ("매장유산유존지역", "가 유물산포지", 1)]);

    let narrow = [CENTRE[0] - 20.0, CENTRE[1] - 20.0, CENTRE[0] + 20.0, CENTRE[1] + 20.0];
    assert!(app.sheet_heritage_badges(narrow).is_empty(), "this sheet shows none of the sites that are left");
}

/// The badge (`HeritageLayoutNumbers.cpp:103-110, 1027-1110`): a circle in the colour of the kind, its
/// diameter `max(2.3, 1.0 + digits × 0.725) × 1.3` mm of paper, the number in bold 1.82 mm, black on a light
/// colour and white on a dark one (luminance 0.62).
#[test]
fn a_badge_has_the_colour_of_its_kind_and_grows_with_its_digits() {
    let one = badge_look(7, [0x8E, 0x44, 0xAD], 4.0);
    assert!((one.diameter - 2.3 * 1.3 * 4.0).abs() < 1e-4, "{}", one.diameter);
    assert_eq!(one.ink, [255, 255, 255]);
    assert!((one.letter - 1.82 * 4.0).abs() < 1e-4);
    let three = badge_look(123, [0xD4, 0xAA, 0x00], 4.0);
    assert!((three.diameter - (1.0 + 3.0 * 0.725) * 1.3 * 4.0).abs() < 1e-4, "{}", three.diameter);
    assert_eq!(three.ink, [0, 0, 0]);
}
