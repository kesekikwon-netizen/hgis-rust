//! DBF 읽기 — 인코딩 판정(`.cpg` → CP949 폴백)·필드 종류·값 문자열화.
//!
//! `shp_reader` 에서 분리했다(300줄 규칙).
//!
//! ## 한글 인코딩 처리 (C++ `LayerOps::prepareShapefileEncoding` 대응)
//!
//! 1. `.cpg` 가 있으면 그 이름을 따른다(`UTF-8`, `CP949`, `949`, `EUC-KR`, `ANSI`, `SYSTEM`).
//!    C++ `D:\hgis\src\core\LayerOps.cpp` `prepareShapefileEncoding` 1~2단계와 같다.
//! 2. `.cpg` 가 없으면 **CP949 폴백**. 한국 공공데이터·지적도·유적도 SHP 의 관례다
//!    (같은 파일 393줄 주석 "한국 공공데이터·지적도·유적도 SHP 기본값").
//! 3. 다만 `.cpg` 가 없고 본문이 **온전한 UTF-8 로 해독되면서 4바이트 이상의 멀티바이트**를
//!    가지면 UTF-8 로 본다. C++ `CadastralEncoding::detectDbfEncoding`(`D:\hgis\src\core\
//!    CadastralEncoding.cpp`) 과 `prepareShapefileEncoding` 2단계의 `utf8MultiByteCount >= 4`
//!    판정을 그대로 옮겼다. 이 한 단계가 없으면 `.cpg` 없는 UTF-8 SHP 한글이 `?` 로 깨진다.
//!
//! 인코딩은 `encoding_rs`(이미 워크스페이스에 있음)의 `EUC_KR` = CP949(확장 완성형 포함)를 쓴다.

use std::io::Cursor;
use std::path::Path;

use strata_types::error::{StrataError, StrataResult};

use crate::shp_reader::{find_sibling, ShpFieldKind};

/// `.cpg` 가 알려 주는 인코딩. C++ `prepareShapefileEncoding` 의 반환 문자열 자리.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShpEncoding {
    Utf8,
    /// CP949(EUC-KR·확장 완성형). 한국 공공 SHP 기본값.
    Cp949,
}

impl ShpEncoding {
    const fn dbase_name(self) -> &'static str {
        match self {
            Self::Utf8 => "UTF-8",
            Self::Cp949 => "CP949",
        }
    }
}

/// `.cpg` 내용을 인코딩으로 읽는다. C++ `prepareShapefileEncoding` 1단계와 같은 이름 규칙.
///
/// BOM·앞뒤 공백·끝 개행을 지운 뒤 대문자로 본다. 알아보지 못하는 이름이면 `None`
/// (C++ 은 그 이름을 GDAL 에 그대로 넘기지만, 여기서는 CP949 폴백으로 간다).
fn encoding_from_cpg_label(label: &str) -> Option<ShpEncoding> {
    let upper = label.trim().trim_start_matches('\u{feff}').to_ascii_uppercase();
    if upper.is_empty() {
        return None;
    }
    if upper.contains("UTF-8") || upper.contains("UTF8") || upper == "65001" {
        return Some(ShpEncoding::Utf8);
    }
    if upper.contains("CP949")
        || upper.contains("949")
        || upper.contains("EUC-KR")
        || upper.contains("EUCKR")
        || upper.contains("ANSI")
        || upper.contains("SYSTEM")
    {
        return Some(ShpEncoding::Cp949);
    }
    None
}

/// 본문 샘플이 "온전한 UTF-8 + 멀티바이트 4개 이상" 인가.
///
/// C++ `LayerOps::prepareShapefileEncoding` 2단계의 `hasNonAscii && validUtf8 &&
/// utf8MultiByteCount >= 4` 판정 1:1. 끝에서 잘린 시퀀스는 증거로 삼지 않는다
/// (C++ `CadastralEncoding::detectDbfEncoding` 의 `size - i < 4` 처리와 같은 취지).
fn looks_like_utf8(sample: &[u8]) -> bool {
    if sample.is_ascii() {
        return false;
    }
    let mut count = 0_usize;
    let mut i = 0_usize;
    while i < sample.len() {
        let b = sample[i];
        if b < 0x80 {
            i += 1;
            continue;
        }
        let len = match b {
            0xC2..=0xDF => 2,
            0xE0..=0xEF => 3,
            0xF0..=0xF4 => 4,
            _ => return false,
        };
        if i + len > sample.len() {
            break; // 끝에서 잘린 시퀀스는 판정에서 뺀다
        }
        if sample[i + 1..i + len]
            .iter()
            .any(|&c| c & 0xC0 != 0x80)
        {
            return false;
        }
        count += 1;
        i += len;
    }
    count >= 4
}

/// DBF 인코딩을 정한다. `.shp` 경로와 DBF 바이트를 받는다.
///
/// 1순위 `.cpg`, 2순위 UTF-8 모양이면 UTF-8, 3순위(기본) CP949.
pub fn resolve_encoding(shp_path: &Path, dbf_bytes: &[u8]) -> ShpEncoding {
    if let Some(cpg) = find_sibling(shp_path, "cpg") {
        if let Ok(label) = std::fs::read(&cpg) {
            // .cpg 는 1024바이트 이내의 짧은 이름 파일이다.
            let label = String::from_utf8_lossy(&label[..label.len().min(65_536)]).into_owned();
            if let Some(enc) = encoding_from_cpg_label(&label) {
                return enc;
            }
        }
    }
    let sample = dbf_record_sample(dbf_bytes, 32_768);
    if looks_like_utf8(&sample) {
        ShpEncoding::Utf8
    } else {
        ShpEncoding::Cp949
    }
}

/// DBF 헤더(필드 정의)를 건너뛴 레코드 본문 앞부분. C++ 이 `read(32)`·`read(32768)` 로 하던 자리.
fn dbf_record_sample(dbf: &[u8], limit: usize) -> Vec<u8> {
    let header_len = match dbf.get(8..10) {
        Some(v) => usize::from(u16::from_le_bytes([v[0], v[1]])),
        None => return Vec::new(),
    };
    let start = header_len.min(dbf.len());
    let end = (start + limit).min(dbf.len());
    dbf[start..end].to_vec()
}

/// 필드별 소수 자릿수(헤더를 직접 훑는다).
///
/// `dbase` 0.8 의 `FieldInfo::num_decimal_places` 는 필드 밖으로 노출되지 않아
/// 정수/실수 구분에 필요하다. 레이아웃: 헤더 32바이트, 필드 정의 32바이트씩,
/// 소수 자릿수는 각 필드 정의의 17번째 바이트.
#[must_use]
pub fn field_decimals(dbf: &[u8]) -> Vec<u8> {
    const HEADER: usize = 32;
    const FIELD: usize = 32;
    let header_len = match dbf.get(8..10) {
        Some(v) => usize::from(u16::from_le_bytes([v[0], v[1]])),
        None => return Vec::new(),
    };
    if header_len < HEADER + 1 {
        return Vec::new();
    }
    let count = (header_len - HEADER - 1) / FIELD;
    (0..count)
        .filter_map(|i| dbf.get(HEADER + i * FIELD + 17).copied())
        .collect()
}

/// `dbase::FieldType` → [`ShpFieldKind`].
#[must_use]
pub fn field_kind(field_type: shapefile::dbase::FieldType, decimals: u8) -> ShpFieldKind {
    use shapefile::dbase::FieldType;
    match field_type {
        FieldType::Character | FieldType::Memo => ShpFieldKind::Text,
        FieldType::Date | FieldType::DateTime => ShpFieldKind::Date,
        FieldType::Logical => ShpFieldKind::Logical,
        FieldType::Integer => ShpFieldKind::Integer,
        FieldType::Numeric | FieldType::Float if decimals == 0 => ShpFieldKind::Integer,
        FieldType::Numeric | FieldType::Float | FieldType::Currency | FieldType::Double => {
            ShpFieldKind::Real
        }
    }
}

/// DBF 를 지정 인코딩으로 연다. C++ 이 `SHAPE_ENCODING` 을 걸고 OGR 로 열던 자리.
pub fn open_dbf(
    dbf_bytes: Vec<u8>,
    encoding: ShpEncoding,
) -> StrataResult<shapefile::dbase::Reader<Cursor<Vec<u8>>>> {
    let name = encoding.dbase_name();
    let dyn_encoding =
        shapefile::dbase::encoding::DynEncoding::from_name(name).ok_or_else(|| {
            StrataError::Storage(format!("DBF 인코딩 '{name}' 을 쓸 수 없습니다"))
        })?;
    shapefile::dbase::ReaderBuilder::new()
        .with_encoding(dyn_encoding)
        .build(Cursor::new(dbf_bytes))
        .map_err(|e| StrataError::Storage(format!("DBF 를 열지 못했습니다: {e}")))
}

/// DBF 값을 문자열로 만든다. 숫자는 호출자가 파싱한다.
///
/// 숫자·날짜·논리값은 이미 ASCII 이므로 인코딩과 무관하다. 문자값만 복호화된 UTF-8 이다.
/// 공백뿐인 문자값은 `None`(빈 값)으로 본다 — `dbase` 가 이미 그렇게 준다.
#[must_use]
pub fn value_text(value: &shapefile::dbase::FieldValue, decimals: u8) -> Option<String> {
    use shapefile::dbase::FieldValue;
    match value {
        FieldValue::Character(Some(s)) | FieldValue::Memo(s) => Some(s.clone()),
        FieldValue::Character(None) => None,
        FieldValue::Numeric(Some(v)) => Some(format_number(*v, decimals)),
        FieldValue::Numeric(None) | FieldValue::Float(None) | FieldValue::Date(None) => None,
        FieldValue::Float(Some(v)) => Some(format_number(f64::from(*v), decimals)),
        FieldValue::Logical(Some(b)) => Some(if *b { "T" } else { "F" }.to_owned()),
        FieldValue::Logical(None) => None,
        FieldValue::Integer(v) => Some(v.to_string()),
        FieldValue::Double(v) => Some(format_number(*v, decimals)),
        FieldValue::Currency(v) => Some(format_number(*v, decimals)),
        // C++ 쪽 날짜 값 표기는 검수·보고서에서 `YYYY-MM-DD` 를 쓴다.
        FieldValue::Date(Some(d)) => Some(format!("{:04}-{:02}-{:02}", d.year(), d.month(), d.day())),
        FieldValue::DateTime(dt) => {
            let day = dt.date();
            Some(format!(
                "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
                day.year(),
                day.month(),
                day.day(),
                dt.time().hours(),
                dt.time().minutes(),
                dt.time().seconds()
            ))
        }
    }
}

/// 소수 자릿수만큼 자르되 뒤쪽 0 은 남긴다.
///
/// DBF 는 자릿수를 헤더에 적어 두므로 `5174.30` 처럼 값의 모양을 지킨다.
/// 자릿수 정보가 없으면(0) 정수면 정수로 쓴다.
fn format_number(v: f64, decimals: u8) -> String {
    if !v.is_finite() {
        return String::new();
    }
    if decimals == 0 {
        if v.fract() == 0.0 && v.abs() < 1e15 {
            return format!("{v:.0}");
        }
        return format!("{v}");
    }
    let width = usize::from(decimals);
    format!("{v:.width$}")
}

#[cfg(test)]
#[path = "shp_read_dbf_tests.rs"]
mod tests;
