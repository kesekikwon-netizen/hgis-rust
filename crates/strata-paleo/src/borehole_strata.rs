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
}
