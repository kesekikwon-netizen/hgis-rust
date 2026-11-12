//! 조사 파일 열기 — C++ `SurveyStorage::validateForOpen`(`SurveyStorage.cpp:371-389`)과
//! `LayerOps::addNonEmptySavedGpkgLayers` 가 읽는 표 목록(`LayerOps.cpp:1670-1693`).
//!
//! 파일은 읽기 전용으로만 연다 — 열기는 조사 파일을 바꾸지 않는다(C++ 도 열 때 이전하지 않는다,
//! `SurveySchema.h` migrateGenerationCopy 설명).

use std::collections::BTreeMap;
use std::path::Path;

use rusqlite::types::ValueRef;
use rusqlite::{Connection, OpenFlags};

use strata_types::error::{StrataError, StrataResult};
use strata_types::schema::DomainLayerKey;

use crate::edit_buffer::{FeatureRecord, LayerEditBuffer};
use crate::gpkg_geometry::gpkg_blob_to_wkt;

/// 조사 파일의 도형 표 하나(gpkg_contents 순서)와 그 도형들. 도메인이 아닌 표는 `layer.key` 가 None.
/// `baseline` 은 열었을 때(저장한 뒤)의 도형 — 저장은 `layer.features` 와 이것이 다른 것만 쓴다.
#[derive(Debug, Clone)]
pub struct SurveyFileTable {
    pub table: String,
    /// gpkg_geometry_columns 의 좌표계 번호(새 도형을 쓸 때 머리말에 넣는다).
    pub srs_id: i32,
    /// gpkg_geometry_columns 의 도형 형식 이름(POLYGON·LINESTRING·POINT…).
    pub geometry_type: String,
    /// fid·도형 칸을 뺀 항목(이름, 선언 형식) — 표 순서 그대로. C++ detectNameField 가 순서·형식으로 고른다.
    pub columns: Vec<(String, String)>,
    pub layer: LayerEditBuffer,
    pub baseline: Vec<FeatureRecord>,
}

fn open_read_only(path: &Path) -> rusqlite::Result<Connection> {
    Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX)
}

/// C++ `SurveyStorage.cpp:371-389` — GPKG 로 열리고 `PRAGMA quick_check` 가 ok 인지. 오류 문구는 C++ 그대로.
pub fn validate_survey_for_open(path: &Path) -> Result<(), String> {
    let not_gpkg = || "읽을 수 있는 GeoPackage 조사 파일이 아닙니다.".to_owned();
    let conn = open_read_only(path).map_err(|_| not_gpkg())?;
    let has_contents: i64 = conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'gpkg_contents'",
            [],
            |r| r.get(0),
        )
        .map_err(|_| not_gpkg())?;
    if has_contents != 1 {
        return Err(not_gpkg());
    }
    match conn.query_row("PRAGMA quick_check", [], |r| r.get::<_, String>(0)) {
        Ok(s) if s == "ok" => Ok(()),
        _ => Err("조사 파일의 무결성을 확인하지 못했습니다. 현재 작업은 유지됩니다.".to_owned()),
    }
}

/// C++ `LayerOps.cpp:1643-1648` kaIsGpkgMetadataTable.
fn is_metadata_table(name: &str) -> bool {
    let n = name.to_lowercase();
    ["gpkg_", "rtree_", "sqlite_", "qgis_"].iter().any(|p| n.starts_with(p)) || n == "layer_styles"
}

fn db(e: rusqlite::Error) -> StrataError {
    StrataError::Storage(format!("조사 파일을 읽지 못했습니다: {e}"))
}

/// gpkg_contents 의 도형 표(rowid 순)를 모두 읽는다. 목록에만 있고 실제 표가 없으면 건너뛴다
/// (C++ 은 그런 표를 열지 못해 넘어간다). NULL 값은 속성에 넣지 않는다.
pub fn read_survey_tables(path: &Path) -> StrataResult<Vec<SurveyFileTable>> {
    let conn = open_read_only(path).map_err(db)?;
    let mut stmt = conn
        .prepare(
            "SELECT c.table_name, g.column_name, g.srs_id, g.geometry_type_name FROM gpkg_contents c
             LEFT JOIN gpkg_geometry_columns g ON g.table_name = c.table_name
             WHERE c.data_type = 'features' ORDER BY c.rowid",
        )
        .map_err(db)?;
    type Listed = (String, Option<String>, Option<i32>, Option<String>);
    let listed: Vec<Listed> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
        .map_err(db)?
        .collect::<Result<_, _>>()
        .map_err(db)?;

    let mut out = Vec::new();
    for (table, geom_col, srs_id, geometry_type) in listed {
        if table.is_empty() || is_metadata_table(&table) {
            continue;
        }
        let exists: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                [&table],
                |r| r.get(0),
            )
            .map_err(db)?;
        if exists == 0 {
            continue;
        }
        let layer = read_table(&conn, &table, geom_col.as_deref())?;
        let baseline = layer.features.clone();
        let columns = read_columns(&conn, &table, geom_col.as_deref())?;
        out.push(SurveyFileTable {
            table,
            srs_id: srs_id.unwrap_or(0),
            geometry_type: geometry_type.unwrap_or_default().to_ascii_uppercase(),
            columns,
            layer,
            baseline,
        });
    }
    Ok(out)
}

/// 표의 항목(이름, 선언 형식)을 표 순서대로 — 기본 키(fid)와 도형 칸은 뺀다.
fn read_columns(conn: &Connection, table: &str, geom_col: Option<&str>) -> StrataResult<Vec<(String, String)>> {
    let mut stmt = conn
        .prepare("SELECT name, type, pk FROM pragma_table_info(?1) ORDER BY cid")
        .map_err(db)?;
    let rows: Vec<(String, String, i64)> = stmt
        .query_map([table], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .map_err(db)?
        .collect::<Result<_, _>>()
        .map_err(db)?;
    Ok(rows
        .into_iter()
        .filter(|(name, _, pk)| *pk == 0 && Some(name.as_str()) != geom_col)
        .map(|(name, ty, _)| (name, ty))
        .collect())
}

fn read_table(conn: &Connection, table: &str, geom_col: Option<&str>) -> StrataResult<LayerEditBuffer> {
    let quoted = format!("\"{}\"", table.replace('"', "\"\""));
    let fid_col: String = conn
        .query_row(&format!("SELECT name FROM pragma_table_info('{}') WHERE pk = 1", table.replace('\'', "''")), [], |r| r.get(0))
        .map_err(db)?;
    let mut stmt = conn.prepare(&format!("SELECT * FROM {quoted}")).map_err(db)?;
    let names: Vec<String> = stmt.column_names().iter().map(|s| (*s).to_owned()).collect();
    let mut rows = stmt.query([]).map_err(db)?;
    let mut layer = DomainLayerKey::from_id(table).map_or_else(LayerEditBuffer::default, LayerEditBuffer::new);
    while let Some(row) = rows.next().map_err(db)? {
        let mut fid = 0_i64;
        let mut wkt_geom = String::new();
        let mut attributes = BTreeMap::new();
        for (i, name) in names.iter().enumerate() {
            let value = row.get_ref(i).map_err(db)?;
            if *name == fid_col {
                fid = value.as_i64().map_err(|e| StrataError::Storage(e.to_string()))?;
            } else if Some(name.as_str()) == geom_col {
                if let ValueRef::Blob(b) = value {
                    wkt_geom = gpkg_blob_to_wkt(b).map(|(_, w)| w).unwrap_or_default();
                }
            } else {
                let text = match value {
                    ValueRef::Null | ValueRef::Blob(_) => continue,
                    ValueRef::Integer(v) => v.to_string(),
                    ValueRef::Real(v) => v.to_string(),
                    ValueRef::Text(t) => String::from_utf8_lossy(t).into_owned(),
                };
                attributes.insert(name.clone(), text);
            }
        }
        layer.features.push(FeatureRecord { fid, wkt_geom, attributes });
    }
    Ok(layer)
}
