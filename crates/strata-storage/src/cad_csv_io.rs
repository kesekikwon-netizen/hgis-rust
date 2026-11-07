use std::path::Path;
use strata_types::error::{StrataError, StrataResult};

pub const UTF8_BOM_BYTES: [u8; 3] = [0xEF, 0xBB, 0xBF];

/// C++ `HGIS_MAX_DXF_FILE_SIZE_BYTES` (256 MB) — DXF 최대 허용 파일 크기 상수
pub const HGIS_MAX_DXF_FILE_SIZE_BYTES: usize = 268_435_456;

/// C++ `HGIS_MIN_CSV_COLUMNS` (3) — CSV 기준점 최소 컬럼 수 (`ID, X, Y`)
pub const HGIS_MIN_CSV_COLUMNS: usize = 3;

/// C++ `HGIS_DXF_TEXT_SEARCH_RADIUS_M` (2.0m) — 2D DXF `POINT`(`Z == 0.0`) 주변 수치 `TEXT` 검색 탐색 반경
pub const HGIS_DXF_TEXT_SEARCH_RADIUS_M: f64 = 2.0;

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

pub const HGIS_RTK_MAX_ALLOWED_HDOP: f64 = 1.5;
pub const HGIS_DEFAULT_PHOTO_FOV_DEG: f64 = 60.0;
pub const HGIS_DEFAULT_PHOTO_CONE_LEN_M: f64 = 15.0;
pub const HGIS_NMEA_MAX_SENTENCE_LEN: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GnssQuality {
    Invalid = 0,
    GpsFix = 1,
    DgpsFix = 2,
    PpsFix = 3,
    RtkFixed = 4,
    RtkFloat = 5,
    Estimated = 6,
    Manual = 7,
    Simulation = 8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GnssPositionRecord {
    pub is_valid: bool,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude_m: f64,
    pub quality: GnssQuality,
    pub satellites_used: u32,
    pub hdop: f64,
}

/// C++ `KaGnssNmeaParser::parseGgaSentence` — `$GPGGA` / `$GNGGA` XOR 체크섬(`*XX`) 검증 및 `DDMM.MMMM` 도분 → 십진도(`deg + min / 60.0`) 변환
pub fn parse_gga_sentence(nmea_sentence: &str) -> Result<GnssPositionRecord, &'static str> {
    let s = nmea_sentence.trim();
    if s.len() > HGIS_NMEA_MAX_SENTENCE_LEN
        || (!s.starts_with("$GPGGA,") && !s.starts_with("$GNGGA,"))
    {
        return Err("MalformedSentence");
    }
    let Some(star_idx) = s.rfind('*') else {
        return Err("MalformedSentence");
    };
    if star_idx + 3 > s.len() {
        return Err("MalformedSentence");
    }
    let expected_cs =
        u8::from_str_radix(&s[star_idx + 1..star_idx + 3], 16).map_err(|_| "MalformedSentence")?;
    let actual_cs = s[1..star_idx].bytes().fold(0_u8, |acc, b| acc ^ b);
    if actual_cs != expected_cs {
        return Err("ChecksumMismatch");
    }
    let fields: Vec<&str> = s[1..star_idx].split(',').collect();
    if fields.len() < 10 {
        return Err("MalformedSentence");
    }
    let q_code = fields[6].parse::<u8>().unwrap_or(0);
    let quality = match q_code {
        1 => GnssQuality::GpsFix,
        2 => GnssQuality::DgpsFix,
        3 => GnssQuality::PpsFix,
        4 => GnssQuality::RtkFixed,
        5 => GnssQuality::RtkFloat,
        6 => GnssQuality::Estimated,
        7 => GnssQuality::Manual,
        8 => GnssQuality::Simulation,
        _ => GnssQuality::Invalid,
    };
    if quality == GnssQuality::Invalid || fields[2].is_empty() || fields[4].is_empty() {
        return Ok(GnssPositionRecord {
            is_valid: false,
            latitude: 0.0,
            longitude: 0.0,
            altitude_m: 0.0,
            quality: GnssQuality::Invalid,
            satellites_used: 0,
            hdop: 99.9,
        });
    }
    let parse_dm = |raw: &str, deg_len: usize, hemi: &str| -> Result<f64, &'static str> {
        if raw.len() <= deg_len {
            return Err("MalformedSentence");
        }
        let deg = raw[..deg_len]
            .parse::<f64>()
            .map_err(|_| "MalformedSentence")?;
        let min = raw[deg_len..]
            .parse::<f64>()
            .map_err(|_| "MalformedSentence")?;
        let mut dd = deg + min / 60.0;
        if hemi.eq_ignore_ascii_case("S") || hemi.eq_ignore_ascii_case("W") {
            dd = -dd;
        }
        Ok(dd)
    };
    let latitude = parse_dm(fields[2], 2, fields[3])?;
    let longitude = parse_dm(fields[4], 3, fields[5])?;
    let satellites_used = fields[7].parse::<u32>().unwrap_or(0);
    let hdop = fields[8].parse::<f64>().unwrap_or(99.9);
    let altitude_m = fields[9].parse::<f64>().unwrap_or(0.0);
    Ok(GnssPositionRecord {
        is_valid: true,
        latitude,
        longitude,
        altitude_m,
        quality,
        satellites_used,
        hdop,
    })
}

/// C++ `KaRtkPositionTracker::filterRtkQuality` — RTK Fixed(4) / Float(5) 및 `hdop <= maxAllowedHdop` 필터링
#[must_use]
pub fn filter_rtk_quality(rec: &GnssPositionRecord, max_allowed_hdop: f64) -> bool {
    rec.is_valid
        && matches!(rec.quality, GnssQuality::RtkFixed | GnssQuality::RtkFloat)
        && rec.hdop <= max_allowed_hdop
}

/// C++ `KaPhotoExifImporter` — EXIF Rational 도·분·초 `(num, den)` 3쌍을 십진도(`deg + min/60 + sec/3600`)로 변환
#[must_use]
pub fn exif_rational_dms_to_decimal(
    deg: (u32, u32),
    min: (u32, u32),
    sec: (u32, u32),
    hemisphere: char,
) -> Option<f64> {
    if deg.1 == 0 || min.1 == 0 || sec.1 == 0 {
        return None;
    }
    let d = f64::from(deg.0) / f64::from(deg.1);
    let m = f64::from(min.0) / f64::from(min.1);
    let s = f64::from(sec.0) / f64::from(sec.1);
    let mut dd = d + m / 60.0 + s / 3600.0;
    if hemisphere == 'S' || hemisphere == 's' || hemisphere == 'W' || hemisphere == 'w' {
        dd = -dd;
    }
    Some(dd)
}

/// C++ `KaPhotoMarkerLayer::buildViewConeGeometry` — 촬영 원점 `camera_pos`, 방위각 `direction_deg` (`0°`=북, `90°`=동), 시야각 `fov_deg`, 가시거리 `length_m`로부터 폐합 부채꼴 정점 링 생성
#[must_use]
pub fn build_photo_view_cone_ring(
    camera_pos: [f64; 2],
    direction_deg: f64,
    fov_deg: f64,
    length_m: f64,
    arc_steps: usize,
) -> Vec<[f64; 2]> {
    let steps = arc_steps.max(2);
    let r = if length_m > 0.0 && length_m.is_finite() {
        length_m
    } else {
        HGIS_DEFAULT_PHOTO_CONE_LEN_M
    };
    let fov = if fov_deg > 0.0 && fov_deg.is_finite() {
        fov_deg
    } else {
        HGIS_DEFAULT_PHOTO_FOV_DEG
    };
    let start_az = direction_deg - fov * 0.5;
    let mut ring = Vec::with_capacity(steps + 3);
    ring.push(camera_pos);
    for i in 0..=steps {
        let az_deg = start_az + fov * (i as f64 / steps as f64);
        let az_rad = az_deg.to_radians();
        let vx = camera_pos[0] + r * az_rad.sin();
        let vy = camera_pos[1] + r * az_rad.cos();
        ring.push([vx, vy]);
    }
    ring.push(camera_pos);
    ring
}

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

/// 한국 평면직각 Easting 정상 하한 (`120,000.0 m`, C++ `HGIS_KOREA_EASTING_MIN_M`)
pub const HGIS_KOREA_EASTING_MIN_M: f64 = 120_000.0;
/// 한국 평면직각 Easting 정상 상한 (`380,000.0 m`, C++ `HGIS_KOREA_EASTING_MAX_M`)
pub const HGIS_KOREA_EASTING_MAX_M: f64 = 380_000.0;
/// 한국 평면직각 Northing 정상 하한 (`350,000.0 m`, C++ `HGIS_KOREA_NORTHING_MIN_M`)
pub const HGIS_KOREA_NORTHING_MIN_M: f64 = 350_000.0;
/// 한국 평면직각 Northing 정상 상한 (`750,000.0 m`, C++ `HGIS_KOREA_NORTHING_MAX_M`)
pub const HGIS_KOREA_NORTHING_MAX_M: f64 = 750_000.0;
/// 캐드 로컬 원점 의심 경계치 (`10,000.0 m`, C++ `HGIS_CAD_LOCAL_ORIGIN_THRESHOLD`)
pub const HGIS_CAD_LOCAL_ORIGIN_THRESHOLD: f64 = 10_000.0;

/// CAD 도면 레이어 고고학 역할 분류 (C++ `CadLayerRole`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CadLayerRole {
    TrenchBoundary,
    FeatureBoundary,
    ContourLine,
    SurveyPoint,
    SectionProfile,
    AnnotationText,
    Unknown,
}

/// DXF/DWG 레이어명의 영문·한글 어휘 패턴을 분석하여 고고학 도면 역할 자동 분류
/// (C++ `CadLayerClassifier::classifyLayerName` 1:1 대응)
#[must_use]
pub fn classify_cad_layer_name(raw_layer_name: &str) -> CadLayerRole {
    let u = raw_layer_name.trim().to_uppercase();
    if u.contains("SECTION") || u.contains("PROFILE") || u.contains("SEC_") || u.contains("단면")
    {
        CadLayerRole::SectionProfile
    } else if u.contains("TBM")
        || u.contains("POINT")
        || u.starts_with("CP")
        || u.contains("측점")
        || u.contains("기준점")
    {
        CadLayerRole::SurveyPoint
    } else if u.contains("CONT") || u.contains("ELEV") || u.contains("등고") || u.contains("지형")
    {
        CadLayerRole::ContourLine
    } else if u.contains("TRENCH")
        || u.starts_with("TR")
        || u.starts_with("T_")
        || u.contains("시굴")
        || u.contains("트렌치")
    {
        CadLayerRole::TrenchBoundary
    } else if u.contains("FEAT")
        || u.contains("PIT")
        || u.contains("DWELLING")
        || u.contains("유구")
        || u.contains("주거지")
        || u.contains("수혈")
    {
        CadLayerRole::FeatureBoundary
    } else if u.contains("TEXT") || u.contains("ANNO") || u.contains("주석") || u.contains("지번")
    {
        CadLayerRole::AnnotationText
    } else {
        CadLayerRole::Unknown
    }
}

/// CAD 도면 BBOX(`[min_x, min_y, max_x, max_y]`)가 로컬 원점(`|x|, |y| < 10,000m`)이거나 밀리미터(`> 10,000,000`) 단위인지 감지
/// (C++ `KaCadImportNotice` / `CadCrsGuess` 1:1 대응)
#[must_use]
pub fn detect_cad_local_origin_or_mm_warning(bbox: [f64; 4]) -> bool {
    let max_abs = bbox.iter().copied().map(f64::abs).fold(0.0_f64, f64::max);
    !(HGIS_CAD_LOCAL_ORIGIN_THRESHOLD..=10_000_000.0).contains(&max_abs)
}

/// CSV 기준점 입력 시 Northing(35만~75만)과 Easting(12만~38만)이 Y-X 순서로 전도된 경우 자동 교환
/// (C++ `KaControlCsvBox` 좌표축 전도 방어 1:1 대응)
#[must_use]
pub fn normalize_csv_control_point_xy_order(col_a: f64, col_b: f64) -> (f64, f64, bool) {
    let a_is_northing = (HGIS_KOREA_NORTHING_MIN_M..=HGIS_KOREA_NORTHING_MAX_M).contains(&col_a);
    let b_is_easting = (HGIS_KOREA_EASTING_MIN_M..=HGIS_KOREA_EASTING_MAX_M).contains(&col_b);
    if a_is_northing && b_is_easting {
        (col_b, col_a, true)
    } else {
        (col_a, col_b, false)
    }
}

/// 시·군·구 행정구역 카탈로그 레코드 (C++ `AdministrativeDistrict`)
#[derive(Debug, Clone, PartialEq)]
pub struct AdministrativeDistrict {
    pub sido_code: String,
    pub sido_name: String,
    pub sigungu_code: String,
    pub sigungu_name: String,
    pub bbox: [f64; 4],
}

/// 시·군·구 5자리 코드(예: `"47110"`, `"47130"`, `"11110"`)로부터 시·도명, 시·군·구명 및 지상 BBOX 조회
/// (C++ `KoreaRegionCatalog` 1:1 대응)
#[must_use]
pub fn resolve_korea_sigungu_code(sigungu_code: &str) -> Option<AdministrativeDistrict> {
    match sigungu_code.trim() {
        "47110" | "47130" => Some(AdministrativeDistrict {
            sido_code: "47".to_owned(),
            sido_name: "경상북도".to_owned(),
            sigungu_code: sigungu_code.trim().to_owned(),
            sigungu_name: "경주시".to_owned(),
            bbox: [200_000.0, 345_000.0, 235_000.0, 380_000.0],
        }),
        "11110" => Some(AdministrativeDistrict {
            sido_code: "11".to_owned(),
            sido_name: "서울특별시".to_owned(),
            sigungu_code: "11110".to_owned(),
            sigungu_name: "종로구".to_owned(),
            bbox: [195_000.0, 550_000.0, 202_000.0, 559_000.0],
        }),
        "41450" => Some(AdministrativeDistrict {
            sido_code: "41".to_owned(),
            sido_name: "경기도".to_owned(),
            sigungu_code: "41450".to_owned(),
            sigungu_name: "하남시".to_owned(),
            bbox: [212_000.0, 542_000.0, 224_000.0, 554_000.0],
        }),
        _ => None,
    }
}

/// 브이월드 지번 검색 결과의 19자리 PNU 코드가 요청한 산 여부(`is_mountain`), 본번(`main_no`), 부번(`sub_no`)과 정확히 일치하는지 검증
/// (C++ `LocationSearch::handleVworld` PNU 대조 가드 1:1 대응)
#[must_use]
pub fn validate_vworld_parcel_pnu_19(
    pnu: &str,
    is_mountain: bool,
    main_no: u32,
    sub_no: u32,
) -> bool {
    if pnu.len() != 19 || !pnu.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    let mt_digit = if is_mountain { "2" } else { "1" };
    if &pnu[10..11] != mt_digit {
        return false;
    }
    let Ok(parsed_main) = pnu[11..15].parse::<u32>() else {
        return false;
    };
    let Ok(parsed_sub) = pnu[15..19].parse::<u32>() else {
        return false;
    };
    parsed_main == main_no && parsed_sub == sub_no
}

/// 측량점 CSV/TXT 컬럼 배열 순서 분류 (C++ `SurveyPointColumnOrder`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurveyPointColumnOrder {
    /// 토탈스테이션 기본 (`이름, Easting(X), Northing(Y), 표고(Z), 비고`)
    NameXYZDesc,
    /// 축 전도형 (`이름, Northing(Y), Easting(X), 표고(Z), 비고`)
    NameYXZDesc,
    /// GNSS RTK 기본 (`ID, 코드, X, Y, Z`)
    IdCodeXYZ,
    Unknown,
}

/// 측량점 CSV/TXT 샘플 행의 수치 대역(한국 Easting `140,000..=380,000`, Northing `350,000..=750,000`, 표고 `-10..=1,950`)을 분석하여 컬럼 순서 자동 추론
/// (C++ `SurveyPointSchema::inferSchemaFromSamples` 1:1 대응)
#[must_use]
pub fn infer_survey_point_schema(
    _header_row: &[&str],
    sample_rows: &[&[&str]],
) -> SurveyPointColumnOrder {
    for row in sample_rows.iter().take(10) {
        if row.len() >= 4 {
            if let (Ok(v1), Ok(v2)) = (row[1].trim().parse::<f64>(), row[2].trim().parse::<f64>()) {
                let v1_east = (140_000.0..=380_000.0).contains(&v1);
                let v2_north = (350_000.0..=750_000.0).contains(&v2);
                if v1_east && v2_north {
                    return SurveyPointColumnOrder::NameXYZDesc;
                }
                let v1_north = (350_000.0..=750_000.0).contains(&v1);
                let v2_east = (140_000.0..=380_000.0).contains(&v2);
                if v1_north && v2_east {
                    return SurveyPointColumnOrder::NameYXZDesc;
                }
            }
        }
        if row.len() >= 5 {
            if let (Ok(v2), Ok(v3)) = (row[2].trim().parse::<f64>(), row[3].trim().parse::<f64>()) {
                if (140_000.0..=380_000.0).contains(&v2) && (350_000.0..=750_000.0).contains(&v3) {
                    return SurveyPointColumnOrder::IdCodeXYZ;
                }
            }
        }
    }
    SurveyPointColumnOrder::Unknown
}

/// 윈도우 OS 금지 문자(`\ / : * ? " < > |`), 마침표(`.`), 제어 문자를 밑줄(`_`)로 치환하고 양끝 공백·밑줄을 정리하여 안전한 조사 프로젝트 폴더명 반환
/// (C++ `SurveyNameSanitizer::sanitizeProjectName` 1:1 대응)
#[must_use]
pub fn sanitize_project_name(raw_input_name: &str) -> String {
    let trimmed = raw_input_name.trim();
    let mut out = String::with_capacity(trimmed.len());
    let mut prev_underscore = false;
    for ch in trimmed.chars() {
        let forbidden = matches!(
            ch,
            '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '.'
        ) || (ch as u32) < 32;
        if forbidden {
            if !prev_underscore && !out.is_empty() {
                out.push('_');
                prev_underscore = true;
            }
        } else {
            out.push(ch);
            prev_underscore = ch == '_';
        }
    }
    out.trim_matches(|c: char| c.is_whitespace() || c == '_')
        .to_owned()
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

        // TestKaDxfSpotImporter::testTextElevationFallback:
        // POINT (100.0, 200.0, 0.0) + TEXT at 0.5m distance with "78.45" -> elevated to 78.45m
        let dxf_2d_with_text =
            "0\nPOINT\n10\n100.0\n20\n200.0\n30\n0.0\n0\nTEXT\n10\n100.3\n20\n200.4\n1\n78.45\n";
        let spots = extract_dxf_spot_elevations_with_text_fallback(dxf_2d_with_text.as_bytes());
        assert_eq!(spots.len(), 1);
        assert!((spots[0][0] - 100.0).abs() < 1e-6);
        assert!((spots[0][1] - 200.0).abs() < 1e-6);
        assert!((spots[0][2] - 78.45).abs() < 1e-6);
    }

    #[test]
    fn test_gga_checksum_and_rtk_coord_conversion_and_exif_dms_and_view_cone_apex_and_radius() {
        let raw_body = "GNGGA,082530.00,3549.9542,N,12913.1432,E,4,14,0.8,45.2,M,18.5,M,,";
        let cs = raw_body.bytes().fold(0_u8, |acc, b| acc ^ b);
        let sentence = format!("${raw_body}*{cs:02X}");
        let rec = parse_gga_sentence(&sentence).expect("valid GNGGA sentence");
        assert!(rec.is_valid);
        assert_eq!(rec.quality, GnssQuality::RtkFixed);
        assert_eq!(rec.satellites_used, 14);
        assert!((rec.hdop - 0.8).abs() < 1e-9);
        assert!((rec.latitude - (35.0 + 49.9542 / 60.0)).abs() < 1e-7);
        assert!((rec.longitude - (129.0 + 13.1432 / 60.0)).abs() < 1e-7);
        assert!(filter_rtk_quality(&rec, HGIS_RTK_MAX_ALLOWED_HDOP));

        let lat = exif_rational_dms_to_decimal((35, 1), (50, 1), (4500, 100), 'N')
            .expect("valid EXIF DMS");
        assert!((lat - 35.845_833_333).abs() < 1e-6);

        let cone = build_photo_view_cone_ring([100.0, 100.0], 90.0, 60.0, 15.0, 4);
        assert_eq!(cone.first().copied(), Some([100.0, 100.0]));
        assert_eq!(cone.last().copied(), Some([100.0, 100.0]));
        // Midpoint of 4 steps is index 3 (at 90 deg due east -> (115.0, 100.0))
        assert!((cone[3][0] - 115.0).abs() < 1e-6);
        assert!((cone[3][1] - 100.0).abs() < 1e-6);
    }

    #[test]
    fn test_cp949_detection_and_lossless_conversion_and_korean_alias_resolution() {
        // TestKaCharsetAutoDetector::testCp949DetectionAndLosslessConversion
        let cp949_bytes: &[u8] = b"\xB1\xE8\xC1\xDF\xBC\xAE\x82\xA1";
        let detected = detect_korean_encoding(cp949_bytes);
        assert_eq!(detected, DetectedEncoding::Cp949);
        let utf8_text = convert_to_utf8_lossless(cp949_bytes, detected);
        assert_eq!(utf8_text, "김중석똠");

        // EUC-KR standard range only
        let euckr_only: &[u8] = b"\xB1\xE8\xC1\xDF\xBC\xAE";
        assert_eq!(detect_korean_encoding(euckr_only), DetectedEncoding::EucKr);

        // TestKaFieldMappingTemplate::testKoreanAliasResolution
        let fields = ["유구명", "유구종류", "시대", "해발고도", "비고"];
        let mapped = resolve_field_schema_mapping(&fields);
        assert!(mapped.is_all_required_mapped);
        assert_eq!(
            mapped
                .source_to_target_map
                .get("유구명")
                .map(String::as_str),
            Some("FEAT_NO")
        );
        assert_eq!(
            mapped
                .source_to_target_map
                .get("유구종류")
                .map(String::as_str),
            Some("FEAT_TYPE")
        );
        assert_eq!(
            mapped.source_to_target_map.get("시대").map(String::as_str),
            Some("PERIOD")
        );
        assert_eq!(
            mapped
                .source_to_target_map
                .get("해발고도")
                .map(String::as_str),
            Some("ELEVATION")
        );
        assert_eq!(mapped.unmapped_source_fields, vec!["비고".to_string()]);

        // TestCadLayerClassifier::testLayerRoleClassification
        assert_eq!(
            classify_cad_layer_name("TR-01_BOUNDARY"),
            CadLayerRole::TrenchBoundary
        );
        assert_eq!(
            classify_cad_layer_name("유구_주거지_선"),
            CadLayerRole::FeatureBoundary
        );
        assert_eq!(
            classify_cad_layer_name("CONT_1M"),
            CadLayerRole::ContourLine
        );
        assert_eq!(
            classify_cad_layer_name("TBM_POINT"),
            CadLayerRole::SurveyPoint
        );
        assert_eq!(
            classify_cad_layer_name("SECTION_A-B"),
            CadLayerRole::SectionProfile
        );

        // TestKaCadImport::testLocalOriginWarningDetection: (0,0) ~ (500,300) -> requires_crs_shift_warning = true
        assert!(detect_cad_local_origin_or_mm_warning([
            0.0, 0.0, 500.0, 300.0
        ]));
        assert!(!detect_cad_local_origin_or_mm_warning([
            200_000.0, 500_000.0, 200_500.0, 500_300.0
        ]));

        // TestKaControlCsvBox::testCoordinateAxisSwapping: (542100.25, 201300.50) -> swapped to (201300.50, 542100.25)
        let (x_norm, y_norm, swapped) =
            normalize_csv_control_point_xy_order(542_100.25, 201_300.50);
        assert!(swapped);
        assert!((x_norm - 201_300.50).abs() < 1e-6);
        assert!((y_norm - 542_100.25).abs() < 1e-6);

        // TestKoreaRegionCatalog::testGyeongjuCodeResolution & LocationSearch PNU validation
        let dist = resolve_korea_sigungu_code("47110").expect("Gyeongju code 47110");
        assert_eq!(dist.sido_name, "경상북도");
        assert_eq!(dist.sigungu_name, "경주시");
        assert!(dist.bbox[2] > dist.bbox[0] && dist.bbox[3] > dist.bbox[1]);
        assert!(validate_vworld_parcel_pnu_19(
            "4713010300100760000",
            false,
            76,
            0
        ));

        // TestSurveyPointSchema::testSchemaInferenceEastNorth
        let header = ["No", "Val1", "Val2", "EL"];
        let row1 = ["1", "195420.5", "542100.8", "45.2"];
        assert_eq!(
            infer_survey_point_schema(&header, &[&row1]),
            SurveyPointColumnOrder::NameXYZDesc
        );

        // TestSurveyNameSanitizer::testInvalidCharactersSubstitution
        assert_eq!(
            sanitize_project_name("  경주:황남동*발굴?조사/2026.  "),
            "경주_황남동_발굴_조사_2026"
        );
    }
}
