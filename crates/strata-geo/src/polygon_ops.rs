use geo::algorithm::bool_ops::BooleanOps;
use geo::{Area, Coord, LineString, MultiPolygon, Polygon};
use strata_types::error::{StrataError, StrataResult};

pub const HGIS_SNAP_ANGLE_ORTHO: f64 = 90.0;
pub const HGIS_SNAP_ANGLE_45: f64 = 45.0;
pub const HGIS_MIN_POLYGON_VERTICES: usize = 4;
pub const HGIS_MIN_LINE_VERTICES: usize = 2;

/// C++ `snapToOrtho45` 템플릿 — Shift 키 누름 시 원점 기준 45도 배수 각도로 커서 좌표 스냅
#[must_use]
pub fn snap_to_ortho_45(origin: [f64; 2], cursor: [f64; 2]) -> [f64; 2] {
    let dx = cursor[0] - origin[0];
    let dy = cursor[1] - origin[1];
    let dist = dx.hypot(dy);
    if dist < 1e-12 {
        return origin;
    }
    let angle_deg = dy.atan2(dx).to_degrees();
    let snapped_deg = (angle_deg / HGIS_SNAP_ANGLE_45).round() * HGIS_SNAP_ANGLE_45;
    let rad = snapped_deg.to_radians();
    [origin[0] + dist * rad.cos(), origin[1] + dist * rad.sin()]
}

/// C++ `FeatureEditController::deleteVertex` (Polygon) — 폐합 정점 포함 최소 4점(삼각형) 이하일 때 정점 삭제 차단
pub fn delete_polygon_vertex_checked(ring: &mut Vec<[f64; 2]>, vertex_index: usize) -> bool {
    if ring.len() <= HGIS_MIN_POLYGON_VERTICES || vertex_index >= ring.len() {
        return false;
    }
    let is_closed = ring.first() == ring.last();
    if is_closed && (vertex_index == 0 || vertex_index + 1 == ring.len()) {
        ring.remove(0);
        if let Some(&new_first) = ring.first() {
            if let Some(last_mut) = ring.last_mut() {
                *last_mut = new_first;
            }
        }
    } else {
        ring.remove(vertex_index);
    }
    true
}

/// C++ `FeatureEditController::deleteVertex` (LineString) — 최소 2점 이하일 때 정점 삭제 차단
pub fn delete_line_vertex_checked(line: &mut Vec<[f64; 2]>, vertex_index: usize) -> bool {
    if line.len() <= HGIS_MIN_LINE_VERTICES || vertex_index >= line.len() {
        return false;
    }
    line.remove(vertex_index);
    true
}

/// C++ `PolygonOps::splitPolygonWithLine` — 절단선 교차점이 2개 미만이어서 분할 조각이 2개 미만일 때 에러 반환 및 최대 면적 조각 FID 승계
pub fn split_polygon_with_line_checked(
    subject: &Polygon<f64>,
    line_start: [f64; 2],
    line_end: [f64; 2],
    orig_feat_no: &str,
) -> Result<Vec<(String, Polygon<f64>)>, String> {
    let mut pieces = split_polygon_by_line(subject, line_start, line_end);
    if pieces.len() < 2 {
        return Err("절단선이 폴리곤 경계를 2회 이상 완전 관통해야 합니다.".to_owned());
    }
    pieces.sort_by(|a, b| {
        b.unsigned_area()
            .partial_cmp(&a.unsigned_area())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut out = Vec::with_capacity(pieces.len());
    for (idx, poly) in pieces.into_iter().enumerate() {
        let feat_no = if idx == 0 {
            orig_feat_no.to_owned()
        } else {
            format!("{orig_feat_no}-{idx}")
        };
        out.push((feat_no, poly));
    }
    Ok(out)
}

/// C++ `GeorefHelmert::calculateTransform` 반환 상태 (`enum class HelmertStatus`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelmertStatus {
    Ok,
    InsufficientPoints,
    SingularMatrix,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HelmertTransform2D {
    pub scale: f64,
    pub rotation_rad: f64,
    pub tx: f64,
    pub ty: f64,
    pub rms_error_m: f64,
}

impl HelmertTransform2D {
    #[must_use]
    pub fn apply(&self, xy: [f64; 2]) -> [f64; 2] {
        let c = self.rotation_rad.cos() * self.scale;
        let s = self.rotation_rad.sin() * self.scale;
        [
            c * xy[0] - s * xy[1] + self.tx,
            s * xy[0] + c * xy[1] + self.ty,
        ]
    }
}

pub fn erase_polygon(subject: &Polygon<f64>, cutter: &Polygon<f64>) -> MultiPolygon<f64> {
    subject.difference(cutter)
}

pub fn union_polygons(a: &Polygon<f64>, b: &Polygon<f64>) -> MultiPolygon<f64> {
    a.union(b)
}

#[must_use]
pub fn split_polygon_by_line(
    subject: &Polygon<f64>,
    line_start: [f64; 2],
    line_end: [f64; 2],
) -> Vec<Polygon<f64>> {
    let dx = line_end[0] - line_start[0];
    let dy = line_end[1] - line_start[1];
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-6 {
        return vec![subject.clone()];
    }
    // 0.002m(2mm) 절단선 블레이드로 Boolean difference를 수행해 조각을 얻는다.
    let nx = -dy / len * 0.001;
    let ny = dx / len * 0.001;
    let blade = Polygon::new(
        LineString::from(vec![
            Coord {
                x: line_start[0] + nx,
                y: line_start[1] + ny,
            },
            Coord {
                x: line_end[0] + nx,
                y: line_end[1] + ny,
            },
            Coord {
                x: line_end[0] - nx,
                y: line_end[1] - ny,
            },
            Coord {
                x: line_start[0] - nx,
                y: line_start[1] - ny,
            },
            Coord {
                x: line_start[0] + nx,
                y: line_start[1] + ny,
            },
        ]),
        vec![],
    );
    subject
        .difference(&blade)
        .0
        .into_iter()
        .filter(|p| p.exterior().0.len() >= 4 && p.unsigned_area() >= 1e-4)
        .collect()
}

/// C++ `GeorefHelmert::calculateTransform(gcps, outResult) -> HelmertStatus`
/// 2점 미만(`InsufficientPoints`), 공선점/퇴화 행렬(`SingularMatrix`) 검사를 포함합니다.
#[must_use]
pub fn calculate_helmert_transform_checked(
    src_pts: &[[f64; 2]],
    dst_pts: &[[f64; 2]],
) -> (HelmertStatus, Option<HelmertTransform2D>) {
    if src_pts.len() < 2 || src_pts.len() != dst_pts.len() {
        return (HelmertStatus::InsufficientPoints, None);
    }
    let n = src_pts.len() as f64;
    let (mut sx, mut sy) = (0.0, 0.0);
    for s in src_pts {
        sx += s[0];
        sy += s[1];
    }
    sx /= n;
    sy /= n;

    let (mut sxx, mut syy, mut sxy) = (0.0, 0.0, 0.0);
    for s in src_pts {
        let dx = s[0] - sx;
        let dy = s[1] - sy;
        sxx += dx * dx;
        syy += dy * dy;
        sxy += dx * dy;
    }
    if sxx + syy < 1e-12 {
        return (HelmertStatus::SingularMatrix, None);
    }
    if src_pts.len() >= 3 && (sxx * syy - sxy * sxy).abs() < 1e-9 {
        return (HelmertStatus::SingularMatrix, None);
    }
    match solve_helmert_georef(src_pts, dst_pts) {
        Ok(tf) if tf.scale > 1e-9 && tf.scale.is_finite() => (HelmertStatus::Ok, Some(tf)),
        _ => (HelmertStatus::SingularMatrix, None),
    }
}

pub fn solve_helmert_georef(
    src_pts: &[[f64; 2]],
    dst_pts: &[[f64; 2]],
) -> StrataResult<HelmertTransform2D> {
    if src_pts.len() < 2 || src_pts.len() != dst_pts.len() {
        return Err(StrataError::InvalidInput(
            "정합(GCP) 기준점은 최소 2쌍 이상 동일 개수여야 합니다".into(),
        ));
    }
    let n = src_pts.len() as f64;
    let (mut sx, mut sy, mut dx, mut dy) = (0.0, 0.0, 0.0, 0.0);
    for (s, d) in src_pts.iter().zip(dst_pts.iter()) {
        sx += s[0];
        sy += s[1];
        dx += d[0];
        dy += d[1];
    }
    sx /= n;
    sy /= n;
    dx /= n;
    dy /= n;

    let (mut num_a, mut num_b, mut den) = (0.0, 0.0, 0.0);
    for (s, d) in src_pts.iter().zip(dst_pts.iter()) {
        let xs = s[0] - sx;
        let ys = s[1] - sy;
        let xd = d[0] - dx;
        let yd = d[1] - dy;
        num_a += xs * xd + ys * yd;
        num_b += xs * yd - ys * xd;
        den += xs * xs + ys * ys;
    }
    if den < 1e-12 {
        return Err(StrataError::InvalidInput("퇴화된 GCP 좌표쌍입니다".into()));
    }
    let a = num_a / den;
    let b = num_b / den;
    let scale = (a * a + b * b).sqrt();
    let rotation_rad = b.atan2(a);
    let tx = dx - (a * sx - b * sy);
    let ty = dy - (b * sx + a * sy);

    let mut tf = HelmertTransform2D {
        scale,
        rotation_rad,
        tx,
        ty,
        rms_error_m: 0.0,
    };
    let mut sq_err = 0.0;
    for (s, d) in src_pts.iter().zip(dst_pts.iter()) {
        let p = tf.apply(*s);
        sq_err += (p[0] - d[0]).powi(2) + (p[1] - d[1]).powi(2);
    }
    tf.rms_error_m = (sq_err / n).sqrt();
    Ok(tf)
}

pub const HGIS_CIRCULAR_FEATURE_RATIO_LIMIT: f64 = 1.05;
pub const HGIS_MIN_VALID_POLYGON_VERTICES: usize = 3;
pub const HGIS_DEFAULT_NULL_STRING_DASH: &str = "-";
pub const HGIS_MAX_ASPECT_RATIO_CLAMP: f64 = 999.0;

#[derive(Debug, Clone, PartialEq)]
pub struct FeatureOrientationMetrics {
    pub major_axis_m: f64,
    pub minor_axis_m: f64,
    pub aspect_ratio: f64,
    pub raw_angle_deg_from_north: f64,
    pub archaeological_azimuth: String,
}

/// C++ `KaFeatureOrientationAnalyzer::formatArchaeologicalAzimuth` — 북 기준 방위각 `[0°, 180°)`을 고고학 표준 표기(`N-XX°-E`, `N-XX°-W`, `N`, `E`)로 변환
#[must_use]
pub fn format_archaeological_azimuth(angle_deg_from_north: f64) -> String {
    if !angle_deg_from_north.is_finite() {
        return HGIS_DEFAULT_NULL_STRING_DASH.to_owned();
    }
    let angle = angle_deg_from_north.rem_euclid(180.0);
    if !(0.5..=179.5).contains(&angle) {
        "N".to_owned()
    } else if (angle - 90.0).abs() < 0.5 {
        "E".to_owned()
    } else if angle < 90.0 {
        format!("N-{angle:.0}°-E")
    } else {
        format!("N-{:.0}°-W", 180.0 - angle)
    }
}

/// C++ `KaMinimumBoundingRectCalculator::calculateFeatureMbrMetrics` — Convex Hull + 회전 캘리퍼스 기반 최소 면적 외접 직사각형(MBR) 장축·단축·장단비 및 주축 방위각 산출
#[must_use]
pub fn calculate_feature_mbr_metrics(pts: &[[f64; 2]]) -> FeatureOrientationMetrics {
    use geo::ConvexHull;

    let valid_coords: Vec<Coord<f64>> = pts
        .iter()
        .filter(|p| p[0].is_finite() && p[1].is_finite())
        .map(|&[x, y]| Coord { x, y })
        .collect();
    if valid_coords.len() < HGIS_MIN_VALID_POLYGON_VERTICES {
        return FeatureOrientationMetrics {
            major_axis_m: 0.0,
            minor_axis_m: 0.0,
            aspect_ratio: 1.0,
            raw_angle_deg_from_north: 0.0,
            archaeological_azimuth: HGIS_DEFAULT_NULL_STRING_DASH.to_owned(),
        };
    }
    let poly = Polygon::new(LineString::from(valid_coords), vec![]);
    let hull = poly.convex_hull();
    let hull_pts = &hull.exterior().0;
    if hull_pts.len() < 4 {
        return FeatureOrientationMetrics {
            major_axis_m: 0.0,
            minor_axis_m: 0.0,
            aspect_ratio: 1.0,
            raw_angle_deg_from_north: 0.0,
            archaeological_azimuth: HGIS_DEFAULT_NULL_STRING_DASH.to_owned(),
        };
    }

    let mut best_area = f64::INFINITY;
    let mut best_major = 0.0_f64;
    let mut best_minor = 0.0_f64;
    let mut best_major_dir = [0.0_f64, 1.0_f64];

    for win in hull_pts.windows(2) {
        let dx = win[1].x - win[0].x;
        let dy = win[1].y - win[0].y;
        let len = dx.hypot(dy);
        if len <= 1e-9 {
            continue;
        }
        let ux = dx / len;
        let uy = dy / len;
        let vx = -uy;
        let vy = ux;
        let mut min_u = f64::INFINITY;
        let mut max_u = f64::NEG_INFINITY;
        let mut min_v = f64::INFINITY;
        let mut max_v = f64::NEG_INFINITY;
        for c in hull_pts {
            let pu = c.x * ux + c.y * uy;
            let pv = c.x * vx + c.y * vy;
            min_u = min_u.min(pu);
            max_u = max_u.max(pu);
            min_v = min_v.min(pv);
            max_v = max_v.max(pv);
        }
        let span_u = max_u - min_u;
        let span_v = max_v - min_v;
        let area = span_u * span_v;
        if area < best_area {
            best_area = area;
            if span_u >= span_v {
                best_major = span_u;
                best_minor = span_v;
                best_major_dir = [ux, uy];
            } else {
                best_major = span_v;
                best_minor = span_u;
                best_major_dir = [vx, vy];
            }
        }
    }

    let aspect_ratio = if best_minor <= 1e-6 {
        HGIS_MAX_ASPECT_RATIO_CLAMP
    } else {
        (best_major / best_minor).min(HGIS_MAX_ASPECT_RATIO_CLAMP)
    };
    // 북기준 시계 방향 각도: atan2(dx, dy)
    let raw_angle_deg_from_north = best_major_dir[0]
        .atan2(best_major_dir[1])
        .to_degrees()
        .rem_euclid(180.0);
    let archaeological_azimuth = if aspect_ratio <= HGIS_CIRCULAR_FEATURE_RATIO_LIMIT {
        "원형(무방위)".to_owned()
    } else {
        format_archaeological_azimuth(raw_angle_deg_from_north)
    };
    FeatureOrientationMetrics {
        major_axis_m: best_major,
        minor_axis_m: best_minor,
        aspect_ratio,
        raw_angle_deg_from_north,
        archaeological_azimuth,
    }
}

/// C++ `KaReportTableGenerator::naturalSortCompare` — 유구번호 문자열-숫자 청크 자연 정렬(`"1호" < "2호" < "10호" < "10-1호" < "10-2호" < "20호"`)
#[must_use]
pub fn natural_sort_compare(s1: &str, s2: &str) -> std::cmp::Ordering {
    let mut b1 = s1.chars().peekable();
    let mut b2 = s2.chars().peekable();
    while let (Some(&c1), Some(&c2)) = (b1.peek(), b2.peek()) {
        if c1.is_ascii_digit() && c2.is_ascii_digit() {
            let mut n1 = 0_u64;
            while let Some(&d) = b1.peek() {
                if let Some(v) = d.to_digit(10) {
                    n1 = n1.saturating_mul(10).saturating_add(u64::from(v));
                    b1.next();
                } else {
                    break;
                }
            }
            let mut n2 = 0_u64;
            while let Some(&d) = b2.peek() {
                if let Some(v) = d.to_digit(10) {
                    n2 = n2.saturating_mul(10).saturating_add(u64::from(v));
                    b2.next();
                } else {
                    break;
                }
            }
            let cmp = n1.cmp(&n2);
            if cmp != std::cmp::Ordering::Equal {
                return cmp;
            }
        } else {
            let is_branch_sep1 = c1 == '-' || c1 == '_';
            let is_branch_sep2 = c2 == '-' || c2 == '_';
            if is_branch_sep1 != is_branch_sep2 {
                // 본번("10호")이 부번("10-1호")보다 먼저 오도록 정렬
                return is_branch_sep1.cmp(&is_branch_sep2);
            }
            let cmp = c1.cmp(&c2);
            if cmp != std::cmp::Ordering::Equal {
                return cmp;
            }
            b1.next();
            b2.next();
        }
    }
    b1.count().cmp(&b2.count())
}

/// C++ `KaFeatureCatalogExporter` — 결측/빈 문자열 속성을 표준 대시(`"-"`)로 포맷팅
#[must_use]
pub fn format_nullable_catalog_field(raw: Option<&str>) -> String {
    match raw.map(str::trim) {
        Some(s) if !s.is_empty() && !s.eq_ignore_ascii_case("null") => s.to_owned(),
        _ => HGIS_DEFAULT_NULL_STRING_DASH.to_owned(),
    }
}

/// 병합 후 제거할 최소 슬리버 구멍 면적 (㎡, C++ `HGIS_MIN_SLIVER_AREA_M2`)
pub const HGIS_MIN_SLIVER_AREA_M2: f64 = 0.05;
/// 슬리버 정리용 스냅 허용 오차 (m, C++ `HGIS_SLIVER_SNAP_TOLERANCE_M`)
pub const HGIS_SLIVER_SNAP_TOLERANCE_M: f64 = 0.005;

/// 유구 폴리곤 내부에 교란 구역 내측 링(Hole)을 펀칭 추가 (C++ `KaHolePunchTool::punchInteriorHole`)
#[must_use]
pub fn punch_interior_hole_polygon(target: &Polygon<f64>, hole_ring: &[[f64; 2]]) -> Polygon<f64> {
    if hole_ring.len() < 3 {
        return target.clone();
    }
    let mut coords: Vec<Coord<f64>> = hole_ring.iter().map(|&[x, y]| Coord { x, y }).collect();
    if coords.first() != coords.last() {
        if let Some(&first) = coords.first() {
            coords.push(first);
        }
    }
    let mut interiors = target.interiors().to_vec();
    interiors.push(LineString::new(coords));
    Polygon::new(target.exterior().clone(), interiors)
}

/// 다각형 내부의 미세 슬리버 구멍(`area < min_sliver_area_m2`)을 제거하여 정리
/// (C++ `KaBooleanPolygonEditor::cleanSliversAndSpikes`)
#[must_use]
pub fn clean_sliver_interior_rings(poly: &Polygon<f64>, min_sliver_area_m2: f64) -> Polygon<f64> {
    let threshold = min_sliver_area_m2.max(0.0);
    let cleaned_interiors: Vec<LineString<f64>> = poly
        .interiors()
        .iter()
        .filter(|ring| {
            let hole_poly = Polygon::new((*ring).clone(), vec![]);
            hole_poly.unsigned_area() >= threshold
        })
        .cloned()
        .collect();
    Polygon::new(poly.exterior().clone(), cleaned_interiors)
}

/// 어핀 변환 최소 GCP 수 (C++ `HGIS_MIN_AFFINE_GCP_COUNT`)
pub const HGIS_MIN_AFFINE_GCP_COUNT: usize = 3;
/// 헬머트 변환 최소 GCP 수 (C++ `HGIS_MIN_HELMERT_GCP_COUNT`)
pub const HGIS_MIN_HELMERT_GCP_COUNT: usize = 2;
/// 실측도 허용 최대 RMSE 한계 (m, C++ `HGIS_MAX_ALLOWABLE_RMSE_M`)
pub const HGIS_MAX_ALLOWABLE_RMSE_M: f64 = 0.25;
/// 특이 행렬 판정 임계치 (C++ `HGIS_MATRIX_SINGULAR_EPSILON`)
pub const HGIS_MATRIX_SINGULAR_EPSILON: f64 = 1e-12;

/// 지오레퍼런싱 변환 모델 유형 (C++ `TransformType`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformType {
    /// 4-파라미터 헬머트 (최소 2점)
    Helmert4Param,
    /// 6-파라미터 2D 어핀 (`X = a*u + b*v + c`, `Y = d*u + e*v + f` / 최소 3점)
    Affine6Param,
}

/// 이미지 픽셀 좌표 - 실세계 좌표 기준점 쌍 (C++ `GcpPointPair`)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GcpPointPair {
    pub source_pixel: [f64; 2],
    pub target_world: [f64; 2],
    pub is_enabled: bool,
}

/// GCP 변환 해 산출 에러 (C++ `GcpSolverError`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GcpSolverError {
    InsufficientGcpCount,
    SingularMatrixCollinear,
}

/// 지오레퍼런싱 변환 계수 및 잔차·RMSE 결과 (C++ `GeoreferenceResult`)
#[derive(Debug, Clone, PartialEq)]
pub struct GeoreferenceResult {
    pub is_success: bool,
    pub transform_type: TransformType,
    /// `[a, b, c, d, e, f]` where `X = a*u + b*v + c`, `Y = d*u + e*v + f`
    pub transform_coeffs: [f64; 6],
    pub residuals_m: Vec<f64>,
    pub rmse_m: f64,
    pub exceeds_allowable_rmse: bool,
}

#[allow(clippy::needless_range_loop)]
fn solve_3x3_linear_system(mut m: [[f64; 3]; 3], mut rhs: [f64; 3]) -> Option<[f64; 3]> {
    for col in 0..3 {
        let mut pivot = col;
        for row in (col + 1)..3 {
            if m[row][col].abs() > m[pivot][col].abs() {
                pivot = row;
            }
        }
        if m[pivot][col].abs() < HGIS_MATRIX_SINGULAR_EPSILON {
            return None;
        }
        m.swap(col, pivot);
        rhs.swap(col, pivot);
        let div = m[col][col];
        for j in col..3 {
            m[col][j] /= div;
        }
        rhs[col] /= div;
        for row in 0..3 {
            if row != col {
                let factor = m[row][col];
                for j in col..3 {
                    m[row][j] -= factor * m[col][j];
                }
                rhs[row] -= factor * rhs[col];
            }
        }
    }
    Some(rhs)
}

/// 최소자승법 기반 6-파라미터 Affine 또는 4-파라미터 Helmert 지오레퍼런싱 변환 계수 및 RMSE 산출
/// (C++ `KaGcpTransformSolver::solveTransform` 1:1 대응)
pub fn solve_gcp_transform(
    gcp_list: &[GcpPointPair],
    transform_type: TransformType,
) -> Result<GeoreferenceResult, GcpSolverError> {
    let active: Vec<&GcpPointPair> = gcp_list.iter().filter(|g| g.is_enabled).collect();
    let min_req = match transform_type {
        TransformType::Affine6Param => HGIS_MIN_AFFINE_GCP_COUNT,
        TransformType::Helmert4Param => HGIS_MIN_HELMERT_GCP_COUNT,
    };
    if active.len() < min_req {
        return Err(GcpSolverError::InsufficientGcpCount);
    }

    let coeffs = match transform_type {
        TransformType::Affine6Param => {
            let mut ata = [[0.0_f64; 3]; 3];
            let mut atx = [0.0_f64; 3];
            let mut aty = [0.0_f64; 3];
            for g in &active {
                let u = g.source_pixel[0];
                let v = g.source_pixel[1];
                let x = g.target_world[0];
                let y = g.target_world[1];
                let row = [u, v, 1.0];
                for i in 0..3 {
                    for j in 0..3 {
                        ata[i][j] += row[i] * row[j];
                    }
                    atx[i] += row[i] * x;
                    aty[i] += row[i] * y;
                }
            }
            let abc =
                solve_3x3_linear_system(ata, atx).ok_or(GcpSolverError::SingularMatrixCollinear)?;
            let def =
                solve_3x3_linear_system(ata, aty).ok_or(GcpSolverError::SingularMatrixCollinear)?;
            [abc[0], abc[1], abc[2], def[0], def[1], def[2]]
        }
        TransformType::Helmert4Param => {
            let src_pts: Vec<[f64; 2]> = active.iter().map(|g| g.source_pixel).collect();
            let dst_pts: Vec<[f64; 2]> = active.iter().map(|g| g.target_world).collect();
            let (status, h) = calculate_helmert_transform_checked(&src_pts, &dst_pts);
            match status {
                HelmertStatus::Ok => {
                    let h = h.ok_or(GcpSolverError::SingularMatrixCollinear)?;
                    let c = h.rotation_rad.cos() * h.scale;
                    let s = h.rotation_rad.sin() * h.scale;
                    [c, -s, h.tx, s, c, h.ty]
                }
                HelmertStatus::InsufficientPoints => {
                    return Err(GcpSolverError::InsufficientGcpCount);
                }
                HelmertStatus::SingularMatrix => {
                    return Err(GcpSolverError::SingularMatrixCollinear);
                }
            }
        }
    };

    let mut residuals_m = Vec::with_capacity(active.len());
    let mut sum_sq = 0.0_f64;
    for g in &active {
        let u = g.source_pixel[0];
        let v = g.source_pixel[1];
        let x_hat = coeffs[0] * u + coeffs[1] * v + coeffs[2];
        let y_hat = coeffs[3] * u + coeffs[4] * v + coeffs[5];
        let res = (x_hat - g.target_world[0]).hypot(y_hat - g.target_world[1]);
        residuals_m.push(res);
        sum_sq += res * res;
    }
    let rmse_m = (sum_sq / active.len() as f64).sqrt();
    Ok(GeoreferenceResult {
        is_success: true,
        transform_type,
        transform_coeffs: coeffs,
        residuals_m,
        rmse_m,
        exceeds_allowable_rmse: rmse_m > HGIS_MAX_ALLOWABLE_RMSE_M,
    })
}

/// 유물 클러스터링 기본 반경 (m, C++ `HGIS_DBSCAN_DEFAULT_EPSILON_M`)
pub const HGIS_DBSCAN_DEFAULT_EPSILON_M: f64 = 1.5;
/// DBSCAN 코어 점 최소 이웃 수 (C++ `HGIS_DBSCAN_DEFAULT_MIN_PTS`)
pub const HGIS_DBSCAN_DEFAULT_MIN_PTS: usize = 4;
/// 3급 도근점 최소 요구 폐합비 분모 (1/3000, C++ `HGIS_TRAVERSE_3RD_ORDER_RATIO`)
pub const HGIS_TRAVERSE_3RD_ORDER_RATIO: i32 = 3000;
/// 3급 도근점 각오차 허용 한계 계수 (초, C++ `HGIS_ANGULAR_SEC_FACTOR_3RD`)
pub const HGIS_ANGULAR_SEC_FACTOR_3RD: f64 = 30.0;

/// DBSCAN 유물 밀집 군집 결과 (C++ `ArtifactClusterResult`)
#[derive(Debug, Clone, PartialEq)]
pub struct ArtifactClusterResult {
    pub cluster_id: i32,
    pub member_fids: Vec<i64>,
    pub centroid: [f64; 2],
    pub convex_hull_area_m2: f64,
    pub material_counts: std::collections::BTreeMap<String, usize>,
}

/// 유구별 공반 유물 재질 대칭 빈도 행렬 (C++ `CoOccurrenceMatrixResult`)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoOccurrenceMatrixResult {
    pub material_labels: Vec<String>,
    pub matrix: Vec<Vec<usize>>,
    pub total_per_feature: Vec<usize>,
}

/// 출토 유물 점군에 대한 DBSCAN 공간 군집화 수행 (유효 클러스터 목록과 노이즈 점 개수 반환)
/// (C++ `KaArtifactClusterAnalyzer::computeDbscanClusters` 1:1 대응)
#[must_use]
pub fn compute_dbscan_artifact_clusters(
    artifacts: &[(i64, [f64; 2], &str)],
    epsilon_m: f64,
    min_pts: usize,
) -> (Vec<ArtifactClusterResult>, usize) {
    use geo::ConvexHull;

    let n = artifacts.len();
    if n == 0 || min_pts == 0 || n < min_pts {
        return (Vec::new(), n);
    }
    let eps = epsilon_m.max(0.0);
    // -2: unvisited, -1: noise, >= 0: cluster_id
    let mut labels = vec![-2_i32; n];
    let mut cluster_id = 0_i32;

    let region_query = |idx: usize| -> Vec<usize> {
        let p = artifacts[idx].1;
        (0..n)
            .filter(|&j| {
                let q = artifacts[j].1;
                (p[0] - q[0]).hypot(p[1] - q[1]) <= eps
            })
            .collect()
    };

    for i in 0..n {
        if labels[i] != -2 {
            continue;
        }
        let neighbors = region_query(i);
        if neighbors.len() < min_pts {
            labels[i] = -1;
            continue;
        }
        labels[i] = cluster_id;
        let mut queue: std::collections::VecDeque<usize> =
            neighbors.into_iter().filter(|&j| j != i).collect();
        while let Some(curr) = queue.pop_front() {
            if labels[curr] == -1 {
                labels[curr] = cluster_id;
            }
            if labels[curr] != -2 {
                continue;
            }
            labels[curr] = cluster_id;
            let curr_neighbors = region_query(curr);
            if curr_neighbors.len() >= min_pts {
                for nb in curr_neighbors {
                    if labels[nb] == -2 || labels[nb] == -1 {
                        queue.push_back(nb);
                    }
                }
            }
        }
        cluster_id += 1;
    }

    let noise_count = labels.iter().filter(|&&l| l == -1).count();
    let mut clusters = Vec::with_capacity(cluster_id as usize);
    for cid in 0..cluster_id {
        let mut member_fids = Vec::new();
        let mut coords = Vec::new();
        let mut sx = 0.0_f64;
        let mut sy = 0.0_f64;
        let mut material_counts = std::collections::BTreeMap::new();
        for (idx, &(fid, xy, mat)) in artifacts.iter().enumerate() {
            if labels[idx] == cid {
                member_fids.push(fid);
                coords.push(Coord { x: xy[0], y: xy[1] });
                sx += xy[0];
                sy += xy[1];
                *material_counts.entry(mat.to_owned()).or_insert(0) += 1;
            }
        }
        let cnt = member_fids.len() as f64;
        let centroid = if cnt > 0.0 {
            [sx / cnt, sy / cnt]
        } else {
            [0.0, 0.0]
        };
        let poly = Polygon::new(LineString::new(coords), vec![]);
        let hull_area = poly.convex_hull().unsigned_area();
        clusters.push(ArtifactClusterResult {
            cluster_id: cid,
            member_fids,
            centroid,
            convex_hull_area_m2: hull_area,
            material_counts,
        });
    }
    (clusters, noise_count)
}

/// 동일 유구 내 출토 유물 재질 간 공반(Co-occurrence) 대칭 빈도 행렬 산출
/// (C++ `KaCoOccurrenceMatrix::computeMaterialCoOccurrence` 1:1 대응)
#[must_use]
pub fn compute_material_co_occurrence(
    feature_artifacts: &[Vec<&str>],
    material_labels: &[&str],
) -> CoOccurrenceMatrixResult {
    let m = material_labels.len();
    let mut matrix = vec![vec![0_usize; m]; m];
    let mut total_per_feature = Vec::with_capacity(feature_artifacts.len());
    for mats in feature_artifacts {
        total_per_feature.push(mats.len());
        for (i, &label_a) in material_labels.iter().enumerate() {
            let has_a = mats.contains(&label_a);
            if !has_a {
                continue;
            }
            for (j, &label_b) in material_labels.iter().enumerate() {
                if mats.contains(&label_b) {
                    matrix[i][j] += 1;
                }
            }
        }
    }
    CoOccurrenceMatrixResult {
        material_labels: material_labels.iter().map(|&s| s.to_owned()).collect(),
        matrix,
        total_per_feature,
    }
}

/// 다각망 측선 관측 레코드 (C++ `TraverseTraverseLeg`)
#[derive(Debug, Clone, PartialEq)]
pub struct TraverseLeg {
    pub from_station: String,
    pub to_station: String,
    pub delta_x_m: f64,
    pub delta_y_m: f64,
    pub measured_distance_m: f64,
}

/// 폐합 다각망(Traverse) 각오차·선형 폐합비 검증 및 Bowditch 좌표 보정 결과 (C++ `NetworkAdjustmentResult`)
#[derive(Debug, Clone, PartialEq)]
pub struct NetworkAdjustmentResult {
    pub is_acceptable: bool,
    pub angular_error_sec: f64,
    pub max_allowable_angular_sec: f64,
    pub linear_closing_error_m: f64,
    pub total_traverse_length_m: f64,
    pub closing_ratio_denominator: i32,
    pub adjusted_coords: Vec<[f64; 2]>,
}

/// 3급 도근점 기준 폐합 다각망 선형 폐합비(`1/N >= 1/3000`)·각오차 검증 및 콤파스 법칙(Bowditch) 좌표 조정
/// (C++ `KaSurveyNetworkAdjuster::adjustClosedTraverse` 1:1 대응)
#[must_use]
pub fn adjust_closed_traverse(
    legs: &[TraverseLeg],
    fixed_known_coord: [f64; 2],
    angular_error_sec: f64,
) -> NetworkAdjustmentResult {
    let n = legs.len();
    let total_len: f64 = legs.iter().map(|l| l.measured_distance_m.max(0.0)).sum();
    if n < 2 || total_len <= 1e-6 || !total_len.is_finite() {
        return NetworkAdjustmentResult {
            is_acceptable: false,
            angular_error_sec,
            max_allowable_angular_sec: 0.0,
            linear_closing_error_m: 0.0,
            total_traverse_length_m: 0.0,
            closing_ratio_denominator: 0,
            adjusted_coords: Vec::new(),
        };
    }
    let fx: f64 = legs.iter().map(|l| l.delta_x_m).sum();
    let fy: f64 = legs.iter().map(|l| l.delta_y_m).sum();
    let fs = fx.hypot(fy);
    let ratio_denom = if fs <= 1e-9 {
        999_999
    } else {
        (total_len / fs).round() as i32
    };
    let max_allowable_angular_sec = HGIS_ANGULAR_SEC_FACTOR_3RD * (n as f64).sqrt();
    let is_acceptable = ratio_denom >= HGIS_TRAVERSE_3RD_ORDER_RATIO
        && angular_error_sec.abs() <= max_allowable_angular_sec;

    let mut adjusted_coords = Vec::with_capacity(n + 1);
    let mut curr = fixed_known_coord;
    adjusted_coords.push(curr);
    for leg in legs {
        let weight = leg.measured_distance_m.max(0.0) / total_len;
        curr[0] += leg.delta_x_m - fx * weight;
        curr[1] += leg.delta_y_m - fy * weight;
        adjusted_coords.push(curr);
    }

    NetworkAdjustmentResult {
        is_acceptable,
        angular_error_sec,
        max_allowable_angular_sec,
        linear_closing_error_m: fs,
        total_traverse_length_m: total_len,
        closing_ratio_denominator: ratio_denom,
        adjusted_coords,
    }
}

pub const HGIS_MIN_PARCEL_OVERLAP_M2: f64 = 0.1;
pub const HGIS_MAX_PARCEL_RECORDS_LIMIT: usize = 100_000;
pub const HGIS_AREA_ROUND_DECIMAL_PLACES: u32 = 2;
pub const HGIS_RATIO_ROUND_DECIMAL_PLACES: u32 = 1;

#[derive(Debug, Clone, PartialEq)]
pub struct CadastralParcelInput {
    pub pnu: String,
    pub jibun: String,
    pub jimok: String,
    pub official_area_m2: f64,
    pub exterior_ring: Vec<[f64; 2]>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CadastralParcelRecord {
    pub pnu: String,
    pub jibun: String,
    pub jimok: String,
    pub official_area_m2: f64,
    pub total_geom_area_m2: f64,
    pub overlap_area_m2: f64,
    pub inclusion_ratio_pct: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParcelInclusionSummary {
    pub total_parcels_count: usize,
    pub total_official_area_m2: f64,
    pub total_overlap_area_m2: f64,
    pub overall_inclusion_ratio_pct: f64,
    pub parcel_records: Vec<CadastralParcelRecord>,
}

/// C++ `KaCadastralOverlapAnalyzer::analyzeCadastralInclusion` — 조사구역 다각형과 지적 필지 다각형의 교차 면적(`min_intersection_area_m2 >= 0.1㎡` 슬리버 필터링), 소수점 둘째 자리 면적 반올림 및 소수점 첫째 자리 편입률 산출
#[must_use]
pub fn analyze_cadastral_inclusion(
    survey_boundary_ring: &[[f64; 2]],
    parcels: &[CadastralParcelInput],
    min_intersection_area_m2: f64,
) -> ParcelInclusionSummary {
    use geo::{Area, BooleanOps};

    if survey_boundary_ring.len() < 3 {
        return ParcelInclusionSummary {
            total_parcels_count: 0,
            total_official_area_m2: 0.0,
            total_overlap_area_m2: 0.0,
            overall_inclusion_ratio_pct: 0.0,
            parcel_records: Vec::new(),
        };
    }

    let survey_poly = Polygon::new(
        LineString::from(
            survey_boundary_ring
                .iter()
                .map(|&[x, y]| Coord { x, y })
                .collect::<Vec<_>>(),
        ),
        vec![],
    );
    let min_area = if min_intersection_area_m2.is_finite() && min_intersection_area_m2 >= 0.0 {
        min_intersection_area_m2
    } else {
        HGIS_MIN_PARCEL_OVERLAP_M2
    };

    let mut parcel_records = Vec::new();
    let mut total_official = 0.0;
    let mut total_overlap = 0.0;

    for p in parcels.iter().take(HGIS_MAX_PARCEL_RECORDS_LIMIT) {
        if p.exterior_ring.len() < 3 {
            continue;
        }
        let parcel_poly = Polygon::new(
            LineString::from(
                p.exterior_ring
                    .iter()
                    .map(|&[x, y]| Coord { x, y })
                    .collect::<Vec<_>>(),
            ),
            vec![],
        );
        let geom_area = parcel_poly.unsigned_area();
        if geom_area <= 1e-9 || !geom_area.is_finite() {
            continue;
        }
        let inter_mp = survey_poly.intersection(&parcel_poly);
        let raw_overlap = inter_mp.unsigned_area();
        if !raw_overlap.is_finite() || raw_overlap < min_area {
            continue;
        }

        let base_area = if p.official_area_m2 > 0.0 && p.official_area_m2.is_finite() {
            p.official_area_m2
        } else {
            geom_area
        };
        let overlap_rounded = (raw_overlap * 100.0).round() / 100.0;
        let raw_ratio = if base_area > 0.0 {
            (raw_overlap / base_area * 100.0).clamp(0.0, 100.0)
        } else {
            0.0
        };
        let ratio_rounded = (raw_ratio * 10.0).round() / 10.0;

        total_official += base_area;
        total_overlap += overlap_rounded;
        parcel_records.push(CadastralParcelRecord {
            pnu: p.pnu.clone(),
            jibun: p.jibun.clone(),
            jimok: p.jimok.clone(),
            official_area_m2: base_area,
            total_geom_area_m2: (geom_area * 100.0).round() / 100.0,
            overlap_area_m2: overlap_rounded,
            inclusion_ratio_pct: ratio_rounded,
        });
    }

    let overall_ratio = if total_official > 0.0 {
        ((total_overlap / total_official * 100.0).clamp(0.0, 100.0) * 10.0).round() / 10.0
    } else {
        0.0
    };

    ParcelInclusionSummary {
        total_parcels_count: parcel_records.len(),
        total_official_area_m2: (total_official * 100.0).round() / 100.0,
        total_overlap_area_m2: (total_overlap * 100.0).round() / 100.0,
        overall_inclusion_ratio_pct: overall_ratio,
        parcel_records,
    }
}

/// 도면 정합 최우수 등급(Grade A) RMSE 상한 (0.10m, C++ `HGIS_GEOREF_EXCELLENT_RMSE_M`)
pub const HGIS_GEOREF_EXCELLENT_RMSE_M: f64 = 0.10;
/// 도면 정합 실무 허용 등급(Grade B) RMSE 상한 (0.25m, C++ `HGIS_GEOREF_MAX_PERMISSIBLE_M`)
pub const HGIS_GEOREF_MAX_PERMISSIBLE_M: f64 = 0.25;
/// 서브픽셀 십자선 탐색 윈도우 반경 (7px -> 15x15 윈도우, C++ `HGIS_SUBPIXEL_WINDOW_RADIUS`)
pub const HGIS_SUBPIXEL_WINDOW_RADIUS: i32 = 7;

/// 도면 정합 잔차(RMSE) 품질 등급 (C++ `GeorefQualityGrade`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeorefQualityGrade {
    GradeAExcellent,
    GradeBGood,
    GradeCCaution,
    GradeDCritical,
}

/// 서브픽셀 15x15 가중 무게중심 탐색 결과 (C++ `SubPixelWindowResult`)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SubPixelWindowResult {
    pub refined_u: f64,
    pub refined_v: f64,
    pub contrast_score: f64,
    pub is_confident: bool,
}

/// 정합 잔차(RMSE, 단위: m)로부터 A~D 4단계 정합 품질 등급 산정
/// (C++ `GeorefQuality::evaluateRmseGrade` 1:1 대응)
#[must_use]
pub fn evaluate_rmse_grade(rmse_m: f64) -> GeorefQualityGrade {
    if !rmse_m.is_finite() || !(0.0..=0.50).contains(&rmse_m) {
        GeorefQualityGrade::GradeDCritical
    } else if rmse_m <= HGIS_GEOREF_EXCELLENT_RMSE_M {
        GeorefQualityGrade::GradeAExcellent
    } else if rmse_m <= HGIS_GEOREF_MAX_PERMISSIBLE_M {
        GeorefQualityGrade::GradeBGood
    } else {
        GeorefQualityGrade::GradeCCaution
    }
}

/// 기준점 개수(`pair_count`) 및 잔차(`rmse_m`, `max_residual_m`)에 따른 한국어 정합 품질 안내 문구 생성
/// (C++ `GeorefQuality::georefQualitySummary` 1:1 대응)
#[must_use]
pub fn georef_quality_summary_ko(pair_count: usize, rmse_m: f64, max_residual_m: f64) -> String {
    match pair_count {
        0 | 1 => "지도와 그림에서 같은 자리를 최소 2곳 찍어 주세요".to_owned(),
        2 => "기준점 2개 · 한 점 더 찍으면 기울기도 맞습니다".to_owned(),
        3 => "기준점 3개 · 한 점 더 찍으면 어긋남을 잽니다".to_owned(),
        _ => format!(
            "기준점 {pair_count}개 · 평균 어긋남 {rmse_m:.2} m · 최대 {max_residual_m:.2} m"
        ),
    }
}

/// 15x15(`2 * HGIS_SUBPIXEL_WINDOW_RADIUS + 1`) 그레이스케일 패치에서 어두운 도면 십자선 교차점의 서브픽셀 무게중심 추출 (대비 부족 시 정수 클릭 좌표 폴백)
/// (C++ `GeorefPixelWindow::findSubPixelCrosshairCenter` 1:1 대응)
#[must_use]
pub fn find_subpixel_crosshair_center_15x15(
    gray_patch_15x15: &[u8],
    click_u0: i32,
    click_v0: i32,
) -> SubPixelWindowResult {
    let side = (2 * HGIS_SUBPIXEL_WINDOW_RADIUS + 1) as usize;
    if gray_patch_15x15.len() < side * side {
        return SubPixelWindowResult {
            refined_u: f64::from(click_u0),
            refined_v: f64::from(click_v0),
            contrast_score: 0.0,
            is_confident: false,
        };
    }
    let mut min_val = u8::MAX;
    let mut max_val = u8::MIN;
    for &px in &gray_patch_15x15[..side * side] {
        min_val = min_val.min(px);
        max_val = max_val.max(px);
    }
    let contrast = f64::from(max_val.saturating_sub(min_val)) / 255.0;
    if contrast < 0.15 {
        return SubPixelWindowResult {
            refined_u: f64::from(click_u0),
            refined_v: f64::from(click_v0),
            contrast_score: contrast,
            is_confident: false,
        };
    }
    let threshold = f64::from(min_val) + f64::from(max_val - min_val) * 0.35;
    let mut row_dark = vec![0.0_f64; side];
    let mut col_dark = vec![0.0_f64; side];
    for row in 0..side {
        for col in 0..side {
            let val = f64::from(gray_patch_15x15[row * side + col]);
            if val <= threshold {
                let d = threshold - val + 1.0;
                row_dark[row] += d;
                col_dark[col] += d;
            }
        }
    }
    let mut sum_w = 0.0_f64;
    let mut sum_du = 0.0_f64;
    let mut sum_dv = 0.0_f64;
    for (row, &rd) in row_dark.iter().enumerate() {
        for (col, &cd) in col_dark.iter().enumerate() {
            let val = f64::from(gray_patch_15x15[row * side + col]);
            if val <= threshold {
                let w = (threshold - val + 1.0) * rd.powi(2) * cd.powi(2);
                let du = (col as i32) - HGIS_SUBPIXEL_WINDOW_RADIUS;
                let dv = (row as i32) - HGIS_SUBPIXEL_WINDOW_RADIUS;
                sum_w += w;
                sum_du += w * f64::from(du);
                sum_dv += w * f64::from(dv);
            }
        }
    }
    if sum_w <= 1e-9 {
        return SubPixelWindowResult {
            refined_u: f64::from(click_u0),
            refined_v: f64::from(click_v0),
            contrast_score: contrast,
            is_confident: false,
        };
    }
    SubPixelWindowResult {
        refined_u: f64::from(click_u0) + (sum_du / sum_w).clamp(-7.0, 7.0),
        refined_v: f64::from(click_v0) + (sum_dv / sum_w).clamp(-7.0, 7.0),
        contrast_score: contrast,
        is_confident: true,
    }
}

/// 정합 결과 저장 시 기존 파일을 덮어쓰지 않도록 `stem_2.ext` ~ `stem_9999.ext` 고유 출력 경로 생성
/// (C++ `GeorefBackup::uniqueOutputPath` 1:1 대응)
#[must_use]
pub fn unique_georef_output_path(
    preferred_path: &std::path::Path,
    existing_paths: &std::collections::BTreeSet<std::path::PathBuf>,
) -> std::path::PathBuf {
    if !existing_paths.contains(preferred_path) && !preferred_path.exists() {
        return preferred_path.to_path_buf();
    }
    let parent = preferred_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new(""));
    let stem = preferred_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("정합결과");
    let ext = preferred_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("tif");
    for n in 2..=9999_u32 {
        let candidate = if ext.is_empty() {
            parent.join(format!("{stem}_{n}"))
        } else {
            parent.join(format!("{stem}_{n}.{ext}"))
        };
        if !existing_paths.contains(&candidate) && !candidate.exists() {
            return candidate;
        }
    }
    parent.join(format!("{stem}_9999.{ext}"))
}

/// Liang-Barsky 알고리즘을 사용해 선분(`seg_start -> seg_end`)을 조사구역 사각 경계(`bbox = [min_x, min_y, max_x, max_y]`) 내부로 정밀 절단(Clip)
/// (C++ `SurveyScopeClip::clipVectorLayerByBoundary` 선형 교차 절단 1:1 대응)
#[must_use]
pub fn clip_line_segment_by_bbox(
    seg_start: [f64; 2],
    seg_end: [f64; 2],
    bbox: [f64; 4],
) -> Option<([f64; 2], [f64; 2])> {
    let min_x = bbox[0].min(bbox[2]);
    let min_y = bbox[1].min(bbox[3]);
    let max_x = bbox[0].max(bbox[2]);
    let max_y = bbox[1].max(bbox[3]);

    let dx = seg_end[0] - seg_start[0];
    let dy = seg_end[1] - seg_start[1];
    let p = [-dx, dx, -dy, dy];
    let q = [
        seg_start[0] - min_x,
        max_x - seg_start[0],
        seg_start[1] - min_y,
        max_y - seg_start[1],
    ];

    let mut u1 = 0.0_f64;
    let mut u2 = 1.0_f64;
    for i in 0..4 {
        if p[i].abs() <= 1e-12 {
            if q[i] < 0.0 {
                return None;
            }
        } else {
            let t = q[i] / p[i];
            if p[i] < 0.0 {
                if t > u2 {
                    return None;
                }
                if t > u1 {
                    u1 = t;
                }
            } else {
                if t < u1 {
                    return None;
                }
                if t < u2 {
                    u2 = t;
                }
            }
        }
    }
    Some((
        [seg_start[0] + u1 * dx, seg_start[1] + u1 * dy],
        [seg_start[0] + u2 * dx, seg_start[1] + u2 * dy],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn polygon_split_and_helmert_georef_produce_two_pieces_and_submillimeter_rms(
    ) -> StrataResult<()> {
        let rect = Polygon::new(
            LineString::from(vec![
                Coord { x: 0.0, y: 0.0 },
                Coord { x: 100.0, y: 0.0 },
                Coord { x: 100.0, y: 100.0 },
                Coord { x: 0.0, y: 100.0 },
                Coord { x: 0.0, y: 0.0 },
            ]),
            vec![],
        );
        let pieces = split_polygon_by_line(&rect, [50.0, -10.0], [50.0, 110.0]);
        assert_eq!(pieces.len(), 2);

        let src = [[0.0, 0.0], [100.0, 0.0], [0.0, 100.0]];
        let dst = [
            [200_000.0, 500_000.0],
            [200_200.0, 500_000.0],
            [200_000.0, 500_200.0],
        ];
        let tf = solve_helmert_georef(&src, &dst)?;
        assert!((tf.scale - 2.0).abs() < 1e-6);
        assert!(tf.rms_error_m < 1e-6);
        let mapped = tf.apply([50.0, 50.0]);
        assert!((mapped[0] - 200_100.0).abs() < 1e-5);
        assert!((mapped[1] - 500_100.0).abs() < 1e-5);
        Ok(())
    }

    #[test]
    fn test_georef_helmert_45deg_scale2_and_collinear_singular_matrix_detection() {
        let rot = std::f64::consts::FRAC_PI_4; // 45 deg = 0.785398163...
        let scale = 2.0_f64;
        let tx = 100.0_f64;
        let ty = 200.0_f64;
        let c = scale * rot.cos();
        let s = scale * rot.sin();

        let src = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
        let dst: Vec<[f64; 2]> = src
            .iter()
            .map(|&[x, y]| [c * x - s * y + tx, s * x + c * y + ty])
            .collect();

        let (status, tf_opt) = calculate_helmert_transform_checked(&src, &dst);
        assert_eq!(status, HelmertStatus::Ok);
        let tf = tf_opt.expect("helmert transform must exist");
        assert!((tf.scale - 2.0).abs() < 1e-6);
        assert!((tf.rotation_rad - rot).abs() < 1e-6);
        assert!(tf.rms_error_m < 1e-9);

        // 4 collinear points on y = 2x -> SingularMatrix
        let collinear_src = [[0.0, 0.0], [1.0, 2.0], [2.0, 4.0], [3.0, 6.0]];
        let (col_status, col_tf) = calculate_helmert_transform_checked(&collinear_src, &dst);
        assert_eq!(col_status, HelmertStatus::SingularMatrix);
        assert!(col_tf.is_none());
    }

    #[test]
    fn test_polygon_ops_split_10x10_square_yields_two_50m2_pieces() {
        let sq = Polygon::new(
            LineString::from(vec![
                Coord { x: 0.0, y: 0.0 },
                Coord { x: 10.0, y: 0.0 },
                Coord { x: 10.0, y: 10.0 },
                Coord { x: 0.0, y: 10.0 },
                Coord { x: 0.0, y: 0.0 },
            ]),
            vec![],
        );
        let pieces = split_polygon_by_line(&sq, [5.0, -2.0], [5.0, 12.0]);
        assert_eq!(pieces.len(), 2);
        let total_area: f64 = pieces.iter().map(Area::unsigned_area).sum();
        assert!((total_area - 100.0).abs() < 0.1);
        for p in &pieces {
            assert!((p.unsigned_area() - 50.0).abs() < 0.1);
        }

        let checked = split_polygon_with_line_checked(&sq, [5.0, -2.0], [5.0, 12.0], "구덩이-10")
            .expect("valid split");
        assert_eq!(checked.len(), 2);
        assert_eq!(checked[0].0, "구덩이-10");
        assert_eq!(checked[1].0, "구덩이-10-1");

        // 내부에서 끝나는 불완전 절단선은 실패해야 함
        let incomplete = split_polygon_with_line_checked(&sq, [5.0, -2.0], [5.0, 5.0], "구덩이-10");
        assert!(incomplete.is_err());
    }

    #[test]
    fn test_delete_vertex_triangle_blocked_and_ortho_45_snap() {
        let mut triangle = vec![[0.0, 0.0], [10.0, 0.0], [5.0, 10.0], [0.0, 0.0]];
        assert_eq!(triangle.len(), 4);
        assert!(!delete_polygon_vertex_checked(&mut triangle, 1));
        assert_eq!(triangle.len(), 4);

        let mut quad = vec![
            [0.0, 0.0],
            [10.0, 0.0],
            [10.0, 10.0],
            [0.0, 10.0],
            [0.0, 0.0],
        ];
        assert!(delete_polygon_vertex_checked(&mut quad, 2));
        assert_eq!(quad.len(), 4);

        let mut seg = vec![[0.0, 0.0], [10.0, 0.0]];
        assert!(!delete_line_vertex_checked(&mut seg, 0));
        assert_eq!(seg.len(), 2);

        let snapped = snap_to_ortho_45([0.0, 0.0], [10.0, 1.0]);
        assert!((snapped[1] - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_n15e_and_n30w_mbr_azimuth_and_natural_sort_and_null_dash() {
        // 2.0m x 6.0m rectangle rotated 15 deg clockwise (N-15°-E)
        let base_rect = [
            [-1.0_f64, -3.0],
            [1.0, -3.0],
            [1.0, 3.0],
            [-1.0, 3.0],
            [-1.0, -3.0],
        ];
        // Clockwise by 15 deg: x' = x cos15 + y sin15, y' = -x sin15 + y cos15
        let rad = 15.0_f64.to_radians();
        let (s, c) = rad.sin_cos();
        let rot_rect: Vec<[f64; 2]> = base_rect
            .iter()
            .map(|&[x, y]| [x * c + y * s, -x * s + y * c])
            .collect();
        let metrics = calculate_feature_mbr_metrics(&rot_rect);
        assert!((metrics.major_axis_m - 6.0).abs() < 1e-3);
        assert!((metrics.minor_axis_m - 2.0).abs() < 1e-3);
        assert!((metrics.aspect_ratio - 3.0).abs() < 1e-3);
        assert_eq!(metrics.archaeological_azimuth, "N-15°-E");
        assert_eq!(format_archaeological_azimuth(150.0), "N-30°-W");

        let mut list = vec!["10호", "2호", "1호", "10-2호", "10-1호", "20호"];
        list.sort_by(|a, b| natural_sort_compare(a, b));
        assert_eq!(list, vec!["1호", "2호", "10호", "10-1호", "10-2호", "20호"]);
        assert_eq!(format_nullable_catalog_field(None), "-");
        assert_eq!(format_nullable_catalog_field(Some("   ")), "-");
    }

    #[test]
    fn test_interior_hole_punch_10x10_minus_2x2_yields_96m2_and_sliver_cleaning_under_0_05m2() {
        // TestKaBooleanPolygonEditor::testInteriorHolePunch:
        // 10m x 10m outer polygon (100 m²) minus 2m x 2m interior hole (4 m²) -> 1 interior ring, 96.0 m² area
        let outer = Polygon::new(
            LineString::from(vec![
                Coord { x: 0.0, y: 0.0 },
                Coord { x: 10.0, y: 0.0 },
                Coord { x: 10.0, y: 10.0 },
                Coord { x: 0.0, y: 10.0 },
                Coord { x: 0.0, y: 0.0 },
            ]),
            vec![],
        );
        let hole = [[4.0, 4.0], [6.0, 4.0], [6.0, 6.0], [4.0, 6.0], [4.0, 4.0]];
        let punched = punch_interior_hole_polygon(&outer, &hole);
        assert_eq!(punched.interiors().len(), 1);
        assert!((punched.unsigned_area() - 96.0).abs() < 1e-6);

        // TestKaBooleanPolygonEditor::testSliverCleaningUnder0_05m2:
        // Sliver interior hole of 0.2m x 0.1m = 0.02 m² (< 0.05 m²) -> removed (0 interior rings)
        let sliver_hole = [[1.0, 1.0], [1.2, 1.0], [1.2, 1.1], [1.0, 1.1], [1.0, 1.0]];
        let with_sliver = punch_interior_hole_polygon(&outer, &sliver_hole);
        assert_eq!(with_sliver.interiors().len(), 1);
        let cleaned = clean_sliver_interior_rings(&with_sliver, HGIS_MIN_SLIVER_AREA_M2);
        assert_eq!(cleaned.interiors().len(), 0);
        assert!((cleaned.unsigned_area() - 100.0).abs() < 1e-6);
    }

    #[test]
    fn test_perfect_affine_transformation_and_insufficient_gcp_error() {
        // TestKaGcpTransformSolver::testPerfectAffineTransformation
        let gcps = vec![
            GcpPointPair {
                source_pixel: [0.0, 0.0],
                target_world: [100.0, 200.0],
                is_enabled: true,
            },
            GcpPointPair {
                source_pixel: [100.0, 0.0],
                target_world: [200.0, 200.0],
                is_enabled: true,
            },
            GcpPointPair {
                source_pixel: [0.0, 100.0],
                target_world: [100.0, 300.0],
                is_enabled: true,
            },
            GcpPointPair {
                source_pixel: [100.0, 100.0],
                target_world: [200.0, 300.0],
                is_enabled: true,
            },
        ];
        let res = solve_gcp_transform(&gcps, TransformType::Affine6Param)
            .expect("4 non-collinear GCPs should succeed");
        assert!(res.is_success);
        let expected = [1.0, 0.0, 100.0, 0.0, 1.0, 200.0];
        for (got, exp) in res.transform_coeffs.iter().zip(expected.iter()) {
            assert!((got - exp).abs() < 1e-9);
        }
        assert!(res.rmse_m <= 1e-9);
        assert!(!res.exceeds_allowable_rmse);

        // TestKaGcpTransformSolver::testInsufficientGcpError
        let err = solve_gcp_transform(&gcps[..2], TransformType::Affine6Param);
        assert_eq!(err, Err(GcpSolverError::InsufficientGcpCount));
    }

    #[test]
    fn test_dbscan_cluster_formation_and_closed_traverse_precision() {
        // TestKaArtifactClusterAnalyzer::testDbscanClusterFormation:
        // 4 points within 0.5m radius + 1 isolated point 10m away
        let pts = vec![
            (1, [0.0, 0.0], "Pottery"),
            (2, [0.3, 0.0], "Pottery"),
            (3, [0.0, 0.3], "Stone"),
            (4, [0.3, 0.3], "Pottery"),
            (5, [10.0, 10.0], "Iron"),
        ];
        let (clusters, noise_count) = compute_dbscan_artifact_clusters(
            &pts,
            HGIS_DBSCAN_DEFAULT_EPSILON_M,
            HGIS_DBSCAN_DEFAULT_MIN_PTS,
        );
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].cluster_id, 0);
        assert_eq!(clusters[0].member_fids.len(), 4);
        assert_eq!(noise_count, 1);

        let co = compute_material_co_occurrence(
            &[vec!["Pottery", "Stone"], vec!["Pottery", "Iron"]],
            &["Pottery", "Stone", "Iron"],
        );
        assert_eq!(co.matrix[0][0], 2);
        assert_eq!(co.matrix[0][1], 1);
        assert_eq!(co.matrix[0][2], 1);

        // TestKaSurveyNetworkAdjuster::testClosedTraversePrecision:
        // 4 legs of 100.0m each (total 400.0m), net fx = +0.03m, fy = -0.04m -> fs = 0.05m, ratio = 1/8000
        let legs = vec![
            TraverseLeg {
                from_station: "CP-1".to_string(),
                to_station: "CP-2".to_string(),
                delta_x_m: 100.0,
                delta_y_m: 0.0,
                measured_distance_m: 100.0,
            },
            TraverseLeg {
                from_station: "CP-2".to_string(),
                to_station: "CP-3".to_string(),
                delta_x_m: 0.0,
                delta_y_m: 100.0,
                measured_distance_m: 100.0,
            },
            TraverseLeg {
                from_station: "CP-3".to_string(),
                to_station: "CP-4".to_string(),
                delta_x_m: -99.97,
                delta_y_m: 0.0,
                measured_distance_m: 100.0,
            },
            TraverseLeg {
                from_station: "CP-4".to_string(),
                to_station: "CP-1".to_string(),
                delta_x_m: 0.0,
                delta_y_m: -100.04,
                measured_distance_m: 100.0,
            },
        ];
        let adj = adjust_closed_traverse(&legs, [1000.0, 2000.0], 10.0);
        assert!((adj.linear_closing_error_m - 0.05).abs() < 1e-6);
        assert_eq!(adj.closing_ratio_denominator, 8000);
        assert!(adj.is_acceptable);
        let closed_end = adj.adjusted_coords.last().expect("adjusted end coord");
        assert!((closed_end[0] - 1000.0).abs() < 1e-9);
        assert!((closed_end[1] - 2000.0).abs() < 1e-9);

        // TestKaCadastralOverlapAnalyzer::testParcelOverlapAndRounding & testSliverFilteringUnder0_1m2
        let survey_ring = [
            [0.0, 0.0],
            [20.0, 0.0],
            [20.0, 25.0],
            [0.0, 25.0],
            [0.0, 0.0],
        ];
        let parcels = vec![
            CadastralParcelInput {
                pnu: "4713010300101050001".to_string(),
                jibun: "105-1 대".to_string(),
                jimok: "대".to_string(),
                official_area_m2: 1000.0,
                exterior_ring: vec![
                    [0.0, 0.0],
                    [40.0, 0.0],
                    [40.0, 25.0],
                    [0.0, 25.0],
                    [0.0, 0.0],
                ],
            },
            // Sliver parcel with 0.2m x 0.2m = 0.04 m² overlap (< 0.1 m²)
            CadastralParcelInput {
                pnu: "4713010300101050002".to_string(),
                jibun: "105-2 전".to_string(),
                jimok: "전".to_string(),
                official_area_m2: 500.0,
                exterior_ring: vec![
                    [19.8, 0.0],
                    [30.0, 0.0],
                    [30.0, 0.2],
                    [19.8, 0.2],
                    [19.8, 0.0],
                ],
            },
        ];
        let summary =
            analyze_cadastral_inclusion(&survey_ring, &parcels, HGIS_MIN_PARCEL_OVERLAP_M2);
        assert_eq!(summary.total_parcels_count, 1);
        assert_eq!(summary.parcel_records.len(), 1);
        let rec = &summary.parcel_records[0];
        assert_eq!(rec.jibun, "105-1 대");
        assert!((rec.overlap_area_m2 - 500.0).abs() < 1e-6);
        assert!((rec.inclusion_ratio_pct - 50.0).abs() < 1e-6);
    }

    #[test]
    fn test_rmse_grade_evaluation_and_subpixel_centering_on_crosshair() {
        // TestGeorefQuality::testRmseGradeEvaluation:
        // 0.08m -> GradeA, 0.18m -> GradeB, 0.35m -> GradeC, 0.65m -> GradeD
        assert_eq!(
            evaluate_rmse_grade(0.08),
            GeorefQualityGrade::GradeAExcellent
        );
        assert_eq!(evaluate_rmse_grade(0.18), GeorefQualityGrade::GradeBGood);
        assert_eq!(evaluate_rmse_grade(0.35), GeorefQualityGrade::GradeCCaution);
        assert_eq!(
            evaluate_rmse_grade(0.65),
            GeorefQualityGrade::GradeDCritical
        );
        assert!(
            georef_quality_summary_ko(2, 0.0, 0.0).contains("한 점 더 찍으면 기울기도 맞습니다")
        );
        assert!(
            georef_quality_summary_ko(4, 0.08, 0.12).contains("평균 어긋남 0.08 m · 최대 0.12 m")
        );

        // TestGeorefPixelWindow::testSubPixelCenteringOnCrosshair:
        // 15x15 white background (255) with a dark crosshair centered at offset (+1, -1) from center (7, 7), i.e., col=8, row=6
        let mut patch = vec![255_u8; 15 * 15];
        for i in 0..15 {
            patch[6 * 15 + i] = 20;
            patch[i * 15 + 8] = 20;
        }
        patch[6 * 15 + 8] = 0;
        let sub = find_subpixel_crosshair_center_15x15(&patch, 100, 200);
        assert!(sub.is_confident);
        assert!((sub.refined_u - 101.0).abs() <= 0.15);
        assert!((sub.refined_v - 199.0).abs() <= 0.15);

        // Flat low-contrast patch falls back to integer click
        let flat = vec![200_u8; 15 * 15];
        let fallback = find_subpixel_crosshair_center_15x15(&flat, 100, 200);
        assert!(!fallback.is_confident);
        assert!((fallback.refined_u - 100.0).abs() < 1e-9);
        assert!((fallback.refined_v - 200.0).abs() < 1e-9);

        let mut taken = std::collections::BTreeSet::new();
        let base = std::path::PathBuf::from("/tmp/plan_georef.tif");
        taken.insert(base.clone());
        let uniq = unique_georef_output_path(&base, &taken);
        assert!(uniq.ends_with("plan_georef_2.tif"));

        // TestSurveyScopeClip::testLineIntersectionClipping:
        // 100m x 100m survey box [0, 0, 100, 100], contour line (-50, 50) -> (150, 50) (200m total) -> clipped length = 100.0m
        let (c0, c1) =
            clip_line_segment_by_bbox([-50.0, 50.0], [150.0, 50.0], [0.0, 0.0, 100.0, 100.0])
                .expect("line intersects survey box");
        let clipped_len = (c1[0] - c0[0]).hypot(c1[1] - c0[1]);
        assert!((clipped_len - 100.0).abs() <= 1e-4);
    }
}
