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
        } else if name.contains("충적") || name.contains("퇴적") || name.contains("붕적")
        {
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
