//! CAD 레이어 분류와 국내 좌표계·행정구역 코드 검사.
//!
//! `cad_csv_io` 에서 분리했다.

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
