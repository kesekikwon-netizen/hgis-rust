//! 표준 GeoPackage(OGR 이 만드는 것과 같은 모양)의 SQL 조각과 `ST_*` 함수.
//!
//! C++ 앱은 GDAL/OGR 의 GPKG 드라이버가 이 일을 한다. OGR 이 만든 파일에는 공간 색인
//! (rtree) 트리거가 있고 그 트리거가 `ST_IsEmpty`·`ST_MinX` 등을 부른다. 이 함수들은
//! GDAL 이 넣어 주는 것이라 순수 SQLite 에는 없다. 그래서 여기서 직접 등록한다 —
//! 등록하지 않으면 C++ 로 만든 조사 파일에 도형을 넣거나 지울 때 SQLite 가 실패한다.

use rusqlite::functions::{Context, FunctionFlags};
use rusqlite::Connection;

use strata_types::error::{StrataError, StrataResult};
use strata_types::schema::{FieldType, GeometryKind, LayerSchema};

use crate::wkb_codec::decode_gpkg_wkb;

/// `PRAGMA application_id` — 'GPKG'.
pub const GPKG_APPLICATION_ID: i64 = 1_196_444_487;
/// `PRAGMA user_version` — GeoPackage 1.2. (GDAL 2 이상 어느 판이든 경고 없이 연다.)
pub const GPKG_USER_VERSION: i64 = 10_200;

const WKT_4326: &str = r#"GEOGCS["WGS 84",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563,AUTHORITY["EPSG","7030"]],AUTHORITY["EPSG","6326"]],PRIMEM["Greenwich",0,AUTHORITY["EPSG","8901"]],UNIT["degree",0.0174532925199433,AUTHORITY["EPSG","9122"]],AXIS["Latitude",NORTH],AXIS["Longitude",EAST],AUTHORITY["EPSG","4326"]]"#;

/// 한국 TM 작업 좌표계의 이름과 WKT. GDAL 이 EPSG:5186 에 쓰는 글자와 같다
/// (`samples/demo_survey/demo.gpkg` 의 `gpkg_spatial_ref_sys` 에서 확인).
#[must_use]
pub fn korean_tm_srs(srs_id: i32) -> Option<(String, String)> {
    let (belt, meridian) = match srs_id {
        5185 => ("West", 125),
        5186 => ("Central", 127),
        5187 => ("East", 129),
        5188 => ("East Sea", 131),
        _ => return None,
    };
    let name = format!("KGD2002 / {belt} Belt 2010");
    let wkt = format!(
        r#"PROJCS["{name}",GEOGCS["KGD2002",DATUM["Korean_Geodetic_Datum_2002",SPHEROID["GRS 1980",6378137,298.257222101,AUTHORITY["EPSG","7019"]],AUTHORITY["EPSG","6737"]],PRIMEM["Greenwich",0,AUTHORITY["EPSG","8901"]],UNIT["degree",0.0174532925199433,AUTHORITY["EPSG","9122"]],AUTHORITY["EPSG","4737"]],PROJECTION["Transverse_Mercator"],PARAMETER["latitude_of_origin",38],PARAMETER["central_meridian",{meridian}],PARAMETER["scale_factor",1],PARAMETER["false_easting",200000],PARAMETER["false_northing",600000],UNIT["metre",1,AUTHORITY["EPSG","9001"]],AXIS["Northing",NORTH],AXIS["Easting",EAST],AUTHORITY["EPSG","{srs_id}"]]"#
    );
    Some((name, wkt))
}

/// GPKG 지오메트리 바이트의 범위 `[min_x, max_x, min_y, max_y]`.
/// 머리말에 범위가 있으면 그것을, 없으면 도형을 풀어 계산한다. 빈 도형이면 `None`.
#[must_use]
pub fn blob_envelope(blob: &[u8]) -> Option<[f64; 4]> {
    if blob.len() < 8 || &blob[0..2] != b"GP" {
        return None;
    }
    let flags = blob[3];
    if flags & 0x10 != 0 {
        return None; // 빈 도형 표시
    }
    let little = flags & 0x01 != 0;
    let env_kind = (flags >> 1) & 0x07;
    if (1..=4).contains(&env_kind) && blob.len() >= 40 {
        let read = |i: usize| -> f64 {
            let mut a = [0_u8; 8];
            a.copy_from_slice(&blob[8 + i * 8..16 + i * 8]);
            if little {
                f64::from_le_bytes(a)
            } else {
                f64::from_be_bytes(a)
            }
        };
        let env = [read(0), read(1), read(2), read(3)];
        if env.iter().all(|v| v.is_finite()) {
            return Some(env);
        }
    }
    let (_, geom) = decode_gpkg_wkb(blob).ok()?;
    Some(geom.envelope_xy())
}

fn blob_arg(ctx: &Context<'_>) -> rusqlite::Result<Option<[f64; 4]>> {
    let raw = ctx
        .get_raw(0)
        .as_blob_or_null()
        .map_err(|e| rusqlite::Error::UserFunctionError(Box::new(e)))?;
    Ok(raw.and_then(blob_envelope))
}

/// OGR 의 rtree 트리거가 부르는 `ST_IsEmpty`·`ST_MinX`·`ST_MaxX`·`ST_MinY`·`ST_MaxY` 를 등록한다.
pub fn register_gpkg_functions(conn: &Connection) -> StrataResult<()> {
    let flags = FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC;
    let fail = |e: rusqlite::Error| StrataError::Storage(format!("ST 함수 등록 실패: {e}"));
    conn.create_scalar_function("ST_IsEmpty", 1, flags, |ctx| {
        Ok(blob_arg(ctx)?.is_none())
    })
    .map_err(fail)?;
    for (name, idx) in [("ST_MinX", 0), ("ST_MaxX", 1), ("ST_MinY", 2), ("ST_MaxY", 3)] {
        conn.create_scalar_function(name, 1, flags, move |ctx| {
            Ok(blob_arg(ctx)?.map(|e| e[idx]))
        })
        .map_err(fail)?;
    }
    Ok(())
}

/// 새 GPKG 의 필수 표(좌표계·목차·지오메트리 열·확장·OGR 도형 수)를 만든다.
pub fn create_core_tables(conn: &Connection, srs_id: i32) -> StrataResult<()> {
    let (srs_name, srs_wkt) = korean_tm_srs(srs_id)
        .ok_or_else(|| StrataError::Storage(format!("지원하지 않는 작업 좌표계 EPSG:{srs_id}")))?;
    conn.execute_batch(&format!(
        "PRAGMA application_id = {GPKG_APPLICATION_ID};
         PRAGMA user_version = {GPKG_USER_VERSION};
         CREATE TABLE gpkg_spatial_ref_sys (srs_name TEXT NOT NULL,srs_id INTEGER NOT NULL PRIMARY KEY,organization TEXT NOT NULL,organization_coordsys_id INTEGER NOT NULL,definition  TEXT NOT NULL,description TEXT);
         CREATE TABLE gpkg_contents (table_name TEXT NOT NULL PRIMARY KEY,data_type TEXT NOT NULL,identifier TEXT UNIQUE,description TEXT DEFAULT '',last_change DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),min_x DOUBLE, min_y DOUBLE,max_x DOUBLE, max_y DOUBLE,srs_id INTEGER,CONSTRAINT fk_gc_r_srs_id FOREIGN KEY (srs_id) REFERENCES gpkg_spatial_ref_sys(srs_id));
         CREATE TABLE gpkg_geometry_columns (table_name TEXT NOT NULL,column_name TEXT NOT NULL,geometry_type_name TEXT NOT NULL,srs_id INTEGER NOT NULL,z TINYINT NOT NULL,m TINYINT NOT NULL,CONSTRAINT pk_geom_cols PRIMARY KEY (table_name, column_name),CONSTRAINT uk_gc_table_name UNIQUE (table_name),CONSTRAINT fk_gc_tn FOREIGN KEY (table_name) REFERENCES gpkg_contents(table_name),CONSTRAINT fk_gc_srs FOREIGN KEY (srs_id) REFERENCES gpkg_spatial_ref_sys (srs_id));
         CREATE TABLE gpkg_extensions (table_name TEXT,column_name TEXT,extension_name TEXT NOT NULL,definition TEXT NOT NULL,scope TEXT NOT NULL,CONSTRAINT ge_tce UNIQUE (table_name, column_name, extension_name));
         CREATE TABLE gpkg_ogr_contents(table_name TEXT NOT NULL PRIMARY KEY,feature_count INTEGER DEFAULT NULL);
         INSERT INTO gpkg_spatial_ref_sys VALUES ('Undefined Cartesian SRS', -1, 'NONE', -1, 'undefined', 'undefined Cartesian coordinate reference system');
         INSERT INTO gpkg_spatial_ref_sys VALUES ('Undefined geographic SRS', 0, 'NONE', 0, 'undefined', 'undefined geographic coordinate reference system');"
    ))
    .map_err(|e| StrataError::Storage(format!("GPKG 기본 표 생성 실패: {e}")))?;
    let insert = "INSERT INTO gpkg_spatial_ref_sys VALUES (?1, ?2, 'EPSG', ?2, ?3, NULL)";
    conn.execute(insert, rusqlite::params!["WGS 84 geodetic", 4326, WKT_4326])
        .and_then(|_| conn.execute(insert, rusqlite::params![srs_name, srs_id, srs_wkt]))
        .map_err(|e| StrataError::Storage(format!("좌표계 정의 기록 실패: {e}")))?;
    Ok(())
}

/// 파일에 작업 좌표계 정의가 없으면 넣는다(다른 좌표계 파일에 새 표를 만들 때).
pub fn ensure_srs_row(conn: &Connection, srs_id: i32) -> StrataResult<()> {
    let Some((name, wkt)) = korean_tm_srs(srs_id) else {
        return Ok(());
    };
    conn.execute(
        "INSERT OR IGNORE INTO gpkg_spatial_ref_sys VALUES (?1, ?2, 'EPSG', ?2, ?3, NULL)",
        rusqlite::params![name, srs_id, wkt],
    )
    .map(|_| ())
    .map_err(|e| StrataError::Storage(format!("좌표계 정의 기록 실패: {e}")))
}

const fn geometry_type_name(kind: GeometryKind) -> &'static str {
    match kind {
        GeometryKind::Polygon => "POLYGON",
        GeometryKind::LineString => "LINESTRING",
        GeometryKind::Point => "POINT",
    }
}

pub(crate) const fn sql_type(field_type: FieldType) -> &'static str {
    match field_type {
        FieldType::String => "TEXT",
        FieldType::Double => "REAL",
    }
}

/// 도메인 레이어 표 하나를 OGR 과 같은 모양으로 만든다:
/// 도형 표 + 목차 + 지오메트리 열 + rtree 색인과 트리거 + 도형 수 트리거.
pub fn create_feature_table(conn: &Connection, schema: &LayerSchema, srs_id: i32) -> StrataResult<()> {
    let t = schema.id;
    let geom_type = geometry_type_name(schema.geometry);
    let mut cols = String::new();
    for field in &schema.fields {
        cols.push_str(&format!(", \"{}\" {}", field.name, sql_type(field.field_type)));
    }
    let rt = format!("rtree_{t}_geom");
    let not_empty = |who: &str| format!("({who}.\"geom\" NOTNULL AND NOT ST_IsEmpty({who}.\"geom\"))");
    let is_empty = |who: &str| format!("({who}.\"geom\" ISNULL OR ST_IsEmpty({who}.\"geom\"))");
    let box_of_new = "ST_MinX(NEW.\"geom\"), ST_MaxX(NEW.\"geom\"),ST_MinY(NEW.\"geom\"), ST_MaxY(NEW.\"geom\")";
    let sql = format!(
        "CREATE TABLE \"{t}\" ( \"fid\" INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL, \"geom\" {geom_type}{cols});
         INSERT INTO gpkg_contents (table_name,data_type,identifier,description,srs_id) VALUES ('{t}','features','{t}','',{srs_id});
         INSERT INTO gpkg_geometry_columns VALUES ('{t}','geom','{geom_type}',{srs_id},0,0);
         INSERT INTO gpkg_ogr_contents VALUES ('{t}',0);
         INSERT INTO gpkg_extensions VALUES ('{t}','geom','gpkg_rtree_index','http://www.geopackage.org/spec120/#extension_rtree','write-only');
         CREATE VIRTUAL TABLE \"{rt}\" USING rtree(id, minx, maxx, miny, maxy);
         CREATE TRIGGER \"{rt}_insert\" AFTER INSERT ON \"{t}\" WHEN {ne_new} BEGIN INSERT OR REPLACE INTO \"{rt}\" VALUES (NEW.\"fid\",{box_of_new}); END;
         CREATE TRIGGER \"{rt}_update6\" AFTER UPDATE OF \"geom\" ON \"{t}\" WHEN OLD.\"fid\" = NEW.\"fid\" AND {ne_new} AND {ne_old} BEGIN UPDATE \"{rt}\" SET minx = ST_MinX(NEW.\"geom\"), maxx = ST_MaxX(NEW.\"geom\"),miny = ST_MinY(NEW.\"geom\"), maxy = ST_MaxY(NEW.\"geom\") WHERE id = NEW.\"fid\";END;
         CREATE TRIGGER \"{rt}_update7\" AFTER UPDATE OF \"geom\" ON \"{t}\" WHEN OLD.\"fid\" = NEW.\"fid\" AND {ne_new} AND {e_old} BEGIN INSERT INTO \"{rt}\" VALUES (NEW.\"fid\",{box_of_new}); END;
         CREATE TRIGGER \"{rt}_update2\" AFTER UPDATE OF \"geom\" ON \"{t}\" WHEN OLD.\"fid\" = NEW.\"fid\" AND {e_new} BEGIN DELETE FROM \"{rt}\" WHERE id = OLD.\"fid\"; END;
         CREATE TRIGGER \"{rt}_update5\" AFTER UPDATE ON \"{t}\" WHEN OLD.\"fid\" != NEW.\"fid\" AND {ne_new} BEGIN DELETE FROM \"{rt}\" WHERE id = OLD.\"fid\"; INSERT OR REPLACE INTO \"{rt}\" VALUES (NEW.\"fid\",{box_of_new}); END;
         CREATE TRIGGER \"{rt}_update4\" AFTER UPDATE ON \"{t}\" WHEN OLD.\"fid\" != NEW.\"fid\" AND {e_new} BEGIN DELETE FROM \"{rt}\" WHERE id IN (OLD.\"fid\", NEW.\"fid\"); END;
         CREATE TRIGGER \"{rt}_delete\" AFTER DELETE ON \"{t}\" WHEN old.\"geom\" NOT NULL BEGIN DELETE FROM \"{rt}\" WHERE id = OLD.\"fid\"; END;
         CREATE TRIGGER \"trigger_insert_feature_count_{t}\" AFTER INSERT ON \"{t}\" BEGIN UPDATE gpkg_ogr_contents SET feature_count = feature_count + 1 WHERE lower(table_name) = lower('{t}'); END;
         CREATE TRIGGER \"trigger_delete_feature_count_{t}\" AFTER DELETE ON \"{t}\" BEGIN UPDATE gpkg_ogr_contents SET feature_count = feature_count - 1 WHERE lower(table_name) = lower('{t}'); END;",
        ne_new = not_empty("NEW"),
        ne_old = not_empty("OLD"),
        e_new = is_empty("NEW"),
        e_old = is_empty("OLD"),
    );
    conn.execute_batch(&sql)
        .map_err(|e| StrataError::Storage(format!("「{t}」 표 생성 실패: {e}")))
}

/// 표가 있는지.
pub fn table_exists(conn: &Connection, name: &str) -> bool {
    conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type IN ('table','view') AND name = ?1",
        [name],
        |_| Ok(()),
    )
    .is_ok()
}
