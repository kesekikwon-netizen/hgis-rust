//! Reading a shapefile as Korean public data comes: Korean column names and values in CP949, no `.cpg`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use gdal::spatial_ref::SpatialRef;
use gdal::vector::{Feature, FieldDefn, Geometry, LayerAccess, LayerOptions, OGRFieldType, OGRwkbGeometryType};
use gdal::DriverManager;

use super::{read_shapefile, read_shapefile_within, shapefile_facts};
use crate::{Cell, ColumnKind, ShapeKind, Shapes};

/// Two made-up sites as squares in EPSG:5179, written in CP949; the `.cpg` GDAL writes beside is removed.
fn korean_shapefile(dir: &Path) -> PathBuf {
    let path = dir.join("가상유적.shp");
    let mut file = DriverManager::get_driver_by_name("ESRI Shapefile").unwrap().create_vector_only(&path).unwrap();
    let crs = SpatialRef::from_epsg(5179).unwrap();
    let layer = file.create_layer(LayerOptions { name: "가상유적", srs: Some(&crs), ty: OGRwkbGeometryType::wkbMultiPolygon, options: Some(&["ENCODING=CP949"]) }).unwrap();
    for (name, kind) in [("유적명", OGRFieldType::OFTString), ("유산코드", OGRFieldType::OFTString), ("면적", OGRFieldType::OFTInteger), ("조사일", OGRFieldType::OFTDate)] {
        FieldDefn::new(name, kind).unwrap().add_to_layer(&layer).unwrap();
    }
    for (name, code, area, x) in [("가상 고분군", "A-1", 120, 1_000_000.0), ("가상 산성", "B-2", 3400, 1_000_500.0)] {
        let mut feature = Feature::new(layer.defn()).unwrap();
        feature.set_field_string(0, name).unwrap();
        feature.set_field_string(1, code).unwrap();
        feature.set_field_integer(2, area).unwrap();
        let wkt = format!("MULTIPOLYGON ((({x} 2000000, {} 2000000, {} 2000100, {x} 2000100, {x} 2000000)))", x + 100.0, x + 100.0);
        feature.set_geometry(Geometry::from_wkt(&wkt).unwrap()).unwrap();
        feature.create(&layer).unwrap();
    }
    drop(file);
    let _ = std::fs::remove_file(dir.join("가상유적.cpg"));
    path
}

/// The file's facts: polygons in EPSG:5179 with the Korean column names; a column of a kind these tables do
/// not hold (a date) is left out instead of spoiling the whole file.
#[test]
fn a_korean_shapefile_says_what_it_holds() {
    let dir = tempfile::tempdir().unwrap();
    let facts = shapefile_facts(&korean_shapefile(dir.path()), "CP949").unwrap();
    assert_eq!((facts.kind, facts.epsg, facts.rows), (ShapeKind::Polygons, Some(5179), 2));
    assert_eq!(facts.columns, [("유적명".to_owned(), ColumnKind::Text), ("유산코드".to_owned(), ColumnKind::Text), ("면적".to_owned(), ColumnKind::Integer)]);
}

/// The rows come with the named columns, in the order asked, their Korean text as it was written.
#[test]
fn a_korean_shapefile_is_read_with_its_text() {
    let dir = tempfile::tempdir().unwrap();
    let mut rows = Vec::new();
    read_shapefile(&korean_shapefile(dir.path()), "CP949", &["유산코드", "유적명"], &mut |row| {
        rows.push(row);
        true
    })
    .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].cells, [Cell::Text("A-1".to_owned()), Cell::Text("가상 고분군".to_owned())]);
    assert_eq!(rows[1].cells, [Cell::Text("B-2".to_owned()), Cell::Text("가상 산성".to_owned())]);
    let Shapes::Polygons(polygons) = &rows[1].shapes else { panic!("polygons") };
    assert_eq!(polygons[0][0][0], [1_000_500.0, 2_000_000.0]);
}

/// Only the rows whose box meets the rectangle are handed over (`QgsFeatureRequest::setFilterRect`, as
/// `CadastralImport.cpp:170` reads the parcels of a whole county); the facts stay those of the whole file.
#[test]
fn a_shapefile_is_read_within_a_rectangle() {
    let dir = tempfile::tempdir().unwrap();
    let file = korean_shapefile(dir.path());
    let mut names = Vec::new();
    let facts = read_shapefile_within(&file, "CP949", &["유적명"], [999_900.0, 1_999_900.0, 1_000_200.0, 2_000_200.0], &mut |row| {
        names.push(row.cells[0].clone());
        true
    })
    .unwrap();
    assert_eq!(names, [Cell::Text("가상 고분군".to_owned())]);
    assert_eq!(facts.rows, 2);
    let mut met = 0;
    read_shapefile_within(&file, "CP949", &[], [0.0, 0.0, 10.0, 10.0], &mut |_| {
        met += 1;
        true
    })
    .unwrap();
    assert_eq!(met, 0, "a rectangle that meets no shape brings no row");
}

/// A column that is not there, and a file that is no shapefile, are refused.
#[test]
fn what_cannot_be_read_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let file = korean_shapefile(dir.path());
    assert!(read_shapefile(&file, "CP949", &["없는 열"], &mut |_| true).is_err());
    assert!(shapefile_facts(&dir.path().join("없음.shp"), "CP949").is_err());
    let text = dir.path().join("note.shp");
    std::fs::write(&text, "not a shapefile").unwrap();
    assert!(shapefile_facts(&text, "CP949").is_err());
}
