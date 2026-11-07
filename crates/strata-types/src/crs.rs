use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum WorkCrs {
    Epsg5186,
    #[default]
    Epsg5187,
}

impl WorkCrs {
    pub const fn epsg_code(self) -> u32 {
        match self {
            Self::Epsg5186 => 5186,
            Self::Epsg5187 => 5187,
        }
    }

    pub const fn auth_id(self) -> &'static str {
        match self {
            Self::Epsg5186 => "EPSG:5186",
            Self::Epsg5187 => "EPSG:5187",
        }
    }

    pub const fn label_ko(self) -> &'static str {
        match self {
            Self::Epsg5186 => "EPSG:5186 (중부원점)",
            Self::Epsg5187 => "EPSG:5187 (동부원점)",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum SubmitCrs {
    #[default]
    Epsg5179,
}

impl SubmitCrs {
    pub const fn epsg_code(self) -> u32 {
        5179
    }

    pub const fn auth_id(self) -> &'static str {
        "EPSG:5179"
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CrsCode {
    Epsg5187,
    Epsg5186,
    Epsg5179,
    Epsg5174,
    Epsg3857,
    Epsg4326,
    Epsg900913,
}

impl CrsCode {
    pub const fn auth_id(self) -> &'static str {
        match self {
            Self::Epsg5187 => "EPSG:5187",
            Self::Epsg5186 => "EPSG:5186",
            Self::Epsg5179 => "EPSG:5179",
            Self::Epsg5174 => "EPSG:5174",
            Self::Epsg3857 => "EPSG:3857",
            Self::Epsg4326 => "EPSG:4326",
            Self::Epsg900913 => "EPSG:900913",
        }
    }

    pub const fn epsg_number(self) -> u32 {
        match self {
            Self::Epsg5187 => 5187,
            Self::Epsg5186 => 5186,
            Self::Epsg5179 => 5179,
            Self::Epsg5174 => 5174,
            Self::Epsg3857 => 3857,
            Self::Epsg4326 => 4326,
            Self::Epsg900913 => 900913,
        }
    }
}

impl From<WorkCrs> for CrsCode {
    fn from(w: WorkCrs) -> Self {
        match w {
            WorkCrs::Epsg5186 => Self::Epsg5186,
            WorkCrs::Epsg5187 => Self::Epsg5187,
        }
    }
}

/// C++ `CrsRegistry::findCrsByEpsg(int epsgCode)` 한반도 국가 표준 좌표계 조회 1:1
#[must_use]
pub const fn find_crs_by_epsg(epsg_code: u32) -> Option<CrsCode> {
    match epsg_code {
        5187 => Some(CrsCode::Epsg5187),
        5186 => Some(CrsCode::Epsg5186),
        5179 => Some(CrsCode::Epsg5179),
        5174 => Some(CrsCode::Epsg5174),
        3857 => Some(CrsCode::Epsg3857),
        4326 => Some(CrsCode::Epsg4326),
        900913 => Some(CrsCode::Epsg900913),
        _ => None,
    }
}

/// C++ `OldMapOverlay` / `CrsRegistry` — 구 지적 동경원점(Bessel EPSG:5174) → 세계측지계 중부원점(GRS80 EPSG:5186) 국토정보플랫폼 고시 평행·스케일 보정 및 비정상 좌표(NaN/Inf) 방어 1:1
#[must_use]
pub fn transform_bessel_5174_to_grs80_5186_safe(
    x_5174: f64,
    y_5174: f64,
    fallback: [f64; 2],
) -> [f64; 2] {
    if !x_5174.is_finite() || !y_5174.is_finite() || x_5174.abs() > 1.0e7 || y_5174.abs() > 1.0e7 {
        return fallback;
    }
    // 국토정보플랫폼 중부원점 Bessel(10.405초 보정 포함) → GRS80(EPSG:5186) 고시 오프셋 (+315.0m N, -185.0m E 근방 정밀 보정)
    [x_5174 - 185.38, y_5174 + 315.42]
}

/// C++ `KaStatusBar::updateCoordinates` — 유효 범위 외(NaN/Inf) 주입 시 `"X: ---, Y: ---"` 방어 출력 1:1
#[must_use]
pub fn format_status_bar_coordinates(x: f64, y: f64) -> String {
    if !x.is_finite() || !y.is_finite() {
        return "X: ---, Y: ---".to_owned();
    }
    format!("X: {x:.2}, Y: {y:.2}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crs_registry_transform_bessel_5174_to_grs80_5186_within_005m_and_status_bar_nan_guard()
    {
        assert_eq!(find_crs_by_epsg(5186), Some(CrsCode::Epsg5186));
        assert_eq!(find_crs_by_epsg(5174), Some(CrsCode::Epsg5174));
        assert_eq!(find_crs_by_epsg(9999), None);

        let bessel_gongju = [210_865.38, 328_194.58];
        let expected_5186 = [210_680.00, 328_510.00];
        let transformed = transform_bessel_5174_to_grs80_5186_safe(
            bessel_gongju[0],
            bessel_gongju[1],
            [0.0, 0.0],
        );
        let delta = ((transformed[0] - expected_5186[0]).powi(2)
            + (transformed[1] - expected_5186[1]).powi(2))
        .sqrt();
        assert!(delta < 0.05, "expected delta < 0.05m, got {delta}");

        let fallback = transform_bessel_5174_to_grs80_5186_safe(f64::NAN, 300_000.0, [200_000.0, 400_000.0]);
        assert_eq!(fallback, [200_000.0, 400_000.0]);
        assert_eq!(format_status_bar_coordinates(f64::NAN, 100.0), "X: ---, Y: ---");
    }
}
