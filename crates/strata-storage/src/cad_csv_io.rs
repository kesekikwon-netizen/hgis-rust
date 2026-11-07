use std::path::Path;
use strata_types::error::{StrataError, StrataResult};

#[derive(Debug, Clone, PartialEq)]
pub struct ControlPointRow {
    pub point_id: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

pub fn decode_korean_text(bytes: &[u8]) -> String {
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.trim_start_matches('\u{feff}').to_string();
    }
    let (cow, _, _) = encoding_rs::EUC_KR.decode(bytes);
    cow.into_owned()
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
