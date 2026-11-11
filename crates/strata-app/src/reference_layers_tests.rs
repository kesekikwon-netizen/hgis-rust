//! 참조 레이어 시험 — 좌표계 알아내기와 SHP 올리기.

use strata_storage::shp_file_writer::{write_shapefile, ShpField, ShpKind, ShpRecord};
use strata_storage::survey_doc::FieldValue;
use strata_storage::wkb_codec::WkbGeometry;
use strata_types::crs::{CrsCode, WorkCrs};

use super::{detect_prj_crs, load_reference_shp};
use crate::app_state::StrataAppState;

/// GDAL(PROJ)이 내는 ESRI WKT 그대로.
const PRJ_5174: &str = r#"PROJCS["Korean_1985_Modified_Korea_Central_Belt",GEOGCS["GCS_Korean_Datum_1985",DATUM["D_Korean_Datum_1985",SPHEROID["Bessel_1841",6377397.155,299.1528128]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]],PROJECTION["Transverse_Mercator"],PARAMETER["False_Easting",200000.0],PARAMETER["False_Northing",500000.0],PARAMETER["Central_Meridian",127.002890277778],PARAMETER["Scale_Factor",1.0],PARAMETER["Latitude_Of_Origin",38.0],UNIT["Meter",1.0]]"#;
const PRJ_5186: &str = r#"PROJCS["KGD2002_Central_Belt_2010",GEOGCS["GCS_KGD2002",DATUM["D_Korea_Geodetic_Datum_2002",SPHEROID["GRS_1980",6378137.0,298.257222101]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]],PROJECTION["Transverse_Mercator"],PARAMETER["False_Easting",200000.0],PARAMETER["False_Northing",600000.0],PARAMETER["Central_Meridian",127.0],PARAMETER["Scale_Factor",1.0],PARAMETER["Latitude_Of_Origin",38.0],UNIT["Meter",1.0]]"#;

#[test]
fn the_prj_text_tells_which_korean_crs_a_shapefile_uses() {
    assert_eq!(detect_prj_crs(PRJ_5174), Some(CrsCode::Epsg5174));
    assert_eq!(detect_prj_crs(PRJ_5186), Some(CrsCode::Epsg5186));
    assert_eq!(
        detect_prj_crs(&PRJ_5186.replace("127.0]", "129.0]")),
        Some(CrsCode::Epsg5187)
    );
    assert_eq!(
        detect_prj_crs(strata_check_export::submit_survey_package::EPSG_5179_ESRI_WKT),
        Some(CrsCode::Epsg5179)
    );
    assert_eq!(
        detect_prj_crs(r#"GEOGCS["GCS_WGS_1984",DATUM["D_WGS_1984",SPHEROID["WGS_1984",6378137.0,298.257223563]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]]"#),
        Some(CrsCode::Epsg4326)
    );
    assert_eq!(detect_prj_crs(""), None);
}

fn parcel_shp(dir: &std::path::Path, prj: &str) -> std::path::PathBuf {
    let base = dir.join("연속지적도");
    let ring = vec![
        [198_000.0, 350_000.0],
        [198_030.0, 350_000.0],
        [198_030.0, 350_020.0],
        [198_000.0, 350_020.0],
        [198_000.0, 350_000.0],
    ];
    let records = vec![ShpRecord {
        geom: WkbGeometry::Polygon(vec![ring]),
        values: vec![FieldValue::Text("산81 임".to_owned())],
    }];
    write_shapefile(
        &base,
        ShpKind::Polygon,
        &[ShpField { name: "JIBUN".to_owned(), numeric: false }],
        &records,
        prj,
    )
    .expect("SHP 쓰기");
    base.with_extension("shp")
}

#[test]
fn an_old_cadastral_shapefile_lands_in_the_work_crs_with_its_parcel_numbers() {
    let dir = std::env::temp_dir().join(format!("strata_ref_5174_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let shp = parcel_shp(&dir, PRJ_5174);
    let (layer, note) = load_reference_shp(&shp, WorkCrs::Epsg5186, None).expect("읽기");
    assert_eq!(note, "EPSG:5174 → EPSG:5186");
    assert_eq!((layer.name.as_str(), layer.shape_type, layer.shapes.len()), ("연속지적도", 5, 1));
    assert_eq!(layer.shapes[0].label, "산81 임");
    // PROJ: EPSG:5174 (198000, 350000) → EPSG:5186 (198071.2299, 450305.9710)
    let p = layer.shapes[0].parts[0][0];
    assert!((p[0] - 198_071.229_9).abs() < 0.02 && (p[1] - 450_305.971_0).abs() < 0.02, "{p:?}");
}

#[test]
fn dropping_a_shapefile_adds_a_reference_layer_to_the_map_and_the_list() {
    let dir = std::env::temp_dir().join(format!("strata_ref_app_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let shp = parcel_shp(&dir, PRJ_5186);
    let mut app = StrataAppState::new_startup();
    app.work_crs = WorkCrs::Epsg5186;

    assert!(app.import_reference_shp(&shp));
    assert_eq!(app.home_map.reference_layers.len(), 1);
    let entry = app
        .left_layers
        .entries
        .iter()
        .find(|e| e.layer_key == "ref:연속지적도")
        .expect("목록에 줄이 생긴다");
    assert!(entry.read_only && entry.feature_count == 1);
    assert!(!app.is_dirty, "참조 레이어는 조사 데이터가 아니다");
    // 조사가 비어 있으므로 화면이 올린 자료로 간다.
    let c = app.home_map.viewport.center_xy;
    assert!((c[0] - 198_015.0).abs() < 1.0 && (c[1] - 350_010.0).abs() < 1.0, "{c:?}");
    let msg = app.home_map.draw_guide_message.clone().unwrap_or_default();
    assert!(msg.contains("도형 1개") && msg.contains("작업 좌표계와 같음"), "{msg}");

    // 같은 파일을 또 올리면 이름에 번호가 붙는다.
    assert!(app.import_reference_shp(&shp));
    assert_eq!(app.home_map.reference_layers[1].name, "연속지적도 (2)");

    // SHP 가 아닌 파일은 올리지 않고 이유를 알린다.
    let bad = dir.join("아님.shp");
    std::fs::write(&bad, b"not a shapefile").expect("쓰기");
    assert!(!app.import_reference_shp(&bad));
    assert_eq!(app.home_map.reference_layers.len(), 2);
}

#[test]
fn a_downloaded_zip_is_unpacked_and_only_shapes_near_the_survey_are_loaded() {
    use std::io::Write;
    use strata_canvas2d::map_tools::MapToolKind;
    use strata_types::schema::DomainLayerKey;

    let dir = std::env::temp_dir().join(format!("strata_ref_zip_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let shp = parcel_shp(&dir, PRJ_5186); // 필지는 (198000~198030, 350000~350020)
    let zip_path = dir.join("받은지적도.zip");
    {
        let mut w = zip::ZipWriter::new(std::fs::File::create(&zip_path).expect("zip"));
        let opt = zip::write::SimpleFileOptions::default();
        for ext in ["shp", "shx", "dbf", "prj", "cpg"] {
            w.start_file(format!("LSMD_CONT_LDREG.{ext}"), opt).expect("항목");
            w.write_all(&std::fs::read(shp.with_extension(ext)).expect("읽기")).expect("쓰기");
        }
        w.finish().expect("끝");
    }

    // 조사구역이 필지에서 1 km 떨어져 있으면 5 km 안이라 올라온다.
    let mut near = StrataAppState::new_startup();
    near.work_crs = WorkCrs::Epsg5186;
    near.draw_target = DomainLayerKey::SurveyArea;
    let _ = near.commit_drawn_shape(
        MapToolKind::DrawPolygon,
        vec![[199_000.0, 350_000.0], [199_100.0, 350_000.0], [199_100.0, 350_100.0], [199_000.0, 350_100.0]],
    );
    assert_eq!(near.import_reference_zip(&zip_path), 1);
    assert_eq!(near.home_map.reference_layers[0].shapes.len(), 1);
    assert_eq!(near.home_map.reference_layers[0].shapes[0].label, "산81 임");

    // 조사구역이 50 km 떨어져 있으면 올리지 않고 이유를 알린다.
    let mut far = StrataAppState::new_startup();
    far.work_crs = WorkCrs::Epsg5186;
    far.draw_target = DomainLayerKey::SurveyArea;
    let _ = far.commit_drawn_shape(
        MapToolKind::DrawPolygon,
        vec![[250_000.0, 350_000.0], [250_100.0, 350_000.0], [250_100.0, 350_100.0], [250_000.0, 350_100.0]],
    );
    assert_eq!(far.import_reference_zip(&zip_path), 0);
    assert!(far.home_map.reference_layers.is_empty());
    assert!(far
        .home_map
        .draw_guide_message
        .as_deref()
        .unwrap_or("")
        .contains("조사구역 5 km 안에 드는 도형이 없어"));
}
