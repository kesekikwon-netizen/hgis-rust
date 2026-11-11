//! 표준 GeoPackage 조사 파일 쓰기 — 새 조사 만들기와 저장.
//!
//! C++ `SurveyProjectFactory::createNewSurvey` 는 7개 도메인 표를 가진 새 파일을 만들고,
//! `SurveyStorage::persistWorkspace` 는 「세대 사본을 만들어 → 고치고 → 검증하고 → 원본과
//! 바꾼다」. 여기서도 같은 순서로 한다. 원본은 마지막 교체 전까지 손대지 않으므로
//! 중간에 실패해도 사용자의 파일은 그대로다.

use std::path::{Path, PathBuf};

use rusqlite::types::Value;
use rusqlite::{params_from_iter, Connection};

use strata_types::error::{StrataError, StrataResult};
use strata_types::schema::{DomainLayerKey, LayerSchema, SurveySchemaV3};

use crate::gpkg_std_sql::{
    blob_envelope, create_core_tables, create_feature_table, ensure_srs_row,
    register_gpkg_functions, sql_type, table_exists,
};
use crate::survey_doc::{FieldValue, SurveyDoc, SurveyFeature, SurveyLayer};
use crate::survey_gpkg_read::{storage_err, table_shape, TableShape};
use crate::wkb_codec::encode_gpkg_wkb;

fn generation_path(path: &Path) -> PathBuf {
    path.with_extension("gpkg.gen-tmp")
}

fn open_for_write(path: &Path) -> StrataResult<Connection> {
    let conn = Connection::open(path).map_err(|e| storage_err("조사 파일 쓰기 열기 실패", e))?;
    register_gpkg_functions(&conn)?;
    Ok(conn)
}

fn write_new_file(path: &Path, srs_id: i32) -> StrataResult<()> {
    let conn = open_for_write(path)?;
    create_core_tables(&conn, srs_id)?;
    for schema in &SurveySchemaV3::default().layers {
        create_feature_table(&conn, schema, srs_id)?;
    }
    Ok(())
}

/// 새 조사 파일을 만든다(7개 도메인 표, 도형 0개).
///
/// C++ `SurveyProjectFactory.cpp:44` 와 같이 같은 이름의 파일이 이미 있으면 덮지 않고 거부한다.
pub fn create_standard_survey(path: &Path, srs_id: i32) -> StrataResult<SurveyDoc> {
    if path.exists() {
        return Err(StrataError::Storage(format!(
            "같은 이름의 조사 파일이 이미 있습니다: {}",
            path.display()
        )));
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let staging = generation_path(path);
    let _ = std::fs::remove_file(&staging);
    if let Err(e) = write_new_file(&staging, srs_id) {
        let _ = std::fs::remove_file(&staging);
        return Err(e);
    }
    std::fs::rename(&staging, path)?;
    Ok(SurveyDoc::new(srs_id))
}

fn to_sql(v: &FieldValue) -> Value {
    match v {
        FieldValue::Null => Value::Null,
        FieldValue::Text(s) => Value::Text(s.clone()),
        FieldValue::Real(f) => Value::Real(*f),
        FieldValue::Int(i) => Value::Integer(*i),
        FieldValue::Blob(b) => Value::Blob(b.clone()),
    }
}

fn geom_blob(feature: &SurveyFeature, srs_id: i32) -> Value {
    if let Some(blob) = feature.source_blob.as_ref() {
        return Value::Blob(blob.clone());
    }
    feature
        .geom
        .as_ref()
        .map_or(Value::Null, |g| Value::Blob(encode_gpkg_wkb(srs_id, g)))
}

/// 저장 세대에서 스키마 3 의 빠진 열을 더한다(C++ `SurveySchema::migrateGenerationCopy`).
fn add_missing_columns(conn: &Connection, schema: &LayerSchema, shape: &mut TableShape) -> StrataResult<()> {
    for field in &schema.fields {
        if shape.attrs.iter().any(|c| c == field.name) {
            continue;
        }
        conn.execute_batch(&format!(
            "ALTER TABLE \"{}\" ADD COLUMN \"{}\" {}",
            schema.id,
            field.name,
            sql_type(field.field_type)
        ))
        .map_err(|e| storage_err("열 추가 실패", e))?;
        shape.attrs.push(field.name.to_owned());
    }
    Ok(())
}

fn sync_layer(conn: &Connection, layer: &mut SurveyLayer, schema: &LayerSchema, srs_id: i32) -> StrataResult<()> {
    let table = schema.id;
    if !table_exists(conn, table) {
        ensure_srs_row(conn, srs_id)?;
        create_feature_table(conn, schema, srs_id)?;
    }
    let mut shape = table_shape(conn, table)?;
    add_missing_columns(conn, schema, &mut shape)?;
    let fail = |e| storage_err(&format!("「{table}」 저장 실패"), e);

    // 1) 메모리에 없는 도형은 파일에서 지운다.
    let kept: Vec<String> = layer
        .features
        .iter()
        .filter_map(|f| f.fid)
        .map(|fid| fid.to_string())
        .collect();
    let delete_sql = if kept.is_empty() {
        format!("DELETE FROM \"{table}\"")
    } else {
        format!(
            "DELETE FROM \"{table}\" WHERE \"{}\" NOT IN ({})",
            shape.pk,
            kept.join(",")
        )
    };
    conn.execute(&delete_sql, []).map_err(fail)?;

    // 2) 있던 도형은 고치고, 새 도형은 넣는다. 열 순서: 지오메트리, 속성들.
    let set_list: Vec<String> = std::iter::once(&shape.geom)
        .chain(shape.attrs.iter())
        .enumerate()
        .map(|(i, c)| format!("\"{c}\" = ?{}", i + 1))
        .collect();
    let n = shape.attrs.len() + 1;
    let update_sql = format!(
        "UPDATE \"{table}\" SET {} WHERE \"{}\" = ?{}",
        set_list.join(", "),
        shape.pk,
        n + 1
    );
    let col_list: Vec<String> = std::iter::once(&shape.geom)
        .chain(shape.attrs.iter())
        .map(|c| format!("\"{c}\""))
        .collect();
    let marks: Vec<String> = (1..=n).map(|i| format!("?{i}")).collect();
    let insert_sql = format!(
        "INSERT INTO \"{table}\" ({}) VALUES ({})",
        col_list.join(", "),
        marks.join(", ")
    );

    for feature in &mut layer.features {
        let mut values = vec![geom_blob(feature, srs_id)];
        for name in &shape.attrs {
            values.push(feature.attrs.get(name).map_or(Value::Null, to_sql));
        }
        if let Some(fid) = feature.fid {
            values.push(Value::Integer(fid));
            conn.execute(&update_sql, params_from_iter(values)).map_err(fail)?;
        } else {
            conn.execute(&insert_sql, params_from_iter(values)).map_err(fail)?;
            feature.fid = Some(conn.last_insert_rowid());
        }
    }

    // 3) 목차의 범위와 고친 시각.
    let mut extent: Option<[f64; 4]> = None;
    for feature in &layer.features {
        let env = match (&feature.source_blob, &feature.geom) {
            (Some(blob), _) => blob_envelope(blob),
            (None, Some(g)) => Some(g.envelope_xy()),
            (None, None) => None,
        };
        if let Some(e) = env {
            extent = Some(match extent {
                None => e,
                Some(o) => [o[0].min(e[0]), o[1].max(e[1]), o[2].min(e[2]), o[3].max(e[3])],
            });
        }
    }
    let (min_x, max_x, min_y, max_y) = match extent {
        Some(e) => (Some(e[0]), Some(e[1]), Some(e[2]), Some(e[3])),
        None => (None, None, None, None),
    };
    conn.execute(
        "UPDATE gpkg_contents SET min_x=?1, min_y=?2, max_x=?3, max_y=?4,
             last_change=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE table_name=?5",
        rusqlite::params![min_x, min_y, max_x, max_y, table],
    )
    .map_err(fail)?;
    Ok(())
}

fn write_generation(staging: &Path, doc: &mut SurveyDoc) -> StrataResult<()> {
    let mut conn = open_for_write(staging)?;
    let schema = SurveySchemaV3::default();
    let srs_id = doc.srs_id;
    let tx = conn
        .transaction()
        .map_err(|e| storage_err("저장 트랜잭션 시작 실패", e))?;
    for layer in &mut doc.layers {
        sync_layer(&tx, layer, schema.layer(layer.key), srs_id)?;
    }
    tx.commit().map_err(|e| storage_err("저장 확정 실패", e))?;

    // 검증: 파일 무결성 + 표마다 도형 수가 메모리와 같은지.
    let ok: String = conn
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .map_err(|e| storage_err("무결성 검사 실패", e))?;
    if !ok.eq_ignore_ascii_case("ok") {
        return Err(StrataError::Storage(format!("저장한 파일의 무결성 검사 실패: {ok}")));
    }
    for key in DomainLayerKey::ALL {
        let n: i64 = conn
            .query_row(&format!("SELECT count(*) FROM \"{}\"", key.id()), [], |r| r.get(0))
            .map_err(|e| storage_err("도형 수 확인 실패", e))?;
        if n != doc.layer(key).features.len() as i64 {
            return Err(StrataError::Storage(format!(
                "「{}」 도형 수가 맞지 않습니다(파일 {n}개)",
                key.name_ko()
            )));
        }
    }
    Ok(())
}

/// 조사를 파일에 저장한다. 새 도형은 fid 를 받아 `doc` 에 되적힌다.
///
/// 순서: 세대 사본 → 사본에 반영 → 검증 → 원본과 교체. 실패하면 원본은 그대로 남는다.
/// 파일이 아직 없으면(다른 이름으로 저장) 새 표준 파일을 만들어 쓴다.
pub fn save_survey_gpkg(path: &Path, doc: &mut SurveyDoc) -> StrataResult<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let staging = generation_path(path);
    let _ = std::fs::remove_file(&staging);
    // 표준 조사 파일이면 사본 위에 고친다(다른 표·설정이 보존된다). 표준이 아닌 파일
    // (예전 Rust 판이 만든 것 등)은 사용자가 덮어쓰기를 고른 것이므로 새 파일로 바꾼다.
    let reuse = path.exists() && crate::survey_gpkg_read::read_survey_gpkg(path).is_ok();
    let prepared = if reuse {
        std::fs::copy(path, &staging).map(|_| ()).map_err(StrataError::from)
    } else {
        write_new_file(&staging, doc.srs_id)
    };
    // 실패하면 메모리의 fid 가 파일과 어긋나지 않게 되돌린다.
    let before = doc.clone();
    let result = prepared.and_then(|()| write_generation(&staging, doc));
    if let Err(e) = result {
        *doc = before;
        let _ = std::fs::remove_file(&staging);
        return Err(e);
    }
    let mut last_err = None;
    for attempt in 0..4_u64 {
        match std::fs::rename(&staging, path) {
            Ok(()) => return Ok(()),
            Err(e) => last_err = Some(e),
        }
        std::thread::sleep(std::time::Duration::from_millis(40 * (attempt + 1)));
    }
    *doc = before;
    let _ = std::fs::remove_file(&staging);
    Err(StrataError::Storage(format!(
        "조사 파일을 바꿔 쓰지 못했습니다(다른 프로그램이 열고 있을 수 있습니다): {}",
        last_err.map(|e| e.to_string()).unwrap_or_default()
    )))
}
