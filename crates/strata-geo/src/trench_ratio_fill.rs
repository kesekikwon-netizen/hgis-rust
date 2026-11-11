//! 시굴격자 자동 배치 — 조사구역 면적의 목표 비율(시굴 10 %, 표본 2 %)만큼 트렌치를 깐다.
//!
//! C++ `TrenchGridGenerator.cpp` 의 `buildInArea`(:215-318)·`buildForTargetRatio`(:369-443)와
//! `TrenchTerrain::numberFromHighSide`(:76-110)를 그대로 옮겼다.
//! - 격자는 구역 외접 사각형의 중심에 고정하고 방위(북 기준 시계 방향)만큼 돌린다.
//! - 고른 길이 하나만 쓴다. 구역 안에 다 들어가지 않는 칸은 비운다(자르지 않는다).
//! - 둑(간격)을 1~120 m 로 바꿔 가며 목표 면적에 가장 가까운 격자를 고르고,
//!   남는 칸은 번호 전체에 고르게 흩어 뺀다.
//! - 번호는 방위의 높은 쪽(0° 이면 북쪽) 줄부터, 줄 안에서는 왼쪽부터.

use geo::{Area, BoundingRect, Contains, Coord, LineString, MultiPolygon, Polygon};

/// 폭 최대 2 m, 길이 최대 20 m (C++ `kMaxWidth`, `kMaxLength`).
pub const MAX_TRENCH_WIDTH_M: f64 = 2.0;
pub const MAX_TRENCH_LENGTH_M: f64 = 20.0;
const MAX_CELLS: f64 = 200_000.0;

/// 트렌치 한 칸.
#[derive(Debug, Clone, PartialEq)]
pub struct TrenchCell {
    pub name: String,
    pub width: f64,
    pub length: f64,
    /// 닫힌 고리(5점), 작업 좌표계 미터.
    pub ring: Vec<[f64; 2]>,
}

/// 목표 비율 배치 결과.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RatioFill {
    pub cells: Vec<TrenchCell>,
    pub balk: f64,
    pub ratio_pct: f64,
    pub area_m2: f64,
}

fn valid_dimensions(width: f64, length: f64) -> bool {
    width.is_finite()
        && width > 0.0
        && width <= MAX_TRENCH_WIDTH_M
        && length.is_finite()
        && length > 0.0
        && length <= MAX_TRENCH_LENGTH_M
}

/// 트렌치 면적 합(폭 × 길이).
#[must_use]
pub fn total_area(cells: &[TrenchCell]) -> f64 {
    cells.iter().map(|c| c.width * c.length).sum()
}

/// 구역 안에 온전히 들어가는 칸만 남긴 격자. 실패하면 사용자에게 보일 이유를 돌려준다.
pub fn build_in_area(
    area: &MultiPolygon<f64>,
    width: f64,
    length: f64,
    balk: f64,
    azimuth_deg: f64,
    prefix: &str,
) -> Result<Vec<TrenchCell>, String> {
    let Some(env) = area.bounding_rect() else {
        return Err("조사구역이 없습니다. 조사구역을 먼저 그리거나 선택하세요.".to_owned());
    };
    if !valid_dimensions(width, length) || !balk.is_finite() || balk < 0.0 || !azimuth_deg.is_finite() {
        return Err(
            "트렌치 규격이 맞지 않습니다. 폭은 2 m 이하, 길이는 20 m 이하, 둑은 0 m 이상이어야 합니다."
                .to_owned(),
        );
    }
    let (ax, ay) = ((env.min().x + env.max().x) * 0.5, (env.min().y + env.max().y) * 0.5);
    let (s, c) = azimuth_deg.to_radians().sin_cos();
    let world = |lx: f64, ly: f64| [lx * c + ly * s + ax, -lx * s + ly * c + ay];
    let to_local = |wx: f64, wy: f64| {
        let (dx, dy) = (wx - ax, wy - ay);
        (dx * c - dy * s, dx * s + dy * c)
    };

    // 돌린 좌표계에서 외접 사각형 네 귀의 범위(구역을 모두 덮는다).
    let (mut u_min, mut u_max, mut v_min, mut v_max) = (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
    for cx in [env.min().x, env.max().x] {
        for cy in [env.min().y, env.max().y] {
            let (u, v) = to_local(cx, cy);
            u_min = u_min.min(u);
            u_max = u_max.max(u);
            v_min = v_min.min(v);
            v_max = v_max.max(v);
        }
    }
    let (step_u, step_v) = (width + balk, length + balk);
    let (count_u, count_v) = ((u_max - u_min) / step_u, (v_max - v_min) / step_v);
    let too_many = || "구역에 비해 트렌치 간격이 좁아 칸 수가 한도를 넘습니다. 둑(간격)을 키우세요.".to_owned();
    if !count_u.is_finite() || !count_v.is_finite() || count_u > MAX_CELLS - 2.0 || count_v > MAX_CELLS - 2.0 {
        return Err(too_many());
    }
    let (nu, nv) = (count_u as u64 + 2, count_v as u64 + 2);
    if (nu * nv) as f64 > MAX_CELLS {
        return Err(too_many());
    }

    let mut out = Vec::new();
    for j in 0..nv {
        let v0 = v_min + j as f64 * step_v;
        for i in 0..nu {
            let u0 = u_min + i as f64 * step_u;
            let p0 = world(u0, v0);
            let ring = vec![
                p0,
                world(u0 + width, v0),
                world(u0 + width, v0 + length),
                world(u0, v0 + length),
                p0,
            ];
            let poly = Polygon::new(
                LineString::from(ring.iter().map(|p| Coord { x: p[0], y: p[1] }).collect::<Vec<_>>()),
                Vec::new(),
            );
            // 고른 길이 하나만: 다 들어가지 않는 칸은 비운다.
            if area.contains(&poly) {
                out.push(TrenchCell {
                    name: String::new(),
                    width,
                    length,
                    ring,
                });
            }
        }
    }
    if out.is_empty() {
        return Err(
            "현재 규격과 방향으로는 구역 안에 들어가는 트렌치가 없습니다. 구역이 좁거나 짧으면 회전이나 규격을 바꿔 보세요."
                .to_owned(),
        );
    }
    number_from_high_side(&mut out, azimuth_deg, prefix);
    Ok(out)
}

/// 방위의 높은 쪽 줄부터, 줄 안에서는 왼쪽부터 번호를 매긴다.
pub fn number_from_high_side(cells: &mut Vec<TrenchCell>, azimuth_deg: f64, prefix: &str) {
    let (s, c) = azimuth_deg.to_radians().sin_cos();
    let (ux, uy, vx, vy) = (s, c, c, -s);
    let mut keys: Vec<(f64, f64, usize)> = cells
        .iter()
        .enumerate()
        .map(|(at, cell)| {
            let n = (cell.ring.len().max(2) - 1) as f64;
            let (cx, cy) = cell.ring[..cell.ring.len() - 1]
                .iter()
                .fold((0.0, 0.0), |acc, p| (acc.0 + p[0] / n, acc.1 + p[1] / n));
            // 센티미터로 반올림: 한 줄의 칸은 값이 정확히 같아 왼쪽→오른쪽으로 읽힌다.
            (((cx * ux + cy * uy) * 100.0).round(), cx * vx + cy * vy, at)
        })
        .collect();
    keys.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.total_cmp(&b.1)));
    let ordered: Vec<TrenchCell> = keys
        .iter()
        .enumerate()
        .map(|(n, key)| {
            let mut cell = cells[key.2].clone();
            cell.name = format!("{prefix}{}", n + 1);
            cell
        })
        .collect();
    *cells = ordered;
}

/// 목표 비율(%)에 가장 가까운 개수의 온전한 트렌치를 깐다.
pub fn build_for_target_ratio(
    area: &MultiPolygon<f64>,
    target_pct: f64,
    width: f64,
    azimuth_deg: f64,
    length: f64,
) -> Result<RatioFill, String> {
    if !valid_dimensions(width, length) || !target_pct.is_finite() || target_pct <= 0.0 || target_pct > 100.0 || !azimuth_deg.is_finite() {
        return Err(
            "폭은 2 m 이하, 길이는 20 m 이하, 목표 비율은 0 초과 100% 이하의 유효한 값이어야 합니다."
                .to_owned(),
        );
    }
    let area_m2 = area.unsigned_area();
    let target_area = area_m2 * (target_pct / 100.0);
    if !area_m2.is_finite() || area_m2 <= 1e-6 {
        return Err("조사구역이 없습니다. 조사구역을 먼저 그리거나 선택하세요.".to_owned());
    }
    // 표본조사(3.5 % 이하)는 Sp-, 시굴은 Tr-.
    let prefix = if target_pct <= 3.5 { "Sp-" } else { "Tr-" };
    let piece = width * length;

    let mut best: Option<(Vec<TrenchCell>, f64, usize, f64, bool)> = None; // cells, balk, drop, miss, near
    for step in 1..=120_u32 {
        let balk = f64::from(step);
        let Ok(cells) = build_in_area(area, width, length, balk, azimuth_deg, prefix) else {
            continue;
        };
        let over = total_area(&cells) - target_area;
        let drop = (cells.len() - 1).min((over / piece + 0.5).floor().max(0.0) as usize);
        let miss = (over - drop as f64 * piece).abs();
        let near = miss <= piece * 0.5 + 1e-9;
        let better = match &best {
            None => true,
            Some((_, _, best_drop, best_miss, best_near)) => {
                let nearer = miss < best_miss - 1e-9;
                (near && !best_near)
                    || (near == *best_near
                        && if near {
                            drop < *best_drop || (drop == *best_drop && nearer)
                        } else {
                            nearer
                        })
            }
        };
        if better {
            best = Some((cells, balk, drop, miss, near));
        }
    }
    let Some((cells, balk, drop, _, _)) = best else {
        return Err(format!(
            "현재 구역과 방향에서 {length} m × {width} m 트렌치가 들어가지 않습니다. 더 짧은 길이(10 m)를 고르거나 방향을 바꿔 보세요. 기존 격자는 유지됩니다."
        ));
    };
    // 남는 칸을 번호 전체에 고르게 흩어 뺀다.
    let n = cells.len();
    let mut left_out = vec![false; n];
    for k in 0..drop {
        left_out[(2 * k + 1) * n / (2 * drop)] = true;
    }
    let mut kept: Vec<TrenchCell> = cells
        .into_iter()
        .zip(left_out)
        .filter_map(|(cell, out)| (!out).then_some(cell))
        .collect();
    number_from_high_side(&mut kept, azimuth_deg, prefix);
    let ratio_pct = total_area(&kept) / area_m2 * 100.0;
    Ok(RatioFill {
        cells: kept,
        balk,
        ratio_pct,
        area_m2,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(size: f64) -> MultiPolygon<f64> {
        MultiPolygon::new(vec![Polygon::new(
            LineString::from(vec![(0.0, 0.0), (size, 0.0), (size, size), (0.0, size), (0.0, 0.0)]),
            Vec::new(),
        )])
    }

    #[test]
    fn ten_percent_plan_lands_within_half_a_trench_of_the_target() {
        // 200×200 m = 40,000 m². 10 % = 4,000 m² = 2×20 m 트렌치 100개.
        let fill = build_for_target_ratio(&square(200.0), 10.0, 2.0, 0.0, 20.0).expect("배치");
        let total = total_area(&fill.cells);
        assert!((total - 4_000.0).abs() <= 20.0 + 1e-9, "total={total}");
        assert!((fill.ratio_pct - 10.0).abs() < 0.06, "ratio={}", fill.ratio_pct);
        assert!((1.0..=120.0).contains(&fill.balk));
        // 모든 칸이 구역 안에 있고 규격이 같다.
        for cell in &fill.cells {
            assert_eq!((cell.width, cell.length, cell.ring.len()), (2.0, 20.0, 5));
            assert!(cell.ring.iter().all(|p| (0.0..=200.0).contains(&p[0]) && (0.0..=200.0).contains(&p[1])));
        }
        // 서로 겹치지 않는다(같은 격자의 칸이므로 중심이 모두 다르다).
        let mut centers: Vec<(i64, i64)> = fill
            .cells
            .iter()
            .map(|c| (((c.ring[0][0] + c.ring[2][0]) * 50.0) as i64, ((c.ring[0][1] + c.ring[2][1]) * 50.0) as i64))
            .collect();
        centers.sort_unstable();
        centers.dedup();
        assert_eq!(centers.len(), fill.cells.len());
    }

    #[test]
    fn numbering_starts_on_the_north_row_and_reads_left_to_right() {
        let cells = build_in_area(&square(100.0), 2.0, 20.0, 10.0, 0.0, "Tr-").expect("격자");
        assert_eq!(cells[0].name, "Tr-1");
        let top = cells.iter().map(|c| c.ring[0][1]).fold(f64::NEG_INFINITY, f64::max);
        assert!((cells[0].ring[0][1] - top).abs() < 1e-9, "Tr-1 은 가장 북쪽 줄");
        assert!(cells[0].ring[0][0] < cells[1].ring[0][0], "같은 줄은 서→동");
        assert_eq!(cells.last().map(|c| c.name.clone()), Some(format!("Tr-{}", cells.len())));
    }

    #[test]
    fn a_rotated_grid_keeps_its_trenches_inside_and_turned_by_the_azimuth() {
        let cells = build_in_area(&square(120.0), 2.0, 20.0, 8.0, 30.0, "Tr-").expect("격자");
        assert!(!cells.is_empty());
        for cell in &cells {
            // 긴 변(ring[0]→ring[3])이 북에서 시계 방향 30° 를 가리킨다.
            let (dx, dy) = (cell.ring[3][0] - cell.ring[0][0], cell.ring[3][1] - cell.ring[0][1]);
            assert!((dx.atan2(dy).to_degrees() - 30.0).abs() < 1e-6);
            assert!(((dx * dx + dy * dy).sqrt() - 20.0).abs() < 1e-9);
        }
    }

    #[test]
    fn it_says_why_when_nothing_fits_or_the_spec_is_out_of_range() {
        let err = build_for_target_ratio(&square(8.0), 10.0, 2.0, 0.0, 20.0)
            .err()
            .unwrap_or_default();
        assert!(err.contains("들어가지 않습니다"), "{err}");
        let err = build_in_area(&square(100.0), 3.0, 20.0, 5.0, 0.0, "Tr-")
            .err()
            .unwrap_or_default();
        assert!(err.contains("폭은 2 m 이하"), "{err}");
        // 표본조사 2 % 는 Sp- 로 번호를 단다.
        let sp = build_for_target_ratio(&square(200.0), 2.0, 2.0, 0.0, 10.0).expect("표본");
        assert!(sp.cells[0].name.starts_with("Sp-"));
        assert!((total_area(&sp.cells) - 800.0).abs() <= 10.0 + 1e-9);
    }
}
