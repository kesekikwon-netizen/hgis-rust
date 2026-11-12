//! 조사 파일 열기(읽기) 시험 — C++ `SurveyStorage.cpp:371-389` validateForOpen 과
//! `LayerOps.cpp:1670-1693`(표 목록)·`:1831-1913`(표 읽기). `samples/demo_survey/demo.gpkg`
//! 는 임시 폴더 사본만 연다.

use std::path::{Path, PathBuf};

use rusqlite::Connection;

use strata_types::crs::WorkCrs;
use strata_types::error::{StrataError, StrataResult};
use strata_types::schema::DomainLayerKey;

use crate::gpkg::{GpkgRepository, SurveyWorkspace};
use crate::session_save::sha256_fingerprint;
use crate::survey_gpkg_read::{read_survey_tables, validate_survey_for_open};

const NOT_GPKG: &str = "읽을 수 있는 GeoPackage 조사 파일이 아닙니다.";
const NOT_INTACT: &str = "조사 파일의 무결성을 확인하지 못했습니다. 현재 작업은 유지됩니다.";

fn demo_copy(dir: &Path, name: &str) -> StrataResult<PathBuf> {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg");
    let dst = dir.join(name);
    std::fs::copy(src, &dst)?;
    Ok(dst)
}

fn db(e: rusqlite::Error) -> StrataError {
    StrataError::Storage(e.to_string())
}

/// C++ 이 연 demo.gpkg 와 같은 표·순서·도형 수·속성을 읽고, 파일은 한 바이트도 바꾸지 않는다.
#[test]
fn reads_every_feature_table_of_a_cpp_survey_without_writing_it() -> StrataResult<()> {
    let dir = tempfile::tempdir()?;
    let path = demo_copy(dir.path(), "demo.gpkg")?;
    let before = sha256_fingerprint(&path)?;

    validate_survey_for_open(&path).map_err(StrataError::Storage)?;
    let tables = read_survey_tables(&path)?;
    let summary: Vec<(String, usize)> =
        tables.iter().map(|t| (t.table.clone(), t.layer.features.len())).collect();
    assert_eq!(
        summary,
        [("survey_area", 1), ("feature_poly", 2), ("feature_line", 1), ("section_line", 1),
         ("control_points", 2), ("artifact_point", 1), ("trial_trench", 1)]
            .map(|(t, n)| (t.to_owned(), n))
            .to_vec()
    );
    let poly = &tables[1].layer;
    assert_eq!(poly.key, Some(DomainLayerKey::FeaturePoly));
    let attrs: Vec<(i64, Option<&str>, Option<&str>)> = poly
        .features
        .iter()
        .map(|f| (f.fid, f.attributes.get("kind").map(String::as_str), f.attributes.get("period").map(String::as_str)))
        .collect();
    assert_eq!(attrs, vec![(1, Some("주거지"), Some("청동기시대")), (2, Some("수혈"), Some("삼국시대"))]);
    assert!(poly.features.iter().all(|f| f.wkt_geom.starts_with("POLYGON ((")), "{:?}", poly.features);
    assert_eq!(poly.features[0].attributes.get("note").map(String::as_str), Some(""), "an empty text stays a value");
    assert_eq!(sha256_fingerprint(&path)?, before, "opening must not write the survey file");
    Ok(())
}

/// gpkg_ 등 관리 표는 건너뛰고, 도메인이 아닌 도형 표(예: user_poly_1)도 읽는다(C++ 은 범례에 올린다).
#[test]
fn non_domain_feature_tables_are_read_and_missing_tables_are_skipped() -> StrataResult<()> {
    let dir = tempfile::tempdir()?;
    let path = demo_copy(dir.path(), "extra.gpkg")?;
    {
        let c = Connection::open(&path).map_err(db)?;
        c.execute_batch(
            "CREATE TABLE user_poly_1 (fid INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL, geom POLYGON, name TEXT, note TEXT);
             INSERT INTO gpkg_contents (table_name, data_type, identifier, srs_id) VALUES ('user_poly_1', 'features', 'user_poly_1', 5186);
             INSERT INTO gpkg_geometry_columns VALUES ('user_poly_1', 'geom', 'POLYGON', 5186, 0, 0);
             INSERT INTO user_poly_1 (geom, name) VALUES (NULL, '1구역');
             INSERT INTO gpkg_contents (table_name, data_type, identifier, srs_id) VALUES ('gone_table', 'features', 'gone_table', 5186);",
        )
        .map_err(db)?;
    }
    let tables = read_survey_tables(&path)?;
    let names: Vec<&str> = tables.iter().map(|t| t.table.as_str()).collect();
    assert_eq!(names.last(), Some(&"user_poly_1"));
    assert!(!names.contains(&"gone_table"), "a listed table that does not exist is skipped");
    let extra = &tables[7].layer;
    assert_eq!(extra.key, None);
    assert_eq!(extra.features[0].attributes.get("name").map(String::as_str), Some("1구역"));
    assert_eq!(extra.features[0].wkt_geom, "");
    assert!(!extra.features[0].attributes.contains_key("note"), "NULL values are not attributes");

    // 2A 전 Rust 형식 파일: gpkg_contents 에만 있고 표는 없다 -> 읽을 표 없음(오류 아님).
    let old = dir.path().join("old_rust.gpkg");
    let mut ws = SurveyWorkspace {
        schema_version: 3,
        work_crs: WorkCrs::Epsg5187,
        layers: std::collections::BTreeMap::new(),
        embedded_project_json: String::from("{}"),
    };
    ws.ensure_domain_layer(DomainLayerKey::SurveyArea);
    GpkgRepository::write_workspace_file(&old, &ws)?;
    assert!(read_survey_tables(&old)?.is_empty());
    Ok(())
}

/// C++ validateForOpen 문구 그대로: GeoPackage 가 아니면 첫 문구, 무결성이 깨졌으면 둘째 문구.
#[test]
fn validate_rejects_non_gpkg_and_damaged_files_with_cpp_messages() -> StrataResult<()> {
    let dir = tempfile::tempdir()?;
    let text = dir.path().join("memo.gpkg");
    std::fs::write(&text, b"not a geopackage")?;
    assert_eq!(validate_survey_for_open(&text), Err(NOT_GPKG.to_owned()));

    let plain = dir.path().join("plain.gpkg");
    Connection::open(&plain).map_err(db)?.execute_batch("CREATE TABLE t (a);").map_err(db)?;
    assert_eq!(validate_survey_for_open(&plain), Err(NOT_GPKG.to_owned()));

    // 시굴격자 표 첫 쪽의 종류 바이트를 지워 무결성만 깨뜨린다(gpkg_contents 는 멀쩡하다).
    let damaged = demo_copy(dir.path(), "damaged.gpkg")?;
    let (root, page): (i64, i64) = {
        let c = Connection::open(&damaged).map_err(db)?;
        let root = c
            .query_row("SELECT rootpage FROM sqlite_master WHERE name = 'trial_trench'", [], |r| r.get(0))
            .map_err(db)?;
        let page = c.query_row("PRAGMA page_size", [], |r| r.get(0)).map_err(db)?;
        (root, page)
    };
    let mut bytes = std::fs::read(&damaged)?;
    let at = usize::try_from((root - 1) * page).map_err(|_| StrataError::Storage("offset".into()))?;
    bytes[at] = 0;
    std::fs::write(&damaged, bytes)?;
    assert_eq!(validate_survey_for_open(&damaged), Err(NOT_INTACT.to_owned()));
    Ok(())
}

/// 지도 라벨이 고를 항목(C++ `LayerOps::detectNameField` 는 항목 순서·글자 형식으로 고른다)을 위해
/// 표의 항목을 표 순서 그대로(fid·도형 칸 빼고) 선언 형식과 함께, 도형 형식 이름과 함께 읽는다.
#[test]
fn reads_column_order_types_and_geometry_type() -> StrataResult<()> {
    let dir = tempfile::tempdir()?;
    let path = demo_copy(dir.path(), "demo.gpkg")?;
    let tables = read_survey_tables(&path)?;
    let cp = tables.iter().find(|t| t.table == "control_points").ok_or_else(|| StrataError::Storage("cp".into()))?;
    let cols: Vec<(&str, &str)> = cp.columns.iter().map(|(n, t)| (n.as_str(), t.as_str())).collect();
    assert_eq!(&cols[..4], &[("point_id", "TEXT"), ("x", "REAL"), ("y", "REAL"), ("z", "REAL")]);
    assert_eq!(cols.len(), 14);
    assert_eq!(cp.geometry_type, "POINT");
    let kinds: Vec<(&str, &str)> = tables.iter().map(|t| (t.table.as_str(), t.geometry_type.as_str())).collect();
    assert_eq!(kinds, vec![
        ("survey_area", "POLYGON"), ("feature_poly", "POLYGON"), ("feature_line", "LINESTRING"),
        ("section_line", "LINESTRING"), ("control_points", "POINT"), ("artifact_point", "POINT"),
        ("trial_trench", "POLYGON"),
    ]);
    Ok(())
}
