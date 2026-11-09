//! 기하·속성 규칙 검사(R04·R06·R07·R08·R10·R11·R18·R20·R21·R22).
//!
//! `impl ChecklistEngine` 이 697줄이라 여기로 나눴다.

use crate::check_engine::ChecklistEngine;
use crate::check_rules::ChecklistSeverity;

impl ChecklistEngine {

    /// C++ `ChecklistRules::evaluateRule` — R06 최소 면적(`#define HGIS_AREA_EPSILON 0.05` m² 미달 시 `Error`) 1:1
    #[must_use]
    pub fn check_r06_min_polygon_area(area_m2: f64) -> ChecklistSeverity {
        const HGIS_AREA_EPSILON: f64 = 0.05;
        if area_m2 < HGIS_AREA_EPSILON {
            ChecklistSeverity::Error
        } else {
            ChecklistSeverity::Pass
        }
    }

    /// C++ `ChecklistRules::evaluateRule` — R07 등경분할비 슬리버 비율(`4 * PI * Area / P^2 < 0.015` 시 `Warning`) 1:1
    #[must_use]
    pub fn check_r07_sliver_thinness(area_m2: f64, perimeter_m: f64) -> ChecklistSeverity {
        const HGIS_SLIVER_THINNESS: f64 = 0.015;
        if perimeter_m <= 1e-9 {
            return ChecklistSeverity::Warning;
        }
        let ratio = (4.0 * std::f64::consts::PI * area_m2) / (perimeter_m * perimeter_m);
        if ratio < HGIS_SLIVER_THINNESS {
            ChecklistSeverity::Warning
        } else {
            ChecklistSeverity::Pass
        }
    }

    /// C++ `ChecklistRules::evaluateRule` — R18 유구 번호 정규식(`^(주거|수혈|분묘|토광|수구|석축|구|기타)[0-9]+(-[0-9]+)?$`) 검증 1:1
    #[must_use]
    pub fn check_r18_feature_no_format(feature_no: &str) -> ChecklistSeverity {
        let trimmed = feature_no.trim();
        let prefixes = ["주거", "수혈", "분묘", "토광", "수구", "석축", "구", "기타"];
        let Some(rest) = prefixes.iter().find_map(|p| trimmed.strip_prefix(p)) else {
            return ChecklistSeverity::Error;
        };
        if rest.is_empty() {
            return ChecklistSeverity::Error;
        }
        let valid_digits = if let Some((main_num, sub_num)) = rest.split_once('-') {
            !main_num.is_empty()
                && main_num.chars().all(|c| c.is_ascii_digit())
                && !sub_num.is_empty()
                && sub_num.chars().all(|c| c.is_ascii_digit())
        } else {
            rest.chars().all(|c| c.is_ascii_digit())
        };
        if valid_digits {
            ChecklistSeverity::Pass
        } else {
            ChecklistSeverity::Error
        }
    }

    /// C++ `ChecklistRules::evaluateRule` — R20 국가 표준 시대 코드 검증 1:1
    #[must_use]
    pub fn check_r20_period_code_valid(period: &str) -> ChecklistSeverity {
        const VALID_PERIODS: [&str; 11] = [
            "구석기",
            "신석기",
            "청동기",
            "초기철기",
            "원삼국",
            "삼국",
            "통일신라",
            "고려",
            "조선",
            "근현대",
            "미상",
        ];
        if VALID_PERIODS.contains(&period.trim()) {
            ChecklistSeverity::Pass
        } else {
            ChecklistSeverity::Error
        }
    }

    /// C++ `ChecklistRules::evaluateRule` — R22 조사 일자(`YYYY-MM-DD`) 형식 검증 (`Warning`) 1:1
    #[must_use]
    pub fn check_r22_date_format_valid(date_str: &str) -> ChecklistSeverity {
        let parts: Vec<&str> = date_str.trim().split('-').collect();
        if parts.len() != 3 || parts[0].len() != 4 || parts[1].len() != 2 || parts[2].len() != 2 {
            return ChecklistSeverity::Warning;
        }
        let Ok(y) = parts[0].parse::<u32>() else {
            return ChecklistSeverity::Warning;
        };
        let Ok(m) = parts[1].parse::<u32>() else {
            return ChecklistSeverity::Warning;
        };
        let Ok(d) = parts[2].parse::<u32>() else {
            return ChecklistSeverity::Warning;
        };
        if (1900..=2100).contains(&y) && (1..=12).contains(&m) && (1..=31).contains(&d) {
            ChecklistSeverity::Pass
        } else {
            ChecklistSeverity::Warning
        }
    }

    pub const HGIS_MIN_ERROR_ZOOM_MARGIN: f64 = 20.0;

    pub const HGIS_RING_CLOSE_EPSILON: f64 = 1.0e-6;

    pub const HGIS_DUPLICATE_DIST_EPSILON: f64 = 0.001;

    pub const HGIS_DUPLICATE_AREA_EPSILON: f64 = 0.001;

    pub const HGIS_SPIKE_MIN_ANGLE_DEG: f64 = 2.0;

    pub const HGIS_MIN_CONTROL_POINTS: usize = 2;

    /// C++ `KaSubmitFlow::zoomToErrorLocation` — 점/미세 슬리버 오류로 이동 시 최소 20.0m 여백 BBOX(`40m x 40m`) 확장 공식 1:1
    #[must_use]
    pub fn compute_error_zoom_bbox(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> [f64; 4] {
        let cx = (min_x + max_x) * 0.5;
        let cy = (min_y + max_y) * 0.5;
        let half_w = ((max_x - min_x).abs() * 0.5 * 1.5).max(Self::HGIS_MIN_ERROR_ZOOM_MARGIN);
        let half_h = ((max_y - min_y).abs() * 0.5 * 1.5).max(Self::HGIS_MIN_ERROR_ZOOM_MARGIN);
        [cx - half_w, cy - half_h, cx + half_w, cy + half_h]
    }

    /// C++ `TopologyChecklistEngine` — R04 폐합 링 시작점=끝점 일치 검사 (`d > 1e-6` 시 `Error`) 1:1
    #[must_use]
    pub fn check_r04_ring_closure(
        ring: &[[f64; 2]],
    ) -> (ChecklistSeverity, Option<[f64; 2]>, String) {
        if ring.len() < 4 {
            return (
                ChecklistSeverity::Error,
                ring.last().copied(),
                "폴리곤 링 정점 수가 최소 4개 미만입니다.".to_owned(),
            );
        }
        let first = ring[0];
        let last = ring[ring.len() - 1];
        let d = (last[0] - first[0]).hypot(last[1] - first[1]);
        if d > Self::HGIS_RING_CLOSE_EPSILON {
            (
                ChecklistSeverity::Error,
                Some(last),
                format!(
                    "폴리곤 링이 폐합되지 않았습니다 (시작점-끝점 이격 거리: {:.2} mm)",
                    d * 1000.0
                ),
            )
        } else {
            (ChecklistSeverity::Pass, None, String::new())
        }
    }

    /// C++ `TopologyChecklistEngine` — R08 자가교차(Self-Intersection) 검출 및 교차점 좌표 반환 1:1
    #[must_use]
    pub fn check_r08_self_intersection(
        ring: &[[f64; 2]],
    ) -> (ChecklistSeverity, Option<[f64; 2]>, String) {
        if ring.len() < 4 {
            return (ChecklistSeverity::Pass, None, String::new());
        }
        let seg_count = ring.len() - 1;
        for i in 0..seg_count {
            let a = ring[i];
            let b = ring[i + 1];
            for j in (i + 2)..seg_count {
                // 폐합 링의 첫 선분과 마지막 선분은 시작점=끝점을 공유하므로 제외
                if i == 0 && j + 1 == seg_count {
                    continue;
                }
                let c = ring[j];
                let d = ring[j + 1];
                if let Some(ix) = Self::segment_proper_intersection(a, b, c, d) {
                    return (
                        ChecklistSeverity::Error,
                        Some(ix),
                        "폴리곤 경계선이 자가교차(Self-intersection)합니다.".to_owned(),
                    );
                }
            }
        }
        (ChecklistSeverity::Pass, None, String::new())
    }

    fn segment_proper_intersection(
        a: [f64; 2],
        b: [f64; 2],
        c: [f64; 2],
        d: [f64; 2],
    ) -> Option<[f64; 2]> {
        let r = [b[0] - a[0], b[1] - a[1]];
        let s = [d[0] - c[0], d[1] - c[1]];
        let denom = r[0] * s[1] - r[1] * s[0];
        if denom.abs() < 1e-12 {
            return None;
        }
        let qp = [c[0] - a[0], c[1] - a[1]];
        let t = (qp[0] * s[1] - qp[1] * s[0]) / denom;
        let u = (qp[0] * r[1] - qp[1] * r[0]) / denom;
        if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
            Some([a[0] + t * r[0], a[1] + t * r[1]])
        } else {
            None
        }
    }

    /// C++ `TopologyChecklistEngine` — R10 중복 피처(중심점 거리 `< 1mm` & 면적 차이 `< 0.001 m²`) 검출 1:1
    #[must_use]
    pub fn check_r10_duplicate_feature(
        centroid_a: [f64; 2],
        area_a: f64,
        centroid_b: [f64; 2],
        area_b: f64,
        matched_fid: u64,
    ) -> (ChecklistSeverity, Option<[f64; 2]>, String) {
        let dist = (centroid_a[0] - centroid_b[0]).hypot(centroid_a[1] - centroid_b[1]);
        let area_diff = (area_a - area_b).abs();
        if dist < Self::HGIS_DUPLICATE_DIST_EPSILON && area_diff < Self::HGIS_DUPLICATE_AREA_EPSILON
        {
            (
                ChecklistSeverity::Error,
                Some(centroid_a),
                format!(
                    "피처 FID {matched_fid}과(와) 형상 및 위치가 완전히 일치하는 중복 피처입니다."
                ),
            )
        } else {
            (ChecklistSeverity::Pass, None, String::new())
        }
    }

    /// C++ `TopologyChecklistEngine` — R11 뾰족점(Spike, 내각 `< 2.0°`) 검출 및 `acos` 클램핑(`[-1.0, 1.0]`) 1:1
    #[must_use]
    pub fn check_r11_spike_vertex(
        ring: &[[f64; 2]],
    ) -> (ChecklistSeverity, Option<[f64; 2]>, String) {
        let is_closed = ring.len() >= 4
            && (ring[0][0] - ring[ring.len() - 1][0]).hypot(ring[0][1] - ring[ring.len() - 1][1])
                <= Self::HGIS_RING_CLOSE_EPSILON;
        let unique_len = if is_closed {
            ring.len() - 1
        } else {
            ring.len()
        };
        if unique_len < 3 {
            return (ChecklistSeverity::Pass, None, String::new());
        }
        for i in 0..unique_len {
            let prev = ring[(i + unique_len - 1) % unique_len];
            let curr = ring[i];
            let next = ring[(i + 1) % unique_len];
            let v1x = prev[0] - curr[0];
            let v1y = prev[1] - curr[1];
            let v2x = next[0] - curr[0];
            let v2y = next[1] - curr[1];
            let len1 = v1x.hypot(v1y);
            let len2 = v2x.hypot(v2y);
            if len1 < 1e-9 || len2 < 1e-9 {
                continue;
            }
            let dot = ((v1x * v2x + v1y * v2y) / (len1 * len2)).clamp(-1.0, 1.0);
            let angle_deg = dot.acos().to_degrees();
            if angle_deg < Self::HGIS_SPIKE_MIN_ANGLE_DEG {
                return (
                    ChecklistSeverity::Warning,
                    Some(curr),
                    format!("정점 {i}번의 사잇각이 {angle_deg:.2}도로 침상형(Spike) 돌출입니다."),
                );
            }
        }
        (ChecklistSeverity::Pass, None, String::new())
    }

    /// C++ `TopologyChecklistEngine` — R21 기준점(ControlPoints) 최소 2개 이상 존재 검사 1:1
    #[must_use]
    pub fn check_r21_control_points_count(
        gcp_points: &[[f64; 2]],
        survey_centroid: [f64; 2],
    ) -> (ChecklistSeverity, Option<[f64; 2]>, String) {
        if gcp_points.len() < Self::HGIS_MIN_CONTROL_POINTS {
            let target = gcp_points.first().copied().unwrap_or(survey_centroid);
            (
                ChecklistSeverity::Error,
                Some(target),
                format!(
                    "도면 정합 기준점이 최소 2개 이상 필요합니다 (현재: {}개)",
                    gcp_points.len()
                ),
            )
        } else {
            (ChecklistSeverity::Pass, None, String::new())
        }
    }
}
