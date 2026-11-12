//! 세대 사본에 편집 쓰기 — C++ persistWorkspace 의 「레이어 편집을 세대 파일에 쓴다」
//! (`SurveyStorage.cpp:893-945`).
//!
//! 열었을 때(기준)와 지금 메모리를 비교해 다른 것만 쓴다: 바뀐 행은 바뀐 칸만, 새 행은 넣고,
//! 기준에 있었는데 없어진 행은 지운다. 기준에 없던 행(다른 곳에서 더한 행)은 건드리지 않는다.
//! 같은 편집을 GDAL 이 할 때처럼 쓴 표의 gpkg_contents 범위를 넓히고 last_change 를 새로 적는다.

use std::collections::{BTreeMap, BTreeSet};

use rusqlite::types::Value;
use rusqlite::{params_from_iter, Connection};

use crate::edit_buffer::FeatureRecord;
use crate::gpkg_wkt::wkt_to_parts;
use crate::survey_gpkg_read::SurveyFileTable;
use crate::wkb_codec::encode_gpkg_wkb;

fn q(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

struct TableInfo {
    fid: String,
    geom: Option<String>,
    /// (항목 이름, 선언 형식) — fid·도형 칸은 뺀다.
    cols: Vec<(String, String)>,
}

fn table_info(conn: &Connection, table: &str) -> Result<TableInfo, String> {
    let mut stmt = conn
        .prepare(&format!("SELECT name, type, pk FROM pragma_table_info('{}')", table.replace('\'', "''")))
        .map_err(|e| e.to_string())?;
    let all: Vec<(String, String, i64)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .map_err(|e| e.to_string())?
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;
    let geom: Option<String> = conn
        .query_row("SELECT column_name FROM gpkg_geometry_columns WHERE table_name = ?1", [table], |r| r.get(0))
        .ok();
    let fid = all.iter().find(|c| c.2 == 1).map(|c| c.0.clone()).ok_or("표에 기본 키가 없습니다")?;
    let cols = all
        .into_iter()
        .filter(|c| c.0 != fid && Some(&c.0) != geom.as_ref())
        .map(|c| (c.0, c.1))
        .collect();
    Ok(TableInfo { fid, geom, cols })
}

/// 속성 글을 칸 형식대로 넣을 값으로(REAL·INTEGER 칸은 숫자로 읽히면 숫자). 없는 칸이면 오류.
fn attr_value(info: &TableInfo, key: &str, value: Option<&String>) -> Result<Value, String> {
    let (_, ty) = info.cols.iter().find(|(n, _)| n == key).ok_or_else(|| format!("{key} 항목이 표에 없습니다"))?;
    let ty = ty.to_ascii_uppercase();
    Ok(match value {
        None => Value::Null,
        Some(s) if ty.contains("INT") => s.parse::<i64>().map_or_else(|_| Value::Text(s.clone()), Value::Integer),
        Some(s) if ty.contains("REAL") || ty.contains("DOUB") || ty.contains("FLOA") => {
            s.parse::<f64>().map_or_else(|_| Value::Text(s.clone()), Value::Real)
        }
        Some(s) => Value::Text(s.clone()),
    })
}

/// 도형 WKT 를 저장할 GPKG 도형들로(다중 도형은 조각마다). 빈 글이면 NULL 하나.
fn geometry_blobs(f: &FeatureRecord, srs_id: i32, env: &mut Option<[f64; 4]>) -> Result<Vec<Value>, String> {
    if f.wkt_geom.is_empty() {
        return Ok(vec![Value::Null]);
    }
    let parts = wkt_to_parts(&f.wkt_geom)?;
    let mut out = Vec::with_capacity(parts.len());
    for part in parts {
        let e = part.envelope_xy();
        *env = Some(env.map_or(e, |o| [o[0].min(e[0]), o[1].max(e[1]), o[2].min(e[2]), o[3].max(e[3])]));
        out.push(Value::Blob(encode_gpkg_wkb(srs_id, &part)));
    }
    Ok(out)
}

fn insert(conn: &Connection, t: &str, info: &TableInfo, fid: Option<i64>, geom: Value, attrs: &BTreeMap<String, String>) -> Result<(), String> {
    let mut names = Vec::new();
    let mut values = Vec::new();
    if let Some(fid) = fid {
        names.push(q(&info.fid));
        values.push(Value::Integer(fid));
    }
    if let Some(g) = &info.geom {
        names.push(q(g));
        values.push(geom);
    }
    for (k, v) in attrs {
        names.push(q(k));
        values.push(attr_value(info, k, Some(v))?);
    }
    let marks = vec!["?"; values.len()].join(", ");
    conn.execute(&format!("INSERT INTO {} ({}) VALUES ({marks})", q(t), names.join(", ")), params_from_iter(values))
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// 표 하나의 편집을 쓴다. 쓴 뒤 그 표에 있어야 할 도형 수를 돌려준다(확인용).
pub(crate) fn apply_table_edits(conn: &Connection, t: &SurveyFileTable) -> Result<usize, String> {
    if t.layer.features == t.baseline {
        return Ok(t.baseline.len());
    }
    let info = table_info(conn, &t.table)?;
    let baseline: BTreeMap<i64, &FeatureRecord> = t.baseline.iter().map(|f| (f.fid, f)).collect();
    let mut kept = BTreeSet::new();
    let mut expected = 0_usize;
    let mut env: Option<[f64; 4]> = None;
    for f in &t.layer.features {
        let Some(b) = baseline.get(&f.fid).filter(|_| f.fid > 0) else {
            // 새 행: 다중 도형은 조각마다 한 행, 첫 조각만 기준에 없던 양수 fid 를 쓴다.
            for (i, blob) in geometry_blobs(f, t.srs_id, &mut env)?.into_iter().enumerate() {
                let fid = (i == 0 && f.fid > 0).then_some(f.fid);
                insert(conn, &t.table, &info, fid, blob, &f.attributes)?;
                expected += 1;
            }
            continue;
        };
        kept.insert(f.fid);
        expected += 1;
        if *b == f {
            continue;
        }
        let mut sets: Vec<(String, Value)> = Vec::new();
        let mut extra = Vec::new();
        if f.wkt_geom != b.wkt_geom {
            let mut blobs = geometry_blobs(f, t.srs_id, &mut env)?.into_iter();
            let g = info.geom.clone().ok_or("도형 칸이 없는 표입니다")?;
            sets.push((g, blobs.next().unwrap_or(Value::Null)));
            extra = blobs.collect();
        }
        let keys: BTreeSet<&String> = b.attributes.keys().chain(f.attributes.keys()).collect();
        for k in keys.into_iter().filter(|k| b.attributes.get(*k) != f.attributes.get(*k)) {
            sets.push((k.clone(), attr_value(&info, k, f.attributes.get(k))?));
        }
        let assign: Vec<String> = sets.iter().map(|(n, _)| format!("{} = ?", q(n))).collect();
        let mut values: Vec<Value> = sets.into_iter().map(|(_, v)| v).collect();
        values.push(Value::Integer(f.fid));
        conn.execute(
            &format!("UPDATE {} SET {} WHERE {} = ?", q(&t.table), assign.join(", "), q(&info.fid)),
            params_from_iter(values),
        )
        .map_err(|e| e.to_string())?;
        for blob in extra {
            insert(conn, &t.table, &info, None, blob, &f.attributes)?;
            expected += 1;
        }
    }
    for fid in baseline.keys().filter(|fid| **fid > 0 && !kept.contains(fid)) {
        conn.execute(&format!("DELETE FROM {} WHERE {} = ?1", q(&t.table), q(&info.fid)), [fid])
            .map_err(|e| e.to_string())?;
    }
    conn.execute(
        "UPDATE gpkg_contents SET last_change = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE table_name = ?1",
        [&t.table],
    )
    .map_err(|e| e.to_string())?;
    if let Some([min_x, max_x, min_y, max_y]) = env {
        conn.execute(
            "UPDATE gpkg_contents SET min_x = min(ifnull(min_x, ?2), ?2), min_y = min(ifnull(min_y, ?3), ?3),
                 max_x = max(ifnull(max_x, ?4), ?4), max_y = max(ifnull(max_y, ?5), ?5) WHERE table_name = ?1",
            rusqlite::params![t.table, min_x, min_y, max_x, max_y],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(expected)
}
