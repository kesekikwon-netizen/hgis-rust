//! The numbers of the heritage sites on a sheet (`HeritageLayoutNumbers.cpp:194-217, 625-960`), on made-up
//! sites around a survey area of 100 m (EPSG:5187).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use strata_engine::{PolygonRings, Transformer};

use super::*;

const CENTRE: [f64; 2] = [220_000.0, 370_000.0];

/// A square of `side` metres whose middle lies `east` and `north` of the survey area's.
fn square(east: f64, north: f64, side: f64) -> PolygonRings {
    let (x, y, half) = (CENTRE[0] + east, CENTRE[1] + north, side / 2.0);
    vec![vec![[x - half, y - half], [x + half, y - half], [x + half, y + half], [x - half, y + half], [x - half, y - half]]]
}

fn site(name: &str, shapes: &[PolygonRings]) -> (String, Vec<PolygonRings>) {
    (name.to_owned(), shapes.to_vec())
}

fn survey_area() -> Vec<String> {
    let ring = &square(0.0, 0.0, 100.0)[0];
    vec![format!("POLYGON (({}))", ring.iter().map(|p| format!("{} {}", p[0], p[1])).collect::<Vec<_>>().join(", "))]
}

/// A sheet whose map shows 2 km around the survey area.
fn wide_sheet() -> [f64; 4] {
    [CENTRE[0] - 1000.0, CENTRE[1] - 1000.0, CENTRE[0] + 1000.0, CENTRE[1] + 1000.0]
}

fn listed(numbered: &[Numbered]) -> Vec<(usize, &str, u32)> {
    numbered.iter().map(|site| (site.layer, site.name.as_str(), site.number)).collect()
}

fn inside(at: [f64; 2], shape: &PolygonRings) -> bool {
    let (xs, ys) = (shape[0].iter().map(|p| p[0]), shape[0].iter().map(|p| p[1]));
    let (min_x, max_x) = (xs.clone().fold(f64::MAX, f64::min), xs.fold(f64::MIN, f64::max));
    let (min_y, max_y) = (ys.clone().fold(f64::MAX, f64::min), ys.fold(f64::MIN, f64::max));
    (min_x..=max_x).contains(&at[0]) && (min_y..=max_y).contains(&at[1])
}

/// Only sites within 500 m of the survey area get a number (`:194-196`). Every kind counts 1, 2, 3 … through
/// its layers in the order of the legend (`:723-726, 926`); within a layer the sites stand in the order of
/// their names and the same name is one number (`:831-838`); a site without a name is 「무명」. The number
/// stands on the largest piece of its site (`:845-855`).
#[test]
fn sites_near_the_survey_area_are_numbered_by_kind_and_name() {
    let (small, large) = (square(300.0, 0.0, 40.0), square(380.0, 0.0, 80.0));
    let designated = [site("나 고분", &[square(0.0, 0.0, 60.0)]), site(" 가 산성 ", &[small]), site("가 산성", std::slice::from_ref(&large)), site("", &[square(-300.0, 0.0, 40.0)]), site("먼 사지", &[square(900.0, 0.0, 40.0)])];
    let provincial = [site("다 요지", &[square(0.0, 200.0, 40.0)])];
    let buried = [site("가 유물산포지", &[square(100.0, -200.0, 40.0)])];
    let layers = [SheetLayer { kind: "지정유산", epsg: 5187, sites: &designated }, SheetLayer { kind: "지정유산", epsg: 5187, sites: &provincial }, SheetLayer { kind: "매장유산유존지역", epsg: 5187, sites: &buried }];
    let area = numbered_area(&survey_area(), wide_sheet()).unwrap().expect("an area");

    let numbered = number_sites(&layers, &area, 5187).unwrap();
    assert_eq!(listed(&numbered), [(0, "가 산성", 1), (0, "나 고분", 2), (0, "무명", 3), (1, "다 요지", 4), (2, "가 유물산포지", 1)]);
    assert!(inside(numbered[0].at, &large), "the number of a site of two pieces stands on the larger: {:?}", numbered[0].at);
    // The ground of a site: its pieces within the area, joined (two squares apart stay two; a square that
    // lies wholly within is itself).
    assert_eq!(numbered[0].pieces.len(), 2);
    assert_eq!(numbered[1].pieces.len(), 1);
    let ground: f64 = numbered.iter().take(2).map(|site| strata_engine::polygons_area(&site.pieces).unwrap()).sum();
    assert!((ground - (40.0 * 40.0 + 80.0 * 80.0 + 60.0 * 60.0)).abs() < 1e-6, "{ground}");
    assert!(inside(numbered[1].at, &square(0.0, 0.0, 60.0)));
    assert_eq!((NUMBERED_NEAR_M, UNNAMED), (500.0, "무명"));
}

/// A site that reaches over the 500 m line is numbered, and its number stands on the part within; a site
/// near the survey area but off the map of the sheet has no number (`:757-759`: the paper of the map, cut
/// to the neighbourhood of the survey).
#[test]
fn the_number_stands_on_the_part_within_500_m_and_on_the_sheet() {
    let over = [site("걸친 유적", &[square(600.0, 0.0, 200.0)]), site("가까운 유적", &[square(-200.0, 0.0, 40.0)])];
    let layers = [SheetLayer { kind: "문화유적분포지도", epsg: 5187, sites: &over }];
    let area = numbered_area(&survey_area(), wide_sheet()).unwrap().expect("an area");
    let numbered = number_sites(&layers, &area, 5187).unwrap();
    assert_eq!(listed(&numbered), [(0, "가까운 유적", 1), (0, "걸친 유적", 2)]);
    // The survey area ends 50 m east of its middle; 500 m more is 550 m.
    assert!(numbered[1].at[0] - CENTRE[0] < 550.5 && numbered[1].at[0] - CENTRE[0] > 500.0, "{:?}", numbered[1].at);

    let narrow = [CENTRE[0] - 100.0, CENTRE[1] - 100.0, CENTRE[0] + 100.0, CENTRE[1] + 100.0];
    let on_narrow = numbered_area(&survey_area(), narrow).unwrap().expect("an area");
    assert!(number_sites(&layers, &on_narrow, 5187).unwrap().is_empty(), "both sites lie off this sheet");
}

/// Without a drawn survey area every site on the sheet is numbered (`:198-199`); a sheet that shows nothing
/// near the survey area numbers nothing.
#[test]
fn without_a_survey_area_the_whole_sheet_is_numbered() {
    let sites = [site("먼 사지", &[square(900.0, 0.0, 40.0)]), site("나 고분", &[square(0.0, 0.0, 60.0)])];
    let layers = [SheetLayer { kind: "지정유산", epsg: 5187, sites: &sites }];
    let whole = numbered_area(&[], wide_sheet()).unwrap().expect("the sheet");
    assert_eq!(listed(&number_sites(&layers, &whole, 5187).unwrap()), [(0, "나 고분", 1), (0, "먼 사지", 2)]);

    let far = [CENTRE[0] + 5000.0, CENTRE[1], CENTRE[0] + 6000.0, CENTRE[1] + 1000.0];
    assert!(numbered_area(&survey_area(), far).unwrap().is_none());
}

/// The received layers keep the CRS of their files (EPSG:5179): the sites are tested there, and the place
/// of a number comes back in the work CRS.
#[test]
fn a_layer_in_another_crs_is_numbered_where_it_lies() {
    let to_file = Transformer::new(5187, 5179).unwrap();
    let shape = square(100.0, 100.0, 60.0);
    let in_file: PolygonRings = shape.iter().map(|ring| to_file.transform_all(ring).unwrap()).collect();
    let (near, far) = (site("가 고분", &[in_file]), site("먼 사지", &[square(900.0, 0.0, 40.0).iter().map(|ring| to_file.transform_all(ring).unwrap()).collect()]));
    let sites = [near, far];
    let layers = [SheetLayer { kind: "지정유산", epsg: 5179, sites: &sites }];
    let area = numbered_area(&survey_area(), wide_sheet()).unwrap().expect("an area");
    let numbered = number_sites(&layers, &area, 5187).unwrap();
    assert_eq!(listed(&numbered), [(0, "가 고분", 1)]);
    assert!(inside(numbered[0].at, &shape), "{:?}", numbered[0].at);
    let ground = &numbered[0].pieces;
    assert_eq!(ground.len(), 1);
    assert!(ground[0][0].iter().all(|point| (point[0] - (CENTRE[0] + 100.0)).abs() <= 30.01 && (point[1] - (CENTRE[1] + 100.0)).abs() <= 30.01), "the ground comes back in the work CRS: {ground:?}");
    assert!((strata_engine::polygons_area(ground).unwrap() - 3600.0).abs() < 1.0);
}

/// The sites are read from the copy a layer is drawn from: every row with its name and its shapes, in the
/// CRS of the copy; a row without a name has an empty one.
#[test]
fn the_sites_of_a_layer_are_read_from_its_copy() {
    use strata_engine::{write_new_table, Cell, ColumnKind, Row, ShapeKind, Shapes, TableLayout};
    let dir = tempfile::tempdir().unwrap();
    let copy = dir.path().join("designated_heritage-00.gpkg");
    let layout = TableLayout { table: crate::heritage::TABLE, epsg: 5179, kind: ShapeKind::Polygons, columns: &[(crate::heritage::NAME_COLUMN, ColumnKind::Text), (crate::heritage::CODE_COLUMN, ColumnKind::Text)] };
    let (one, two) = (square(0.0, 0.0, 60.0), square(300.0, 0.0, 40.0));
    let rows = vec![
        Row { shapes: Shapes::Polygons(vec![one.clone(), two.clone()]), cells: vec![Cell::Text("가 산성".to_owned()), Cell::Text("A-1".to_owned())] },
        Row { shapes: Shapes::Polygons(vec![two.clone()]), cells: vec![Cell::Null, Cell::Null] },
    ];
    write_new_table(&copy, &layout, &mut rows.into_iter()).unwrap();

    let (epsg, sites) = read_sites(&copy).unwrap();
    assert_eq!(epsg, 5179);
    assert_eq!(sites.len(), 2);
    assert_eq!((sites[0].0.as_str(), sites[0].1.len()), ("가 산성", 2));
    assert_eq!(sites[0].1[0], one);
    assert_eq!((sites[1].0.as_str(), &sites[1].1), ("", &vec![two]));
    assert!(read_sites(&dir.path().join("없음.gpkg")).is_err());
}
