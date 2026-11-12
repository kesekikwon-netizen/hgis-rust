//! 세대 사본 이전 — C++ `SurveySchema::migrateGenerationCopy`(`SurveySchema.cpp:164-227`).
//!
//! 사본에 있는 도메인 표에만 빠진 3판 항목을 더하고(없는 표는 그대로 없다), 판 번호를 찍는다.
//! 값은 바꾸거나 지우지 않고, 더 높은 판 번호는 낮추지 않는다. GDAL 이 같은 일을 할 때와 같은
//! 결과(`ALTER TABLE ADD COLUMN`, GDAL 데이터셋 메타데이터)를 쓴다. 실패는 부르는 쪽이 적어 두고
//! 저장은 이어 간다(C++ 과 같다).

use rusqlite::{Connection, OptionalExtension};

use crate::survey_gpkg_ddl::{GPKG_EXTENSIONS_DDL, GPKG_METADATA_DDL, SURVEY_LAYERS, SURVEY_SCHEMA_VERSION};

const STAMP_KEY: &str = "KA_HGIS_SCHEMA_VERSION";

fn table_exists(conn: &Connection, name: &str) -> rusqlite::Result<bool> {
    conn.query_row(
        "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
        [name],
        |r| r.get::<_, i64>(0),
    )
    .map(|n| n > 0)
}

/// 사본 하나를 이전한다. 바꿀 것이 없으면 아무 것도 쓰지 않는다.
pub(crate) fn migrate_generation_copy(conn: &Connection) -> Result<(), String> {
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    for layer in &SURVEY_LAYERS {
        if !table_exists(&tx, layer.id).map_err(|e| e.to_string())? {
            continue; // 처음 쓸 때 만드는 표는 없는 채로 둔다
        }
        let mut stmt = tx
            .prepare(&format!("SELECT name FROM pragma_table_info('{}')", layer.id))
            .map_err(|e| e.to_string())?;
        let have: Vec<String> = stmt
            .query_map([], |r| r.get(0))
            .map_err(|e| e.to_string())?
            .collect::<Result<_, _>>()
            .map_err(|e| e.to_string())?;
        for f in layer.fields.iter().filter(|f| !have.iter().any(|h| h.eq_ignore_ascii_case(f.name))) {
            let ty = if f.real { "REAL" } else { "TEXT" };
            tx.execute_batch(&format!(r#"ALTER TABLE "{}" ADD COLUMN "{}" {ty}"#, layer.id, f.name))
                .map_err(|e| format!("{} 표에 {} 항목을 더하지 못했습니다: {e}", layer.id, f.name))?;
        }
    }
    tx.commit().map_err(|e| e.to_string())?;
    // C++ 도 항목 추가를 확정한 뒤 판 번호를 따로 적는다(:205-219).
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    stamp_version(&tx).map_err(|e| format!("조사 파일 사본에 판 번호를 적지 못했습니다: {e}"))?;
    tx.commit().map_err(|e| e.to_string())
}

/// GDAL 데이터셋 메타데이터(KA_HGIS_SCHEMA_VERSION)를 3 으로 — 없으면 2A 새 조사와 같은 표·행을
/// 만들고, 있으면 기본 영역의 항목에 합쳐 GDAL 처럼 이름순으로 다시 쓴다.
fn stamp_version(conn: &Connection) -> Result<(), String> {
    let found: Option<(i64, String)> = if table_exists(conn, "gpkg_metadata").map_err(|e| e.to_string())? {
        conn.query_row(
            "SELECT m.id, m.metadata FROM gpkg_metadata m JOIN gpkg_metadata_reference r ON r.md_file_id = m.id
             WHERE r.reference_scope = 'geopackage' AND m.md_standard_uri = 'http://gdal.org' LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?
    } else {
        None
    };
    let Some((id, xml)) = found else {
        return create_stamp(conn).map_err(|e| e.to_string());
    };
    let Some(mut items) = default_domain_items(&xml) else {
        return Err("GDAL 메타데이터에 기본 영역이 하나가 아니라 판 번호를 건너뜁니다".to_owned());
    };
    let current = items.iter().find(|(k, _)| k == STAMP_KEY).and_then(|(_, v)| v.parse::<u32>().ok());
    if current.is_some_and(|v| v >= SURVEY_SCHEMA_VERSION) {
        return Ok(());
    }
    items.retain(|(k, _)| k != STAMP_KEY);
    items.push((STAMP_KEY.to_owned(), SURVEY_SCHEMA_VERSION.to_string()));
    items.sort();
    conn.execute("UPDATE gpkg_metadata SET metadata = ?1 WHERE id = ?2", rusqlite::params![render(&items), id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// 메타데이터가 없던 파일: 2A 새 조사와 같은 표·확장 행·메타데이터 행·참조 행을 만든다.
fn create_stamp(conn: &Connection) -> rusqlite::Result<()> {
    if !table_exists(conn, "gpkg_extensions")? {
        conn.execute_batch(GPKG_EXTENSIONS_DDL)?;
    }
    if !table_exists(conn, "gpkg_metadata")? {
        conn.execute_batch(GPKG_METADATA_DDL)?;
    }
    for t in ["gpkg_metadata", "gpkg_metadata_reference"] {
        conn.execute(
            "INSERT INTO gpkg_extensions SELECT ?1, NULL, 'gpkg_metadata', 'http://www.geopackage.org/spec120/#extension_metadata', 'read-write'
             WHERE NOT EXISTS (SELECT 1 FROM gpkg_extensions WHERE table_name = ?1 AND extension_name = 'gpkg_metadata')",
            [t],
        )?;
    }
    conn.execute(
        "INSERT INTO gpkg_metadata (md_scope, md_standard_uri, mime_type, metadata) VALUES ('dataset', 'http://gdal.org', 'text/xml', ?1)",
        [render(&[(STAMP_KEY.to_owned(), SURVEY_SCHEMA_VERSION.to_string())])],
    )?;
    let new_id = conn.last_insert_rowid();
    conn.execute(
        "INSERT INTO gpkg_metadata_reference (reference_scope, md_file_id) VALUES ('geopackage', ?1)",
        [new_id],
    )?;
    Ok(())
}

/// 기본 영역(`<Metadata>`) 하나만 있는 GDAL 메타데이터의 항목들. 다른 꼴이면 None.
fn default_domain_items(xml: &str) -> Option<Vec<(String, String)>> {
    if xml.matches("<Metadata").count() != 1 {
        return None;
    }
    let body = xml.split_once("<Metadata>")?.1.split_once("</Metadata>")?.0;
    let mut items = Vec::new();
    for part in body.split("<MDI key=\"").skip(1) {
        let (key, rest) = part.split_once("\">")?;
        let (value, _) = rest.split_once("</MDI>")?;
        items.push((key.to_owned(), value.to_owned()));
    }
    Some(items)
}

/// GDAL `GDALMultiDomainMetadata` 글(2칸·4칸 들여쓰기, 줄 끝 LF).
fn render(items: &[(String, String)]) -> String {
    let mut s = String::from("<GDALMultiDomainMetadata>\n  <Metadata>\n");
    for (k, v) in items {
        s.push_str(&format!("    <MDI key=\"{k}\">{v}</MDI>\n"));
    }
    s.push_str("  </Metadata>\n</GDALMultiDomainMetadata>\n");
    s
}
