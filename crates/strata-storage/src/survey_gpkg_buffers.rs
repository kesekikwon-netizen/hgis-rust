//! C++ test_buffer.cpp:166-198 and SurveyStorage.cpp:703-786: save derived line/point layers in the generation.
//! Layer keys live in gpkg_contents.description until the C++ QGZ custom properties are read by this app.
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use rusqlite::{params, Connection};
use strata_types::buffer::DistanceRing;
use strata_types::crs::WorkCrs;

use crate::gpkg_geometry::gpkg_blob_to_wkt;
use crate::gpkg_wkt::wkt_to_parts;
use crate::survey_gpkg_ddl::{layer_sql_named, SurveyFieldDef, GPKG_EXTENSIONS_DDL, WKT_5186, WKT_5187};
use crate::survey_gpkg_read::SurveyFileTable;
use crate::wkb_codec::{encode_gpkg_wkb, WkbGeometry};

const PREFIX: &str = "ka-hgis:distance-buffer:";
pub const INVALID_BUFFER_KO: &str = "조사 파일의 무결성을 확인하지 못했습니다. 현재 작업은 유지됩니다.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Part {
    Ring,
    Label,
}

impl Part {
    fn token(self) -> &'static str {
        match self {
            Self::Ring => "ring",
            Self::Label => "label",
        }
    }
    fn field(self) -> &'static str {
        match self {
            Self::Ring => "note",
            Self::Label => "label",
        }
    }
    fn geometry(self) -> &'static str {
        match self {
            Self::Ring => "LINESTRING",
            Self::Label => "POINT",
        }
    }
}

#[derive(Debug)]
struct Owned {
    table: String,
    meters: u32,
    part: Part,
}

fn q(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}
fn db(e: rusqlite::Error) -> String {
    e.to_string()
}
fn invalid() -> String {
    INVALID_BUFFER_KO.to_owned()
}

fn owned(conn: &Connection) -> Result<Vec<Owned>, String> {
    let mut stmt = conn.prepare("SELECT table_name,description FROM gpkg_contents WHERE description LIKE 'ka-hgis:distance-buffer:%' ORDER BY rowid").map_err(db)?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))).map_err(db)?.collect::<Result<Vec<_>, _>>().map_err(db)?;
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for (table, description) in rows {
        let fields: Vec<_> = description.strip_prefix(PREFIX).ok_or_else(invalid)?.split(':').collect();
        let ["v1", meters, part] = fields.as_slice() else { return Err(invalid()) };
        let meters = meters.parse::<u32>().ok().filter(|n| *n > 0).ok_or_else(invalid)?;
        let part = match *part {
            "ring" => Part::Ring,
            "label" => Part::Label,
            _ => return Err(invalid()),
        };
        if !seen.insert((meters, part)) {
            return Err(invalid());
        }
        out.push(Owned { table, meters, part });
    }
    Ok(out)
}

fn finite(points: &[[f64; 2]]) -> bool {
    points.iter().flatten().all(|v| v.is_finite())
}

/// Consume only metadata-marked derived tables. Validate everything before the caller clears its current survey.
pub fn read_buffers(path: &Path, mut tables: Vec<SurveyFileTable>) -> Result<(Vec<SurveyFileTable>, BTreeMap<u32, DistanceRing>), String> {
    let conn = crate::sqlite_read::open_survey_read_only(path).map_err(|e| e.to_string())?;
    let (names, buffers) = read_buffers_from_conn(&conn)?;
    tables.retain(|t| !names.contains(&t.table));
    Ok((tables, buffers))
}

// target/cpp-ref/src/core/SurveyStorage.cpp:775-782: probe the written layers, not every unrelated survey geometry.
pub(crate) fn read_buffers_from_conn(conn: &Connection) -> Result<(BTreeSet<String>, BTreeMap<u32, DistanceRing>), String> {
    let owned = owned(conn)?;
    let mut out: BTreeMap<u32, DistanceRing> = BTreeMap::new();
    for layer in &owned {
        let (column, kind, srs_id): (String, String, i32) = conn
            .query_row("SELECT column_name,geometry_type_name,srs_id FROM gpkg_geometry_columns WHERE table_name=?1", [&layer.table], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            })
            .map_err(db)?;
        let crs = match srs_id {
            5186 => WorkCrs::Epsg5186,
            5187 => WorkCrs::Epsg5187,
            _ => return Err(invalid()),
        };
        if kind.to_ascii_uppercase() != layer.part.geometry() {
            return Err(invalid());
        }
        // target/cpp-ref/src/core/BufferAnalysis.cpp:242,268-277: this model can represent only these attributes.
        let text = if layer.part == Part::Ring { format!("주변 {}m", layer.meters) } else { format!("{}m", layer.meters) };
        let mut fields = conn.prepare("SELECT name,type FROM pragma_table_info(?1) WHERE pk=0 AND name<>?2 ORDER BY cid").map_err(db)?;
        let columns = fields
            .query_map(params![layer.table, column], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
            .map_err(db)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(db)?;
        if columns != [(layer.part.field().to_owned(), "TEXT".to_owned())] {
            return Err(invalid());
        }
        let mut stmt = conn.prepare(&format!("SELECT {},{} FROM {} ORDER BY fid", q(&column), q(layer.part.field()), q(&layer.table))).map_err(db)?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, String>(1)?))).map_err(db)?.collect::<Result<Vec<_>, _>>().map_err(db)?;
        let ring = out.entry(layer.meters).or_insert_with(|| DistanceRing { crs, ..Default::default() });
        if ring.crs != crs {
            return Err(invalid());
        }
        for (blob, attribute) in rows {
            if attribute != text {
                return Err(invalid());
            }
            let (header_crs, wkt) = gpkg_blob_to_wkt(&blob).map_err(|_| invalid())?;
            if header_crs != srs_id {
                return Err(invalid());
            }
            let mut parts = wkt_to_parts(&wkt).map_err(|_| invalid())?;
            if parts.len() != 1 {
                return Err(invalid());
            }
            match (layer.part, parts.pop()) {
                (Part::Ring, Some(WkbGeometry::LineString(points))) if points.len() >= 2 && finite(&points) => ring.lines.push(points),
                (Part::Label, Some(WkbGeometry::Point(point))) if finite(&[point]) => ring.labels.push(point),
                _ => return Err(invalid()),
            }
        }
    }
    if out.values().any(|ring| ring.lines.is_empty() || ring.labels.is_empty()) {
        return Err(invalid());
    }
    Ok((owned.into_iter().map(|o| o.table).collect(), out))
}

fn new_table_name(conn: &Connection, meters: u32, part: Part) -> Result<String, String> {
    // target/cpp-ref/src/core/SurveyStorage.cpp:57-66,751-753: sanitized display title and unused suffix.
    let base = format!("주변_{meters}m{}", if part == Part::Label { "_거리" } else { "" });
    let mut name = base.clone();
    let mut suffix = 2;
    loop {
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name=?1 COLLATE NOCASE UNION ALL SELECT 1 FROM gpkg_contents WHERE identifier=?1 COLLATE NOCASE)",
                [&name],
                |r| r.get(0),
            )
            .map_err(db)?;
        if !exists {
            return Ok(name);
        }
        name = format!("{base}_{suffix}");
        suffix += 1;
    }
}

fn write_layer(conn: &Connection, table: &str, part: Part, meters: u32, ring: &DistanceRing) -> Result<(), String> {
    let expected = part.geometry();
    let (column, kind): (String, String) = conn
        .query_row("SELECT column_name,geometry_type_name FROM gpkg_geometry_columns WHERE table_name=?1", [table], |r| Ok((r.get(0)?, r.get(1)?)))
        .map_err(db)?;
    if column != "geom" || kind != expected {
        return Err(invalid());
    }
    let srs = ring.crs.epsg_code();
    conn.execute("UPDATE gpkg_geometry_columns SET srs_id=?2 WHERE table_name=?1", params![table, srs]).map_err(db)?;
    conn.execute(&format!("DELETE FROM {}", q(table)), []).map_err(db)?;
    let geometries: Vec<WkbGeometry> = match part {
        Part::Ring => ring.lines.iter().cloned().map(WkbGeometry::LineString).collect(),
        Part::Label => ring.labels.iter().copied().map(WkbGeometry::Point).collect(),
    };
    let text = if part == Part::Ring { format!("주변 {meters}m") } else { format!("{meters}m") };
    let mut bounds: Option<[f64; 4]> = None;
    for (i, geometry) in geometries.iter().enumerate() {
        let e = geometry.envelope_xy();
        bounds = Some(bounds.map_or(e, |b| [b[0].min(e[0]), b[1].max(e[1]), b[2].min(e[2]), b[3].max(e[3])]));
        let fid = i64::try_from(i + 1).map_err(|_| invalid())?;
        conn.execute(
            &format!("INSERT INTO {} (fid,geom,{}) VALUES (?1,?2,?3)", q(table), q(part.field())),
            params![fid, encode_gpkg_wkb(i32::try_from(srs).map_err(|_| invalid())?, geometry), text],
        )
        .map_err(db)?;
    }
    let [min_x, max_x, min_y, max_y] = bounds.ok_or_else(invalid)?;
    conn.execute(
        "UPDATE gpkg_contents SET srs_id=?2,min_x=?3,min_y=?4,max_x=?5,max_y=?6,last_change=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE table_name=?1",
        params![table, srs, min_x, min_y, max_x, max_y],
    )
    .map_err(db)?;
    let count: i64 = conn.query_row(&format!("SELECT count(*) FROM {}", q(table)), [], |r| r.get(0)).map_err(db)?;
    if usize::try_from(count).ok() != Some(geometries.len()) {
        return Err(invalid());
    }
    Ok(())
}

/// Apply only supplied radii inside the caller's generation transaction. Empty updates preserve saved buffers.
pub(crate) fn write_buffers(conn: &Connection, updates: &BTreeMap<u32, DistanceRing>) -> Result<(), String> {
    if updates.is_empty() {
        return Ok(());
    }
    let owned = owned(conn)?;
    let has_extensions: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='gpkg_extensions')", [], |r| r.get(0)).map_err(db)?;
    if !has_extensions {
        conn.execute_batch(GPKG_EXTENSIONS_DDL).map_err(db)?;
    }
    for (meters, ring) in updates {
        if *meters == 0 || ring.lines.is_empty() || ring.labels.is_empty() || ring.lines.iter().any(|l| l.len() < 2 || !finite(l)) || !finite(&ring.labels) {
            return Err(invalid());
        }
        let (name, wkt) = match ring.crs {
            WorkCrs::Epsg5186 => ("KGD2002 / Central Belt 2010", WKT_5186),
            WorkCrs::Epsg5187 => ("KGD2002 / East Belt 2010", WKT_5187),
        };
        conn.execute(
            "INSERT OR IGNORE INTO gpkg_spatial_ref_sys (srs_name,srs_id,organization,organization_coordsys_id,definition) VALUES (?1,?2,'EPSG',?2,?3)",
            params![name, ring.crs.epsg_code(), wkt],
        )
        .map_err(db)?;
        for part in [Part::Ring, Part::Label] {
            let table = if let Some(layer) = owned.iter().find(|o| o.meters == *meters && o.part == part) {
                layer.table.clone()
            } else {
                let table = new_table_name(conn, *meters, part)?;
                conn.execute_batch(&layer_sql_named(&table, part.geometry(), &[SurveyFieldDef { name: part.field(), real: false }], ring.crs.epsg_code(), false))
                    .map_err(db)?;
                conn.execute("UPDATE gpkg_contents SET description=?2 WHERE table_name=?1", params![table, format!("{PREFIX}v1:{meters}:{}", part.token())])
                    .map_err(db)?;
                table
            };
            write_layer(conn, &table, part, *meters, ring)?;
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "survey_gpkg_buffers_tests.rs"]
mod tests;
