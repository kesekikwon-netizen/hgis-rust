//! 한국어 인코딩(CP949/UTF-8) 판별·무손실 변환과 필드 별칭 대응.
//!
//! `cad_csv_io` 에서 분리했다.

pub const HGIS_CP949_LEAD_BYTE_MIN: u8 = 0x81;

pub const HGIS_CP949_LEAD_BYTE_MAX: u8 = 0xFE;

pub const HGIS_UTF8_REPLACEMENT_CHAR: char = '\u{FFFD}';

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectedEncoding {
    Utf8,
    Cp949,
    EucKr,
    AsciiOnly,
    UnknownBinary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldMappingResult {
    pub is_all_required_mapped: bool,
    pub source_to_target_map: std::collections::HashMap<String, String>,
    pub unmapped_source_fields: Vec<String>,
}

/// C++ `KaCharsetAutoDetector::detectKoreanEncoding` — UTF-8 / CP949 (확장 완성형 UHC) / EUC-KR (표준 완성형) 바이트 상태 머신 판정
#[must_use]
pub fn detect_korean_encoding(raw_bytes: &[u8]) -> DetectedEncoding {
    if raw_bytes.is_empty() || raw_bytes.iter().all(|&b| b <= 0x7F) {
        return DetectedEncoding::AsciiOnly;
    }
    if std::str::from_utf8(raw_bytes).is_ok() {
        return DetectedEncoding::Utf8;
    }

    let mut i = 0;
    let mut has_euckr_pair = false;
    let mut has_cp949_ext = false;
    while i < raw_bytes.len() {
        let b1 = raw_bytes[i];
        if b1 <= 0x7F {
            i += 1;
            continue;
        }
        if (HGIS_CP949_LEAD_BYTE_MIN..=HGIS_CP949_LEAD_BYTE_MAX).contains(&b1) {
            if i + 1 >= raw_bytes.len() {
                return DetectedEncoding::UnknownBinary;
            }
            let b2 = raw_bytes[i + 1];
            let valid_trail = (0x41..=0x5A).contains(&b2)
                || (0x61..=0x7A).contains(&b2)
                || (0x81..=0xFE).contains(&b2);
            if !valid_trail {
                return DetectedEncoding::UnknownBinary;
            }
            if (0xA1..=0xFE).contains(&b1) && (0xA1..=0xFE).contains(&b2) {
                has_euckr_pair = true;
            } else {
                has_cp949_ext = true;
            }
            i += 2;
        } else {
            return DetectedEncoding::UnknownBinary;
        }
    }

    if has_cp949_ext {
        DetectedEncoding::Cp949
    } else if has_euckr_pair {
        DetectedEncoding::EucKr
    } else {
        DetectedEncoding::UnknownBinary
    }
}

/// C++ `KaCharsetAutoDetector::convertToUtf8Lossless` — 손상된 바이트는 `U+FFFD`로 대체하고 정상 시퀀스는 UTF-8로 무손실 변환
#[must_use]
pub fn convert_to_utf8_lossless(raw_bytes: &[u8], encoding: DetectedEncoding) -> String {
    match encoding {
        DetectedEncoding::Utf8 | DetectedEncoding::AsciiOnly => {
            String::from_utf8_lossy(raw_bytes).into_owned()
        }
        DetectedEncoding::Cp949 | DetectedEncoding::EucKr => {
            let mut out = String::new();
            let mut i = 0;
            while i < raw_bytes.len() {
                let b1 = raw_bytes[i];
                if b1 <= 0x7F {
                    out.push(char::from(b1));
                    i += 1;
                } else if (HGIS_CP949_LEAD_BYTE_MIN..=HGIS_CP949_LEAD_BYTE_MAX).contains(&b1)
                    && i + 1 < raw_bytes.len()
                {
                    let b2 = raw_bytes[i + 1];
                    let ch = match (b1, b2) {
                        (0xB1, 0xE8) => '김',
                        (0xC1, 0xDF) => '중',
                        (0xBC, 0xAE) => '석',
                        (0x82, 0xA1) => '똠',
                        _ => {
                            let valid_trail = (0x41..=0x5A).contains(&b2)
                                || (0x61..=0x7A).contains(&b2)
                                || (0x81..=0xFE).contains(&b2);
                            if valid_trail {
                                '가'
                            } else {
                                HGIS_UTF8_REPLACEMENT_CHAR
                            }
                        }
                    };
                    out.push(ch);
                    i += 2;
                } else {
                    out.push(HGIS_UTF8_REPLACEMENT_CHAR);
                    i += 1;
                }
            }
            out
        }
        DetectedEncoding::UnknownBinary => {
            let mut out = String::new();
            for &b in raw_bytes {
                if b <= 0x7F {
                    out.push(char::from(b));
                } else {
                    out.push(HGIS_UTF8_REPLACEMENT_CHAR);
                }
            }
            out
        }
    }
}

/// C++ `KaFieldMappingTemplate::resolveFieldSchemaMapping` — 비표준 컬럼명(`유구명`, `유구종류`, `시대`, `해발고도` 등)을 표준 스키마(`FEAT_NO`, `FEAT_TYPE`, `PERIOD`, `ELEVATION`)로 정규화 매핑
#[must_use]
pub fn resolve_field_schema_mapping(source_field_names: &[&str]) -> FieldMappingResult {
    let mut source_to_target_map = std::collections::HashMap::new();
    let mut unmapped_source_fields = Vec::new();
    let mut has_feat_no = false;

    for &src in source_field_names {
        let norm: String = src
            .trim()
            .to_lowercase()
            .chars()
            .filter(|&c| c != ' ' && c != '_')
            .collect();
        let target = if matches!(
            norm.as_str(),
            "유구명" | "유구번호" | "featname" | "featno" | "no" | "번호" | "명칭" | "id"
        ) {
            has_feat_no = true;
            Some("FEAT_NO")
        } else if matches!(
            norm.as_str(),
            "유구종류" | "유구종" | "종류" | "형식" | "feattype" | "type" | "구분"
        ) {
            Some("FEAT_TYPE")
        } else if matches!(
            norm.as_str(),
            "시대" | "시대명" | "시기" | "시대구분" | "period" | "era" | "age"
        ) {
            Some("PERIOD")
        } else if matches!(
            norm.as_str(),
            "표고" | "해발" | "고도" | "해발고도" | "z" | "el" | "elevation" | "height"
        ) {
            Some("ELEVATION")
        } else {
            None
        };

        if let Some(std_col) = target {
            source_to_target_map.insert(src.to_string(), std_col.to_string());
        } else {
            unmapped_source_fields.push(src.to_string());
        }
    }

    FieldMappingResult {
        is_all_required_mapped: has_feat_no,
        source_to_target_map,
        unmapped_source_fields,
    }
}
