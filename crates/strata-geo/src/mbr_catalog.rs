//! 유구 MBR 지표·방위 표기, 자연 정렬, 빈 값 표기.
//!
//! C++ 대응은 각 항목 주석의 원본 함수명을 따른다.

use geo::{Coord, LineString, Polygon};

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
