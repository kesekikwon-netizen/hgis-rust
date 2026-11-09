//! 조사 기준점 필터와 등고선 셀·브레이크라인 보간.
//!
//! `trench_profile3d` 에서 분리했다.

/// 측량 의심점 판정 고도차 임계치 (15.0 m, C++ `HGIS_SUSPECT_ELEVATION_DIFF_M`)
pub const HGIS_SUSPECT_ELEVATION_DIFF_M: f64 = 15.0;

/// TIN 삼각망 최대 허용 변 길이 (50.0 m, C++ `HGIS_MAX_TIN_EDGE_LENGTH_M`)
pub const HGIS_MAX_TIN_EDGE_LENGTH_M: f64 = 50.0;

/// 등고선 계산 격자 최대 셀 예산 (4,000,000, C++ `SurveyContourBuilder::kMaxCells`)
pub const HGIS_MAX_CONTOUR_GRID_CELLS: i64 = 4_000_000;

/// 단절선(Breakline) 세분화 정점 최대 허용 상한 (200,000, C++ `kMaxBreaklineNodes`)
pub const HGIS_MAX_BREAKLINE_NODES: usize = 200_000;

/// 3차원 측량점 및 의심점 판정 플래그 (C++ `SurveyPoint3D` / `SurveyPoint`)
#[derive(Debug, Clone, PartialEq)]
pub struct SurveyPoint3D {
    pub fid: i64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub code: String,
    pub is_suspect: bool,
}

/// 주변 이웃점 평균 고도 편차 기반 의심점(Spike/Pit) 자동 필터링
/// (C++ `SurveyPointArrange::filterSuspectPoints` 1:1 대응)
pub fn filter_suspect_points(points: &mut [SurveyPoint3D], threshold_z: f64) {
    if points.len() < 3 {
        return;
    }
    let eff_thresh = threshold_z.max(0.5);
    let snapshot: Vec<(f64, f64, f64)> = points.iter().map(|p| (p.x, p.y, p.z)).collect();
    for (i, pt) in points.iter_mut().enumerate() {
        let mut neighbors_z = Vec::new();
        for (j, &(nx, ny, nz)) in snapshot.iter().enumerate() {
            if i == j || !nz.is_finite() {
                continue;
            }
            let dist = (pt.x - nx).hypot(pt.y - ny);
            if dist <= 100.0 {
                neighbors_z.push((dist, nz));
            }
        }
        neighbors_z.sort_by(|a, b| a.0.total_cmp(&b.0));
        let k_slice = &neighbors_z[..neighbors_z.len().min(8)];
        if k_slice.len() >= 2 {
            // 중앙값(median) 기반 로버스트 평균으로 단일 스파이크 오염을 방지
            let mut zs: Vec<f64> = k_slice.iter().map(|&(_, z)| z).collect();
            zs.sort_by(|a, b| a.total_cmp(b));
            let median_z = zs[zs.len() / 2];
            if (pt.z - median_z).abs() > eff_thresh {
                pt.is_suspect = true;
            }
        }
    }
}

/// 측량점 삼각망 생성 가능 여부 사전 검증 (`SurveyContourBuilder::rejection` 1:1 대응)
#[must_use]
pub fn survey_contour_rejection(points: &[SurveyPoint3D]) -> Option<&'static str> {
    let valid: Vec<&SurveyPoint3D> = points.iter().filter(|p| !p.is_suspect).collect();
    if valid.len() < 3 {
        return Some("등고선을 만들려면 점이 3개 이상 필요합니다.");
    }
    let origin = valid[0];
    let mut span = 0.0_f64;
    let mut far_idx = 0usize;
    for (idx, pt) in valid.iter().enumerate().skip(1) {
        let d = (pt.x - origin.x).hypot(pt.y - origin.y);
        if d > span {
            span = d;
            far_idx = idx;
        }
    }
    if span < 1e-6 {
        return Some("점이 일직선이라 삼각망을 만들 수 없습니다.");
    }
    let axis = valid[far_idx];
    let mut max_cross = 0.0_f64;
    for pt in &valid {
        let cross = ((axis.x - origin.x) * (pt.y - origin.y)
            - (axis.y - origin.y) * (pt.x - origin.x))
            .abs();
        max_cross = max_cross.max(cross);
    }
    if max_cross < 0.05 * span {
        return Some("점이 일직선이라 삼각망을 만들 수 없습니다.");
    }
    None
}

/// 측량점 간 최근접 거리 중앙값의 절반으로 기본 계산 격자 크기(`[0.05, 1.0]` m) 자동 산출
/// (C++ `SurveyContourBuilder::autoCellSize` 1:1 대응)
#[must_use]
pub fn auto_contour_cell_size(points: &[SurveyPoint3D]) -> f64 {
    let count = points.len();
    if count < 2 {
        return 0.5;
    }
    let sample = count.min(300);
    let step = (count / sample).max(1);
    let mut nearest = Vec::new();
    for i in (0..count).step_by(step) {
        let mut best = f64::INFINITY;
        for (j, pj) in points.iter().enumerate() {
            if i == j {
                continue;
            }
            let d = (points[i].x - pj.x).hypot(points[i].y - pj.y);
            if d > 1e-6 && d < best {
                best = d;
            }
        }
        if best.is_finite() {
            nearest.push(best);
        }
    }
    if nearest.is_empty() {
        return 0.5;
    }
    nearest.sort_by(|a, b| a.total_cmp(b));
    (nearest[nearest.len() / 2] * 0.5).clamp(0.05, 1.0)
}

/// 최대 셀 예산(`max_cells = 4,000,000`)을 초과하지 않도록 격자 크기를 1cm 단위로 확대 보정
/// (C++ `SurveyContourBuilder::fitCellSize` 1:1 대응)
#[must_use]
pub fn fit_contour_cell_size(cell: f64, span_x: f64, span_y: f64, max_cells: i64) -> f64 {
    if !cell.is_finite()
        || cell <= 0.0
        || !span_x.is_finite()
        || !span_y.is_finite()
        || max_cells < 4
    {
        return cell;
    }
    let cells_for = |size: f64| -> i64 {
        let nx = (span_x / size + 1.0).ceil() as i64;
        let ny = (span_y / size + 1.0).ceil() as i64;
        nx.saturating_mul(ny)
    };
    if cells_for(cell) <= max_cells {
        return cell;
    }
    let mut fitted = cell.max(((span_x.max(1e-3) * span_y.max(1e-3)) / (max_cells as f64)).sqrt());
    fitted = (fitted * 100.0).ceil() / 100.0;
    while cells_for(fitted) > max_cells {
        fitted = ((fitted + 0.01) * 100.0).round() / 100.0;
    }
    fitted
}

/// 3D 단절선(Breakline) 세분화 정점 생성 및 1mm 이내 기존 측량점 중복 제거
/// (C++ `densifySurveyBreaklines` 1:1 대응)
#[must_use]
pub fn densify_survey_breaklines(
    lines: &[Vec<SurveyPoint3D>],
    spacing: f64,
    existing: &[SurveyPoint3D],
) -> Vec<SurveyPoint3D> {
    if lines.is_empty() {
        return Vec::new();
    }
    let mut total_len = 0.0_f64;
    for line in lines {
        for w in line.windows(2) {
            total_len += (w[1].x - w[0].x).hypot(w[1].y - w[0].y);
        }
    }
    let step = (if spacing > 0.0 { spacing } else { 0.5 })
        .max(total_len / (HGIS_MAX_BREAKLINE_NODES as f64))
        .max(1e-3);
    let mm_key = |x: f64, y: f64| -> (i64, i64) {
        ((x * 1000.0).round() as i64, (y * 1000.0).round() as i64)
    };
    let mut taken: std::collections::BTreeSet<(i64, i64)> =
        existing.iter().map(|p| mm_key(p.x, p.y)).collect();
    let mut added = Vec::new();
    let mut push_node = |x: f64, y: f64, z: f64| {
        if !x.is_finite() || !y.is_finite() || !z.is_finite() {
            return;
        }
        let key = mm_key(x, y);
        if taken.insert(key) {
            added.push(SurveyPoint3D {
                fid: -1,
                x,
                y,
                z,
                code: "BREAKLINE".to_owned(),
                is_suspect: false,
            });
        }
    };
    for line in lines {
        for (i, a) in line.iter().enumerate() {
            push_node(a.x, a.y, a.z);
            if let Some(b) = line.get(i + 1) {
                let pieces = ((b.x - a.x).hypot(b.y - a.y) / step).floor() as usize;
                for k in 1..pieces {
                    let t = (k as f64) / (pieces as f64);
                    push_node(
                        a.x + t * (b.x - a.x),
                        a.y + t * (b.y - a.y),
                        a.z + t * (b.z - a.z),
                    );
                }
            }
        }
    }
    added
}
