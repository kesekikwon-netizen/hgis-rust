//! DXF 블록·표고점 추출.
//!
//! `cad_csv_io` 에서 분리했다.

use std::path::Path;

use strata_types::error::StrataResult;

use crate::csv_control_points::decode_korean_text;

/// C++ `HGIS_MAX_DXF_FILE_SIZE_BYTES` (256 MB) — DXF 최대 허용 파일 크기 상수
pub const HGIS_MAX_DXF_FILE_SIZE_BYTES: usize = 268_435_456;

/// C++ `HGIS_DXF_TEXT_SEARCH_RADIUS_M` (2.0m) — 2D DXF `POINT`(`Z == 0.0`) 주변 수치 `TEXT` 검색 탐색 반경
pub const HGIS_DXF_TEXT_SEARCH_RADIUS_M: f64 = 2.0;

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
pub fn parse_dxf_block_inserts(dxf_bytes: &[u8], block_name_filter: &str) -> Vec<CadInsertBlock> {
    let text = decode_korean_text(dxf_bytes);
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let filter_trim = block_name_filter.trim();
    let mut blocks = Vec::new();
    let mut in_insert = false;
    let mut cur_name = String::new();
    let mut cur_x: Option<f64> = None;
    let mut cur_y: Option<f64> = None;
    let mut cur_rot = 0.0_f64;

    let mut flush_insert =
        |name: &mut String, x: &mut Option<f64>, y: &mut Option<f64>, rot: &mut f64| {
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

/// C++ `KaDxfSpotImporter::importSpotElevations` 1:1 대응:
/// DXF `ENTITIES` 내 `POINT` 및 `TEXT`/`MTEXT` 엔티티를 스캔하여 그룹코드 `10`(X), `20`(Y), `30`(Z)을 파싱한다.
/// `POINT`의 Z값이 누락되었거나 `0.0`일 때 반경 `HGIS_DXF_TEXT_SEARCH_RADIUS_M`(`2.0m`) 이내에서 가장 가까운 수치 `TEXT`(그룹코드 `1`)를 찾아 표고로 승격한다.
#[must_use]
pub fn extract_dxf_spot_elevations_with_text_fallback(dxf_bytes: &[u8]) -> Vec<[f64; 3]> {
    if dxf_bytes.len() > HGIS_MAX_DXF_FILE_SIZE_BYTES {
        return Vec::new();
    }
    let text = decode_korean_text(dxf_bytes);
    let lines: Vec<&str> = text.lines().map(str::trim).collect();

    let mut raw_points: Vec<(f64, f64, f64)> = Vec::new();
    let mut text_labels: Vec<(f64, f64, f64)> = Vec::new();

    let mut entity_kind = String::new();
    let mut cur_x: Option<f64> = None;
    let mut cur_y: Option<f64> = None;
    let mut cur_z: Option<f64> = None;
    let mut cur_text_val: Option<f64> = None;

    let flush_entity = |kind: &str,
                        x: Option<f64>,
                        y: Option<f64>,
                        z: Option<f64>,
                        txt_z: Option<f64>,
                        pts: &mut Vec<(f64, f64, f64)>,
                        txts: &mut Vec<(f64, f64, f64)>| {
        let Some(px) = x else { return };
        let Some(py) = y else { return };
        if kind.eq_ignore_ascii_case("POINT") {
            pts.push((px, py, z.unwrap_or(0.0)));
        } else if kind.eq_ignore_ascii_case("TEXT") || kind.eq_ignore_ascii_case("MTEXT") {
            if let Some(tz) = txt_z {
                txts.push((px, py, tz));
            } else if let Some(pz) = z {
                if pz.abs() > 1e-9 {
                    txts.push((px, py, pz));
                }
            }
        }
    };

    let mut i = 0;
    while i + 1 < lines.len() {
        let code = lines[i];
        let val = lines[i + 1];
        if code == "0" {
            if !entity_kind.is_empty() {
                flush_entity(
                    &entity_kind,
                    cur_x,
                    cur_y,
                    cur_z,
                    cur_text_val,
                    &mut raw_points,
                    &mut text_labels,
                );
            }
            entity_kind = val.to_owned();
            cur_x = None;
            cur_y = None;
            cur_z = None;
            cur_text_val = None;
        } else {
            match code {
                "10" => cur_x = val.parse::<f64>().ok(),
                "20" => cur_y = val.parse::<f64>().ok(),
                "30" | "38" => cur_z = val.parse::<f64>().ok(),
                "1" => cur_text_val = val.trim().parse::<f64>().ok(),
                _ => {}
            }
        }
        i += 2;
    }
    if !entity_kind.is_empty() {
        flush_entity(
            &entity_kind,
            cur_x,
            cur_y,
            cur_z,
            cur_text_val,
            &mut raw_points,
            &mut text_labels,
        );
    }

    let r_sq = HGIS_DXF_TEXT_SEARCH_RADIUS_M * HGIS_DXF_TEXT_SEARCH_RADIUS_M;
    let mut out = Vec::new();
    for (px, py, pz) in raw_points {
        if pz.abs() > 1e-9 && pz > -500.0 && pz < 9000.0 {
            out.push([px, py, pz]);
        } else {
            let mut best: Option<(f64, f64)> = None;
            for &(tx, ty, tz) in &text_labels {
                let d2 = (px - tx).powi(2) + (py - ty).powi(2);
                if d2 <= r_sq && best.is_none_or(|(best_d2, _)| d2 < best_d2) {
                    best = Some((d2, tz));
                }
            }
            if let Some((_, fallback_z)) = best {
                out.push([px, py, fallback_z]);
            }
        }
    }
    out
}
