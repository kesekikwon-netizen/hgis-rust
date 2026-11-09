//! C++ `HgisPaleoEngine::idw_interpolate_grid(..., k_nearest = 8, power = 2.0)`
//! (`HgisPaleoBorehole.cpp:149-191`) 의 시추 도면 전용 IDW.
//!
//! 기존 [`crate::terrain_edt_tpi::idw_interpolate_grid`] 와 계산 규칙이 다르다.
//! C++ 시추 경로는 `std::nth_element` 로 k=8 근접점만 쓰고 `d = max(sqrt(d2), 1.0)` 로
//! 거리를 1m 하한으로 클램프한다(`:182`). 기존 Rust 함수는 `d2 < 1e-6` 이면 원값을 즉시
//! 반환하는 다른 규칙이므로 시추 도면에는 쓰지 않는다.

use crate::terrain_edt_tpi::GridTransform;
use rayon::prelude::*;

/// C++ 호출부의 k 근접점 개수 (`GeoInfoBoreholeMaps.cpp:51` 의 `8`).
pub const BOREHOLE_IDW_K_NEAREST: usize = 8;

/// C++ 호출부의 거리 가중 지수 (`GeoInfoBoreholeMaps.cpp:51` 의 `2.0`).
pub const BOREHOLE_IDW_POWER: f64 = 2.0;

/// C++ `std::max(std::sqrt(d2), 1.0)` 클램프 (`HgisPaleoBorehole.cpp:182`) —
/// 관측점과 같은 셀에서도 가중치가 무한대로 발산하지 않도록 거리 하한을 1m 로 둔다.
pub const BOREHOLE_IDW_MIN_DISTANCE_M: f64 = 1.0;

/// C++ `std::nth_element` + k개 누적 규칙 1:1 대응 IDW 격자 보간.
///
/// 셀마다 모든 관측점까지의 제곱거리를 구해 가장 가까운 `k_nearest` 개를 취하고
/// `w = 1 / max(sqrt(d2), 1.0)^power` 로 가중 평균한다. 관측점이 없으면 C++
/// (`HgisPaleoBorehole.cpp:156`)와 같이 0.0 으로 채운 격자를 돌려준다.
#[must_use]
pub fn idw_interpolate_grid_k_nearest(
    tf: &GridTransform,
    points_xyv: &[[f64; 3]],
    k_nearest: usize,
    power: f64,
) -> Vec<f64> {
    let n = tf.width * tf.height;
    if points_xyv.is_empty() || n == 0 {
        return vec![0.0_f64; n];
    }
    // C++ `const size_t K = std::min(N_pts, max(1, k_nearest))` (`:158`)
    let k = k_nearest.max(1).min(points_xyv.len());

    let mut grid: Vec<f64> = (0..n)
        .into_par_iter()
        .map(|idx| {
            let r = idx / tf.width;
            let c = idx % tf.width;
            let [wx, wy] = tf.pixel_to_world(r, c);
            idw_cell_value(points_xyv, wx, wy, k, power)
        })
        .collect();
    // `is_finite` 방어: 좌표가 NaN/Inf 인 입력이 섞이면 보간값이 오염되므로 0.0 으로 되돌린다.
    for v in grid.iter_mut() {
        if !v.is_finite() {
            *v = 0.0;
        }
    }
    grid
}

/// 한 셀의 IDW 값 — C++ `idw_interpolate_grid` 안쪽 루프(`HgisPaleoBorehole.cpp:167-187`).
fn idw_cell_value(points_xyv: &[[f64; 3]], wx: f64, wy: f64, k: usize, power: f64) -> f64 {
    // (제곱거리, 인덱스) — C++ `std::vector<std::pair<double, size_t>> dist_idx`
    let mut dist_idx: Vec<(f64, usize)> = points_xyv
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let dx = wx - p[0];
            let dy = wy - p[1];
            (dx * dx + dy * dy, i)
        })
        .collect();

    // C++ `std::nth_element(begin, begin + K, end)` (`:174-177`) — 앞 k개만 정렬되면 충분하다.
    if k < dist_idx.len() {
        dist_idx.select_nth_unstable_by(k, |a, b| a.0.total_cmp(&b.0));
    }

    let mut w_sum = 0.0_f64;
    let mut v_sum = 0.0_f64;
    for &(d2, idx) in dist_idx.iter().take(k) {
        let d = d2.sqrt().max(BOREHOLE_IDW_MIN_DISTANCE_M);
        let w = 1.0 / d.powf(power);
        w_sum += w;
        v_sum += w * points_xyv[idx][2];
    }
    if w_sum > 0.0 {
        v_sum / w_sum
    } else {
        0.0
    }
}

/// 격자 최소/최대 — C++ `bedrockElevMap` 의 `std::minmax_element` (`GeoInfoBoreholeMaps.cpp:278`)
/// 및 램프 범위 계산에 쓴다.
#[must_use]
pub fn grid_min_max(grid: &[f64]) -> Option<(f64, f64)> {
    let mut it = grid.iter().copied().filter(|v| v.is_finite());
    let first = it.next()?;
    let (mut mn, mut mx) = (first, first);
    for v in it {
        mn = mn.min(v);
        mx = mx.max(v);
    }
    Some((mn, mx))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn small_tf() -> GridTransform {
        GridTransform {
            origin_x: 0.0,
            origin_y: 40.0,
            cell_size: 10.0,
            width: 4,
            height: 4,
        }
    }

    #[test]
    fn idw_dominates_observed_value_at_observation_cell() {
        // 셀 중심 (r=1,c=1) = (15.0, 25.0)
        let tf = small_tf();
        let [cx, cy] = tf.pixel_to_world(1, 1);
        let pts = [
            [cx, cy, 35.5],
            [cx + 100.0, cy, 12.0],
            [cx, cy + 100.0, 48.0],
        ];
        let grid = idw_interpolate_grid_k_nearest(&tf, &pts, 8, 2.0);
        // 셀 (r=1, c=1) 의 인덱스는 `1 * width + 1`.
        let got = grid[tf.width + 1];
        // C++ 는 `d = max(sqrt(d2), 1.0)` 로 클램프하므로(`HgisPaleoBorehole.cpp:182`)
        // 관측점 셀에서도 원값이 **정확히** 복원되지는 않는다. 관측점 가중치가
        // 1/1^2 = 1.0, 100m 떨어진 두 점이 각각 1/10000 이라 원값이 압도적으로 지배한다.
        // 기대값: (1*35.5 + 0.0001*12 + 0.0001*48) / 1.0002 = 35.4989...
        assert!(
            (got - 35.498_900_219_956).abs() < 1e-9,
            "expected 35.498900219956 (1/1.0 clamp weighting), got {got}"
        );
        // 클램프가 없었다면 0으로 나누거나 원값과 정확히 같았을 것이다.
        // 이 규칙의 본질은 "관측점이 나머지를 압도" 이므로 그 성질을 함께 고정한다.
        assert!((got - 35.5).abs() < 0.01, "observed value must dominate");
        assert!(got < 35.5, "far higher value must pull the mean slightly up");
    }

    #[test]
    fn exact_recovery_when_observation_is_the_only_point_in_reach() {
        // k=1 이면 관측점 하나만 쓰이므로 원값이 정확히 복원된다.
        let tf = small_tf();
        let [cx, cy] = tf.pixel_to_world(1, 1);
        let pts = [
            [cx, cy, 35.5],
            [cx + 100.0, cy, 12.0],
            [cx, cy + 100.0, 48.0],
        ];
        let grid = idw_interpolate_grid_k_nearest(&tf, &pts, 1, 2.0);
        assert_eq!(grid[tf.width + 1], 35.5);
    }

    #[test]
    fn distance_clamp_makes_weights_finite_and_single_point_is_constant() {
        let tf = small_tf();
        // 단일 관측점이면 어느 셀이든 그 값과 같다 (가중 평균이 1개 항목).
        let grid = idw_interpolate_grid_k_nearest(&tf, &[[5.0, 5.0, 7.25]], 8, 2.0);
        assert!(grid.iter().all(|v| (v - 7.25).abs() < 1e-12));
        assert!(grid.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn uses_only_k_nearest_of_many_points() {
        let tf = GridTransform {
            origin_x: 0.0,
            origin_y: 10.0,
            cell_size: 10.0,
            width: 1,
            height: 1,
        };
        // 셀 중심 = (5.0, 5.0). 가까운 2점은 값 10/20, 아주 먼 점은 1000.
        let pts = [
            [5.5, 5.5, 10.0],
            [6.0, 5.0, 20.0],
            [5000.0, 5000.0, 1000.0],
        ];
        let grid = idw_interpolate_grid_k_nearest(&tf, &pts, 2, 2.0);
        // 먼 점이 제외되므로 값은 10~20 사이에 머문다.
        assert!(
            (10.0..=20.0).contains(&grid[0]),
            "k=2 must exclude the far point, got {}",
            grid[0]
        );
    }

    #[test]
    fn empty_points_yield_zero_grid_like_cpp() {
        let tf = small_tf();
        let grid = idw_interpolate_grid_k_nearest(&tf, &[], 8, 2.0);
        assert_eq!(grid.len(), tf.width * tf.height);
        assert!(grid.iter().all(|&v| v == 0.0));
    }

    #[test]
    fn grid_min_max_ignores_non_finite_and_handles_empty() {
        assert_eq!(grid_min_max(&[]), None);
        assert_eq!(
            grid_min_max(&[f64::NAN, 3.0, f64::INFINITY, -2.0, 9.0]),
            Some((-2.0, 9.0))
        );
    }
}
