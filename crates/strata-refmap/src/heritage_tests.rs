//! The rules of the heritage data received from the intranet, and its import near the survey area. The test
//! shapefiles are made here (made-up sites); no received data is used.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use gdal::spatial_ref::SpatialRef;
use gdal::vector::{Feature, FieldDefn, Geometry, LayerAccess, LayerOptions, OGRFieldType, OGRwkbGeometryType};
use gdal::DriverManager;
use strata_engine::{read_table, Cell, ColumnKind, Scope, ShapeKind, Transformer};

use super::*;

fn dataset(name: &str) -> &'static HeritageDataset {
    DATASETS.iter().find(|d| d.name == name).unwrap()
}

/// The six kinds with their colours (`HeritageStyle.cpp:26-33`) and their keys (`HeritageLayoutNumbers.cpp:81-88`);
/// a received archive is named `<kind>_<time>.zip` by the site.
#[test]
fn the_six_kinds_of_heritage_data() {
    let listed: Vec<(&str, &str, [u8; 3])> = DATASETS.iter().map(|d| (d.name, d.key, d.colour)).collect();
    assert_eq!(
        listed,
        [
            ("지정유산", "designated_heritage", [0x8E, 0x44, 0xAD]),
            ("현상변경허용기준", "alteration_standard", [0xE6, 0x7E, 0x22]),
            ("매장유산유존지역", "buried_heritage_area", [0x2E, 0x86, 0xC1]),
            ("문화유적분포지도", "heritage_distribution_map", [0x27, 0xAE, 0x60]),
            ("지표조사구역", "surface_survey_area", [0xD4, 0xAA, 0x00]),
            ("발굴조사구역", "excavation_survey_area", [0xC2, 0x18, 0x7D]),
        ]
    );
    assert_eq!(dataset_of_archive("지정유산_261025084854389.zip").map(|d| d.name), Some("지정유산"));
    assert_eq!(dataset_of_archive("발굴조사구역_261025085158906.ZIP").map(|d| d.name), Some("발굴조사구역"));
    assert_eq!(dataset_of_archive("현상변경허용기준.zip").map(|d| d.name), Some("현상변경허용기준"));
    assert!(dataset_of_archive("수치지도_359012.zip").is_none());
}

/// `LayerOps::prepareShapefileEncoding` (`LayerOps.cpp:359-450`): a `.cpg` decides (UTF-8, the Korean code
/// page under its many names, or whatever else it says); without one the first 32 KB of the `.dbf` decide:
/// UTF-8 only with at least four whole multi-byte characters and no broken one, else CP949.
#[test]
fn the_encoding_of_a_received_shapefile() {
    let dir = tempfile::tempdir().unwrap();
    let shp = dir.path().join("유적.shp");
    let with = |cpg: Option<&str>, dbf: &[u8]| {
        std::fs::write(dir.path().join("유적.dbf"), dbf).unwrap();
        match cpg {
            Some(text) => std::fs::write(dir.path().join("유적.cpg"), text).unwrap(),
            None => drop(std::fs::remove_file(dir.path().join("유적.cpg"))),
        }
        shapefile_encoding(&shp)
    };
    let cp949 = [0xC0, 0xAF, 0xC0, 0xFB, 0xB8, 0xED, 0x20, 0xB0, 0xED, 0xBA, 0xD0, 0xB1, 0xBA];
    assert_eq!(with(Some("UTF-8\n"), &cp949), "UTF-8");
    assert_eq!(with(Some("utf8"), b""), "UTF-8");
    for korean in ["CP949", "949", "EUC-KR", "ANSI 1252", "system"] {
        assert_eq!(with(Some(korean), b""), "CP949", "{korean}");
    }
    assert_eq!(with(Some(" ISO-8859-1 "), b""), "ISO-8859-1");
    assert_eq!(with(None, &cp949), "CP949");
    assert_eq!(with(None, b"NAME      C plain ascii only"), "CP949", "nothing but ASCII: the Korean habit");
    assert_eq!(with(None, "유적명 고분군".as_bytes()), "UTF-8");
    assert_eq!(with(None, "유적".as_bytes()), "CP949", "two characters are too few to be sure");
    assert_eq!(with(Some(""), "유적명 고분군".as_bytes()), "UTF-8", "an empty .cpg says nothing");
}

/// `HeritageImport::chooseNameField` (`HeritageImport.cpp:31-37, 131-157`) with the two kinds that show
/// another column first (`:268-271`): the known names in their order, then the same loosely (case, blanks),
/// then a text column whose name holds 「명」, then the first text column.
#[test]
fn the_column_that_names_a_site() {
    let columns = |names: &[(&str, ColumnKind)]| names.iter().map(|(name, kind)| ((*name).to_owned(), *kind)).collect::<Vec<_>>();
    let text = ColumnKind::Text;
    let designated = columns(&[("유산코드", text), ("지정종목", text), ("국가유산명", text), ("면적", ColumnKind::Integer)]);
    assert_eq!(name_column(&designated, dataset("지정유산")).as_deref(), Some("국가유산명"));
    assert_eq!(name_column(&designated, dataset("현상변경허용기준")).as_deref(), Some("지정종목"), "this kind shows 지정종목 first");
    let project = columns(&[("유산코드", text), ("사업명", text), ("유적명", text)]);
    assert_eq!(name_column(&project, dataset("지표조사구역")).as_deref(), Some("유적명"), "유적명 comes before 사업명 in the list");
    assert_eq!(name_column(&project, dataset("발굴조사구역")).as_deref(), Some("사업명"), "this kind shows 사업명 first");
    assert_eq!(name_column(&columns(&[("유산코드", text), ("명칭", text)]), dataset("문화유적분포지도")).as_deref(), Some("명칭"));
    assert_eq!(name_column(&columns(&[("CODE", text), (" Name ", text)]), dataset("지정유산")).as_deref(), Some(" Name "), "loosely");
    assert_eq!(name_column(&columns(&[("CODE", text), ("구역명", text)]), dataset("지정유산")).as_deref(), Some("구역명"), "holds 명");
    assert_eq!(name_column(&columns(&[("면적", ColumnKind::Real), ("CODE", text)]), dataset("지정유산")).as_deref(), Some("CODE"), "the first text column");
    assert_eq!(name_column(&columns(&[("유적명", ColumnKind::Integer)]), dataset("지정유산")), None, "a number names no site");
}

/// Three made-up sites of 200 m in EPSG:5179 around a survey area of the work CRS (EPSG:5187): one on it, one
/// across the 5 km line, one 9 km away.
fn shapefile_near(dir: &Path, centre_work: [f64; 2]) -> PathBuf {
    let to = Transformer::new(5187, 5179).unwrap();
    let path = dir.join("국가지정유산.shp");
    let mut file = DriverManager::get_driver_by_name("ESRI Shapefile").unwrap().create_vector_only(&path).unwrap();
    let crs = SpatialRef::from_epsg(5179).unwrap();
    let layer = file.create_layer(LayerOptions { name: "국가지정유산", srs: Some(&crs), ty: OGRwkbGeometryType::wkbMultiPolygon, options: Some(&["ENCODING=CP949"]) }).unwrap();
    for name in ["유산코드", "국가유산명"] {
        FieldDefn::new(name, OGRFieldType::OFTString).unwrap().add_to_layer(&layer).unwrap();
    }
    for (code, name, east) in [("A-1", "가상 고분군", 0.0), ("B-2", "가상 산성", 5050.0), ("C-3", "가상 사지", 9000.0)] {
        let [x, y] = to.transform([centre_work[0] + east, centre_work[1]]).unwrap();
        let mut feature = Feature::new(layer.defn()).unwrap();
        feature.set_field_string(0, code).unwrap();
        feature.set_field_string(1, name).unwrap();
        let wkt = format!("MULTIPOLYGON ((({} {}, {} {}, {} {}, {} {}, {} {})))", x - 100.0, y - 100.0, x + 100.0, y - 100.0, x + 100.0, y + 100.0, x - 100.0, y + 100.0, x - 100.0, y - 100.0);
        feature.set_geometry(Geometry::from_wkt(&wkt).unwrap()).unwrap();
        feature.create(&layer).unwrap();
    }
    drop(file);
    let _ = std::fs::remove_file(dir.join("국가지정유산.cpg"));
    path
}

fn names_in(table: &Path) -> Vec<(String, String)> {
    let mut names = Vec::new();
    read_table(table, TABLE, &[NAME_COLUMN, CODE_COLUMN], &mut |row| {
        let text = |cell: &Cell| if let Cell::Text(text) = cell { text.clone() } else { String::new() };
        names.push((text(&row.cells[0]), text(&row.cells[1])));
        true
    })
    .unwrap();
    names
}

/// `SurveyScopeClip::keepIntersectingIfSurvey` (`SurveyScopeClip.cpp:75-132`): the shapes that touch the
/// survey area grown by 5 km are kept whole, the others are left out; the copy that is written holds the
/// site's name and code under fixed column names, in the CRS of the file. Without a survey area nothing is
/// left out (`:89`). A file wholly outside writes no copy.
#[test]
fn the_sites_near_the_survey_area_are_kept_whole() {
    let dir = tempfile::tempdir().unwrap();
    let centre = [220_000.0, 370_000.0];
    let shp = shapefile_near(dir.path(), centre);
    let area = format!("POLYGON (({} {}, {} {}, {} {}, {} {}, {} {}))", centre[0] - 50.0, centre[1] - 50.0, centre[0] + 50.0, centre[1] - 50.0, centre[0] + 50.0, centre[1] + 50.0, centre[0] - 50.0, centre[1] + 50.0, centre[0] - 50.0, centre[1] - 50.0);
    let scope = Scope::around(&[area], NEAR_M).unwrap().unwrap();
    assert!((NEAR_M - 5000.0).abs() < f64::EPSILON);

    let near = dir.path().join("near.gpkg");
    let imported = import_shapefile(&shp, dataset("지정유산"), Some((&scope, 5187)), &near).unwrap();
    assert_eq!((imported.read, imported.kept, imported.kind, imported.epsg, imported.name_column.as_deref()), (3, 2, ShapeKind::Polygons, 5179, Some("국가유산명")));
    assert_eq!(names_in(&near), [("가상 고분군".to_owned(), "A-1".to_owned()), ("가상 산성".to_owned(), "B-2".to_owned())]);

    let all = dir.path().join("all.gpkg");
    assert_eq!(import_shapefile(&shp, dataset("지정유산"), None, &all).unwrap().kept, 3);
    assert_eq!(names_in(&all).len(), 3);

    let far_area = "POLYGON ((100000 100000, 100100 100000, 100100 100100, 100000 100100, 100000 100000))".to_owned();
    let far = Scope::around(&[far_area], NEAR_M).unwrap().unwrap();
    let none = dir.path().join("none.gpkg");
    assert_eq!(import_shapefile(&shp, dataset("지정유산"), Some((&far, 5187)), &none).unwrap().kept, 0);
    assert!(!none.exists(), "nothing near: no copy is written");
    assert!(import_shapefile(&dir.path().join("없음.shp"), dataset("지정유산"), None, &none).is_err());
}
