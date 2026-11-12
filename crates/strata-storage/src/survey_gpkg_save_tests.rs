//! 조사 저장 시험 — C++ `SurveyStorage::persistWorkspace`(`SurveyStorage.cpp:788-1010`)·
//! `publishSurveyGeneration`(:510-519). 기대값은 같은 편집을 GDAL(OGR)로 demo.gpkg 사본에 한
//! 결과에서 왔다. demo.gpkg 는 임시 폴더 사본만 쓴다.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use rusqlite::Connection;

use strata_types::error::{StrataError, StrataResult};

use crate::edit_buffer::FeatureRecord;
use crate::gpkg_sql_functions::register_gpkg_sql_functions;
use crate::session_save::sha256_fingerprint;
use crate::survey_gpkg_read::{read_survey_tables, SurveyFileTable};
use crate::survey_gpkg_save::{save_survey, SaveFailure};

pub(crate) const NEW_POLY: &str = "POLYGON ((200100 600100,200130 600100,200130 600140,200100 600100))";

pub(crate) fn demo_copy(dir: &Path, name: &str) -> StrataResult<PathBuf> {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/demo_survey/demo.gpkg");
    let dst = dir.join(name);
    std::fs::copy(src, &dst)?;
    Ok(dst)
}

pub(crate) fn db(e: rusqlite::Error) -> StrataError {
    StrataError::Storage(e.to_string())
}

pub(crate) fn rows(path: &Path, sql: &str) -> StrataResult<Vec<String>> {
    let c = Connection::open(path).map_err(db)?;
    let mut stmt = c.prepare(sql).map_err(db)?;
    let out = stmt.query_map([], |r| r.get::<_, String>(0)).map_err(db)?.collect::<Result<Vec<_>, _>>().map_err(db);
    out
}

pub(crate) fn table<'a>(tables: &'a mut [SurveyFileTable], name: &str) -> &'a mut SurveyFileTable {
    tables.iter_mut().find(|t| t.table == name).expect("table is read")
}

pub(crate) fn new_feature(wkt: &str, attrs: &[(&str, &str)]) -> FeatureRecord {
    let attributes: BTreeMap<String, String> = attrs.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect();
    FeatureRecord { fid: -1, wkt_geom: wkt.to_owned(), attributes }
}

pub(crate) fn leftovers(dir: &Path) -> StrataResult<Vec<String>> {
    Ok(std::fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with(".ka-survey-") || n.contains("-저장"))
        .collect())
}

/// 새 유구 면 하나를 더해 저장하면 GDAL 이 같은 편집을 했을 때와 같다: 그 표만 3건, 새 fid 3,
/// rtree·도형 수·범위·last_change 갱신, 기존 행은 도형 바이트까지 그대로. 같은 파일에 쓴다.
#[test]
fn save_writes_a_new_feature_like_gdal_and_keeps_everything_else() -> StrataResult<()> {
    let dir = tempfile::tempdir()?;
    let path = demo_copy(dir.path(), "demo.gpkg")?;
    let before_blobs = rows(&path, "SELECT hex(geom) FROM feature_poly ORDER BY fid")?;
    let before_change = rows(&path, "SELECT last_change FROM gpkg_contents WHERE table_name = 'feature_poly'")?;
    let mut tables = read_survey_tables(&path)?;
    table(&mut tables, "feature_poly").layer.features.push(new_feature(NEW_POLY, &[("kind", "수혈"), ("period", "삼국")]));

    let written = save_survey(&path, &tables).map_err(|e| StrataError::Storage(format!("{e:?}")))?;
    assert_eq!(written, std::path::absolute(&path)?);
    assert!(leftovers(dir.path())?.is_empty(), "{:?}", leftovers(dir.path())?);

    let counts: Vec<usize> = read_survey_tables(&path)?.iter().map(|t| t.layer.features.len()).collect();
    assert_eq!(counts, vec![1, 3, 1, 1, 2, 1, 1]);
    assert_eq!(rows(&path, "SELECT fid || '|' || kind || '|' || period FROM feature_poly ORDER BY fid")?,
               vec!["1|주거지|청동기시대", "2|수혈|삼국시대", "3|수혈|삼국"]);
    assert_eq!(rows(&path, "SELECT hex(geom) FROM feature_poly WHERE fid < 3 ORDER BY fid")?, before_blobs);
    assert_eq!(rows(&path, "SELECT CAST(feature_count AS TEXT) FROM gpkg_ogr_contents WHERE table_name = 'feature_poly'")?, vec!["3"]);
    let c = Connection::open(&path).map_err(db)?;
    let bbox: [f64; 4] = c.query_row("SELECT minx, maxx, miny, maxy FROM rtree_feature_poly_geom WHERE id = 3", [], |r| {
        Ok([r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?])
    }).map_err(db)?;
    assert!(bbox[0] <= 200_100.0 && bbox[1] >= 200_130.0 && bbox[2] <= 600_100.0 && bbox[3] >= 600_140.0, "{bbox:?}");
    // GDAL 은 쓴 표의 범위를 넓히고 last_change 를 새로 적는다.
    assert_eq!(rows(&path, "SELECT min_x || ',' || min_y || ',' || max_x || ',' || max_y FROM gpkg_contents WHERE table_name = 'feature_poly'")?,
               vec!["200010.0,450010.0,200130.0,600140.0"]);
    assert_ne!(rows(&path, "SELECT last_change FROM gpkg_contents WHERE table_name = 'feature_poly'")?, before_change);
    assert_eq!(rows(&path, "PRAGMA integrity_check")?, vec!["ok"]);
    assert_eq!(rows(&path, "SELECT CAST(application_id AS TEXT) || '|' || CAST(user_version AS TEXT) FROM pragma_application_id, pragma_user_version")?,
               vec!["1196444487|10400"]);
    Ok(())
}

/// 표를 바꾸고 지운 것은 열었을 때(기준)와 다른 것만 쓴다. 기준에 없던 행은 건드리지 않는다.
#[test]
fn changed_and_removed_rows_follow_the_opened_baseline() -> StrataResult<()> {
    let dir = tempfile::tempdir()?;
    let path = demo_copy(dir.path(), "demo.gpkg")?;
    let blob1 = rows(&path, "SELECT hex(geom) FROM feature_poly WHERE fid = 1")?;
    let mut tables = read_survey_tables(&path)?;
    let poly = table(&mut tables, "feature_poly");
    poly.layer.features.retain(|f| f.fid != 2);
    poly.layer.features[0].attributes.insert("kind".to_owned(), "주거지(수정)".to_owned());

    save_survey(&path, &tables).map_err(|e| StrataError::Storage(format!("{e:?}")))?;
    assert_eq!(rows(&path, "SELECT fid || '|' || kind || '|' || period || '|' || feature_no FROM feature_poly")?,
               vec!["1|주거지(수정)|청동기시대|1호"]);
    assert_eq!(rows(&path, "SELECT hex(geom) FROM feature_poly WHERE fid = 1")?, blob1, "unchanged geometry is not rewritten");
    assert_eq!(rows(&path, "SELECT CAST(count(*) AS TEXT) FROM rtree_feature_poly_geom")?, vec!["1"]);
    assert_eq!(rows(&path, "SELECT CAST(feature_count AS TEXT) FROM gpkg_ogr_contents WHERE table_name = 'feature_poly'")?, vec!["1"]);
    Ok(())
}

/// 열어 둔 뒤 다른 곳에서 행이 더해졌으면(기준에 없음) 지우지 않고, 도형 수가 맞지 않아 저장을
/// 멈춘다 — 원본은 그대로.
#[test]
fn rows_added_elsewhere_make_the_save_stop_with_the_original_intact() -> StrataResult<()> {
    let dir = tempfile::tempdir()?;
    let path = demo_copy(dir.path(), "demo.gpkg")?;
    let mut tables = read_survey_tables(&path)?;
    {
        let c = Connection::open(&path).map_err(db)?;
        register_gpkg_sql_functions(&c).map_err(db)?;
        c.execute("INSERT INTO feature_poly (kind) VALUES ('다른 곳')", []).map_err(db)?;
    }
    let before = sha256_fingerprint(&path)?;
    table(&mut tables, "feature_poly").layer.features.retain(|f| f.fid != 2);
    assert!(save_survey(&path, &tables).is_err());
    assert_eq!(sha256_fingerprint(&path)?, before);
    assert!(leftovers(dir.path())?.is_empty());
    Ok(())
}

/// 쓸 수 없는 편집(지원하지 않는 3차원 도형)은 그 레이어 오류로 멈추고 원본은 그대로.
#[test]
fn an_edit_that_cannot_be_written_fails_with_the_layer_and_leaves_the_original() -> StrataResult<()> {
    let dir = tempfile::tempdir()?;
    let path = demo_copy(dir.path(), "demo.gpkg")?;
    let before = sha256_fingerprint(&path)?;
    let mut tables = read_survey_tables(&path)?;
    table(&mut tables, "feature_poly").layer.features.push(new_feature("POLYGON Z ((0 0 1,1 0 1,1 1 1,0 0 1))", &[]));
    match save_survey(&path, &tables) {
        Err(SaveFailure::Layer { table, .. }) => assert_eq!(table, "feature_poly"),
        other => panic!("expected a layer failure, got {other:?}"),
    }
    assert_eq!(sha256_fingerprint(&path)?, before);
    assert!(leftovers(dir.path())?.is_empty());
    Ok(())
}

/// 단일 POLYGON 표에 다중 면을 넣으면 조각마다 따로 저장한다(C++ CLAUDE 규칙: 0 면적 고리 방지).
#[test]
fn a_multipolygon_edit_is_stored_as_separate_features() -> StrataResult<()> {
    let dir = tempfile::tempdir()?;
    let path = demo_copy(dir.path(), "demo.gpkg")?;
    let mut tables = read_survey_tables(&path)?;
    table(&mut tables, "feature_poly").layer.features.push(new_feature(
        "MULTIPOLYGON (((0 0,1 0,1 1,0 0)),((5 5,6 5,6 6,5 5)))",
        &[("kind", "조각")],
    ));
    save_survey(&path, &tables).map_err(|e| StrataError::Storage(format!("{e:?}")))?;
    assert_eq!(rows(&path, "SELECT kind FROM feature_poly WHERE fid > 2 ORDER BY fid")?, vec!["조각", "조각"]);
    assert_eq!(rows(&path, "SELECT hex(substr(geom, 41, 5)) FROM feature_poly WHERE fid > 2")?, vec!["0103000000", "0103000000"]);
    Ok(())
}
