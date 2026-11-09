//! 시추 기록 파싱(XML·Munsell·토질 기술)과 깊이 정리.
//!
//! `borehole_strata` 에서 분리했다.

use crate::borehole_types::classify_uscs;

use crate::borehole_types::{BoreholeRawInput, ParsedBoreholeSummary, StratumLayer};

pub fn parse_single_borehole(bh: &BoreholeRawInput) -> ParsedBoreholeSummary {
    let mut fill_thickness_m = 0.0_f64;
    let mut alluvial_thickness_m = 0.0_f64;
    let mut weathered_top_depth_m = 0.0_f64;
    let mut found_weathered = false;
    let mut coarse_thick = 0.0_f64;
    let mut fine_thick = 0.0_f64;
    let mut dominant_uscs = String::new();
    let mut has_organic = false;

    for s in &bh.strata {
        let thick = (s.depth_to_m - s.depth_from_m).max(0.0);
        let name = s.layer_name.trim();
        let desc = s.description.trim();
        if desc.contains("유기") || desc.contains("초탄") || desc.contains("식물") {
            has_organic = true;
        }
        if name.contains("매립") || name.contains("성토") || name.contains("표토") {
            fill_thickness_m += thick;
        } else if name.contains("충적") || name.contains("퇴적") || name.contains("붕적") {
            alluvial_thickness_m += thick;
            if dominant_uscs.is_empty() && !s.uscs_code.trim().is_empty() {
                dominant_uscs = s.uscs_code.trim().to_ascii_uppercase();
            }
            match classify_uscs(&s.uscs_code) {
                "COARSE" => coarse_thick += thick,
                "FINE" => fine_thick += thick,
                _ => {}
            }
        } else if !found_weathered
            && (name.contains("풍화")
                || name.contains("연암")
                || name.contains("경암")
                || name.contains("기반암"))
        {
            weathered_top_depth_m = s.depth_from_m;
            found_weathered = true;
        }
    }
    if !found_weathered {
        weathered_top_depth_m = fill_thickness_m + alluvial_thickness_m;
    }
    let sed_type = if coarse_thick >= fine_thick {
        String::from("COARSE")
    } else {
        String::from("FINE")
    };
    let marker_color_hex = if sed_type == "COARSE" {
        String::from("#D97706")
    } else {
        String::from("#2563EB")
    };

    ParsedBoreholeSummary {
        hole_id: bh.hole_id.clone(),
        x: bh.x,
        y: bh.y,
        ground_elev_m: bh.ground_elev_m,
        fill_thickness_m,
        alluvial_thickness_m,
        weathered_top_depth_m,
        water_depth_m: bh.water_depth_m,
        paleo_surface_elev_m: bh.ground_elev_m - fill_thickness_m,
        weathered_top_elev_m: bh.ground_elev_m - weathered_top_depth_m,
        dominant_uscs,
        sed_type,
        has_organic_inclusion: has_organic,
        marker_color_hex,
    }
}

/// 역전된 시추공 층위 심도(`depth_from_m > depth_to_m`)를 자동 교환하여 단면 뒤틀림 방지
#[allow(clippy::manual_swap)]
pub fn sanitize_borehole_strata_depths(strata: &mut [StratumLayer]) {
    for s in strata {
        if s.depth_from_m > s.depth_to_m {
            std::mem::swap(&mut s.depth_from_m, &mut s.depth_to_m);
        }
    }
}

/// 주상도 기둥 기본 폭 (mm, C++ `HGIS_DEFAULT_COLUMN_WIDTH_MM`)
pub const HGIS_DEFAULT_COLUMN_WIDTH_MM: f64 = 35.0;

/// 1:20 기본 축척 (1cm -> 0.5mm, C++ `HGIS_DEFAULT_COLUMN_SCALE`)
pub const HGIS_DEFAULT_COLUMN_SCALE: f64 = 0.05;

/// 인출선 최소 상하 간격 (mm, C++ `HGIS_MIN_LEADER_LINE_SPACING_MM`)
pub const HGIS_MIN_LEADER_LINE_SPACING_MM: f64 = 5.0;

/// 파싱 실패 시 대체 회색 (C++ `HGIS_FALLBACK_SOIL_COLOR_HEX`)
pub const HGIS_FALLBACK_SOIL_COLOR_HEX: &str = "#B4B4B4";

/// 먼셀 토색첩 파싱 결과 (C++ `MunsellColorRecord`)
#[derive(Debug, Clone, PartialEq)]
pub struct MunsellColorRecord {
    pub raw_notation: String,
    pub hue: String,
    pub value: f64,
    pub chroma: f64,
    pub srgb_color: [u8; 3],
    pub korean_color_name: String,
}

/// 먼셀 기호 파싱 에러 (C++ `MunsellParseError`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MunsellParseError {
    InvalidFormat,
}

/// 먼셀 토색 기호(`"10YR 4/6"`, `"2.5Y 5/3"`, `"5YR 3/4"`) 파싱 및 표준 한국어 토색명·sRGB 매핑
/// (C++ `KaMunsellSoilColorParser::parseMunsellNotation` 1:1 대응)
pub fn parse_munsell_notation(notation_str: &str) -> Result<MunsellColorRecord, MunsellParseError> {
    let trimmed = notation_str.trim();
    let mut parts = trimmed.split_whitespace();
    let hue_part = parts.next().ok_or(MunsellParseError::InvalidFormat)?;
    let vc_part = parts.next().ok_or(MunsellParseError::InvalidFormat)?;
    if parts.next().is_some() {
        return Err(MunsellParseError::InvalidFormat);
    }
    let has_digit = hue_part.chars().any(|c| c.is_ascii_digit());
    let has_alpha = hue_part.chars().any(|c| c.is_ascii_uppercase());
    if !has_digit || !has_alpha {
        return Err(MunsellParseError::InvalidFormat);
    }
    let (val_str, chr_str) = vc_part
        .split_once('/')
        .ok_or(MunsellParseError::InvalidFormat)?;
    let value: f64 = val_str
        .parse()
        .map_err(|_| MunsellParseError::InvalidFormat)?;
    let chroma: f64 = chr_str
        .parse()
        .map_err(|_| MunsellParseError::InvalidFormat)?;
    if !(0.0..=10.0).contains(&value) || !(0.0..=20.0).contains(&chroma) {
        return Err(MunsellParseError::InvalidFormat);
    }

    let (korean_name, srgb) = match (
        hue_part,
        (value * 10.0).round() as i32,
        (chroma * 10.0).round() as i32,
    ) {
        ("10YR", 40, 60) => ("황갈색", [156, 104, 36]),
        ("2.5Y", 50, 30) => ("회황색", [148, 132, 92]),
        ("5YR", 30, 40) => ("암갈색", [96, 58, 42]),
        ("7.5YR", 40, 40) => ("갈색", [128, 86, 52]),
        ("10YR", 30, 20) => ("흑갈색", [74, 62, 50]),
        _ => ("(미분류 토색)", [180, 180, 180]),
    };

    Ok(MunsellColorRecord {
        raw_notation: trimmed.to_owned(),
        hue: hue_part.to_owned(),
        value,
        chroma,
        srgb_color: srgb,
        korean_color_name: korean_name.to_owned(),
    })
}

/// 층위별 토색·토성·포함물·조밀도를 결합한 정형 학술 보고서 토층 주석 문장 생성
/// (C++ `KaSoilDescriptionGenerator::generateSoilDescription` 1:1 대응)
#[must_use]
pub fn generate_soil_description(
    color_label: &str,
    soil_texture: &str,
    inclusions: &str,
    compactness: &str,
) -> String {
    format!(
        "{color_label}의 {soil_texture}로서, {inclusions}이(가) 포함되어 있으며 층위 상태는 {compactness}함."
    )
}

/// 트렌치 주상도 지표면 심도(`cm`) 및 절대 해발 표고(`EL. m`) 이중 눈금자 결과
#[derive(Debug, Clone, PartialEq)]
pub struct DoubleRulerTick {
    pub depth_label: String,
    pub elevation_label: String,
    pub absolute_elevation_m: f64,
}

/// 지표면 해발 표고(`ground_elevation_m`)와 하향 심도(`depth_cm`)로부터 주상도 이중 눈금자 라벨 산출
/// (C++ `KaStratigraphicColumnLog` & `KaDepthRulerItem` 1:1 대응)
#[must_use]
pub fn compute_double_ruler_tick(ground_elevation_m: f64, depth_cm: f64) -> DoubleRulerTick {
    let d_cm = depth_cm.max(0.0);
    let abs_el = ground_elevation_m - (d_cm / 100.0);
    DoubleRulerTick {
        depth_label: format!("{d_cm:.0} cm"),
        elevation_label: format!("EL. {abs_el:.2} m"),
        absolute_elevation_m: abs_el,
    }
}

/// USCS 통일분류기호(`uscs`), 지층 설명(`desc`), 충적층 두께(`alluvial_m`)로부터 퇴적환경 한국어 분류 도출
/// (C++ `classifyUscsEnv` in `GeoInfoBoreholeService.cpp` 1:1 대응)
#[must_use]
pub fn classify_uscs_deposit_env_ko(
    uscs: &str,
    desc: &str,
    alluvial_thickness_m: f64,
) -> &'static str {
    if alluvial_thickness_m <= 0.01 {
        return "풍화대·기반암 우세";
    }
    let u = uscs.trim().to_ascii_uppercase();
    if u.starts_with('G') || desc.contains("자갈") || desc.contains("호박돌") {
        "자갈·붕적(선상지·하상)"
    } else if u.starts_with('C')
        || u.starts_with('M')
        || u.starts_with('O')
        || u.starts_with("PT")
        || desc.contains("점토")
        || desc.contains("실트")
    {
        "점토·실트 충적(구하도·저습지)"
    } else {
        "사질 충적(자연제방·미고지)"
    }
}
