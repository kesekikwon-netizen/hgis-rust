//! 조사 파일에 조사구역 표 하나 더 — C++ `LayerOps::createSurveyAreaLayer`(LayerOps.cpp:1446-1532)가
//! QgsVectorFileWriter(GDAL GPKG)로 쓰는 새 표: 도형 POLYGON, name·survey_id(TEXT), area_m2(REAL), note(TEXT).
//! 표·공간 색인·트리거 SQL 은 새 조사와 같은 GDAL 순서(`survey_gpkg_ddl::layer_sql_named`).
//! C++ 은 같은 이름 표를 덮어쓰지만(CreateOrOverwriteLayer) 여기서는 이미 있는 표를 덮지 않고 실패한다 — 사용자 자료 보호.

use std::path::Path;

use rusqlite::Connection;

use crate::survey_gpkg_ddl::{layer_sql_named, SurveyFieldDef, GPKG_EXTENSIONS_DDL};

/// 새 조사구역 표의 항목(C++ 메모리 레이어 fields 차례).
pub const SURVEY_AREA_EXTRA_FIELDS: [SurveyFieldDef; 4] = [
    SurveyFieldDef { name: "name", real: false },
    SurveyFieldDef { name: "survey_id", real: false },
    SurveyFieldDef { name: "area_m2", real: true },
    SurveyFieldDef { name: "note", real: false },
];

/// `path` 조사 파일에 `table` 조사구역 표를 만든다(좌표계 `srs_id`).
///
/// # Errors
/// 같은 이름 표가 이미 있거나 파일에 쓰지 못함(글은 사용자에게 보일 까닭).
pub fn add_survey_area_table(path: &Path, table: &str, srs_id: u32) -> Result<(), String> {
    let mut conn = Connection::open(path).map_err(|e| e.to_string())?;
    let exists = |conn: &Connection, name: &str| -> rusqlite::Result<bool> {
        conn.query_row("SELECT count(*) FROM sqlite_master WHERE name = ?1", [name], |r| r.get::<_, i64>(0)).map(|n| n > 0)
    };
    if exists(&conn, table).map_err(|e| e.to_string())? {
        return Err(format!("같은 이름의 표({table})가 이미 있습니다."));
    }
    let first = !exists(&conn, "gpkg_extensions").map_err(|e| e.to_string())?;
    let mut sql = layer_sql_named(table, "POLYGON", &SURVEY_AREA_EXTRA_FIELDS, srs_id, false);
    if first {
        sql.insert_str(0, GPKG_EXTENSIONS_DDL);
    }
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute_batch(&sql).map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    /// 새 조사에 survey_area_2 를 더하면 읽기에 그 표가 나오고(항목·좌표계·형식), 같은 이름은 다시 만들지 않는다.
    #[test]
    fn adds_a_survey_area_table_once() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("새조사.gpkg");
        crate::gpkg::GpkgRepository::create_new_survey(&path, strata_types::crs::WorkCrs::Epsg5187).expect("create");
        add_survey_area_table(&path, "survey_area_2", 5187).expect("add");
        let tables = crate::survey_gpkg_read::read_survey_tables(&path).expect("read");
        let t = tables.iter().find(|t| t.table == "survey_area_2").expect("new table");
        assert_eq!((t.srs_id, t.geometry_type.as_str()), (5187, "POLYGON"));
        let cols: Vec<&str> = t.columns.iter().map(|(c, _)| c.as_str()).collect();
        assert_eq!(cols, vec!["name", "survey_id", "area_m2", "note"]);
        assert!(add_survey_area_table(&path, "survey_area_2", 5187).is_err());
        assert!(add_survey_area_table(&path, "survey_area", 5187).is_err(), "never overwrites the default table");
    }
}
