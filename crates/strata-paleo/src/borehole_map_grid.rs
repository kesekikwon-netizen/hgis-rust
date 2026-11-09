//! C++ `GeoInfoBoreholeMaps.cpp:242-246` 의 180×180 격자 구성과
//! `GeoInfoBoreholeParser.cpp:168-202` 의 150m 공간 균형 시추공 선발.

use crate::terrain_edt_tpi::GridTransform;

/// C++ `constexpr int kGrid = 180` (`GeoInfoBoreholeMaps.cpp:244`).
pub const BOREHOLE_MAP_GRID_SIZE: usize = 180;

/// C++ `std::max(300.0, ...)` 패딩 하한 (`GeoInfoBoreholeMaps.cpp:242`).
pub const BOREHOLE_MAP_MIN_PAD_M: f64 = 300.0;

/// C++ `std::max(5.0, ...)` 셀 크기 하한 (`GeoInfoBoreholeMaps.cpp:245`).
pub const BOREHOLE_MAP_MIN_CELL_M: f64 = 5.0;

/// C++ `selectSpatiallyBalancedCandidates(..., cellMeters = 150.0)` 기본 셀
/// (`GeoInfoBoreholeService.h:95-97`, `GeoInfoBoreholeParser.cpp:177` 의 `max(25.0, cellMeters)`).
pub const BOREHOLE_CANDIDATE_CELL_M: f64 = 150.0;

/// C++ `cell = std::max(25.0, cellMeters)` 하한 (`GeoInfoBoreholeParser.cpp:177`).
pub const BOREHOLE_CANDIDATE_MIN_CELL_M: f64 = 25.0;

/// 관측점 최소/최대 좌표 범위.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeoBounds {
    pub min_x: f64,
    pub max_x: f64,
    pub min_y: f64,
    pub max_y: f64,
}

impl GeoBounds {
    /// 경계를 점 목록에서 누적한다. 점이 하나도 없으면 `None`.
    #[must_use]
    pub fn from_points(points_xy: &[[f64; 2]]) -> Option<Self> {
        let mut it = points_xy
            .iter()
            .filter(|p| p[0].is_finite() && p[1].is_finite());
        let first = *it.next()?;
        let mut b = Self {
            min_x: first[0],
            max_x: first[0],
            min_y: first[1],
            max_y: first[1],
        };
        for p in it {
            b.min_x = b.min_x.min(p[0]);
            b.max_x = b.max_x.max(p[0]);
            b.min_y = b.min_y.min(p[1]);
            b.max_y = b.max_y.max(p[1]);
        }
        Some(b)
    }

    /// C++ `std::max(maxX - minX, maxY - minY)` — 두 축 span 중 큰 값.
    #[must_use]
    pub fn max_span(&self) -> f64 {
        (self.max_x - self.min_x).max(self.max_y - self.min_y)
    }
}

/// C++ `pad = std::max(300.0, std::max(spanX, spanY) * 0.25)` (`GeoInfoBoreholeMaps.cpp:242`).
#[must_use]
pub fn compute_pad(span_x: f64, span_y: f64) -> f64 {
    BOREHOLE_MAP_MIN_PAD_M.max(span_x.max(span_y) * 0.25)
}

/// C++ `cell = std::max(5.0, std::max(spanX, spanY) / kGrid)` (`GeoInfoBoreholeMaps.cpp:245`).
#[must_use]
pub fn compute_cell_size(span_x: f64, span_y: f64) -> f64 {
    BOREHOLE_MAP_MIN_CELL_M.max(span_x.max(span_y) / BOREHOLE_MAP_GRID_SIZE as f64)
}

/// 패딩이 적용된 최종 격자 경계와 `GridTransform` 을 만든다.
///
/// C++ 순서 그대로: 원 경계 → `pad` 확장 → `cell` 산출 → `{minX, maxY, cell, 180, 180}`.
#[must_use]
pub fn build_borehole_grid(bounds: &GeoBounds) -> (GridTransform, f64) {
    let span_x = bounds.max_x - bounds.min_x;
    let span_y = bounds.max_y - bounds.min_y;
    let pad = compute_pad(span_x, span_y);
    let min_x = bounds.min_x - pad;
    let max_x = bounds.max_x + pad;
    let min_y = bounds.min_y - pad;
    let max_y = bounds.max_y + pad;
    // `GeoInfoBoreholeMaps.cpp:245` 는 패딩 후 폭으로 셀을 구한다.
    let cell = compute_cell_size(max_x - min_x, max_y - min_y);
    let tf = GridTransform {
        origin_x: min_x,
        // C++ 는 `GridTransform{minX, maxY, cell, kGrid, kGrid}` (`:246`) — origin_y 는 북쪽 상단.
        origin_y: max_y,
        cell_size: cell,
        width: BOREHOLE_MAP_GRID_SIZE,
        height: BOREHOLE_MAP_GRID_SIZE,
    };
    debug_assert!(min_y <= max_y);
    (tf, pad)
}

/// C++ `GeoInfoBoreholeService::selectSpatiallyBalancedCandidates` 1:1 대응
/// (`GeoInfoBoreholeParser.cpp:168-202`).
///
/// 중심점 거리순으로 정렬한 뒤 `projPrefix:gx:gy` 버킷 라운드로빈으로 뽑는다.
/// `projPrefix` 는 `code.section("BH", 0, 0)`, 즉 코드에서 `"BH"` 앞부분이다.
#[must_use]
pub fn select_spatially_balanced_candidates(
    raw_candidates: &[(String, [f64; 2], bool)],
    center: [f64; 2],
    max_count: usize,
    cell_meters: f64,
) -> Vec<String> {
    if raw_candidates.is_empty() || max_count == 0 {
        return Vec::new();
    }
    let mut sorted = raw_candidates.to_vec();
    sorted.sort_by(|a, b| {
        let da = (a.1[0] - center[0]).powi(2) + (a.1[1] - center[1]).powi(2);
        let db = (b.1[0] - center[0]).powi(2) + (b.1[1] - center[1]).powi(2);
        da.total_cmp(&db)
    });
    let cell = cell_meters.max(BOREHOLE_CANDIDATE_MIN_CELL_M);

    let mut bucket_order: Vec<String> = Vec::new();
    let mut buckets: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for (code, pt, has_pt) in sorted {
        let proj_prefix = code.split("BH").next().unwrap_or("").trim().to_owned();
        let key = if has_pt {
            let gx = (pt[0] / cell).floor() as i64;
            let gy = (pt[1] / cell).floor() as i64;
            format!("{proj_prefix}:{gx}:{gy}")
        } else {
            proj_prefix
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_is_180_by_180_follows_cpp_pad_and_cell_rules() {
        // 200m x 100m 범위
        let bounds = GeoBounds {
            min_x: 0.0,
            max_x: 200.0,
            min_y: 0.0,
            max_y: 100.0,
        };
        let (tf, pad) = build_borehole_grid(&bounds);

        // pad = max(300, 200 * 0.25 = 50) = 300
        assert!((pad - 300.0).abs() < 1e-12);
        assert!((tf.origin_x - (-300.0)).abs() < 1e-12);
        // origin_y = maxY + pad
        assert!((tf.origin_y - 400.0).abs() < 1e-12);

        assert_eq!(tf.width, 180);
        assert_eq!(tf.height, 180);
        // cell 은 **패딩 후** 폭으로 구하고(`:245`), 패딩 후 span = 800 이므로
        // 800/180 = 4.44 < 5.0 하한에 걸려 5.0 이 된다.
        assert!((tf.cell_size - BOREHOLE_MAP_MIN_CELL_M).abs() < 1e-12);
        assert!(tf.cell_size >= BOREHOLE_MAP_MIN_CELL_M);
        assert!(pad >= BOREHOLE_MAP_MIN_PAD_M);
    }

    #[test]
    fn tiny_extent_still_gets_300m_pad_and_5m_cell_floor() {
        // 아주 좁은 범위: span 0 -> pad 300, 패딩 후 span 600 -> cell = max(5, 600/180)
        let bounds = GeoBounds {
            min_x: 1000.0,
            max_x: 1000.0,
            min_y: 1000.0,
            max_y: 1000.0,
        };
        let (tf, pad) = build_borehole_grid(&bounds);
        assert!((pad - 300.0).abs() < 1e-12);
        // 패딩 후 span = 600 → 600/180 = 3.333 < 5 이므로 5.0 하한
        assert!((tf.cell_size - 5.0).abs() < 1e-12);
        assert_eq!((tf.width, tf.height), (180, 180));
    }

    #[test]
    fn compute_pad_and_cell_match_cpp_formulas() {
        // span 이 충분히 크면 25% 규칙
        assert!((compute_pad(4000.0, 1000.0) - 1000.0).abs() < 1e-12);
        assert!((compute_pad(1000.0, 4000.0) - 1000.0).abs() < 1e-12);
        assert!((compute_pad(100.0, 100.0) - 300.0).abs() < 1e-12);
        // cell 5m 하한
        assert!((compute_cell_size(100.0, 100.0) - 5.0).abs() < 1e-12);
        assert!((compute_cell_size(1800.0, 900.0) - 10.0).abs() < 1e-12);
    }

    #[test]
    fn bounds_from_points_ignores_non_finite() {
        assert_eq!(GeoBounds::from_points(&[]), None);
        // 좌표 하나라도 유한하지 않은 점은 경계 계산에서 제외한다 (NaN/Inf 전파 방지).
        let b = GeoBounds::from_points(&[
            [1.0, 2.0],
            [f64::NAN, 5.0],
            [-3.0, 8.0],
            [4.0, f64::INFINITY],
        ]);
        let Some(b) = b else {
            panic!("two finite points remain, so bounds must be Some");
        };
        assert!((b.min_x - (-3.0)).abs() < 1e-12);
        assert!((b.max_x - 1.0).abs() < 1e-12);
        assert!((b.min_y - 2.0).abs() < 1e-12);
        assert!((b.max_y - 8.0).abs() < 1e-12);
        assert!((b.max_span() - 6.0).abs() < 1e-12);
    }

    #[test]
    fn balanced_selection_round_robins_buckets_and_respects_max_count() {
        // 동일 150m 셀에 3공, 다른 셀에 1공
        let cands = vec![
            ("A-BH-1".to_owned(), [10.0, 10.0], true),
            ("A-BH-2".to_owned(), [20.0, 20.0], true),
            ("A-BH-3".to_owned(), [30.0, 30.0], true),
            ("B-BH-1".to_owned(), [5000.0, 5000.0], true),
        ];
        let picked =
            select_spatially_balanced_candidates(&cands, [0.0, 0.0], 2, BOREHOLE_CANDIDATE_CELL_M);
        assert_eq!(picked.len(), 2);
        // 라운드로빈이므로 두 버킷에서 하나씩 나온다.
        assert!(picked.iter().any(|c| c.starts_with("A-")));
        assert!(picked.iter().any(|c| c.starts_with("B-")));

        // max_count 0 / 빈 입력
        assert!(select_spatially_balanced_candidates(&cands, [0.0, 0.0], 0, 150.0).is_empty());
        assert!(select_spatially_balanced_candidates(&[], [0.0, 0.0], 5, 150.0).is_empty());
    }

    #[test]
    fn selection_without_geometry_groups_by_project_prefix() {
        let cands = vec![
            ("P1-BH-1".to_owned(), [0.0, 0.0], false),
            ("P1-BH-2".to_owned(), [0.0, 0.0], false),
            ("P2-BH-1".to_owned(), [0.0, 0.0], false),
        ];
        let picked = select_spatially_balanced_candidates(&cands, [0.0, 0.0], 3, 150.0);
        assert_eq!(picked.len(), 3);
        // 도형 없는 후보는 projPrefix 만으로 묶이므로 P1 두 공이 한 버킷이 되어
        // 라운드로빈 첫 pass 에서 P1-1, P2-1 이, 두 번째 pass 에서 P1-2 가 나온다.
        assert_eq!(picked[0], "P1-BH-1");
        assert_eq!(picked[1], "P2-BH-1");
        assert_eq!(picked[2], "P1-BH-2");
    }
}
