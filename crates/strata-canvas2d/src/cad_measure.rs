//! 측정값(거리·면적·평·방위)과 도면 좌표 콜아웃.
//!
//! `cad_interaction` 에서 분리했다.

#[derive(Debug, Clone, PartialEq)]
pub struct MeasureReadout {
    pub total_distance_m: f64,
    pub polygon_area_m2: f64,
    /// 찍은 순서대로 이은 다각형의 둘레 — `KaMeasureMapTool.cpp:369,399` 「둘레」
    pub polygon_perimeter_m: f64,
}

pub const HGIS_METERS_TO_KM_THRESHOLD: f64 = 1000.0;

pub const HGIS_SQMETERS_TO_HA_THRESHOLD: f64 = 10000.0;

/// C++ `KaMeasureTool::SegmentMetrics` — 선분 거리(m) 및 북기준 시계방향 방위각(0° ~ 360°)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SegmentMetrics {
    pub distance_m: f64,
    pub azimuth_deg: f64,
}

/// C++ `KaMeasureTool::computeSegmentMetrics` — 북쪽 0.0°, 동쪽 90.0°, 남쪽 180.0°, 서쪽 270.0° 시계방향 방위각 계산
#[must_use]
pub fn compute_segment_metrics(start_pt: [f64; 2], end_pt: [f64; 2]) -> SegmentMetrics {
    let dx = end_pt[0] - start_pt[0];
    let dy = end_pt[1] - start_pt[1];
    let distance_m = dx.hypot(dy);
    if distance_m < 1e-12 {
        return SegmentMetrics {
            distance_m: 0.0,
            azimuth_deg: 0.0,
        };
    }
    let theta_deg = dy.atan2(dx).to_degrees();
    let azimuth_deg = (90.0 - theta_deg).rem_euclid(360.0);
    SegmentMetrics {
        distance_m,
        azimuth_deg,
    }
}

/// C++ `KaMeasureTool` 거리 포맷 — `< 1000.0m`는 `"850.50 m"`, `>= 1000.0m`는 `"1.50 km"`
#[must_use]
pub fn format_measure_distance(distance_m: f64) -> String {
    if distance_m >= HGIS_METERS_TO_KM_THRESHOLD {
        format!("{:.2} km", distance_m / HGIS_METERS_TO_KM_THRESHOLD)
    } else {
        format!("{distance_m:.2} m")
    }
}

/// C++ `KaMeasureTool` 면적 포맷 — `< 10000.0m²`는 `"200.00 ㎡"`, `>= 10000.0m²`는 `"2.50 ha"`
#[must_use]
pub fn format_measure_area(area_m2: f64) -> String {
    if area_m2 >= HGIS_SQMETERS_TO_HA_THRESHOLD {
        format!("{:.2} ha", area_m2 / HGIS_SQMETERS_TO_HA_THRESHOLD)
    } else {
        format!("{area_m2:.2} ㎡")
    }
}

/// 1평 환산 제곱미터 계수 (`3.305785`, C++ `HGIS_PYEONG_CONVERSION_FACTOR`)
pub const HGIS_PYEONG_CONVERSION_FACTOR: f64 = 3.305_785;

/// 좌표 콜아웃 말풍선 모서리 반경 (`2.0 mm`, C++ `HGIS_CALLOUT_CORNER_RADIUS_MM`)
pub const HGIS_CALLOUT_CORNER_RADIUS_MM: f64 = 2.0;

/// 슈레이스 공식(Shoelace Formula)으로 다각형 면적(`㎡`) 및 한국 고고학 발굴 평수(`㎡ / 3.305785`) 산출
/// (C++ `MeasureOps::calculatePolygonAreaM2` 1:1 대응)
#[must_use]
pub fn calculate_polygon_area_and_pyeong(points: &[[f64; 2]]) -> (f64, f64) {
    if points.len() < 3 {
        return (0.0, 0.0);
    }
    let mut shoelace = 0.0_f64;
    for i in 0..points.len() {
        let j = (i + 1) % points.len();
        shoelace += points[i][0] * points[j][1] - points[j][0] * points[i][1];
    }
    let area_m2 = 0.5 * shoelace.abs();
    let pyeong = area_m2 / HGIS_PYEONG_CONVERSION_FACTOR;
    (area_m2, pyeong)
}

/// 방위각(`degrees`)을 도·분·초(`D°MM′SS″`) 문자열로 포맷팅
/// (C++ `MeasureOps::formatBearing` 1:1 대응)
#[must_use]
pub fn format_bearing_dms(degrees: f64) -> String {
    if !degrees.is_finite() {
        return "—".to_owned();
    }
    let total = (degrees * 3600.0).round() as i64;
    let wrapped = total.rem_euclid(360 * 3600);
    let d = wrapped / 3600;
    let m = (wrapped % 3600) / 60;
    let s = wrapped % 60;
    format!("{d}°{m:02}′{s:02}″")
}

/// 방위각 다이얼 위젯 한국어 안내 문구 생성
/// (C++ `KaAzimuthDial::note` 1:1 대응)
#[must_use]
pub fn azimuth_dial_note_ko(degrees: f64) -> String {
    let turn = degrees.rem_euclid(360.0);
    if turn.abs() < 0.5 || (turn - 360.0).abs() < 0.5 {
        "위가 북쪽입니다".to_owned()
    } else {
        format!("북에서 시계 방향 {:.0}°", turn)
    }
}

/// 도면 좌표 인출선 콜아웃 문자열(`X: ...\nY: ...\nEL: ...`) 생성
/// (C++ `KaDrawingCoordCallout::drawCoordCallout` 1:1 대응)
#[must_use]
pub fn format_drawing_coord_callout(world_xy: [f64; 2], ground_z: f64) -> String {
    fn fmt_with_commas(v: f64) -> String {
        let sign = if v < 0.0 { "-" } else { "" };
        let abs = v.abs();
        let whole = abs.trunc() as u64;
        let frac = ((abs - (whole as f64)) * 100.0).round() as u64;
        let s = whole.to_string();
        let mut out = String::new();
        for (i, ch) in s.chars().rev().enumerate() {
            if i > 0 && i % 3 == 0 {
                out.push(',');
            }
            out.push(ch);
        }
        let whole_comma: String = out.chars().rev().collect();
        format!("{sign}{whole_comma}.{frac:02}")
    }
    format!(
        "X: {}m\nY: {}m\nEL: {:.2}m",
        fmt_with_commas(world_xy[0]),
        fmt_with_commas(world_xy[1]),
        ground_z
    )
}
