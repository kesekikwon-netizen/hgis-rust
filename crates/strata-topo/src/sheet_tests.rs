//! A small sheet written here: its layers are classed by the code book, its belt is found from where they
//! lie, and the lines of contours, roads, buildings, water and boundaries inside the sheet are the ones
//! that go on the map.

use strata_engine::{DxfShape, Transformer};
use strata_net::ngii_topographic::TopographicSheets;

use super::test_dxf::{drawing, Entity};
use super::*;
use crate::source_crs::{NOT_GRS80, NO_BELT_FITS};

/// Sheet 367154 as drawn in EPSG:5186: `at(fx, fy)` is the point that far across the sheet (0 to 1).
fn at(fx: f64, fy: f64) -> [f64; 2] {
    let e = TopographicSheets::resolve("367154").expect("sheet").extent();
    Transformer::new(4737, 5186).expect("crs").transform([e[0] + (e[2] - e[0]) * fx, e[1] + (e[3] - e[1]) * fy]).expect("point")
}

/// A contour, a road, a wood's edge, a place name, the frame, and a stray line on layer 「0」 far away.
fn sheet_file(dir: &std::path::Path) -> std::path::PathBuf {
    let frame = [at(0.0, 0.0), at(1.0, 0.0), at(1.0, 1.0), at(0.0, 1.0), at(0.0, 0.0)];
    let text = drawing(&[
        Entity::Line("F0017111", Some(120.0), &[at(0.2, 0.2), at(0.5, 0.3), at(0.8, 0.2)]),
        Entity::Line("F0017111", Some(140.0), &[at(0.3, 0.6), at(0.6, 0.7)]),
        Entity::Line("A0013110", None, &[at(0.1, 0.5), at(0.9, 0.5)]),
        Entity::Line("D0010000", None, &[at(0.4, 0.4), at(0.45, 0.45)]),
        Entity::Text("H0040000", at(0.5, 0.5)),
        Entity::Line("H0017334", None, &frame),
        Entity::Line("0", None, &[[0.0, 0.0], [10.0, 10.0]]),
    ]);
    let path = dir.join("367154.dxf");
    std::fs::write(&path, text).expect("write");
    path
}

#[test]
fn the_layers_of_a_sheet_are_classed_and_its_frame_is_kept() {
    let dir = tempfile::tempdir().expect("tempdir");
    let scanned = scan(&sheet_file(dir.path())).expect("scan");
    let got: Vec<(&str, DxfShape, u64, Category, &str)> = scanned.layers.iter().map(|l| (l.cad_layer.as_str(), l.shape, l.count, l.category, l.name.as_str())).collect();
    assert_eq!(
        got,
        [
            ("0", DxfShape::Line, 1, Category::Unknown, "미분류 · 0"),
            ("A0013110", DxfShape::Line, 1, Category::Road, "(기존도로)미분류"),
            ("D0010000", DxfShape::Line, 1, Category::Vegetation, "경지계(미분류)"),
            ("F0017111", DxfShape::Line, 2, Category::Contour, "(볼록지)주곡선"),
            ("H0017334", DxfShape::Line, 1, Category::Facility, "도곽"),
            ("H0040000", DxfShape::Point, 1, Category::PlaceName, "지명(미분류)"),
        ]
    );
    // The contour layer's box holds both contours.
    let contours = &scanned.layers[3];
    let (low, high) = (at(0.2, 0.2), at(0.8, 0.7));
    assert!((contours.extent[0] - low[0]).abs() < 30.0 && (contours.extent[3] - high[1]).abs() < 30.0, "{:?}", contours.extent);
    assert_eq!(scanned.frame.len(), 1);
    assert_eq!((scanned.frame[0].len(), scanned.frame[0][0]), (5, at(0.0, 0.0)));
    assert_eq!(scanned.boxes().len(), 6);

    assert!(scan(&dir.path().join("none.dxf")).is_err());
}

/// The belt is EPSG:5186 (the coordinates fit the sheet in that belt only). The lines of the contours and
/// the road go on the map; the wood's edge (not one of the kinds on the map), the place name (no line), the
/// frame and the stray layer do not, and the stray layer is named to the user.
#[test]
fn a_sheet_is_placed_and_the_lines_for_the_map_are_chosen() {
    let dir = tempfile::tempdir().expect("tempdir");
    let scanned = scan(&sheet_file(dir.path())).expect("scan");
    let placed = place("367154", "GRS80", &scanned, None).expect("placed");
    assert_eq!(placed.epsg, 5186);
    assert_eq!(placed.shown.iter().map(String::as_str).collect::<Vec<_>>(), ["A0013110", "F0017111"]);
    assert_eq!(placed.warnings, ["도곽 밖 또는 범위 미확인 항목: 미분류 · 0"]);
    assert!(placed.evidence.contains("EPSG:5186"));

    // What the solver refuses is refused in its words.
    assert_eq!(place("367154", "BESSEL", &scanned, None), Err(NOT_GRS80.to_owned()));
    assert_eq!(place("378044", "GRS80", &scanned, None), Err(NO_BELT_FITS.to_owned()));
}

/// A sheet none of whose layers lies inside it has nothing for the map.
#[test]
fn a_sheet_with_nothing_inside_it_is_said() {
    // One placing layer, as a point exactly on the sheet's corner grown by the margin: it places the sheet.
    // Every layer the solver flags is left off; with all of them flagged nothing is placed.
    let stray = SheetLayer { cad_layer: "0".to_owned(), shape: DxfShape::Line, count: 1, category: Category::Unknown, name: "미분류 · 0".to_owned(), extent: [0.0, 0.0, 10.0, 10.0] };
    let scanned = Scanned { layers: vec![stray], frame: Vec::new() };
    // Without a placing layer the solver itself refuses.
    assert_eq!(place("367154", "GRS80", &scanned, None), Err(crate::source_crs::NO_PRIMARY_LAYER.to_owned()));
}
