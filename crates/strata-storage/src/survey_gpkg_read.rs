//! 표준 GeoPackage 조사 파일 읽기 — C++ 앱(GDAL/OGR)이 만든 `.gpkg` 를 그대로 연다.
//!
//! C++ 은 `QgsVectorLayer("…|layername=survey_area", "ogr")` 로 표를 연다. 여기서는
//! SQLite 표를 직접 읽는다: 7개 도메인 표의 fid·지오메트리·모든 속성 열.
//! 파일은 읽기 전용으로만 연다(원본을 건드리지 않는다).

use std::path::Path;

use rusqlite::types::ValueRef;
use rusqlite::{Connection, OpenFlags};

use strata_types::error::{StrataError, StrataResult};
use strata_types::schema::DomainLayerKey;

use crate::gpkg_schema::GPKG_MAGIC_HEADER;
use crate::gpkg_std_sql::table_exists;
use crate::survey_doc::{FieldValue, SurveyDoc, SurveyFeature};
use crate::wkb_codec::decode_gpkg_wkb;

/// 표의 열 구성: 기본 키 열, 지오메트리 열, 나머지 속성 열.
#[derive(Debug, Clone)]
pub(crate) struct TableShape {
    pub pk: String,
    pub geom: String,
    pub attrs: Vec<String>,
}

pub(crate) fn storage_err(what: &str, e: rusqlite::Error) -> StrataError {
    StrataError::Storage(format!("{what}: {e}"))
}

/// `PRAGMA table_info` 와 `gpkg_geometry_columns` 로 표의 열 구성을 알아낸다.
pub(crate) fn table_shape(conn: &Connection, table: &str) -> StrataResult<TableShape> {
    let geom: String = conn
        .query_row(
            "SELECT column_name FROM gpkg_geometry_columns WHERE table_name = ?1",
            [table],
            |r| r.get(0),
        )
        .unwrap_or_else(|_| "geom".to_owned());
    let mut pk = String::from("fid");
    let mut attrs = Vec::new();
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info(\"{table}\")"))
        .map_err(|e| storage_err("표 구조 조회 실패", e))?;
    let rows = stmt
        .query_map([], |r| Ok((r.get::<_, String>(1)?, r.get::<_, i64>(5)?)))
        .map_err(|e| storage_err("표 구조 조회 실패", e))?;
    for row in rows {
        let (name, pk_order) = row.map_err(|e| storage_err("표 구조 조회 실패", e))?;
        if pk_order > 0 {
            pk = name;
        } else if name != geom {
            attrs.push(name);
        }
    }
    Ok(TableShape { pk, geom, attrs })
}

fn field_value(v: ValueRef<'_>) -> FieldValue {
    match v {
        ValueRef::Null => FieldValue::Null,
        ValueRef::Integer(i) => FieldValue::Int(i),
        ValueRef::Real(f) => FieldValue::Real(f),
        ValueRef::Text(t) => FieldValue::Text(String::from_utf8_lossy(t).into_owned()),
        ValueRef::Blob(b) => FieldValue::Blob(b.to_vec()),
    }
}

fn read_layer(conn: &Connection, table: &str) -> StrataResult<Vec<SurveyFeature>> {
    let shape = table_shape(conn, table)?;
    let mut sql = format!("SELECT \"{}\", \"{}\"", shape.pk, shape.geom);
    for name in &shape.attrs {
        sql.push_str(&format!(", \"{name}\""));
    }
    sql.push_str(&format!(" FROM \"{table}\" ORDER BY \"{}\"", shape.pk));

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| storage_err(&format!("「{table}」 읽기 실패"), e))?;
    let mut rows = stmt
        .query([])
        .map_err(|e| storage_err(&format!("「{table}」 읽기 실패"), e))?;
    let mut out = Vec::new();
    while let Some(row) = rows
        .next()
        .map_err(|e| storage_err(&format!("「{table}」 읽기 실패"), e))?
    {
        let fid: i64 = row.get(0).map_err(|e| storage_err("fid 읽기 실패", e))?;
        let source_blob = match row.get_ref(1).map_err(|e| storage_err("도형 읽기 실패", e))? {
            ValueRef::Blob(b) => Some(b.to_vec()),
            _ => None,
        };
        let geom = source_blob
            .as_deref()
            .and_then(|b| decode_gpkg_wkb(b).ok())
            .map(|(_, g)| g);
        let mut feature = SurveyFeature {
            fid: Some(fid),
            geom,
            source_blob,
            attrs: Default::default(),
        };
        for (i, name) in shape.attrs.iter().enumerate() {
            let v = row
                .get_ref(i + 2)
                .map_err(|e| storage_err("속성 읽기 실패", e))?;
            feature.attrs.insert(name.clone(), field_value(v));
        }
        out.push(feature);
    }
    Ok(out)
}

/// 조사 파일을 읽어 `SurveyDoc` 으로 돌려준다.
///
/// 표준 GeoPackage 가 아니거나 도메인 표가 하나도 없으면 오류다.
pub fn read_survey_gpkg(path: &Path) -> StrataResult<SurveyDoc> {
    let mut head = [0_u8; 16];
    let read_ok = std::fs::File::open(path)
        .and_then(|mut f| std::io::Read::read_exact(&mut f, &mut head))
        .is_ok();
    if !read_ok || &head != GPKG_MAGIC_HEADER {
        return Err(StrataError::Storage(
            "GeoPackage 조사 파일이 아닙니다".into(),
        ));
    }
    let conn = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| storage_err("조사 파일 열기 실패", e))?;
    if !table_exists(&conn, "gpkg_contents") || !table_exists(&conn, "gpkg_geometry_columns") {
        return Err(StrataError::Storage(
            "GeoPackage 목차(gpkg_contents)가 없습니다".into(),
        ));
    }

    let mut srs_id: Option<i32> = None;
    let mut doc = SurveyDoc::new(0);
    let mut found = 0usize;
    for key in DomainLayerKey::ALL {
        let table = key.id();
        if !table_exists(&conn, table) {
            continue;
        }
        found += 1;
        if srs_id.is_none() {
            srs_id = conn
                .query_row(
                    "SELECT srs_id FROM gpkg_geometry_columns WHERE table_name = ?1",
                    [table],
                    |r| r.get(0),
                )
                .ok();
        }
        doc.layer_mut(key).features = read_layer(&conn, table)?;
    }
    if found == 0 {
        return Err(StrataError::Storage(
            "조사 레이어(survey_area 등)가 하나도 없는 파일입니다".into(),
        ));
    }
    doc.srs_id = srs_id.unwrap_or(5187);
    Ok(doc)
}
