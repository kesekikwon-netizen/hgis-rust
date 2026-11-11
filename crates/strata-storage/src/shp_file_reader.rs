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

type DbfTable = (Vec<String>, Vec<BTreeMap<String, String>>);

fn read_dbf(path: &Path, utf8: bool) -> StrataResult<DbfTable> {
    let Ok(b) = std::fs::read(path) else {
        return Ok((Vec::new(), Vec::new())); // 속성 없는 SHP 도 연다.
    };
    if b.len() < 33 {
        return Err(err("DBF 머리가 너무 짧습니다"));
    }
    let n_records = u32::from_le_bytes([b[4], b[5], b[6], b[7]]) as usize;
    let header_len = u16::from_le_bytes([b[8], b[9]]) as usize;
    let record_len = u16::from_le_bytes([b[10], b[11]]) as usize;
    let mut fields: Vec<(String, usize)> = Vec::new();
    let mut at = 32;
    while at + 32 <= header_len.min(b.len()) && b[at] != 0x0D {
        let name_end = b[at..at + 11].iter().position(|c| *c == 0).unwrap_or(11);
        fields.push((decode_text(&b[at..at + name_end], utf8), b[at + 16] as usize));
        at += 32;
    }
    let mut rows = Vec::with_capacity(n_records);
    for r in 0..n_records {
        let start = header_len + r * record_len;
        let Some(rec) = b.get(start..start + record_len) else {
            break;
        };
        let mut row = BTreeMap::new();
        let mut col = 1; // 첫 바이트는 삭제 표시
        for (name, width) in &fields {
            let cell = rec.get(col..col + width).unwrap_or(&[]);
            row.insert(name.clone(), decode_text(cell, utf8));
            col += width;
        }
        rows.push(row);
    }
    Ok((fields.into_iter().map(|f| f.0).collect(), rows))
}

/// Shapefile 을 읽는다. `path` 는 `.shp` 경로.
pub fn read_shapefile(path: &Path) -> StrataResult<ShpDataset> {
    let b = std::fs::read(path)?;
    if b.len() < 100 || be_i32(&b, 0)? != 9994 {
        return Err(err("Shapefile(.shp)이 아닙니다"));
    }
    let shape_type = le_i32(&b, 32)? % 10;
    let cpg = std::fs::read_to_string(path.with_extension("cpg")).unwrap_or_default();
    let cpg = cpg.trim().to_ascii_uppercase();
    let utf8 = cpg.contains("UTF-8") || cpg.contains("UTF8") || cpg == "65001";
    let (fields, rows) = read_dbf(&path.with_extension("dbf"), utf8)?;

    let mut shapes = Vec::new();
    let mut pos = 100;
    while pos + 8 <= b.len() {
        let words = be_i32(&b, pos + 4)?.max(0) as usize;
        let content = b
            .get(pos + 8..pos + 8 + 2 * words)
            .ok_or_else(|| err("SHP 레코드가 파일 끝을 넘습니다"))?;
        shapes.push(ShpShape {
            parts: parse_shape(content)?,
            attrs: rows.get(shapes.len()).cloned().unwrap_or_default(),
        });
        pos += 8 + 2 * words;
    }
    Ok(ShpDataset {
        shape_type,
        shapes,
        fields,
        prj_wkt: std::fs::read_to_string(path.with_extension("prj")).unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shp_file_writer::{write_shapefile, ShpField, ShpKind, ShpRecord};
    use crate::survey_doc::FieldValue;
    use crate::wkb_codec::WkbGeometry;

    #[test]
    fn what_the_writer_writes_the_reader_reads_back_with_korean_text() -> StrataResult<()> {
        let dir = tempfile::tempdir()?;
        let base = dir.path().join("유구");
        // 구멍이 있는 면 하나.
        let outer = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0], [0.0, 0.0]];
        let hole = vec![[4.0, 4.0], [6.0, 4.0], [6.0, 6.0], [4.0, 6.0], [4.0, 4.0]];
        let fields = vec![
            ShpField { name: "kind".to_owned(), numeric: false },
            ShpField { name: "depth_m".to_owned(), numeric: true },
        ];
        let records = vec![ShpRecord {
            geom: WkbGeometry::Polygon(vec![outer, hole]),
            values: vec![FieldValue::Text("주거지".to_owned()), FieldValue::Real(1.25)],
        }];
        write_shapefile(&base, ShpKind::Polygon, &fields, &records, "PRJ")?;

        let ds = read_shapefile(&base.with_extension("shp"))?;
        assert_eq!((ds.shape_type, ds.shapes.len(), ds.prj_wkt.as_str()), (5, 1, "PRJ"));
        assert_eq!(ds.fields, vec!["kind".to_owned(), "depth_m".to_owned()]);
        let shape = &ds.shapes[0];
        assert_eq!(shape.parts.len(), 2, "바깥 고리와 구멍");
        assert_eq!(shape.parts[0].len(), 5);
        assert_eq!(shape.attrs["kind"], "주거지");
        assert!((shape.attrs["depth_m"].parse::<f64>().unwrap_or(0.0) - 1.25).abs() < 1e-12);
        Ok(())
    }

    #[test]
    fn cp949_attributes_without_a_cpg_file_are_read_as_korean() -> StrataResult<()> {
        let dir = tempfile::tempdir()?;
        let base = dir.path().join("jijeok");
        let records = vec![ShpRecord {
            geom: WkbGeometry::Point([200_000.0, 450_000.0]),
            values: vec![FieldValue::Text(String::new())],
        }];
        write_shapefile(&base, ShpKind::Point, &[ShpField { name: "JIBUN".to_owned(), numeric: false }], &records, "")?;
        // 국토부 지적도처럼: .cpg 가 없고 값이 CP949 로 적힌 DBF.
        std::fs::remove_file(base.with_extension("cpg"))?;
        let dbf_path = base.with_extension("dbf");
        let mut dbf = std::fs::read(&dbf_path)?;
        let header_len = u16::from_le_bytes([dbf[8], dbf[9]]) as usize;
        let (cp949, _, _) = encoding_rs::EUC_KR.encode("산81 임");
        dbf[header_len + 1..header_len + 1 + cp949.len()].copy_from_slice(&cp949);
        std::fs::write(&dbf_path, dbf)?;

        let ds = read_shapefile(&base.with_extension("shp"))?;
        assert_eq!(ds.shapes[0].attrs["JIBUN"], "산81 임");
        assert_eq!(ds.shapes[0].parts, vec![vec![[200_000.0, 450_000.0]]]);
        Ok(())
    }

    #[test]
    fn a_file_that_is_not_a_shapefile_is_refused() {
        let dir = tempfile::tempdir().unwrap_or_else(|_| std::process::abort());
        let path = dir.path().join("x.shp");
        let _ = std::fs::write(&path, vec![0_u8; 200]);
        assert!(read_shapefile(&path).is_err());
    }
}
