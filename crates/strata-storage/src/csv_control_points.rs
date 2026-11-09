//! 기준점 CSV 가져오기 — BOM 처리·행 검사·좌표 순서 정규화.
//!
//! `cad_csv_io` 에서 분리했다.

use strata_types::error::{StrataError, StrataResult};

pub const UTF8_BOM_BYTES: [u8; 3] = [0xEF, 0xBB, 0xBF];

/// C++ `HGIS_MIN_CSV_COLUMNS` (3) — CSV 기준점 최소 컬럼 수 (`ID, X, Y`)
pub const HGIS_MIN_CSV_COLUMNS: usize = 3;

#[derive(Debug, Clone, PartialEq)]
pub struct ControlPointRow {
    pub point_id: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// C++ `CsvBenchmarkImporter::CsvImportResult` — 파싱된 기준점 목록, 경고 메시지 리스트, 스킵된 행 수
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CsvImportResult {
    pub points: Vec<ControlPointRow>,
    pub warnings: Vec<String>,
    pub skipped_rows: usize,
}

pub fn decode_korean_text(bytes: &[u8]) -> String {
    let stripped = bytes.strip_prefix(&UTF8_BOM_BYTES).unwrap_or(bytes);
    if let Ok(s) = std::str::from_utf8(stripped) {
        return s.trim_start_matches('\u{feff}').to_string();
    }
    let (cow, _, _) = encoding_rs::EUC_KR.decode(stripped);
    cow.into_owned()
}

/// C++ `CsvBenchmarkImporter::importControlPoints` — UTF-8 BOM 제거, 헤더 행 자동 감지, 비정상 숫자 행 건너뛰기 및 경고 기록
#[must_use]
pub fn import_control_points_csv_checked(bytes: &[u8]) -> CsvImportResult {
    let text = decode_korean_text(bytes);
    let mut res = CsvImportResult::default();
    for (idx, raw_line) in text.lines().enumerate() {
        let line_no = idx + 1;
        let trimmed = raw_line.trim().trim_start_matches('\u{feff}');
        if trimmed.is_empty() {
            continue;
        }
        let cols: Vec<&str> = trimmed.split([',', '\t']).map(str::trim).collect();
        if idx == 0 {
            let c0_upper = cols[0].to_ascii_uppercase();
            let c1_upper = cols.get(1).copied().unwrap_or("").to_ascii_uppercase();
            if c0_upper == "ID" || c1_upper == "X" || c0_upper.contains("NAME") {
                continue;
            }
        }
        if cols.len() < 3 {
            res.skipped_rows += 1;
            res.warnings.push(format!(
                "[Line {line_no}] 열 개수 미달로 행을 건너뜁니다: {trimmed}"
            ));
            continue;
        }
        let clean_x = cols[1].replace(',', "");
        let clean_y = cols[2].replace(',', "");
        let (Ok(x), Ok(y)) = (clean_x.parse::<f64>(), clean_y.parse::<f64>()) else {
            res.skipped_rows += 1;
            res.warnings.push(format!(
                "[Line {line_no}] 숫자 파싱 실패로 행을 건너뜁니다: {trimmed}"
            ));
            continue;
        };
        let z = cols
            .get(3)
            .and_then(|s| s.replace(',', "").parse::<f64>().ok())
            .unwrap_or(0.0);
        res.points.push(ControlPointRow {
            point_id: cols[0].to_owned(),
            x,
            y,
            z,
        });
    }
    res
}

pub fn parse_control_points_csv(bytes: &[u8]) -> StrataResult<Vec<ControlPointRow>> {
    let text = decode_korean_text(bytes);
    let mut rows = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let cols: Vec<&str> = trimmed.split([',', '\t']).map(str::trim).collect();
        if cols.len() < 3 {
            continue;
        }
        let Ok(x) = cols[1].parse::<f64>() else {
            continue;
        };
        let Ok(y) = cols[2].parse::<f64>() else {
            continue;
        };
        let z = cols
            .get(3)
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        rows.push(ControlPointRow {
            point_id: cols[0].to_string(),
            x,
            y,
            z,
        });
    }
    if rows.is_empty() {
        return Err(StrataError::InvalidInput(
            "CSV에서 유효한 기준점 좌표를 찾지 못했습니다".into(),
        ));
    }
    Ok(rows)
}
