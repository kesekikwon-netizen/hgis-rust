//! ESRI Shapefile 쓰기 — `.shp`·`.shx`·`.dbf`·`.prj`·`.cpg` 를 규격대로 쓴다.
//!
//! C++ 은 `QgsVectorFileWriter`(GDAL 의 ESRI Shapefile 드라이버)가 이 일을 한다.
//! 예전 Rust 판 `shp_writer::write_shapefile_utf8` 은 규격에 맞지 않는 파일(머리 4바이트 뒤에
//! 글자)을 썼다. 이 모듈은 ESRI Shapefile Technical Description(1998)의 바이트 배치를 따른다:
//! 파일 머리 100바이트(빅엔디언 길이 + 리틀엔디언 범위), 레코드마다 번호·길이(빅엔디언)와
//! 도형(리틀엔디언), dBase III 속성 표.

use std::path::Path;

use strata_types::error::{StrataError, StrataResult};

use crate::survey_doc::FieldValue;
use crate::wkb_codec::WkbGeometry;

/// 도형 종류(Shapefile 의 shape type 번호).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShpKind {
    Point = 1,
    PolyLine = 3,
    Polygon = 5,
}

/// 속성 열 하나.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShpField {
    pub name: String,
    /// `true` 면 숫자(N 24.15), 아니면 글자(C).
    pub numeric: bool,
}

/// 도형 하나와 그 속성(열 순서대로).
#[derive(Debug, Clone, PartialEq)]
pub struct ShpRecord {
    pub geom: WkbGeometry,
    pub values: Vec<FieldValue>,
}

const NUMERIC_WIDTH: usize = 24;
const NUMERIC_DECIMALS: u8 = 15;

fn signed_area(ring: &[[f64; 2]]) -> f64 {
    ring.windows(2).map(|w| w[0][0] * w[1][1] - w[1][0] * w[0][1]).sum::<f64>() * 0.5
}

/// 바깥 고리는 시계 방향, 구멍은 반시계 방향(Shapefile 규칙)으로 맞춘다.
fn oriented(ring: &[[f64; 2]], clockwise: bool) -> Vec<[f64; 2]> {
    let mut out = ring.to_vec();
    if (signed_area(ring) < 0.0) != clockwise {
        out.reverse();
    }
    out
}

/// 도형의 부분(고리·선)들. 점은 한 점짜리 부분 하나.
fn parts_of(geom: &WkbGeometry) -> Vec<Vec<[f64; 2]>> {
    match geom {
        WkbGeometry::Point(p) => vec![vec![*p]],
        WkbGeometry::LineString(pts) => vec![pts.clone()],
        WkbGeometry::Polygon(rings) => rings
            .iter()
            .enumerate()
            .map(|(i, r)| oriented(r, i == 0))
            .collect(),
    }
}

fn bbox(parts: &[Vec<[f64; 2]>]) -> [f64; 4] {
    let mut b = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
    for p in parts.iter().flatten() {
        b = [b[0].min(p[0]), b[1].min(p[1]), b[2].max(p[0]), b[3].max(p[1])];
    }
    b
}

fn record_content(kind: ShpKind, geom: &WkbGeometry) -> Vec<u8> {
    let parts = parts_of(geom);
    let mut out = Vec::new();
    out.extend_from_slice(&(kind as i32).to_le_bytes());
    if kind == ShpKind::Point {
        let p = parts.first().and_then(|r| r.first()).copied().unwrap_or([0.0, 0.0]);
        out.extend_from_slice(&p[0].to_le_bytes());
        out.extend_from_slice(&p[1].to_le_bytes());
        return out;
    }
    for v in bbox(&parts) {
        out.extend_from_slice(&v.to_le_bytes());
    }
    let n_points: usize = parts.iter().map(Vec::len).sum();
    out.extend_from_slice(&(parts.len() as i32).to_le_bytes());
    out.extend_from_slice(&(n_points as i32).to_le_bytes());
    let mut start = 0_i32;
    for part in &parts {
        out.extend_from_slice(&start.to_le_bytes());
        start += part.len() as i32;
    }
    for p in parts.iter().flatten() {
        out.extend_from_slice(&p[0].to_le_bytes());
        out.extend_from_slice(&p[1].to_le_bytes());
    }
    out
}

/// `.shp`/`.shx` 공통 머리 100바이트. `total_bytes` 는 파일 전체 길이.
fn file_header(kind: ShpKind, total_bytes: usize, extent: [f64; 4]) -> Vec<u8> {
    let mut h = Vec::with_capacity(100);
    h.extend_from_slice(&9994_i32.to_be_bytes());
    h.extend_from_slice(&[0_u8; 20]);
    h.extend_from_slice(&((total_bytes / 2) as i32).to_be_bytes());
    h.extend_from_slice(&1000_i32.to_le_bytes());
    h.extend_from_slice(&(kind as i32).to_le_bytes());
    for v in extent {
        h.extend_from_slice(&v.to_le_bytes());
    }
    h.extend_from_slice(&[0_u8; 32]); // Z·M 범위(쓰지 않음)
    h
}

fn text_of(v: &FieldValue) -> String {
    match v {
        FieldValue::Text(s) => s.clone(),
        FieldValue::Real(f) => f.to_string(),
        FieldValue::Int(i) => i.to_string(),
        FieldValue::Null | FieldValue::Blob(_) => String::new(),
    }
}

/// 글자를 `width` 바이트 안에서 UTF-8 글자 경계로 자른다.
fn clip_utf8(s: &str, width: usize) -> &[u8] {
    let mut end = s.len().min(width);
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s.as_bytes()[..end]
}

fn dbf_bytes(fields: &[ShpField], records: &[ShpRecord]) -> Vec<u8> {
    // 글자 열 폭: 가장 긴 값(UTF-8 바이트), 최소 80(GDAL 기본), 최대 254.
    let widths: Vec<usize> = fields
        .iter()
        .enumerate()
        .map(|(i, f)| {
            if f.numeric {
                NUMERIC_WIDTH
            } else {
                records
                    .iter()
                    .map(|r| r.values.get(i).map_or(0, |v| text_of(v).len()))
                    .max()
                    .unwrap_or(0)
                    .clamp(80, 254)
            }
        })
        .collect();
    let record_len = 1 + widths.iter().sum::<usize>();
    let header_len = 32 + 32 * fields.len() + 1;
    let mut out = Vec::with_capacity(header_len + record_len * records.len() + 1);
    out.push(0x03); // dBase III, 메모 없음
    out.extend_from_slice(&[126, 11, 11]); // 고친 날짜 2026-11-11 (YY-1900, MM, DD)
    out.extend_from_slice(&(records.len() as u32).to_le_bytes());
    out.extend_from_slice(&(header_len as u16).to_le_bytes());
    out.extend_from_slice(&(record_len as u16).to_le_bytes());
    out.extend_from_slice(&[0_u8; 20]);
    for (f, w) in fields.iter().zip(&widths) {
        let mut name = [0_u8; 11];
        let clipped = clip_utf8(&f.name, 10);
        name[..clipped.len()].copy_from_slice(clipped);
        out.extend_from_slice(&name);
        out.push(if f.numeric { b'N' } else { b'C' });
        out.extend_from_slice(&[0_u8; 4]);
        out.push(*w as u8);
        out.push(if f.numeric { NUMERIC_DECIMALS } else { 0 });
        out.extend_from_slice(&[0_u8; 14]);
    }
    out.push(0x0D);
    for r in records {
        out.push(b' '); // 지워지지 않은 레코드
        for (i, (f, w)) in fields.iter().zip(&widths).enumerate() {
            let value = r.values.get(i).unwrap_or(&FieldValue::Null);
            let mut cell = vec![b' '; *w];
            if f.numeric {
                let number = match value {
                    FieldValue::Real(v) if v.is_finite() => Some(*v),
                    FieldValue::Int(v) => Some(*v as f64),
                    FieldValue::Text(s) => s.trim().parse::<f64>().ok(),
                    _ => None,
                };
                if let Some(v) = number {
                    // 오른쪽 정렬. 24칸을 넘으면 소수 자리를 줄여 맞춘다.
                    let mut text = format!("{v:.15}");
                    let mut decimals = 15_usize;
                    while text.len() > *w && decimals > 0 {
                        decimals -= 1;
                        text = format!("{v:.decimals$}");
                    }
                    let bytes = text.as_bytes();
                    if bytes.len() <= *w {
                        cell[*w - bytes.len()..].copy_from_slice(bytes);
                    }
                }
            } else {
                let text = text_of(value);
                let bytes = clip_utf8(&text, *w);
                cell[..bytes.len()].copy_from_slice(bytes);
            }
            out.extend_from_slice(&cell);
        }
    }
    out.push(0x1A);
    out
}

/// Shapefile 한 벌을 쓴다. `base` 는 확장자 없는(또는 `.shp`) 경로, `prj_wkt` 는 좌표계 WKT.
pub fn write_shapefile(
    base: &Path,
    kind: ShpKind,
    fields: &[ShpField],
    records: &[ShpRecord],
    prj_wkt: &str,
) -> StrataResult<()> {
    for r in records {
        let ok = matches!(
            (&r.geom, kind),
            (WkbGeometry::Point(_), ShpKind::Point)
                | (WkbGeometry::LineString(_), ShpKind::PolyLine)
                | (WkbGeometry::Polygon(_), ShpKind::Polygon)
        );
        if !ok {
            return Err(StrataError::Storage(
                "도형 종류가 Shapefile 종류와 맞지 않습니다".into(),
            ));
        }
    }
    if let Some(parent) = base.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let contents: Vec<Vec<u8>> = records.iter().map(|r| record_content(kind, &r.geom)).collect();
    let mut extent = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
    for r in records {
        let b = bbox(&parts_of(&r.geom));
        extent = [extent[0].min(b[0]), extent[1].min(b[1]), extent[2].max(b[2]), extent[3].max(b[3])];
    }
    if records.is_empty() {
        extent = [0.0; 4];
    }
    let shp_len = 100 + contents.iter().map(|c| 8 + c.len()).sum::<usize>();
    let mut shp = file_header(kind, shp_len, extent);
    let mut shx = file_header(kind, 100 + 8 * contents.len(), extent);
    for (i, content) in contents.iter().enumerate() {
        shx.extend_from_slice(&((shp.len() / 2) as i32).to_be_bytes());
        shx.extend_from_slice(&((content.len() / 2) as i32).to_be_bytes());
        shp.extend_from_slice(&((i + 1) as i32).to_be_bytes());
        shp.extend_from_slice(&((content.len() / 2) as i32).to_be_bytes());
        shp.extend_from_slice(content);
    }
    std::fs::write(base.with_extension("shp"), shp)?;
    std::fs::write(base.with_extension("shx"), shx)?;
    std::fs::write(base.with_extension("dbf"), dbf_bytes(fields, records))?;
    std::fs::write(base.with_extension("prj"), prj_wkt)?;
    std::fs::write(base.with_extension("cpg"), "UTF-8")?;
    Ok(())
}

/// 쓴 `.shp` 를 다시 읽어 레코드 수를 센다(쓴 직후 확인용).
pub fn count_shp_records(shp_path: &Path) -> StrataResult<usize> {
    let bytes = std::fs::read(shp_path)?;
    if bytes.len() < 100 || bytes[0..4] != 9994_i32.to_be_bytes() {
        return Err(StrataError::Storage("Shapefile 머리가 아닙니다".into()));
    }
    let mut pos = 100;
    let mut n = 0;
    while pos + 8 <= bytes.len() {
        let words = i32::from_be_bytes([bytes[pos + 4], bytes[pos + 5], bytes[pos + 6], bytes[pos + 7]]);
        pos += 8 + 2 * words.max(0) as usize;
        n += 1;
    }
    if pos != bytes.len() {
        return Err(StrataError::Storage("Shapefile 레코드 길이가 맞지 않습니다".into()));
    }
    Ok(n)
}
