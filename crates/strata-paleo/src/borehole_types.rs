//! 시추 입력·요약·지층 타입과 USCS 분류.
//!
//! `borehole_strata` 에서 분리했다.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StratumLayer {
    pub depth_from_m: f64,
    pub depth_to_m: f64,
    pub layer_name: String,
    pub uscs_code: String,
    pub description: String,
    pub n_value: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoreholeRawInput {
    pub hole_id: String,
    pub x: f64,
    pub y: f64,
    pub ground_elev_m: f64,
    pub water_depth_m: f64,
    pub strata: Vec<StratumLayer>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParsedBoreholeSummary {
    pub hole_id: String,
    pub x: f64,
    pub y: f64,
    pub ground_elev_m: f64,
    pub fill_thickness_m: f64,
    pub alluvial_thickness_m: f64,
    pub weathered_top_depth_m: f64,
    pub water_depth_m: f64,
    pub paleo_surface_elev_m: f64,
    pub weathered_top_elev_m: f64,
    pub dominant_uscs: String,
    pub sed_type: String,
    pub has_organic_inclusion: bool,
    pub marker_color_hex: String,
}

pub fn classify_uscs(uscs_raw: &str) -> &'static str {
    let u = uscs_raw.trim().to_ascii_uppercase();
    if u.starts_with("GW")
        || u.starts_with("GP")
        || u.starts_with("GM")
        || u.starts_with("GC")
        || u.starts_with("SW")
        || u.starts_with("SP")
        || u.starts_with("SM")
        || u.starts_with("SC")
    {
        "COARSE"
    } else if u.starts_with("ML")
        || u.starts_with("CL")
        || u.starts_with("OL")
        || u.starts_with("MH")
        || u.starts_with("CH")
        || u.starts_with("OH")
        || u.starts_with("PT")
    {
        "FINE"
    } else {
        "UNKNOWN"
    }
}

/// C++ `BoreholeStrata.h` 고고학 입지 분석용 4대 퇴적 환경 코드 (`enum class PaleoSedimentType`) 1:1
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaleoSedimentType {
    BedrockResidual,
    FluvialTerrace,
    LeveeFanSand,
    BackswampClay,
}

/// C++ `BoreholeStrata::classifyUscsTo4Paleo(uscsCode, nValue, depth) -> PaleoSedimentType` 1:1
#[must_use]
pub fn classify_uscs_to_4_paleo(uscs_code: &str, n_value: f64, depth_m: f64) -> PaleoSedimentType {
    let u = uscs_code.trim().to_ascii_uppercase();
    if n_value >= 50.0 || u == "RK" || u == "WR" || u == "SR" || u == "HR" {
        PaleoSedimentType::BedrockResidual
    } else if u.starts_with("GW")
        || u.starts_with("GP")
        || u.starts_with("GM")
        || u.starts_with("GC")
        || (depth_m >= 8.0 && (u.starts_with("SW") || u.starts_with("SP") || n_value >= 30.0))
    {
        PaleoSedimentType::FluvialTerrace
    } else if u.starts_with("SM")
        || u.starts_with("SC")
        || u.starts_with("SW")
        || u.starts_with("SP")
    {
        PaleoSedimentType::LeveeFanSand
    } else {
        PaleoSedimentType::BackswampClay
    }
}
