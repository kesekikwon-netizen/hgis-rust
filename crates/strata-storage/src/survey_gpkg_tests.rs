//! 표준 조사 파일 읽기·쓰기 시험. 정답 자료는 C++ 쪽 GDAL 로 만든
//! `samples/demo_survey/demo.gpkg` 다(원본은 읽기만 하고, 쓰는 시험은 임시 폴더 사본으로 한다).

use std::path::PathBuf;

use rusqlite::Connection;
use strata_types::error::StrataResult;
use strata_types::schema::DomainLayerKey as K;

use crate::gpkg_std_sql::{blob_envelope, register_gpkg_functions};
use crate::survey_doc::{FieldValue, SurveyFeature};
use crate::survey_gpkg_read::read_survey_gpkg;
use crate::survey_gpkg_write::{create_standard_survey, save_survey_gpkg};
use crate::wkb_codec::{encode_gpkg_wkb, WkbGeometry};

fn demo_gpkg() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg")
}

fn square(x: f64, y: f64, size: f64) -> WkbGeometry {
    WkbGeometry::Polygon(vec![vec![
        [x, y],
        [x + size, y],
        [x + size, y + size],
        [x, y + size],
        [x, y],
    ]])
}

fn count(conn: &Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |r| r.get(0)).unwrap_or(-1)
}

#[test]
fn reads_cpp_demo_survey_with_all_seven_layers() -> StrataResult<()> {
    let doc = read_survey_gpkg(&demo_gpkg())?;
    assert_eq!(doc.srs_id, 5186);
    let counts: Vec<usize> = K::ALL.iter().map(|k| doc.layer(*k).features.len()).collect();
    // demo.ka-survey.json 의 기대값: 조사구역 1, 유구 면 2, 유구 선 1, 단면선 1, 기준점 2, 유물 1, 시굴 1
    assert_eq!(counts, vec![1, 2, 1, 1, 2, 1, 1]);

    let trench = &doc.layer(K::TrialTrench).features[0];
    assert_eq!(trench.fid, Some(1));
    assert_eq!(trench.text("name"), "TR1");
    assert_eq!(trench.attrs.get("length"), Some(&FieldValue::Real(20.0)));
    assert!(matches!(trench.geom, Some(WkbGeometry::Polygon(_))));

    let gcp = &doc.layer(K::ControlPoints).features[0];
    assert_eq!(gcp.text("point_id"), "GCP1");
    assert_eq!(gcp.geom, Some(WkbGeometry::Point([200_000.0, 450_000.0])));

    let section = &doc.layer(K::SectionLine).features[0];
    assert_eq!(section.text("section_id"), "A-A'");
    assert!(matches!(section.geom, Some(WkbGeometry::LineString(_))));

    // 한글 속성이 깨지지 않고 읽힌다.
    assert_eq!(doc.layer(K::FeaturePoly).features[0].text("period"), "청동기시대");
    assert_eq!(doc.extent(), Some([199_990.0, 200_110.0, 449_990.0, 450_090.0]));
    Ok(())
}

#[test]
fn saving_into_a_cpp_file_keeps_it_standard_and_keeps_untouched_shapes() -> StrataResult<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("demo.gpkg");
    std::fs::copy(demo_gpkg(), &path)?;

    let mut doc = read_survey_gpkg(&path)?;
    let untouched_blob = doc.layer(K::SurveyArea).features[0].source_blob.clone();

    // 새 유구 면 하나, 유물 삭제, 시굴 이름 고치기.
    let mut drawn = SurveyFeature::new_drawn(square(200_050.0, 450_050.0, 5.0));
    drawn.set_text("kind", "수혈");
    doc.add_feature(K::FeaturePoly, drawn);
    doc.layer_mut(K::ArtifactPoint).features.clear();
    doc.layer_mut(K::TrialTrench).features[0].set_text("name", "TR-A");
    save_survey_gpkg(&path, &mut doc)?;

    // 새 도형이 fid 를 받았다.
    assert_eq!(doc.layer(K::FeaturePoly).features[2].fid, Some(3));

    let again = read_survey_gpkg(&path)?;
    assert_eq!(again.layer(K::FeaturePoly).features.len(), 3);
    assert_eq!(again.layer(K::FeaturePoly).features[2].text("kind"), "수혈");
    assert_eq!(again.layer(K::FeaturePoly).features[2].geom, Some(square(200_050.0, 450_050.0, 5.0)));
    assert!(again.layer(K::ArtifactPoint).features.is_empty());
    assert_eq!(again.layer(K::TrialTrench).features[0].text("name"), "TR-A");
    // 건드리지 않은 도형은 바이트까지 그대로다.
    assert_eq!(again.layer(K::SurveyArea).features[0].source_blob, untouched_blob);

    // OGR 이 기대하는 부속 표가 맞게 따라온다: 공간 색인, 도형 수, 범위.
    let conn = Connection::open(&path).map_err(|e| strata_types::error::StrataError::Storage(e.to_string()))?;
    assert_eq!(count(&conn, "SELECT count(*) FROM rtree_feature_poly_geom"), 3);
    assert_eq!(count(&conn, "SELECT count(*) FROM rtree_artifact_point_geom"), 0);
    assert_eq!(count(&conn, "SELECT feature_count FROM gpkg_ogr_contents WHERE table_name='feature_poly'"), 3);
    assert_eq!(count(&conn, "SELECT feature_count FROM gpkg_ogr_contents WHERE table_name='artifact_point'"), 0);
    let max_x: f64 = conn
        .query_row("SELECT max_x FROM gpkg_contents WHERE table_name='feature_poly'", [], |r| r.get(0))
        .unwrap_or(0.0);
    assert!((max_x - 200_055.0).abs() < 1e-9);
    let ok: String = conn.query_row("PRAGMA integrity_check", [], |r| r.get(0)).unwrap_or_default();
    assert_eq!(ok, "ok");
    // 저장 세대에서 스키마 3 의 빠진 열(uid 등)이 생긴다 — C++ `migrateGenerationCopy` 와 같다.
    assert_eq!(count(&conn, "SELECT count(*) FROM pragma_table_info('feature_poly') WHERE name='uid'"), 1);
    // 임시 세대 파일이 남지 않는다.
    assert!(!path.with_extension("gpkg.gen-tmp").exists());
    Ok(())
}

#[test]
fn new_survey_has_seven_standard_tables_and_refuses_to_overwrite() -> StrataResult<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("새조사.gpkg");
    let doc = create_standard_survey(&path, 5187)?;
    assert_eq!(doc.srs_id, 5187);
    assert_eq!(doc.total_features(), 0);

    let conn = Connection::open(&path).map_err(|e| strata_types::error::StrataError::Storage(e.to_string()))?;
    assert_eq!(count(&conn, "SELECT count(*) FROM gpkg_contents WHERE data_type='features'"), 7);
    assert_eq!(count(&conn, "SELECT count(*) FROM gpkg_geometry_columns WHERE srs_id=5187"), 7);
    assert_eq!(count(&conn, "SELECT count(*) FROM gpkg_spatial_ref_sys WHERE srs_id IN (-1,0,4326,5187)"), 4);
    assert_eq!(count(&conn, "PRAGMA application_id"), 1_196_444_487);
    assert_eq!(count(&conn, "SELECT count(*) FROM pragma_table_info('control_points')"), 19);
    drop(conn);

    let reread = read_survey_gpkg(&path)?;
    assert_eq!(reread.srs_id, 5187);
    assert_eq!(reread.total_features(), 0);

    // C++ `SurveyProjectFactory.cpp:44` — 같은 이름 파일이 있으면 덮지 않는다.
    assert!(create_standard_survey(&path, 5187).is_err());
    Ok(())
}

#[test]
fn drawn_shapes_survive_save_and_reopen_in_a_new_survey() -> StrataResult<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("현장.gpkg");
    let mut doc = create_standard_survey(&path, 5187)?;

    let mut area = SurveyFeature::new_drawn(square(219_700.0, 360_900.0, 100.0));
    area.set_text("survey_name", "현장");
    doc.add_feature(K::SurveyArea, area);
    doc.add_feature(
        K::FeatureLine,
        SurveyFeature::new_drawn(WkbGeometry::LineString(vec![[219_710.0, 360_910.0], [219_760.0, 360_950.0]])),
    );
    let mut gcp = SurveyFeature::new_drawn(WkbGeometry::Point([219_705.0, 360_905.0]));
    gcp.set_text("point_id", "GCP-1");
    gcp.attrs.insert("z".to_owned(), FieldValue::Real(48.5));
    doc.add_feature(K::ControlPoints, gcp);
    save_survey_gpkg(&path, &mut doc)?;

    let again = read_survey_gpkg(&path)?;
    assert_eq!(again.non_empty_keys(), vec![K::SurveyArea, K::FeatureLine, K::ControlPoints]);
    assert_eq!(again.layer(K::SurveyArea).features[0].text("survey_name"), "현장");
    assert_eq!(again.layer(K::SurveyArea).features[0].geom, Some(square(219_700.0, 360_900.0, 100.0)));
    assert_eq!(again.layer(K::ControlPoints).features[0].attrs.get("z"), Some(&FieldValue::Real(48.5)));

    // 다시 저장해도 도형이 늘지 않는다.
    let mut second = again.clone();
    save_survey_gpkg(&path, &mut second)?;
    assert_eq!(read_survey_gpkg(&path)?.total_features(), 3);
    Ok(())
}

#[test]
fn st_functions_answer_like_gdal_for_the_rtree_triggers() -> StrataResult<()> {
    let blob = encode_gpkg_wkb(5187, &square(10.0, 20.0, 5.0));
    assert_eq!(blob_envelope(&blob), Some([10.0, 15.0, 20.0, 25.0]));
    assert_eq!(blob_envelope(b"xx"), None);

    let conn = Connection::open_in_memory().map_err(|e| strata_types::error::StrataError::Storage(e.to_string()))?;
    register_gpkg_functions(&conn)?;
    let (min_x, max_y, empty): (f64, f64, bool) = conn
        .query_row("SELECT ST_MinX(?1), ST_MaxY(?1), ST_IsEmpty(?1)", [&blob], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })
        .map_err(|e| strata_types::error::StrataError::Storage(e.to_string()))?;
    assert!((min_x - 10.0).abs() < 1e-12 && (max_y - 25.0).abs() < 1e-12 && !empty);
    Ok(())
}

#[test]
fn a_file_that_is_not_a_survey_is_refused_with_a_plain_message() {
    let dir = tempfile::tempdir().unwrap_or_else(|_| std::process::abort());
    let path = dir.path().join("아님.gpkg");
    let _ = std::fs::write(&path, b"not sqlite");
    assert!(read_survey_gpkg(&path).is_err());
}
