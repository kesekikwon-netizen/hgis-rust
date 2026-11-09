//! C++ `addIsopachContourLayer` 의 GPKG 저장부 (`GeoInfoBoreholeMaps.cpp:127-135`) —
//! `QgsVectorFileWriter::writeAsVectorFormatV3(..., "GPKG", layerName)` 가 만들던
//! `borehole_isopach_contours` 레이어를 rusqlite 로 쓴다.
//!
//! 스키마 DDL 은 `crates/strata-storage/src/gpkg.rs:313-381` 의 프로젝트 표준과 맞추되
//! 이 크레이트는 strata-storage 에 의존하지 않는다. 지오메트리 블롭 인코딩은
//! [`crate::borehole_gpkg_geom`] 에 있다.

use std::path::Path;

use rusqlite::{params, Connection};
use strata_types::error::{StrataError, StrataResult};

use crate::borehole_gpkg_geom::{
    encode_gpkg_linestring, GPKG_APPLICATION_ID, GPKG_GEOMETRY_TYPE, GPKG_USER_VERSION,
};
use crate::borehole_isopach::{
    BOREHOLE_ISOPACH_FIELD_LABEL, BOREHOLE_ISOPACH_FIELD_THICKNESS, BOREHOLE_ISOPACH_LAYER_NAME,
    IsopachSegment,
};

pub use crate::borehole_gpkg_geom::{
    encode_gpkg_linestring as encode_contour_geometry, GPKG_GEOM_FLAGS_LE_XY,
};

/// 등층후선 세그먼트를 GPKG 로 기록한다.
///
/// C++ 는 임시 메모리 레이어를 만든 뒤 파일로 쓰지만(`:91-135`), 결과 파일 내용
/// (레이어명·속성 2개·LineString 지오메트리)은 같다.
pub fn write_isopach_contours_gpkg(
    out_path: &Path,
    srs_id: i32,
    segments: &[IsopachSegment],
) -> StrataResult<()> {
    if let Some(parent) = out_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    // C++ `QFile::remove(gpkgPath)` (`:90`) — 기존 파일을 지우고 새로 만든다.
    if out_path.exists() {
        std::fs::remove_file(out_path)?;
    }

    let conn = Connection::open(out_path)
        .map_err(|e| StrataError::Storage(format!("GPKG 생성 실패: {e}")))?;
    create_gpkg_schema(&conn, srs_id)?;
    // 등층후선은 한 번에 1,000줄이 넘는다. 트랜잭션 없이 한 줄씩 넣으면
    // 줄마다 암묵 트랜잭션이 열려 fsync 가 1,000번 넘게 일어나고,
    // 3공짜리 시험 하나가 160초를 넘겼다. 한 트랜잭션으로 묶는다.
    {
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| StrataError::Storage(format!("GPKG 트랜잭션 시작 실패: {e}")))?;
        insert_contour_features(&tx, srs_id, segments)?;
        tx.commit()
            .map_err(|e| StrataError::Storage(format!("GPKG 트랜잭션 커밋 실패: {e}")))?;
    }
    conn.close()
        .map_err(|(_, e)| StrataError::Storage(format!("GPKG 닫기 실패: {e}")))?;
    Ok(())
}

/// GeoPackage 메타데이터 3개 테이블과 등층후선 테이블을 만든다.
fn create_gpkg_schema(conn: &Connection, srs_id: i32) -> StrataResult<()> {
    let header = format!(
        "PRAGMA application_id = {GPKG_APPLICATION_ID};
         PRAGMA user_version = {GPKG_USER_VERSION};
         PRAGMA journal_mode = DELETE;
         PRAGMA synchronous = NORMAL;
         CREATE TABLE gpkg_spatial_ref_sys (
             srs_name TEXT NOT NULL,
             srs_id INTEGER NOT NULL PRIMARY KEY,
             organization TEXT NOT NULL,
             organization_coordsys_id INTEGER NOT NULL,
             definition TEXT NOT NULL,
             description TEXT
         );
         CREATE TABLE gpkg_contents (
             table_name TEXT NOT NULL PRIMARY KEY,
             data_type TEXT NOT NULL,
             identifier TEXT UNIQUE,
             description TEXT DEFAULT '',
             last_change DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
             min_x DOUBLE, min_y DOUBLE, max_x DOUBLE, max_y DOUBLE,
             srs_id INTEGER,
             CONSTRAINT fk_gc_r_srs_id FOREIGN KEY (srs_id) REFERENCES gpkg_spatial_ref_sys(srs_id)
         );
         CREATE TABLE gpkg_geometry_columns (
             table_name TEXT NOT NULL PRIMARY KEY,
             column_name TEXT NOT NULL DEFAULT 'geom',
             geometry_type_name TEXT NOT NULL,
             srs_id INTEGER NOT NULL,
             z TINYINT NOT NULL DEFAULT 0,
             m TINYINT NOT NULL DEFAULT 0
         );"
    );
    conn.execute_batch(&header)
        .map_err(|e| StrataError::Storage(format!("GPKG 스키마 초기화 실패: {e}")))?;

    // 프로젝트 표준 SRS 3종(`gpkg.rs:321-324`)과 같은 정의를 넣는다.
    conn.execute_batch(
        "INSERT INTO gpkg_spatial_ref_sys VALUES
             ('Korea 2000 / East Belt 2010', 5187, 'EPSG', 5187, 'PROJCS[\"Korea 2000 / East Belt 2010\"]', '동부원점'),
             ('Korea 2000 / Central Belt 2010', 5186, 'EPSG', 5186, 'PROJCS[\"Korea 2000 / Central Belt 2010\"]', '중부원점'),
             ('Korea 2000 / Unified CS', 5179, 'EPSG', 5179, 'PROJCS[\"Korea 2000 / Unified CS\"]', '제출용 UTM-K'),
             ('Undefined cartesian SRS', -1, 'NONE', -1, 'undefined', 'undefined cartesian coordinate reference system');",
    )
    .map_err(|e| StrataError::Storage(format!("GPKG SRS 등록 실패: {e}")))?;

    conn.execute(
        "INSERT INTO gpkg_contents (table_name, data_type, identifier, description, srs_id)
         VALUES (?1, 'features', ?2, '', ?3)",
        params![BOREHOLE_ISOPACH_LAYER_NAME, BOREHOLE_ISOPACH_LAYER_NAME, srs_id],
    )
    .map_err(|e| StrataError::Storage(format!("GPKG gpkg_contents 등록 실패: {e}")))?;
    conn.execute(
        "INSERT INTO gpkg_geometry_columns (table_name, column_name, geometry_type_name, srs_id, z, m)
         VALUES (?1, 'geom', ?2, ?3, 0, 0)",
        params![BOREHOLE_ISOPACH_LAYER_NAME, GPKG_GEOMETRY_TYPE, srs_id],
    )
    .map_err(|e| StrataError::Storage(format!("GPKG gpkg_geometry_columns 등록 실패: {e}")))?;

    // C++ 메모리 레이어 속성: `thickness_m`(Double), `label`(String) — `:93-96`
    conn.execute_batch(&format!(
        "CREATE TABLE {BOREHOLE_ISOPACH_LAYER_NAME} (
             fid INTEGER PRIMARY KEY AUTOINCREMENT,
             geom LINESTRING,
             {BOREHOLE_ISOPACH_FIELD_THICKNESS} DOUBLE,
             {BOREHOLE_ISOPACH_FIELD_LABEL} TEXT
         );"
    ))
    .map_err(|e| StrataError::Storage(format!("GPKG 등층후선 테이블 생성 실패: {e}")))?;
    Ok(())
}

/// 세그먼트를 준비된 statement 로 넣고 `gpkg_contents` 경계를 갱신한다.
fn insert_contour_features(
    conn: &Connection,
    srs_id: i32,
    segments: &[IsopachSegment],
) -> StrataResult<()> {
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    {
        let mut stmt = conn
            .prepare(&format!(
                "INSERT INTO {BOREHOLE_ISOPACH_LAYER_NAME} (geom, {BOREHOLE_ISOPACH_FIELD_THICKNESS}, {BOREHOLE_ISOPACH_FIELD_LABEL})
                 VALUES (?1, ?2, ?3)"
            ))
            .map_err(|e| StrataError::Storage(format!("GPKG 등층후선 삽입 준비 실패: {e}")))?;
        for seg in segments {
            let blob = encode_gpkg_linestring(srs_id, &seg.points);
            stmt.execute(params![blob, seg.thickness_m, seg.label])
                .map_err(|e| StrataError::Storage(format!("GPKG 등층후선 피처 삽입 실패: {e}")))?;
            for p in &seg.points {
                min_x = min_x.min(p[0]);
                max_x = max_x.max(p[0]);
                min_y = min_y.min(p[1]);
                max_y = max_y.max(p[1]);
            }
        }
    }

    // C++ `mem.updateExtents()` (`:126`) 에 해당하는 경계 갱신.
    if min_x.is_finite() {
        conn.execute(
            "UPDATE gpkg_contents SET min_x = ?1, min_y = ?2, max_x = ?3, max_y = ?4
             WHERE table_name = ?5",
            params![min_x, min_y, max_x, max_y, BOREHOLE_ISOPACH_LAYER_NAME],
        )
        .map_err(|e| StrataError::Storage(format!("GPKG 경계 갱신 실패: {e}")))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(level: f64, a: [f64; 2], b: [f64; 2]) -> IsopachSegment {
        IsopachSegment {
            thickness_m: level,
            label: format!("{level:.1}m"),
            points: [a, b],
        }
    }

    fn count_rows(conn: &Connection) -> StrataResult<i64> {
        conn.query_row(
            &format!("SELECT COUNT(*) FROM {BOREHOLE_ISOPACH_LAYER_NAME}"),
            [],
            |r| r.get(0),
        )
        .map_err(|e| StrataError::Storage(format!("개수 조회 실패: {e}")))
    }

    #[test]
    fn gpkg_file_passes_integrity_check_and_has_gpkg_metadata() -> StrataResult<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("시추_등층후선.gpkg");
        let segments = vec![
            seg(0.5, [10.0, 25.0], [10.0, 15.0]),
            seg(1.0, [15.0, 25.0], [15.0, 15.0]),
        ];
        write_isopach_contours_gpkg(&path, 5187, &segments)?;
        assert!(path.exists());

        let bytes = std::fs::read(&path)?;
        assert_eq!(&bytes[0..16], b"SQLite format 3\0");

        let conn = Connection::open(&path)
            .map_err(|e| StrataError::Storage(format!("검증 열기 실패: {e}")))?;
        let integrity: String = conn
            .query_row("PRAGMA integrity_check;", [], |r| r.get(0))
            .map_err(|e| StrataError::Storage(format!("integrity_check 실패: {e}")))?;
        assert_eq!(integrity, "ok");

        let app_id: i32 = conn
            .query_row("PRAGMA application_id;", [], |r| r.get(0))
            .map_err(|e| StrataError::Storage(format!("application_id 실패: {e}")))?;
        assert_eq!(app_id, GPKG_APPLICATION_ID);
        let user_version: i32 = conn
            .query_row("PRAGMA user_version;", [], |r| r.get(0))
            .map_err(|e| StrataError::Storage(format!("user_version 실패: {e}")))?;
        assert_eq!(user_version, GPKG_USER_VERSION);
        assert_eq!(count_rows(&conn)?, 2);

        // 속성·지오메트리 왕복 확인
        let (thickness, label, geom): (f64, String, Vec<u8>) = conn
            .query_row(
                &format!(
                    "SELECT {BOREHOLE_ISOPACH_FIELD_THICKNESS}, {BOREHOLE_ISOPACH_FIELD_LABEL}, geom
                     FROM {BOREHOLE_ISOPACH_LAYER_NAME} ORDER BY fid LIMIT 1"
                ),
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .map_err(|e| StrataError::Storage(format!("행 조회 실패: {e}")))?;
        assert!((thickness - 0.5).abs() < 1e-12);
        assert_eq!(label, "0.5m");
        assert_eq!(&geom[0..2], b"GP");

        // gpkg_contents 등록 및 경계 갱신 확인
        let (data_type, min_x, max_x): (String, f64, f64) = conn
            .query_row(
                "SELECT data_type, min_x, max_x FROM gpkg_contents WHERE table_name = ?1",
                params![BOREHOLE_ISOPACH_LAYER_NAME],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .map_err(|e| StrataError::Storage(format!("gpkg_contents 조회 실패: {e}")))?;
        assert_eq!(data_type, "features");
        assert_eq!(min_x, 10.0);
        assert_eq!(max_x, 15.0);

        let geom_type: String = conn
            .query_row(
                "SELECT geometry_type_name FROM gpkg_geometry_columns WHERE table_name = ?1",
                params![BOREHOLE_ISOPACH_LAYER_NAME],
                |r| r.get(0),
            )
            .map_err(|e| StrataError::Storage(format!("geometry_columns 조회 실패: {e}")))?;
        assert_eq!(geom_type, "LINESTRING");
        Ok(())
    }

    #[test]
    fn rewriting_existing_gpkg_replaces_previous_features() -> StrataResult<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("rewrite.gpkg");
        write_isopach_contours_gpkg(&path, 5187, &[seg(0.5, [0.0, 0.0], [1.0, 1.0])])?;
        write_isopach_contours_gpkg(
            &path,
            5187,
            &[
                seg(1.0, [0.0, 0.0], [1.0, 1.0]),
                seg(1.5, [1.0, 1.0], [2.0, 2.0]),
            ],
        )?;
        let conn = Connection::open(&path)
            .map_err(|e| StrataError::Storage(format!("검증 열기 실패: {e}")))?;
        assert_eq!(count_rows(&conn)?, 2, "C++ QFile::remove then rewrite");
        Ok(())
    }

    #[test]
    fn empty_segments_still_produce_valid_gpkg_with_zero_rows() -> StrataResult<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("empty.gpkg");
        write_isopach_contours_gpkg(&path, 5187, &[])?;
        let conn = Connection::open(&path)
            .map_err(|e| StrataError::Storage(format!("검증 열기 실패: {e}")))?;
        assert_eq!(count_rows(&conn)?, 0);
        Ok(())
    }
}
