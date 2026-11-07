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

/// `GeoInfoBoreholeService.cpp` & `HgisPaleoBorehole.cpp` (`67a3f33`) — 밀집 시추공 공간 디클러스터링(격자 셀당 최심도·최다 지층 대표 시추공 1공 보존, 최대 `max_holes` 제한)
#[must_use]
pub fn decluster_boreholes_by_grid(
    raw: &[BoreholeRawInput],
    cell_size_m: f64,
    max_holes: usize,
) -> Vec<BoreholeRawInput> {
    use std::collections::BTreeMap;
    let step = cell_size_m.max(1.0);
    let mut best_per_cell: BTreeMap<(i64, i64), (f64, usize, BoreholeRawInput)> = BTreeMap::new();

    for bh in raw {
        let cx = (bh.x / step).floor() as i64;
        let cy = (bh.y / step).floor() as i64;
        let max_depth = bh
            .strata
            .iter()
            .map(|s| s.depth_to_m)
            .fold(0.0_f64, f64::max);
        let score_strata_cnt = bh.strata.len();
        match best_per_cell.get(&(cx, cy)) {
            Some((prev_depth, prev_cnt, _))
                if *prev_depth > max_depth
                    || ((*prev_depth - max_depth).abs() < 1e-6 && *prev_cnt >= score_strata_cnt) => {}
            _ => {
                best_per_cell.insert((cx, cy), (max_depth, score_strata_cnt, bh.clone()));
            }
        }
    }

    let mut selected: Vec<(f64, usize, BoreholeRawInput)> = best_per_cell.into_values().collect();
    selected.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.1.cmp(&a.1))
    });
    selected
        .into_iter()
        .take(max_holes.max(1))
        .map(|(_, _, bh)| bh)
        .collect()
}

/// C++ `BoreholeDeclustering::decluster(rawBoreholes, minDistanceMeters = 30.0)`
/// 및 `std::atomic<bool> m_cancelRequested` 취소 플래그 지원 거리 임계 디클러스터링
pub fn decluster_boreholes_with_cancel(
    raw: &[BoreholeRawInput],
    min_distance_m: f64,
    max_holes: usize,
    cancel_requested: Option<&std::sync::atomic::AtomicBool>,
) -> Result<Vec<BoreholeRawInput>, &'static str> {
    use std::sync::atomic::Ordering;

    let mut ranked: Vec<(f64, usize, &BoreholeRawInput)> = Vec::with_capacity(raw.len());
    for bh in raw {
        if cancel_requested.is_some_and(|c| c.load(Ordering::Relaxed)) {
            return Err("시추공 디클러스터링 작업이 취소되었습니다");
        }
        let max_depth = bh
            .strata
            .iter()
            .map(|s| s.depth_to_m)
            .fold(0.0_f64, f64::max);
        ranked.push((max_depth, bh.strata.len(), bh));
    }

    ranked.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.1.cmp(&a.1))
    });

    let min_dist_sq = min_distance_m.max(1.0).powi(2);
    let mut kept: Vec<BoreholeRawInput> = Vec::new();
    for (_, _, cand) in ranked {
        if cancel_requested.is_some_and(|c| c.load(Ordering::Relaxed)) {
            return Err("시추공 디클러스터링 작업이 취소되었습니다");
        }
        let too_close = kept.iter().any(|existing| {
            let dx = existing.x - cand.x;
            let dy = existing.y - cand.y;
            dx * dx + dy * dy < min_dist_sq
        });
        if !too_close {
            kept.push(cand.clone());
            if kept.len() >= max_holes.max(1) {
                break;
            }
        }
    }
    Ok(kept)
}

/// `PaleoLayerWriter.cpp` (`8c9f9df`) — KIGAM 지질도·토양도 단일/멀티폴리곤 링 정규화 (열린 링 자동 폐합 및 퇴화 링 제거)
#[must_use]
pub fn normalize_polygons_to_multipolygon(rings: Vec<Vec<[f64; 2]>>) -> Vec<Vec<[f64; 2]>> {
    let mut normalized = Vec::new();
    for mut ring in rings {
        ring.retain(|pt| pt[0].is_finite() && pt[1].is_finite());
        if ring.len() < 3 {
            continue;
        }
        if let (Some(&first), Some(&last)) = (ring.first(), ring.last()) {
            if (first[0] - last[0]).abs() > 1e-9 || (first[1] - last[1]).abs() > 1e-9 {
                ring.push(first);
            }
        }
        if ring.len() >= 4 {
            normalized.push(ring);
        }
    }
    normalized
}

pub const HGIS_BOREHOLE_SEARCH_RADIUS_M: f64 = 100.0;
pub const HGIS_WATER_TABLE_WARN_THRESH_M: f64 = 0.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExcavationSafetyReport {
    pub has_water_table_breach: bool,
    pub max_breach_depth_m: f64,
    pub min_excavation_elevation_m: f64,
    pub interpolated_water_table_m: f64,
}

/// C++ `KaExcavationSafetyChecker::evaluateExcavationSafety` — 주변 시추공 지하수위 IDW 보간 및 굴착 계획 바닥 표고 지하수위 침범(`waterTableBreachM = Z_wt - Z_plan > 0.0`) 판정
#[must_use]
pub fn evaluate_excavation_safety(
    target_xy: [f64; 2],
    plan_bottom_elevation_m: f64,
    boreholes: &[BoreholeRawInput],
    search_radius_m: f64,
) -> Option<ExcavationSafetyReport> {
    if boreholes.is_empty() {
        return None;
    }
    let r1 = if search_radius_m > 0.0 && search_radius_m.is_finite() {
        search_radius_m
    } else {
        HGIS_BOREHOLE_SEARCH_RADIUS_M
    };
    let interpolate_within = |radius: f64| -> Option<f64> {
        let mut sum_w = 0.0;
        let mut sum_wv = 0.0;
        for bh in boreholes {
            let wt_el = bh.ground_elev_m - bh.water_depth_m;
            let d = (target_xy[0] - bh.x).hypot(target_xy[1] - bh.y);
            if d < 1e-6 {
                return Some(wt_el);
            }
            if d <= radius {
                let w = 1.0 / (d * d);
                sum_w += w;
                sum_wv += w * wt_el;
            }
        }
        if sum_w > 0.0 {
            Some(sum_wv / sum_w)
        } else {
            None
        }
    };
    let wt_el = interpolate_within(r1).or_else(|| interpolate_within(500.0))?;
    let breach = wt_el - plan_bottom_elevation_m;
    Some(ExcavationSafetyReport {
        has_water_table_breach: breach > HGIS_WATER_TABLE_WARN_THRESH_M,
        max_breach_depth_m: breach.max(0.0),
        min_excavation_elevation_m: plan_bottom_elevation_m,
        interpolated_water_table_m: wt_el,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn borehole_declustering_keeps_deepest_hole_per_grid_cell_and_normalizes_multipolygons() {
        let h1 = BoreholeRawInput {
            hole_id: "BH-SHALLOW".to_owned(),
            x: 105.0,
            y: 205.0,
            ground_elev_m: 15.0,
            water_depth_m: 2.0,
            strata: vec![StratumLayer {
                depth_from_m: 0.0,
                depth_to_m: 3.0,
                layer_name: "충적층".to_owned(),
                uscs_code: "SM".to_owned(),
                description: "모래".to_owned(),
                n_value: 8.0,
            }],
        };
        let h2 = BoreholeRawInput {
            hole_id: "BH-DEEP".to_owned(),
            x: 112.0,
            y: 208.0,
            ground_elev_m: 15.0,
            water_depth_m: 2.0,
            strata: vec![
                StratumLayer {
                    depth_from_m: 0.0,
                    depth_to_m: 4.5,
                    layer_name: "충적층".to_owned(),
                    uscs_code: "CL".to_owned(),
                    description: "유기질 점토".to_owned(),
                    n_value: 4.0,
                },
                StratumLayer {
                    depth_from_m: 4.5,
                    depth_to_m: 9.0,
                    layer_name: "풍화토".to_owned(),
                    uscs_code: "SM".to_owned(),
                    description: "풍화대".to_owned(),
                    n_value: 50.0,
                },
            ],
        };
        let declustered = decluster_boreholes_by_grid(&[h1, h2], 50.0, 50);
        assert_eq!(declustered.len(), 1);
        assert_eq!(declustered[0].hole_id, "BH-DEEP");

        let polys = normalize_polygons_to_multipolygon(vec![
            vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]], // unclosed triangle -> auto-closed to 4 pts
            vec![[0.0, 0.0], [1.0, 1.0]],                // degenerate 2-pt -> dropped
        ]);
        assert_eq!(polys.len(), 1);
        assert_eq!(polys[0].len(), 4);
    }

    #[test]
    fn test_borehole_declustering_15_holes_within_100m_reduces_to_at_most_4_and_keeps_45m_deepest()
    {
        use std::sync::atomic::AtomicBool;

        let holes: Vec<BoreholeRawInput> = (0..15)
            .map(|i| {
                let depth = if i == 7 { 45.0 } else { 5.0 + f64::from(i) * 2.5 };
                let x = 200_000.0 + f64::from(i % 5) * 12.0;
                let y = 350_000.0 + f64::from(i / 5) * 14.0;
                BoreholeRawInput {
                    hole_id: format!("BH-{i:02}"),
                    x,
                    y,
                    ground_elev_m: 20.0,
                    water_depth_m: 2.0,
                    strata: vec![StratumLayer {
                        depth_from_m: 0.0,
                        depth_to_m: depth,
                        layer_name: "충적층".to_owned(),
                        uscs_code: "SM".to_owned(),
                        description: "실트질 모래".to_owned(),
                        n_value: 15.0,
                    }],
                }
            })
            .collect();

        let cancel = AtomicBool::new(false);
        let declustered = decluster_boreholes_with_cancel(&holes, 30.0, 50, Some(&cancel))
            .expect("decluster should succeed when not cancelled");
        assert!(declustered.len() <= 4);
        assert_eq!(declustered[0].hole_id, "BH-07");
        assert!((declustered[0].strata[0].depth_to_m - 45.0).abs() < 1e-6);

        let cancelled = AtomicBool::new(true);
        assert!(decluster_boreholes_with_cancel(&holes, 30.0, 50, Some(&cancelled)).is_err());
        assert_eq!(
            classify_uscs_to_4_paleo("SM", 15.0, 2.5),
            PaleoSedimentType::LeveeFanSand
        );
        assert_eq!(
            classify_uscs_to_4_paleo("CL", 4.0, 3.0),
            PaleoSedimentType::BackswampClay
        );
        assert_eq!(
            classify_uscs_to_4_paleo("GP", 25.0, 6.0),
            PaleoSedimentType::FluvialTerrace
        );
        assert_eq!(
            classify_uscs_to_4_paleo("SM", 50.0, 12.0),
            PaleoSedimentType::BedrockResidual
        );

        let bh_wt = BoreholeRawInput {
            hole_id: "BH-WT1".to_owned(),
            x: 1000.0,
            y: 2000.0,
            ground_elev_m: 30.0,
            water_depth_m: 2.0, // EL. 28.0m
            strata: vec![],
        };
        let report = evaluate_excavation_safety([1000.0, 2000.0], 27.0, &[bh_wt], 100.0)
            .expect("safety report");
        assert!(report.has_water_table_breach);
        assert!((report.max_breach_depth_m - 1.0).abs() < 1e-9);

        // TestKaMunsellSoilColorParser::testStandardYellowishBrownParsing
        let munsell =
            parse_munsell_notation("10YR 4/6").expect("10YR 4/6 should parse as Yellowish Brown");
        assert_eq!(munsell.hue, "10YR");
        assert!((munsell.value - 4.0).abs() < 1e-6);
        assert!((munsell.chroma - 6.0).abs() < 1e-6);
        assert_eq!(munsell.korean_color_name, "황갈색");
        assert!((140..=170).contains(&munsell.srgb_color[0]));
        assert!((90..=120).contains(&munsell.srgb_color[1]));
        assert!((20..=50).contains(&munsell.srgb_color[2]));
        assert_eq!(
            parse_munsell_notation("10YR 4-6"),
            Err(MunsellParseError::InvalidFormat)
        );

        // TestKaSoilDescriptionGenerator::testSentenceComposition
        let desc = generate_soil_description(
            "황갈색(10YR 4/6)",
            "사질점토",
            "미세 목탄립",
            "치밀",
        );
        assert_eq!(
            desc,
            "황갈색(10YR 4/6)의 사질점토로서, 미세 목탄립이(가) 포함되어 있으며 층위 상태는 치밀함."
        );

        // TestKaStratigraphicColumnLog::testDoubleRulerDepthCalculations
        let tick = compute_double_ruler_tick(25.00, 60.0);
        assert_eq!(tick.depth_label, "60 cm");
        assert_eq!(tick.elevation_label, "EL. 24.40 m");
        assert!((tick.absolute_elevation_m - 24.40).abs() < 1e-6);
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

    let (korean_name, srgb) = match (hue_part, (value * 10.0).round() as i32, (chroma * 10.0).round() as i32) {
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
