//! 공간 균형 시추 추출과 폴리곤 정규화.
//!
//! `borehole_strata` 에서 분리했다.

use crate::borehole_types::BoreholeRawInput;

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
                    || ((*prev_depth - max_depth).abs() < 1e-6
                        && *prev_cnt >= score_strata_cnt) => {}
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

/// 시추공 후보 목록을 중심점(`center_5186`) 거리순 정렬 후 격자 버킷(`projPrefix:gx:gy`) 라운드로빈으로 균형 선발
/// (C++ `GeoInfoBoreholeService::selectSpatiallyBalancedCandidates` 1:1 대응)
#[must_use]
pub fn select_spatially_balanced_boreholes(
    raw_candidates: &[(String, [f64; 2], bool)],
    center_5186: [f64; 2],
    max_count: usize,
    cell_meters: f64,
) -> Vec<String> {
    if raw_candidates.is_empty() || max_count == 0 {
        return Vec::new();
    }
    let mut sorted = raw_candidates.to_vec();
    sorted.sort_by(|a, b| {
        let da = (a.1[0] - center_5186[0]).powi(2) + (a.1[1] - center_5186[1]).powi(2);
        let db = (b.1[0] - center_5186[0]).powi(2) + (b.1[1] - center_5186[1]).powi(2);
        da.total_cmp(&db)
    });
    let cell = cell_meters.max(25.0);
    let mut bucket_order: Vec<String> = Vec::new();
    let mut buckets: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for (code, pt, has_pt) in sorted {
        let proj_prefix = code.split("BH").next().unwrap_or("").trim();
        let key = if has_pt {
            let gx = (pt[0] / cell).floor() as i64;
            let gy = (pt[1] / cell).floor() as i64;
            format!("{proj_prefix}:{gx}:{gy}")
        } else {
            proj_prefix.to_owned()
        };
        if !buckets.contains_key(&key) {
            bucket_order.push(key.clone());
        }
        buckets.entry(key).or_default().push(code);
    }

    let mut result = Vec::with_capacity(max_count);
    for pass in 0.. {
        if result.len() >= max_count {
            break;
        }
        let mut added_any = false;
        for key in &bucket_order {
            if let Some(list) = buckets.get(key) {
                if let Some(item) = list.get(pass) {
                    result.push(item.clone());
                    added_any = true;
                    if result.len() >= max_count {
                        break;
                    }
                }
            }
        }
        if !added_any {
            break;
        }
    }
    result
}
