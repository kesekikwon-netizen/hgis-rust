//! L자형 트렌치 링 생성과 격자 교차점 정렬.
//!


/// `TrenchLShape.cpp::squareL` — 15 m × 15 m 칸 안에서 바깥 모서리 방향(0..=3)에 따른 6각 ㄱ·ㄴ자 폴리곤 생성
/// 폭(`width`)이 암 길이(`arm`) 이상이거나 0 이하인 비정상 파라미터일 때는 빈 벡터(Null 기하)를 반환합니다.
#[must_use]
pub fn build_lshape_ring(
    u0: f64,
    v0: f64,
    arm: f64,
    width: f64,
    bend: u8,
    grid_step: Option<f64>,
) -> Vec<[f64; 2]> {
    if width <= 0.0 || arm <= 0.0 || width >= arm {
        return Vec::new();
    }
    let (u_base, v_base) = if let Some(step) = grid_step.filter(|&s| s > 0.0) {
        ((u0 / step).round() * step, (v0 / step).round() * step)
    } else {
        (u0, v0)
    };
    let east = bend == 1 || bend == 2;
    let north = bend == 1 || bend == 3;
    let cx = if east { u_base + arm } else { u_base };
    let cy = if north { v_base + arm } else { v_base };
    let sx = if east { -1.0 } else { 1.0 };
    let sy = if north { -1.0 } else { 1.0 };
    let at = |du: f64, dv: f64| -> [f64; 2] { [cx + du * sx, cy + dv * sy] };
    vec![
        at(0.0, 0.0),
        at(arm, 0.0),
        at(arm, width),
        at(width, width),
        at(width, arm),
        at(0.0, arm),
        at(0.0, 0.0),
    ]
}

/// C++ `KaTrenchDialog::alignToGridIntersections(QList<QgsFeature> &trenches, double gridInterval = 10.0) -> int`
pub fn align_trenches_to_grid_intersections(
    trenches: &mut [Vec<[f64; 2]>],
    grid_interval_m: f64,
) -> usize {
    let step = grid_interval_m.max(1.0);
    let mut moved_count = 0usize;
    for ring in trenches.iter_mut() {
        let Some(&origin) = ring.first() else {
            continue;
        };
        let snapped_x = (origin[0] / step).round() * step;
        let snapped_y = (origin[1] / step).round() * step;
        let dx = snapped_x - origin[0];
        let dy = snapped_y - origin[1];
        if dx.hypot(dy) > 1e-6 {
            for pt in ring.iter_mut() {
                pt[0] += dx;
                pt[1] += dy;
            }
            moved_count += 1;
        }
    }
    moved_count
}
