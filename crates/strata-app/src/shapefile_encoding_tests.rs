//! A CP949 `.dbf` shows the broken text, then the Korean name after 「깨진 한글 바로잡기」.
//! `LayerOps.cpp:467-484`, `MainWindowContextMenus.cpp:350-356`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use gdal::vector::{
    Feature, FieldDefn, Geometry, LayerAccess, LayerOptions, OGRFieldType, OGRwkbGeometryType,
};
use gdal::DriverManager;
use strata_types::crs::WorkCrs;
use strata_types::schema::LegendGroup;

use crate::app_state::StrataAppState;
use crate::menu_popup::MenuPick;
use crate::ref_vector::data::{LoadSpec, Look, PolygonFill};

/// One square and a Korean name written as CP949. The `.cpg` is then UTF-8, so the first read is wrong.
fn korean_square(dir: &Path) -> PathBuf {
    let path = dir.join("유적.shp");
    {
        let mut file = DriverManager::get_driver_by_name("ESRI Shapefile")
            .unwrap()
            .create_vector_only(&path)
            .unwrap();
        let layer = file
            .create_layer(LayerOptions {
                name: "유적",
                srs: None,
                ty: OGRwkbGeometryType::wkbPolygon,
                options: Some(&["ENCODING=CP949"]),
            })
            .unwrap();
        FieldDefn::new("name", OGRFieldType::OFTString)
            .unwrap()
            .add_to_layer(&layer)
            .unwrap();
        let mut feature = Feature::new(layer.defn()).unwrap();
        feature.set_field_string(0, "가상 고분군").unwrap();
        feature
            .set_geometry(Geometry::from_wkt("POLYGON ((200000 450000, 200100 450000, 200100 450100, 200000 450100, 200000 450000))").unwrap())
            .unwrap();
        feature.create(&layer).unwrap();
    }
    std::fs::write(path.with_extension("cpg"), "UTF-8\n").unwrap();
    path
}

fn app_with(path: &Path) -> (StrataAppState, String) {
    let mut app = StrataAppState::new_startup();
    app.work_crs = WorkCrs::Epsg5187;
    let spec = LoadSpec {
        file: path.to_path_buf(),
        table: String::new(),
        work_epsg: 5187,
        look: Look::Polygons {
            stroke: [0, 0, 0, 255],
            width_mm: 0.2,
            fill: PolygonFill::None,
        },
        label: None,
    };
    app.show_user_vector_layer("user:유적", "유적", spec);
    (app, "user:유적".to_owned())
}

#[test]
fn choosing_cp949_rewrites_the_cpg_and_the_label_becomes_korean() {
    let dir = tempfile::tempdir().unwrap();
    let path = korean_square(dir.path());
    let (mut app, key) = app_with(&path);
    let before = app.shapefile_label_text(&key);
    assert_ne!(
        before,
        vec!["가상 고분군".to_owned()],
        "opened as UTF-8, the CP949 bytes are not the Korean name yet: {before:?}"
    );
    let input = app.layer_menu_input(&key).unwrap();
    assert!(input.shp, "a .shp row offers 깨진 한글 바로잡기");
    assert!(app
        .left_layers
        .entries
        .iter()
        .any(|e| e.layer_key == key && e.group == LegendGroup::SurveyData));
    app.run_layer_menu_action(
        Some(&key),
        &MenuPick {
            id: "layer.encodingValue",
            value: "CP949".to_owned(),
        },
    );
    assert_eq!(
        std::fs::read_to_string(path.with_extension("cpg")).unwrap(),
        "CP949\n"
    );
    assert_eq!(
        app.shapefile_label_text(&key),
        vec!["가상 고분군".to_owned()]
    );
}

#[test]
fn each_menu_value_is_written_to_the_cpg_as_cpp_does() {
    let dir = tempfile::tempdir().unwrap();
    let path = korean_square(dir.path());
    let (mut app, key) = app_with(&path);
    for value in ["CP949", "EUC-KR", "UTF-8", "System"] {
        app.run_layer_menu_action(
            Some(&key),
            &MenuPick {
                id: "layer.encodingValue",
                value: value.to_owned(),
            },
        );
        assert_eq!(
            std::fs::read_to_string(path.with_extension("cpg")).unwrap(),
            format!("{value}\n"),
            "{value}"
        );
    }
}
