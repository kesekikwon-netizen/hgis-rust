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

/// A sheet at 1:5,000 whose paper of 300 x 200 mm has the survey area's middle at (100, 100) mm.
fn paper_at_5000(blocks: Vec<[f64; 4]>) -> SheetPaper {
    let (scale, per_metre) = (5000.0, 1000.0 / 5000.0);
    SheetPaper { to_paper: strata_layout::badge_rings::ToPaper { scale: [per_metre, -per_metre], shift: [100.0 - CENTRE[0] * per_metre, 100.0 + CENTRE[1] * per_metre] }, frame: [0.0, 0.0, 300.0, 200.0], blocks, scale, names: None }
}

/// Names take the place of the numbers when the sheet asks for them and its scale is 1:5,000 or larger; a
/// shown 1:5,000 may be 5000.3 in the map (`HeritageLayoutNumbers.h:66`, `.cpp:630-632`).
#[test]
fn names_are_written_at_5000_and_larger_when_the_sheet_asks() {
    for (scale, wanted, names) in [(5000.0, true, true), (5000.3, true, true), (1000.0, true, true), (5000.6, true, false), (25_000.0, true, false), (1000.0, false, false), (0.0, true, false)] {
        assert_eq!(names_shown(scale, wanted), names, "{scale} {wanted}");
    }
}

/// The paper a name takes is its letters of 2.2 mm and their white edge of 0.5 mm (`nameBoxMm`, `:112-123`).
#[test]
fn a_name_takes_its_letters_and_their_edge() {
    let [wide, tall] = name_box_mm(10.0);
    assert!((wide - 10.5).abs() < 1e-6 && (tall - (2.2 * 1.15 + 0.5)).abs() < 1e-6, "{wide} {tall}");
}

/// Names that would cover each other are moved apart by their own boxes (and the gap of 0.3 mm), the first
/// staying on its site; the step is the height of a name, not the widest badge (`:972-981, 992-995`).
#[test]
fn names_on_one_spot_keep_off_each_other_by_their_boxes() {
    let lone = [CENTRE[0] + 500.0, CENTRE[1]];
    let letters = vec![20.0, 12.0, 8.0];
    let mut badges = vec![badge_at(1, CENTRE, Vec::new()), badge_at(2, CENTRE, Vec::new()), badge_at(3, lone, Vec::new())];
    let paper = SheetPaper { names: Some(letters.clone()), ..paper_at_5000(Vec::new()) };
    place_badges(&mut badges, &paper);

    assert_eq!((badges[0].placed, badges[2].placed), (CENTRE, lone));
    let on_paper = |at: [f64; 2]| [at[0] * paper.to_paper.scale[0] + paper.to_paper.shift[0], at[1] * paper.to_paper.scale[1] + paper.to_paper.shift[1]];
    let paper_box = |index: usize| {
        let ([x, y], [wide, tall]) = (on_paper(badges[index].placed), name_box_mm(letters[index]));
        [x - (wide + 0.3) / 2.0, y - (tall + 0.3) / 2.0, x + (wide + 0.3) / 2.0, y + (tall + 0.3) / 2.0]
    };
    let (first, second) = (paper_box(0), paper_box(1));
    assert!(first[0] >= second[2] - 1e-9 || second[0] >= first[2] - 1e-9 || first[1] >= second[3] - 1e-9 || second[1] >= first[3] - 1e-9, "{first:?} {second:?}");
    // One step of a name's height away: 3.33 mm of paper are 16.65 m at 1:5,000.
    let apart = (badges[1].placed[0] - CENTRE[0]).hypot(badges[1].placed[1] - CENTRE[1]);
    assert!(apart > 0.0 && apart < 16.65 * 8.0, "{apart}");
}

/// With names on the map the legend lists each kind once, by its name and colour, and no site
/// (`:1207, 1281-1293`; the check says 「범례에는 유적 종류만 남깁니다」).
#[test]
fn the_legend_of_names_lists_each_kind_once() {
    let other = Badge { kind: "매장유산유존지역", colour: [0x2E, 0x86, 0xC1], ..badge_at(1, CENTRE, Vec::new()) };
    let badges = vec![badge_at(1, CENTRE, Vec::new()), badge_at(2, CENTRE, Vec::new()), other];
    assert_eq!(legend_kinds(&badges), [("지정유산", [0x8E, 0x44, 0xAD]), ("매장유산유존지역", [0x2E, 0x86, 0xC1])]);
    assert_eq!((legend_site_rows(&badges, true), legend_site_rows(&badges, false)), (2, 3));
}

fn badge_at(number: u32, at: [f64; 2], ground: Vec<strata_layout::badge_rings::Rings>) -> Badge {
    Badge { layer_key: "ref_heritage:designated_heritage:국가지정유산".to_owned(), kind: "지정유산", name: format!("유적 {number}"), number, colour: [0x8E, 0x44, 0xAD], at, placed: at, ground }
}

/// Badges that would cover each other are moved apart, the first staying on its site and the next taking
/// a free place on its own ground; a lone badge stays where it is. The step between two badges is the
/// widest badge and a gap of 0.3 mm of paper (`HeritageLayoutNumbers.cpp:186-192, 983-990`).
#[test]
fn badges_on_one_spot_are_moved_apart_on_their_ground() {
    let strip = vec![vec![[CENTRE[0] - 5.0, CENTRE[1] - 5.0], [CENTRE[0] + 200.0, CENTRE[1] - 5.0], [CENTRE[0] + 200.0, CENTRE[1] + 5.0], [CENTRE[0] - 5.0, CENTRE[1] + 5.0], [CENTRE[0] - 5.0, CENTRE[1] - 5.0]]];
    let lone = [CENTRE[0] + 500.0, CENTRE[1]];
    let mut badges = vec![badge_at(1, CENTRE, vec![strip.clone()]), badge_at(2, CENTRE, vec![strip]), badge_at(3, lone, Vec::new())];
    place_badges(&mut badges, &paper_at_5000(Vec::new()));

    let step = (2.3 * 1.3 + 0.3) / 1000.0 * 5000.0;
    assert_eq!(badges[0].placed, CENTRE, "the first stays on its site");
    let moved = badges[1].placed;
    let apart = (moved[0] - CENTRE[0]).hypot(moved[1] - CENTRE[1]);
    assert!(apart >= step * 0.96 - 1e-6 && apart <= step * 1.01, "one step away: {apart} against {step}");
    assert!((moved[1] - CENTRE[1]).abs() <= 5.0 && moved[0] > CENTRE[0], "on its own ground, the strip to the east: {moved:?}");
    assert_eq!(badges[2].placed, lone);
    assert_eq!((badges[1].at, badges[2].at), (CENTRE, lone), "the point of the site stays");
}

/// A badge whose site lies under the legend card moves off the card, and one at the edge of the map
/// frame moves wholly on to it (`LayoutBadgePlacer.cpp:70-81`).
#[test]
fn a_badge_stays_off_the_legend_card_and_on_the_map() {
    let card = [90.0, 90.0, 110.0, 110.0];
    let at_the_edge = [CENTRE[0] - 499.0, CENTRE[1]];
    let mut badges = vec![badge_at(1, CENTRE, Vec::new()), badge_at(2, at_the_edge, Vec::new())];
    let paper = paper_at_5000(vec![card]);
    place_badges(&mut badges, &paper);
    let on_paper = |at: [f64; 2]| [at[0] * paper.to_paper.scale[0] + paper.to_paper.shift[0], at[1] * paper.to_paper.scale[1] + paper.to_paper.shift[1]];
    let half = 2.3 * 1.3 / 2.0;
    let under = on_paper(badges[0].placed);
    assert!(under[0] + half <= card[0] || under[0] - half >= card[2] || under[1] + half <= card[1] || under[1] - half >= card[3], "off the card: {under:?}");
    let edge = on_paper(badges[1].placed);
    assert!(edge[0] - half >= 0.0 && edge[1] - half >= 0.0 && edge[0] + half <= 300.0 && edge[1] + half <= 200.0, "wholly on the map: {edge:?}");
}
