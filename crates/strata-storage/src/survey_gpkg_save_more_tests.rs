//! 조사 저장 시험(2) — 세대 사본 이전(C++ `SurveySchema.cpp:164-227` migrateGenerationCopy),
//! 모르는 표 보존, WAL 사본(`SurveyStorage.cpp:476-489` VACUUM INTO), 잠긴·읽기 전용 원본은 옆
//! 「-저장.gpkg」(:397-453, `SurveyDurability.cpp:64-68`).

use rusqlite::Connection;

use strata_types::error::{StrataError, StrataResult};

use super::survey_gpkg_save_tests::{db, demo_copy, new_feature, rows, table, NEW_POLY};
use crate::session_save::sha256_fingerprint;
use crate::survey_gpkg_read::read_survey_tables;
use crate::survey_gpkg_save::{copy_survey_snapshot, save_survey};

const STAMP: &str = "<GDALMultiDomainMetadata>\n  <Metadata>\n    <MDI key=\"KA_HGIS_SCHEMA_VERSION\">3</MDI>\n  </Metadata>\n</GDALMultiDomainMetadata>\n";

/// 옛 판 demo(감사 항목 없음)를 저장하면 사본에만 3판 항목을 더하고(GDAL CreateField 와 같은 DDL)
/// 판 번호를 찍는다. 값은 그대로, 모르는 표와 qgis_projects 는 바이트까지 그대로.
#[test]
fn save_migrates_the_copy_and_keeps_unknown_tables_byte_for_byte() -> StrataResult<()> {
    let dir = tempfile::tempdir()?;
    let path = demo_copy(dir.path(), "demo.gpkg")?;
    Connection::open(&path).map_err(db)?.execute_batch(
        "CREATE TABLE site_notes (id INTEGER PRIMARY KEY, memo TEXT, raw BLOB);
         INSERT INTO site_notes VALUES (1, '메모', x'00FF10');
         CREATE TABLE qgis_projects (name TEXT PRIMARY KEY, metadata BLOB, content BLOB);
         INSERT INTO qgis_projects VALUES ('demo', x'0102', x'DEADBEEF');",
    ).map_err(db)?;
    let keep = |p| rows(p, "SELECT hex(id) || hex(memo) || hex(raw) FROM site_notes UNION ALL SELECT hex(name) || hex(metadata) || hex(content) FROM qgis_projects");
    let before = keep(&path)?;
    let tables = read_survey_tables(&path)?;
    save_survey(&path, &tables).map_err(|e| StrataError::Storage(format!("{e:?}")))?;

    assert_eq!(keep(&path)?, before);
    assert_eq!(rows(&path, "SELECT sql FROM sqlite_master WHERE name = 'feature_poly'")?, vec![
        r#"CREATE TABLE "feature_poly" ( "fid" INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL, "geom" POLYGON, "kind" TEXT, "period" TEXT, "feature_no" TEXT, "note" TEXT, "layer_no" TEXT, "depth_m" REAL, "top_el" REAL, "bottom_el" REAL, "relation" TEXT, "status" TEXT, "photo" TEXT, "surveyor" TEXT, "surv_date" TEXT, "uid" TEXT, "created_at" TEXT, "updated_at" TEXT)"#,
    ]);
    assert_eq!(rows(&path, "SELECT fid || '|' || kind || '|' || period FROM feature_poly ORDER BY fid")?, vec!["1|주거지|청동기시대", "2|수혈|삼국시대"]);
    assert_eq!(rows(&path, "SELECT metadata FROM gpkg_metadata")?, vec![STAMP]);
    assert_eq!(rows(&path, "SELECT reference_scope || '|' || md_file_id FROM gpkg_metadata_reference")?, vec!["geopackage|1"]);
    assert_eq!(rows(&path, "SELECT table_name || '|' || extension_name FROM gpkg_extensions WHERE extension_name = 'gpkg_metadata'")?,
               vec!["gpkg_metadata|gpkg_metadata", "gpkg_metadata_reference|gpkg_metadata"]);
    Ok(())
}

/// 이미 다른 GDAL 메타데이터가 있으면 덮지 않고 합친다(GDAL 은 이름순으로 다시 쓴다). 더 높은
/// 판 번호는 낮추지 않는다.
#[test]
fn the_stamp_merges_into_existing_gdal_metadata_and_never_lowers_a_newer_one() -> StrataResult<()> {
    let dir = tempfile::tempdir()?;
    for (name, existing, want) in [
        ("merge.gpkg", "    <MDI key=\"OTHER_ITEM\">x</MDI>\n",
         "    <MDI key=\"KA_HGIS_SCHEMA_VERSION\">3</MDI>\n    <MDI key=\"OTHER_ITEM\">x</MDI>\n"),
        ("newer.gpkg", "    <MDI key=\"KA_HGIS_SCHEMA_VERSION\">4</MDI>\n",
         "    <MDI key=\"KA_HGIS_SCHEMA_VERSION\">4</MDI>\n"),
    ] {
        let path = demo_copy(dir.path(), name)?;
        let xml = |items: &str| format!("<GDALMultiDomainMetadata>\n  <Metadata>\n{items}  </Metadata>\n</GDALMultiDomainMetadata>\n");
        Connection::open(&path).map_err(db)?.execute_batch(&format!(
            "CREATE TABLE gpkg_metadata (id INTEGER CONSTRAINT m_pk PRIMARY KEY ASC NOT NULL,md_scope TEXT NOT NULL DEFAULT 'dataset',md_standard_uri TEXT NOT NULL,mime_type TEXT NOT NULL DEFAULT 'text/xml',metadata TEXT NOT NULL DEFAULT '');
             CREATE TABLE gpkg_metadata_reference (reference_scope TEXT NOT NULL,table_name TEXT,column_name TEXT,row_id_value INTEGER,timestamp DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),md_file_id INTEGER NOT NULL,md_parent_id INTEGER);
             INSERT INTO gpkg_metadata VALUES (1, 'dataset', 'http://gdal.org', 'text/xml', '{}');
             INSERT INTO gpkg_metadata_reference (reference_scope, md_file_id) VALUES ('geopackage', 1);",
            xml(existing)
        )).map_err(db)?;
        let tables = read_survey_tables(&path)?;
        save_survey(&path, &tables).map_err(|e| StrataError::Storage(format!("{e:?}")))?;
        assert_eq!(rows(&path, "SELECT metadata FROM gpkg_metadata")?, vec![xml(want)], "{name}");
        assert_eq!(rows(&path, "SELECT CAST(count(*) AS TEXT) FROM gpkg_metadata_reference")?, vec!["1"], "{name}");
    }
    Ok(())
}

/// 사본은 아직 본 파일에 옮기지 않은(WAL) 행까지 담는다. 원본은 바뀌지 않는다.
#[test]
fn the_generation_copy_includes_rows_still_in_the_wal() -> StrataResult<()> {
    let dir = tempfile::tempdir()?;
    let path = demo_copy(dir.path(), "wal.gpkg")?;
    let writer = Connection::open(&path).map_err(db)?;
    writer.execute_batch(
        "PRAGMA journal_mode = WAL; PRAGMA wal_autocheckpoint = 0;
         CREATE TABLE site_notes (id INTEGER PRIMARY KEY, memo TEXT);
         INSERT INTO site_notes VALUES (1, 'WAL 안의 행');",
    ).map_err(db)?;
    let main_before = std::fs::read(&path)?;
    let copy = dir.path().join("copy.gpkg");
    copy_survey_snapshot(&path, &copy).map_err(StrataError::Storage)?;
    assert_eq!(rows(&copy, "SELECT memo FROM site_notes")?, vec!["WAL 안의 행"]);
    assert_eq!(std::fs::read(&path)?, main_before, "the copy must not write the original");
    drop(writer);
    Ok(())
}

/// 원본을 바꿔 끼울 수 없으면(읽기 전용, 이름 바꾸기를 막고 열어 둔 파일) C++ 처럼 40번
/// 다시 해 본 뒤 옆 「<이름>-저장.gpkg」에 저장하고 그 경로를 돌려준다. 원본은 그대로.
#[test]
#[allow(clippy::permissions_set_readonly_false)] // 시험 끝에 임시 파일을 지울 수 있게 되돌린다
fn a_read_only_or_locked_original_is_saved_beside_it() -> StrataResult<()> {
    let dir = tempfile::tempdir()?;
    let ro = demo_copy(dir.path(), "readonly.gpkg")?;
    let locked = demo_copy(dir.path(), "locked.gpkg")?;
    let mut perms = std::fs::metadata(&ro)?.permissions();
    perms.set_readonly(true);
    std::fs::set_permissions(&ro, perms.clone())?;
    #[cfg(windows)]
    let _hold = {
        use std::os::windows::fs::OpenOptionsExt;
        // 읽기·쓰기는 나누지만 지우기(이름 바꾸기)는 막는다.
        std::fs::OpenOptions::new().read(true).share_mode(0x1 | 0x2).open(&locked)?
    };
    for (path, sibling) in [(&ro, "readonly-저장.gpkg"), (&locked, "locked-저장.gpkg")] {
        if cfg!(not(windows)) && path == &locked {
            continue;
        }
        let before = sha256_fingerprint(path)?;
        let mut tables = read_survey_tables(path)?;
        table(&mut tables, "feature_poly").layer.features.push(new_feature(NEW_POLY, &[("kind", "수혈")]));
        let written = save_survey(path, &tables).map_err(|e| StrataError::Storage(format!("{e:?}")))?;
        assert_eq!(written, std::path::absolute(dir.path().join(sibling))?);
        assert_eq!(sha256_fingerprint(path)?, before, "the original stays as it was");
        assert_eq!(rows(&written, "SELECT CAST(count(*) AS TEXT) FROM feature_poly")?, vec!["3"]);
    }
    perms.set_readonly(false);
    std::fs::set_permissions(&ro, perms)?;
    Ok(())
}
