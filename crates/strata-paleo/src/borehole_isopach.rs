//! C++ `addIsopachContourLayer` (`GeoInfoBoreholeMaps.cpp:98-124`) —
//! 충적층 두께 격자에서 0.5m 간격 등층후선(0.5m~12.0m)을 뽑는다.
//!
//! C++ 는 셀 사각형의 네 변을 각각 선형 보간해 교점을 모으고, 교점이 2개 이상이면
//! 첫 두 점으로 폴리라인을 만든다(`:116-119`). 이 모듈은 같은 순서·같은 클램프를 쓴다.

use crate::terrain_edt_tpi::GridTransform;

/// C++ `for (double lv = 0.5; lv <= 12.0; lv += 0.5)` (`GeoInfoBoreholeMaps.cpp:98`).
pub const BOREHOLE_ISOPACH_LEVEL_START_M: f64 = 0.5;

/// 등층후선 마지막 레벨.
pub const BOREHOLE_ISOPACH_LEVEL_END_M: f64 = 12.0;

/// 등층후선 간격.
pub const BOREHOLE_ISOPACH_LEVEL_STEP_M: f64 = 0.5;

/// C++ GPKG 레이어명 (`GeoInfoBoreholeMaps.cpp:129`).
pub const BOREHOLE_ISOPACH_LAYER_NAME: &str = "borehole_isopach_contours";

/// C++ `thickness_m` 속성 필드명 (`GeoInfoBoreholeMaps.cpp:94`).
pub const BOREHOLE_ISOPACH_FIELD_THICKNESS: &str = "thickness_m";

/// C++ `label` 속성 필드명 (`GeoInfoBoreholeMaps.cpp:95`).
pub const BOREHOLE_ISOPACH_FIELD_LABEL: &str = "label";

/// 등층후선 한 세그먼트 (C++ `QgsFeature` 하나에 해당).
#[derive(Debug, Clone, PartialEq)]
pub struct IsopachSegment {
    /// C++ `QVariant(lv)` — 등층후 값(m).
    pub thickness_m: f64,
    /// C++ `QStringLiteral("%1m").arg(lv, 0, 'f', 1)` — 예: `"0.5m"`, `"1.0m"`.
    pub label: String,
    /// C++ `QgsGeometry::fromPolylineXY({pts[0], pts[1]})` — 두 점 폴리라인.
    pub points: [[f64; 2]; 2],
}

/// C++ `QStringLiteral("%1m").arg(lv, 0, 'f', 1)` — 소수점 한 자리.
#[must_use]
pub fn format_isopach_label(level_m: f64) -> String {
    format!("{level_m:.1}m")
}

/// C++ 루프와 같은 0.5m 단계 레벨 목록(0.5, 1.0, ..., 12.0).
///
/// C++ 는 `lv += 0.5` 부동소수 누적이라 이론상 끝값이 12.0 을 살짝 넘을 수 있으나
/// 실제 비트 패턴에서는 12.0 에 정확히 도달한다. 여기서는 정수 카운터로 같은
/// 목록을 만들어 누적 오차를 없앤다.
#[must_use]
pub fn isopach_levels() -> Vec<f64> {
    let steps = ((BOREHOLE_ISOPACH_LEVEL_END_M - BOREHOLE_ISOPACH_LEVEL_START_M)
        / BOREHOLE_ISOPACH_LEVEL_STEP_M)
        .round() as usize;
    (0..=steps)
        .map(|i| BOREHOLE_ISOPACH_LEVEL_START_M + i as f64 * BOREHOLE_ISOPACH_LEVEL_STEP_M)
        .collect()
}

/// 셀 네 변의 교점을 모은다 — C++ 람다 `edge` (`GeoInfoBoreholeMaps.cpp:108-113`) 1:1.
///
/// `(va < lv && vb >= lv) || (vb < lv && va >= lv)` 일 때만 교점을 추가하고
/// `t = clamp((lv - va) / (vb - va), 0, 1)` 로 선형 보간한다.
fn push_edge_crossing(
    pts: &mut Vec<[f64; 2]>,
    level: f64,
    va: f64,
    vb: f64,
    a: [f64; 2],
    b: [f64; 2],
) {
    let crosses = (va < level && vb >= level) || (vb < level && va >= level);
    if !crosses {
        return;
    }
    let denom = vb - va;
    let t = if denom.abs() > f64::EPSILON {
        ((level - va) / denom).clamp(0.0, 1.0)
    } else {
        0.0
    };
    pts.push([a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])]);
}

/// 한 레벨에 대해 모든 셀을 훑어 등층후선 세그먼트를 만든다
/// (C++ `GeoInfoBoreholeMaps.cpp:98-124`).
#[must_use]
pub fn extract_isopach_segments_at_level(
    tf: &GridTransform,
    grid: &[f64],
    level: f64,
) -> Vec<IsopachSegment> {
    let mut out = Vec::new();
    if tf.width < 2 || tf.height < 2 || grid.len() < tf.width * tf.height {
        return out;
    }
    let label = format_isopach_label(level);
    let w = tf.width;

    for r in 0..(tf.height - 1) {
        for c in 0..(w - 1) {
            let v00 = grid[r * w + c];
            let v10 = grid[r * w + c + 1];
            let v01 = grid[(r + 1) * w + c];
            let v11 = grid[(r + 1) * w + c + 1];
            if ![v00, v10, v01, v11].iter().all(|v| v.is_finite()) {
                continue;
            }

            // C++ `:105-106` — 셀 중심 기준 사각형 네 모서리
            let x0 = tf.origin_x + (c as f64 + 0.5) * tf.cell_size;
            let x1 = x0 + tf.cell_size;
            let y0 = tf.origin_y - (r as f64 + 0.5) * tf.cell_size;
            let y1 = y0 - tf.cell_size;

            let mut pts: Vec<[f64; 2]> = Vec::with_capacity(4);
            // C++ `:114-115` 와 같은 순서: 상, 우, 하, 좌
            push_edge_crossing(&mut pts, level, v00, v10, [x0, y0], [x1, y0]);
            push_edge_crossing(&mut pts, level, v10, v11, [x1, y0], [x1, y1]);
            push_edge_crossing(&mut pts, level, v01, v11, [x0, y1], [x1, y1]);
            push_edge_crossing(&mut pts, level, v00, v01, [x0, y0], [x0, y1]);

            // C++ `if (pts.size() >= 2)` (`:116`)
            if pts.len() >= 2 {
                out.push(IsopachSegment {
                    thickness_m: level,
                    label: label.clone(),
                    points: [pts[0], pts[1]],
                });
            }
        }
    }
    out
}

/// 0.5m~12.0m 전 레벨의 등층후선 세그먼트를 모두 만든다
/// (C++ `GeoInfoBoreholeMaps.cpp:98-124` 전체 루프).
#[must_use]
pub fn extract_all_isopach_segments(tf: &GridTransform, grid: &[f64]) -> Vec<IsopachSegment> {
    let mut out = Vec::new();
    for level in isopach_levels() {
        out.extend(extract_isopach_segments_at_level(tf, grid, level));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tf() -> GridTransform {
        GridTransform {
            origin_x: 0.0,
            origin_y: 30.0,
            cell_size: 10.0,
            width: 3,
            height: 3,
        }
    }

    #[test]
    fn levels_are_half_meter_from_0_5_to_12() {
        let levels = isopach_levels();
        assert_eq!(levels.len(), 24, "0.5..=12.0 step 0.5 => 24 levels");
        assert!((levels[0] - 0.5).abs() < 1e-12);
        assert!((levels[1] - 1.0).abs() < 1e-12);
        assert!((levels[23] - 12.0).abs() < 1e-12);
        // 라벨 형식은 소수점 한 자리 + "m"
        assert_eq!(format_isopach_label(0.5), "0.5m");
        assert_eq!(format_isopach_label(1.0), "1.0m");
        assert_eq!(format_isopach_label(12.0), "12.0m");
    }

    #[test]
    fn linear_grid_yields_contours_with_expected_labels_and_thickness() {
        let tf = tf();
        // col 방향으로 0,1,2,3 ... 증가 -> 등층후 값 0.5, 1.0 등이 col 경계를 가로지른다.
        let mut grid = vec![0.0_f64; 9];
        for r in 0..3 {
            for c in 0..3 {
                grid[r * 3 + c] = c as f64 * 1.0; // 0,1,2
            }
        }
        let segs = extract_all_isopach_segments(&tf, &grid);
        assert!(!segs.is_empty());

        // 값이 0,1,2 뿐이므로 0.5 / 1.0 / 1.5 / 2.0 레벨이 교차로 잡힌다.
        // - 1.0, 2.0 은 `va < lv && vb >= lv` 조건에서 0→1, 1→2 변이 교차한다.
        // - 2.0 은 격자 최댓값과 같아 마지막 열 경계도 교차로 계산된다.
        assert!(segs
            .iter()
            .all(|s| [0.5, 1.0, 1.5, 2.0].contains(&s.thickness_m)));
        let Some(half) = segs.iter().find(|s| s.thickness_m == 0.5) else {
            panic!("expected a 0.5m contour in the linear ramp grid");
        };
        assert_eq!(half.label, "0.5m");
        assert_eq!(half.points.len(), 2);
        // 셀 (r=0,c=0) 은 x0=5.0, x1=15.0 이고 값이 0→1 로 변하므로 t=0.5,
        // 교점 x = 5.0 + 0.5 * 10.0 = 10.0 이다.
        assert!((half.points[0][0] - 10.0).abs() < 1e-9);
        assert!((half.points[1][0] - 10.0).abs() < 1e-9);
        // 위/아래 변에서 만나므로 두 점의 y 는 셀 상·하단 경계(y0=25.0, y1=15.0)이다.
        assert!((half.points[0][1] - 25.0).abs() < 1e-9);
        assert!((half.points[1][1] - 15.0).abs() < 1e-9);
    }

    #[test]
    fn flat_grid_below_level_produces_no_contours() {
        let tf = tf();
        let grid = vec![0.0_f64; 9];
        let segs = extract_all_isopach_segments(&tf, &grid);
        assert!(segs.is_empty(), "no crossing when every value is 0");
    }

    #[test]
    fn non_finite_cells_are_skipped() {
        let tf = tf();
        let mut grid = vec![0.0_f64; 9];
        grid[4] = f64::NAN;
        for (i, v) in grid.iter_mut().enumerate() {
            if i != 4 {
                *v = if i % 2 == 0 { 0.0 } else { 5.0 };
            }
        }
        // NaN 셀이 포함된 셀은 건너뛰지만 나머지는 정상 동작해야 한다.
        let segs = extract_all_isopach_segments(&tf, &grid);
        assert!(segs.iter().all(|s| s.points.iter().all(|p| p[0].is_finite() && p[1].is_finite())));
    }

    #[test]
    fn crossing_requires_mixed_sides_not_equality() {
        // C++ 조건: va < lv <= vb 또는 vb < lv <= va. 양변이 모두 레벨과 같으면 교점 없음.
        let mut pts = Vec::new();
        push_edge_crossing(&mut pts, 1.0, 1.0, 1.0, [0.0, 0.0], [1.0, 0.0]);
        assert!(pts.is_empty());
        // 상승 교차
        push_edge_crossing(&mut pts, 1.0, 0.0, 2.0, [0.0, 0.0], [10.0, 0.0]);
        assert_eq!(pts.len(), 1);
        assert!((pts[0][0] - 5.0).abs() < 1e-9);
        // 하강 교차
        push_edge_crossing(&mut pts, 1.0, 2.0, 0.0, [0.0, 0.0], [10.0, 0.0]);
        assert_eq!(pts.len(), 2);
    }
}
