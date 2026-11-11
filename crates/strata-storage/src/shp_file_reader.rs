//! ESRI Shapefile 읽기 — `.shp` 도형과 `.dbf` 속성(한글 CP949·UTF-8).
//!
//! C++ 은 GDAL/OGR 이 SHP 를 읽는다(받은 지적도·수치지형도·주변유적, 끌어 놓은 SHP).
//! 예전 Rust 판에는 SHP 도형을 읽는 코드가 없어 「레이어 N개 추가됨」 문구만 냈다.
//! 점·선·면(여러 부분 포함)과 dBase III 속성을 읽는다. Z·M 값이 있는 종류는 XY 만 쓴다.

use std::collections::BTreeMap;
use std::path::Path;

use strata_types::error::{StrataError, StrataResult};

/// 도형 하나: 부분(고리·선)들의 목록. 점은 한 점짜리 부분 하나.
#[derive(Debug, Clone, PartialEq)]
pub struct ShpShape {
    pub parts: Vec<Vec<[f64; 2]>>,
    /// 열 이름 → 글자 값(양쪽 공백 제거).
    pub attrs: BTreeMap<String, String>,
}

/// 읽은 Shapefile 한 벌.
#[derive(Debug, Clone, PartialEq)]
pub struct ShpDataset {
    /// 1 점, 3 선, 5 면 (Z·M 종류는 같은 번호로 접는다).
    pub shape_type: i32,
    pub shapes: Vec<ShpShape>,
    /// 속성 열 이름(파일 순서).
    pub fields: Vec<String>,
    /// `.prj` 글자(없으면 빈 글자).
    pub prj_wkt: String,
}

fn err(msg: &str) -> StrataError {
    StrataError::Storage(msg.to_owned())
}

fn le_i32(b: &[u8], at: usize) -> StrataResult<i32> {
    let s = b.get(at..at + 4).ok_or_else(|| err("SHP 가 중간에 끊겼습니다"))?;
    Ok(i32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

fn be_i32(b: &[u8], at: usize) -> StrataResult<i32> {
    let s = b.get(at..at + 4).ok_or_else(|| err("SHP 가 중간에 끊겼습니다"))?;
    Ok(i32::from_be_bytes([s[0], s[1], s[2], s[3]]))
}

fn le_f64(b: &[u8], at: usize) -> StrataResult<f64> {
    let s = b.get(at..at + 8).ok_or_else(|| err("SHP 가 중간에 끊겼습니다"))?;
    let mut a = [0_u8; 8];
    a.copy_from_slice(s);
    Ok(f64::from_le_bytes(a))
}

/// 레코드 내용(도형 종류 번호부터)을 부분 목록으로 푼다. 빈 도형은 빈 목록.
fn parse_shape(content: &[u8]) -> StrataResult<Vec<Vec<[f64; 2]>>> {
    let kind = le_i32(content, 0)? % 10; // 11·13·15(Z), 21·23·25(M) → 1·3·5
    match kind {
        0 => Ok(Vec::new()),
        1 => Ok(vec![vec![[le_f64(content, 4)?, le_f64(content, 12)?]]]),
        3 | 5 => {
            let n_parts = le_i32(content, 36)?.max(0) as usize;
            let n_points = le_i32(content, 40)?.max(0) as usize;
            let starts: Vec<usize> = (0..n_parts)
                .map(|i| le_i32(content, 44 + 4 * i).map(|v| v.max(0) as usize))
                .collect::<StrataResult<_>>()?;
            let base = 44 + 4 * n_parts;
            let mut parts = Vec::with_capacity(n_parts);
            for (i, &start) in starts.iter().enumerate() {
                let end = starts.get(i + 1).copied().unwrap_or(n_points).min(n_points);
                let mut part = Vec::with_capacity(end.saturating_sub(start));
                for k in start..end {
                    part.push([le_f64(content, base + 16 * k)?, le_f64(content, base + 16 * k + 8)?]);
                }
                parts.push(part);
            }
            Ok(parts)
        }
        8 => {
            // MultiPoint: 점마다 부분 하나.
            let n = le_i32(content, 36)?.max(0) as usize;
            (0..n)
                .map(|k| Ok(vec![[le_f64(content, 40 + 16 * k)?, le_f64(content, 48 + 16 * k)?]]))
                .collect()
        }
        other => Err(StrataError::Storage(format!("지원하지 않는 SHP 도형 종류: {other}"))),
    }
}

/// `.cpg` 글자와 DBF 머리의 언어 표시로 인코딩을 정한다. 모르면 CP949(국내 자료의 관례).
fn decode_text(bytes: &[u8], utf8: bool) -> String {
    let trimmed: &[u8] = {
        let end = bytes.iter().rposition(|b| *b != b' ' && *b != 0).map_or(0, |i| i + 1);
        &bytes[..end]
    };
    if utf8 {
        if let Ok(s) = std::str::from_utf8(trimmed) {
            return s.trim().to_owned();
        }
    }
    let (text, _, _) = encoding_rs::EUC_KR.decode(trimmed);
    text.trim().to_owned()
}

/// DBF 표. 머리만 해석해 두고, 필요한 줄만 꺼내 푼다(필지 수십만 개짜리 지적도에서
/// 5 km 안의 몇 천 줄만 읽으려고).
struct DbfTable {
    bytes: Vec<u8>,
    fields: Vec<(String, usize)>,
    header_len: usize,
    record_len: usize,
    utf8: bool,
}

impl DbfTable {
    fn empty() -> Self {
        Self { bytes: Vec::new(), fields: Vec::new(), header_len: 0, record_len: 0, utf8: false }
    }

    fn row(&self, index: usize) -> BTreeMap<String, String> {
        let start = self.header_len + index * self.record_len;
        let Some(rec) = self.bytes.get(start..start + self.record_len) else {
            return BTreeMap::new();
        };
        let mut row = BTreeMap::new();
        let mut col = 1; // 첫 바이트는 삭제 표시
        for (name, width) in &self.fields {
            let cell = rec.get(col..col + width).unwrap_or(&[]);
            row.insert(name.clone(), decode_text(cell, self.utf8));
            col += width;
        }
        row
    }
}

fn read_dbf(path: &Path, utf8: bool) -> StrataResult<DbfTable> {
    let Ok(bytes) = std::fs::read(path) else {
        return Ok(DbfTable::empty()); // 속성 없는 SHP 도 연다.
    };
    if bytes.len() < 33 {
        return Err(err("DBF 머리가 너무 짧습니다"));
    }
    let header_len = u16::from_le_bytes([bytes[8], bytes[9]]) as usize;
    let record_len = u16::from_le_bytes([bytes[10], bytes[11]]) as usize;
    let mut fields: Vec<(String, usize)> = Vec::new();
    let mut at = 32;
    while at + 32 <= header_len.min(bytes.len()) && bytes[at] != 0x0D {
        let name_end = bytes[at..at + 11].iter().position(|c| *c == 0).unwrap_or(11);
        fields.push((decode_text(&bytes[at..at + name_end], utf8), bytes[at + 16] as usize));
        at += 32;
    }
    Ok(DbfTable { bytes, fields, header_len, record_len, utf8 })
}

/// 레코드 내용의 범위 `[min_x, min_y, max_x, max_y]`. 점은 그 점, 빈 도형은 `None`.
fn record_bbox(content: &[u8]) -> Option<[f64; 4]> {
    match le_i32(content, 0).ok()? % 10 {
        1 => {
            let (x, y) = (le_f64(content, 4).ok()?, le_f64(content, 12).ok()?);
            Some([x, y, x, y])
        }
        3 | 5 | 8 => Some([
            le_f64(content, 4).ok()?,
            le_f64(content, 12).ok()?,
            le_f64(content, 20).ok()?,
            le_f64(content, 28).ok()?,
        ]),
        _ => None,
    }
}

/// Shapefile 을 읽는다. `path` 는 `.shp` 경로.
pub fn read_shapefile(path: &Path) -> StrataResult<ShpDataset> {
    read_shapefile_in_box(path, None)
}

/// 범위(파일 좌표계의 `[min_x, min_y, max_x, max_y]`)에 걸친 도형만 읽는다.
///
/// 도형의 좌표와 속성을 풀기 전에 레코드에 적힌 범위로 먼저 거르므로, 필지 수십만 개짜리
/// 지적도에서도 조사구역 주변만 빠르게 읽는다(C++ `CadastralImport.cpp:170` 의 사각형 거르기).
pub fn read_shapefile_in_box(path: &Path, clip: Option<[f64; 4]>) -> StrataResult<ShpDataset> {
    let b = std::fs::read(path)?;
    if b.len() < 100 || be_i32(&b, 0)? != 9994 {
        return Err(err("Shapefile(.shp)이 아닙니다"));
    }
    let shape_type = le_i32(&b, 32)? % 10;
    let cpg = std::fs::read_to_string(path.with_extension("cpg")).unwrap_or_default();
    let cpg = cpg.trim().to_ascii_uppercase();
    let utf8 = cpg.contains("UTF-8") || cpg.contains("UTF8") || cpg == "65001";
    let table = read_dbf(&path.with_extension("dbf"), utf8)?;

    let mut shapes = Vec::new();
    let mut index = 0_usize; // 레코드 순번 = DBF 줄 번호
    let mut pos = 100;
    while pos + 8 <= b.len() {
        let words = be_i32(&b, pos + 4)?.max(0) as usize;
        let content = b
            .get(pos + 8..pos + 8 + 2 * words)
            .ok_or_else(|| err("SHP 레코드가 파일 끝을 넘습니다"))?;
        let wanted = clip.is_none_or(|c| {
            record_bbox(content).is_some_and(|b| b[0] <= c[2] && c[0] <= b[2] && b[1] <= c[3] && c[1] <= b[3])
        });
        if wanted {
            shapes.push(ShpShape {
                parts: parse_shape(content)?,
                attrs: table.row(index),
            });
        }
        index += 1;
        pos += 8 + 2 * words;
    }
    Ok(ShpDataset {
        shape_type,
        shapes,
        fields: table.fields.iter().map(|f| f.0.clone()).collect(),
        prj_wkt: std::fs::read_to_string(path.with_extension("prj")).unwrap_or_default(),
    })
}

#[cfg(test)]
#[path = "shp_file_reader_tests.rs"]
mod tests;
