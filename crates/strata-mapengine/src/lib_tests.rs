//! The RustGIS engine says where the labels of a Strata reference table go, in a Korean belt CRS.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use super::*;

/// Two squares of 400 m in EPSG:5187: x 219 600 to 220 000 (「Qa」) and x 220 200 to 220 600 (「Kqp」), both
/// y 369 800 to 370 200, written as longitude and latitude (what a GeoJSON file holds).
fn two_units(dir: &Path) -> PathBuf {
    // The corners carried from EPSG:5187 by PROJ (gdaltransform of GDAL 3.12, 2026-11-15).
    let west = "[[129.217178164,35.925499528],[129.221610342,35.925491429],[129.221620402,35.929096381],[129.217188022,35.929104481],[129.217178164,35.925499528]]";
    let east = "[[129.223826431,35.925487318],[129.228258606,35.925478973],[129.228268967,35.929083924],[129.223836590,35.929092269],[129.223826431,35.925487318]]";
    let unit = |ring: &str, symbol: &str, name: &str| format!(r#"{{"type":"Feature","properties":{{"기호":"{symbol}","지층":"{name}"}},"geometry":{{"type":"Polygon","coordinates":[{ring}]}}}}"#);
    let body = format!(r#"{{"type":"FeatureCollection","features":[{},{}]}}"#, unit(west, "Qa", "충적층"), unit(east, "Kqp", "석영반암"));
    let path = dir.join("units.geojson");
    std::fs::write(&path, body).unwrap();
    path
}

fn symbols(file: PathBuf) -> LabelSource {
    let look = LabelLook { column: "기호".to_owned(), size_pt: 10.667, ink: [40, 40, 40, 255], bold: false, halo: Some(([255, 255, 255, 220], 2.27)), closest_scale: None };
    LabelSource { key: "ref_geology".to_owned(), file, table: None, look }
}

/// A view of 900 x 600 points centred between the two units, 2 m to the point.
fn view() -> ViewBox {
    ViewBox { centre: [220_300.0, 370_000.0], metres_per_point: 2.0, width_pt: 900.0, height_pt: 600.0 }
}

fn sorted(mut labels: Vec<PlacedText>) -> Vec<PlacedText> {
    labels.sort_by(|a, b| a.at[0].total_cmp(&b.at[0]));
    labels
}

/// Each unit gets its symbol at its middle, where PROJ puts the unit in the work CRS (to a metre: the
/// engine carries coordinates with another library), with the look that was asked for.
#[test]
fn a_label_stands_in_the_middle_of_its_unit_in_the_work_crs() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = LabelEngine::default();
    let labels = sorted(engine.place(5187, &[symbols(two_units(dir.path()))], &view()).unwrap());
    assert_eq!(labels.iter().map(|l| l.text.as_str()).collect::<Vec<_>>(), ["Qa", "Kqp"]);
    for (label, x) in labels.iter().zip([219_800.0, 220_400.0]) {
        assert!((label.at[0] - x).abs() < 1.0 && (label.at[1] - 370_000.0).abs() < 1.0, "{label:?}");
        assert_eq!((label.ink, label.bold), ([40, 40, 40, 255], false));
        let (edge, width) = label.halo.unwrap();
        assert!(edge == [255, 255, 255, 220] && (width - 2.27).abs() < 0.001, "{label:?}");
        assert!((label.size_pt - 10.667).abs() < 0.001);
    }
}

/// A view that shows only a part of a unit: its label stands on that part, as QGIS writes it.
#[test]
fn a_unit_larger_than_the_view_is_labelled_over_the_part_in_view() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = LabelEngine::default();
    // 200 x 200 points at 1 m around a point 50 m inside the west unit's east edge: the view shows
    // x 219 850 to 220 050, the unit ends at 220 000.
    let close = ViewBox { centre: [219_950.0, 370_000.0], metres_per_point: 1.0, width_pt: 200.0, height_pt: 200.0 };
    let labels = engine.place(5187, &[symbols(two_units(dir.path()))], &close).unwrap();
    assert_eq!(labels.len(), 1, "{labels:?}");
    assert!(labels[0].at[0] > 219_850.0 && labels[0].at[0] < 220_000.0, "{:?}", labels[0]);
}

/// A GeoPackage holds several tables: the labels come from the one that is named, not from the first of
/// the file, and a table that is not there is said.
#[test]
fn labels_come_from_the_named_table_of_a_file() {
    use rustgis_data::io::{read_path, write_path_to, GpkgMode, GpkgTable};
    let dir = tempfile::tempdir().unwrap();
    let (units, _) = read_path(&two_units(dir.path())).unwrap();
    let mut others = units.clone();
    for feature in &mut others.features {
        feature.attrs = vec![Value::Text("X".to_owned()), Value::Text("X".to_owned())];
    }
    let file = dir.path().join("maps.gpkg");
    let table = |name: &str| GpkgTable { mode: GpkgMode::AddTable, name: Some(name.to_owned()) };
    // 「a_first」 comes first in the file's list of tables.
    write_path_to(&others, &file, rustgis_data::raster::Crs::Epsg(5186), &table("a_first")).unwrap();
    write_path_to(&units, &file, rustgis_data::raster::Crs::Epsg(5186), &table("geology_map")).unwrap();

    let named = |name: &str| LabelSource { table: Some(name.to_owned()), ..symbols(file.clone()) };
    let texts = |labels: Vec<PlacedText>| sorted(labels).into_iter().map(|l| l.text).collect::<Vec<_>>();
    let mut engine = LabelEngine::default();
    assert_eq!(texts(engine.place(5187, &[named("geology_map")], &view()).unwrap()), ["Qa", "Kqp"]);
    assert_eq!(texts(engine.place(5187, &[named("a_first")], &view()).unwrap()), ["X", "X"]);
    assert!(engine.place(5187, &[named("soil_map")], &view()).is_err());
}

/// The engine keeps labels apart by the size they are written at. Two units of 400 m by 42 m, one over
/// the other: their symbols stand 21 points apart at 2 m to the point. Letters of 10.667 points with an
/// edge of 2.27 need 10.03 + 2 x (2.27 + 2 of the engine's padding) = 18.6 points: both are written. (Taken
/// as the engine's own unit, a third larger, they would need 23.4 and one would go.)
#[test]
fn labels_are_kept_apart_by_the_size_they_are_written_at() {
    let dir = tempfile::tempdir().unwrap();
    // The corners carried from EPSG:5187 by PROJ (gdaltransform of GDAL 3.12, 2026-11-15): x 219 600 and
    // 220 000, y 369 958, 370 000 and 370 042.
    let lower = "[[129.217182058,35.926923485],[129.221614316,35.926915385],[129.221615372,35.927293905],[129.217183093,35.927302005],[129.217182058,35.926923485]]";
    let upper = "[[129.217183093,35.927302005],[129.221615372,35.927293905],[129.221616428,35.927672425],[129.217184128,35.927680525],[129.217183093,35.927302005]]";
    let unit = |ring: &str, symbol: &str| format!(r#"{{"type":"Feature","properties":{{"기호":"{symbol}"}},"geometry":{{"type":"Polygon","coordinates":[{ring}]}}}}"#);
    let file = dir.path().join("stacked.geojson");
    std::fs::write(&file, format!(r#"{{"type":"FeatureCollection","features":[{},{}]}}"#, unit(lower, "Qa"), unit(upper, "Kqp"))).unwrap();

    let around = ViewBox { centre: [219_800.0, 370_000.0], ..view() };
    let mut labels = LabelEngine::default().place(5187, &[symbols(file)], &around).unwrap();
    labels.sort_by(|a, b| a.at[1].total_cmp(&b.at[1]));
    assert_eq!(labels.iter().map(|l| l.text.as_str()).collect::<Vec<_>>(), ["Qa", "Kqp"]);
    assert!((labels[1].at[1] - labels[0].at[1] - 42.0).abs() < 1.0, "{labels:?}");
}

/// Layers come and go with what is asked for; another work CRS moves the places; what cannot be read or
/// drawn is said.
#[test]
fn the_engine_follows_what_is_asked_for() {
    let dir = tempfile::tempdir().unwrap();
    let file = two_units(dir.path());
    let mut engine = LabelEngine::default();
    assert_eq!(engine.place(5187, &[], &view()).unwrap(), []);
    assert_eq!(engine.place(5187, &[symbols(file.clone())], &view()).unwrap().len(), 2);
    assert_eq!(engine.place(5187, &[], &view()).unwrap(), [], "the layer is gone with its row");

    // A table without the label column has no labels.
    let mut other = symbols(file.clone());
    other.look.column = "없는 열".to_owned();
    assert_eq!(engine.place(5187, &[other], &view()).unwrap(), []);

    // Labels only at 1:5000 or closer: this view is about 1:7559 (2 m to a point of 1/96 inch).
    let mut near = symbols(file.clone());
    near.look.closest_scale = Some(5_000.0);
    assert_eq!(engine.place(5187, &[near.clone()], &view()).unwrap(), []);
    // At 1 m to the point the view shows x 219 850 to 220 750: the east unit and 150 m of the west one.
    let closer = ViewBox { metres_per_point: 1.0, ..view() };
    assert_eq!(engine.place(5187, &[near], &closer).unwrap().len(), 2);

    // The same ground in EPSG:5186, where PROJ puts it (gdaltransform, as above).
    let in_5186 = ViewBox { centre: [400_808.607, 372_265.031], ..view() };
    let moved = sorted(engine.place(5186, &[symbols(file.clone())], &in_5186).unwrap());
    assert_eq!(moved.len(), 2, "{moved:?}");
    for (label, at) in moved.iter().zip([[400_308.466, 372_254.782], [400_908.635, 372_267.081]]) {
        assert!((label.at[0] - at[0]).abs() < 1.5 && (label.at[1] - at[1]).abs() < 1.5, "{label:?} against {at:?}");
    }

    assert!(engine.place(999_999, &[symbols(file)], &view()).is_err());
    assert!(engine.place(5187, &[symbols(dir.path().join("none.gpkg"))], &view()).is_err());
}
