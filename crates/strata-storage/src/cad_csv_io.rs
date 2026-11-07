use std::path::Path;
use strata_types::error::{StrataError, StrataResult};

pub const UTF8_BOM_BYTES: [u8; 3] = [0xEF, 0xBB, 0xBF];

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
        let z = cols.get(3).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
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

pub fn extract_dxf_elevation_points(path: &Path) -> StrataResult<Vec<[f64; 3]>> {
    let bytes = std::fs::read(path)?;
    let text = decode_korean_text(&bytes);
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let mut pts = Vec::new();
    let mut cur_x: Option<f64> = None;
    let mut cur_y: Option<f64> = None;
    let mut i = 0;
    while i + 1 < lines.len() {
        let code = lines[i];
        let val = lines[i + 1];
        match code {
            "10" => cur_x = val.parse::<f64>().ok(),
            "20" => cur_y = val.parse::<f64>().ok(),
            "30" | "38" => {
                if let (Some(x), Some(y), Ok(z)) = (cur_x, cur_y, val.parse::<f64>()) {
                    if z > -500.0 && z < 9000.0 {
                        pts.push([x, y, z]);
                    }
                }
            }
            _ => {}
        }
        i += 2;
    }
    Ok(pts)
}

/// C++ `CadCsvDxfImporter::loadDxfControlPoints` — 손상/미지원 DXF 헤더 검사 및 에러 전파
pub fn load_dxf_control_points_validated(path: &Path) -> Result<Vec<[f64; 3]>, String> {
    let pts = extract_dxf_elevation_points(path)
        .map_err(|_| "DXF 형식이 손상되었거나 지원하지 않는 엔티티 버전입니다".to_owned())?;
    if pts.is_empty() {
        return Err("DXF 형식이 손상되었거나 지원하지 않는 엔티티 버전입니다".to_owned());
    }
    Ok(pts)
}

/// C++ `DxfBlockAnnotationParser::CadInsertBlock` — DXF `INSERT` 심볼 블록 정보 1:1
#[derive(Debug, Clone, PartialEq)]
pub struct CadInsertBlock {
    pub block_name: String,
    pub x: f64,
    pub y: f64,
    pub rotation_deg: f64,
}

/// C++ `DxfBlockAnnotationParser::parseBlockInserts(dxfPath, blockNameFilter)` 1:1
pub fn parse_dxf_block_inserts(
    dxf_bytes: &[u8],
    block_name_filter: &str,
) -> Vec<CadInsertBlock> {
    let text = decode_korean_text(dxf_bytes);
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let filter_trim = block_name_filter.trim();
    let mut blocks = Vec::new();
    let mut in_insert = false;
    let mut cur_name = String::new();
    let mut cur_x: Option<f64> = None;
    let mut cur_y: Option<f64> = None;
    let mut cur_rot = 0.0_f64;

    let mut flush_insert = |name: &mut String, x: &mut Option<f64>, y: &mut Option<f64>, rot: &mut f64| {
        if let (Some(px), Some(py)) = (*x, *y) {
            if !name.is_empty() && (filter_trim.is_empty() || name == filter_trim) {
                blocks.push(CadInsertBlock {
                    block_name: name.clone(),
                    x: px,
                    y: py,
                    rotation_deg: *rot,
                });
            }
        }
        name.clear();
        *x = None;
        *y = None;
        *rot = 0.0;
    };

    let mut i = 0;
    while i + 1 < lines.len() {
        let code = lines[i];
        let val = lines[i + 1];
        if code == "0" {
            if in_insert {
                flush_insert(&mut cur_name, &mut cur_x, &mut cur_y, &mut cur_rot);
            }
            in_insert = val.eq_ignore_ascii_case("INSERT");
        } else if in_insert {
            match code {
                "2" => cur_name = val.to_owned(),
                "10" => cur_x = val.parse::<f64>().ok(),
                "20" => cur_y = val.parse::<f64>().ok(),
                "50" => cur_rot = val.parse::<f64>().unwrap_or(0.0),
                _ => {}
            }
        }
        i += 2;
    }
    if in_insert {
        flush_insert(&mut cur_name, &mut cur_x, &mut cur_y, &mut cur_rot);
    }
    blocks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dxf_block_insert_parsing_extracts_5_point_benchmark_blocks() {
        let mut dxf_sample = String::new();
        for idx in 0..5 {
            dxf_sample.push_str(&format!(
                "0\nINSERT\n2\nPOINT_BENCHMARK\n10\n{:.1}\n20\n{:.1}\n50\n{:.1}\n",
                200_000.0 + f64::from(idx) * 10.0,
                350_000.0 + f64::from(idx) * 20.0,
                f64::from(idx) * 15.0
            ));
        }
        let parsed = parse_dxf_block_inserts(dxf_sample.as_bytes(), "POINT_BENCHMARK");
        assert_eq!(parsed.len(), 5);
        assert_eq!(parsed[0].block_name, "POINT_BENCHMARK");
        assert!((parsed[4].x - 200_040.0).abs() < 1e-6);
        assert!((parsed[4].rotation_deg - 60.0).abs() < 1e-6);
    }

    #[test]
    fn test_csv_benchmark_importer_bom_and_skip_malformed_row_extracts_2_valid_points_and_1_warning(
    ) {
        let mut csv_bytes = UTF8_BOM_BYTES.to_vec();
        csv_bytes.extend_from_slice(
            "ID,X,Y,Name\nCP01,200123.45,543210.12,기준점A\nCP02,BAD_NUMBER,543215.00,불량행\nCP03,200150.00,543220.00,기준점B\n".as_bytes(),
        );
        let result = import_control_points_csv_checked(&csv_bytes);
        assert_eq!(result.points.len(), 2);
        assert_eq!(result.skipped_rows, 1);
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("[Line 3]"));
        assert_eq!(result.points[0].point_id, "CP01");
        assert!((result.points[0].x - 200_123.45).abs() < 1e-6);
        assert_eq!(result.points[1].point_id, "CP03");
    }
}
