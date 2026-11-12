//! GDAL GPKG 공간 색인 트리거가 부르는 SQL 함수(ST_MinX·ST_MaxX·ST_MinY·ST_MaxY·ST_IsEmpty).
//!
//! GDAL 은 GPKG 를 열 때 이 함수들을 스스로 등록한다. C++ 조사 파일(과 같은 형식의 새 조사)의
//! 트리거가 이 함수를 부르므로, 조사 파일에 쓰는 Rust 연결도 먼저 등록해야 도형을 넣을 때
//! 「no such function」으로 실패하지 않는다.

use rusqlite::functions::FunctionFlags;
use rusqlite::types::ValueRef;
use rusqlite::Connection;

use crate::wkb_codec::decode_gpkg_wkb;

/// GPKG 도형 머리말(GeoPackage 1.4 §2.1.3)의 범위 [minx, maxx, miny, maxy]. 범위가 없으면
/// (GDAL 은 점에 범위를 쓰지 않는다) WKB 를 풀어 계산한다. 읽을 수 없으면 None.
fn envelope(blob: &[u8]) -> Option<[f64; 4]> {
    if blob.len() < 8 || &blob[..2] != b"GP" {
        return None;
    }
    let little = blob[3] & 0x01 == 0x01;
    if (blob[3] >> 1) & 0x07 == 0 {
        return decode_gpkg_wkb(blob).ok().map(|(_, g)| g.envelope_xy());
    }
    let read = |i: usize| -> Option<f64> {
        let bytes: [u8; 8] = blob.get(8 + i * 8..16 + i * 8)?.try_into().ok()?;
        Some(if little { f64::from_le_bytes(bytes) } else { f64::from_be_bytes(bytes) })
    };
    Some([read(0)?, read(1)?, read(2)?, read(3)?])
}

/// GPKG 머리말의 「빈 도형」 표시(flags 비트 4).
fn is_empty(blob: &[u8]) -> Option<bool> {
    (blob.len() >= 8 && &blob[..2] == b"GP").then(|| blob[3] & 0x10 != 0)
}

/// 조사 파일에 쓰는 연결마다 부른다. 도형이 아니면 NULL 을 돌려 트리거가 건너뛴다.
pub fn register_gpkg_sql_functions(conn: &Connection) -> rusqlite::Result<()> {
    let flags = FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC;
    for (name, idx) in [("ST_MinX", 0), ("ST_MaxX", 1), ("ST_MinY", 2), ("ST_MaxY", 3)] {
        conn.create_scalar_function(name, 1, flags, move |ctx| {
            Ok(match ctx.get_raw(0) {
                ValueRef::Blob(b) => envelope(b).map(|e| e[idx]),
                _ => None,
            })
        })?;
    }
    conn.create_scalar_function("ST_IsEmpty", 1, flags, |ctx| {
        Ok(match ctx.get_raw(0) {
            ValueRef::Blob(b) => is_empty(b).map(i64::from),
            _ => None,
        })
    })
}
