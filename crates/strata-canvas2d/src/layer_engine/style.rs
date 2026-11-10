//! 도메인 레이어별 기본 스타일 표 — C++ `LayerStyleDefaults.cpp:42-78` 1:1.
//!
//! C++ 은 QGIS 의 `QgsSymbol` 을 만들 때 이 표를 쓴다. Rust 에는 QGIS 가 없으므로
//! 표만 그대로 옮기고, 실제 그리기는 [`super::symbol`] 이 한다.
//!
//! 이 표가 없으면 색이 그리는 코드에 흩어져 박힌다(실제로 그랬다).
//! 새 레이어를 추가할 때는 **이 표에 한 줄만 넣으면** 된다.

/// 한 레이어의 그리기 스타일. C++ `LayerStyleDefaults::DomainStyle`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DomainStyle {
    /// 채움색 RGBA. 알파가 0 이면 채우지 않는다.
    pub fill: [u8; 4],
    /// 선·테두리 색 RGBA.
    pub stroke: [u8; 4],
    /// 선 굵기(mm). C++ `strokeWidthMm` 기본 1.2.
    pub stroke_width_mm: f32,
    /// 점 크기(mm). C++ `markerSizeMm` 기본 3.5.
    pub marker_size_mm: f32,
    /// 흰 테두리(케이싱)를 선 아래에 깔지. `section_line` 만 참.
    pub casing: bool,
    /// 케이싱 색.
    pub casing_color: [u8; 4],
    /// 케이싱 굵기(mm).
    pub casing_width_mm: f32,
}

impl Default for DomainStyle {
    /// C++ 기본값 = 일반 파랑 (`QColor(37,99,235,90)` / `(37,99,235,255)`).
    fn default() -> Self {
        Self {
            fill: [37, 99, 235, 90],
            stroke: [37, 99, 235, 255],
            stroke_width_mm: 1.2,
            marker_size_mm: 3.5,
            casing: false,
            casing_color: [255, 255, 255, 255],
            casing_width_mm: 0.0,
        }
    }
}

impl DomainStyle {
    /// 레이어 키에 맞는 기본 스타일. C++ `LayerStyleDefaults::forKey` 와 값이 같다.
    ///
    /// 모르는 키는 일반 파랑(기본값)을 돌려준다 — C++ 과 같은 동작.
    #[must_use]
    pub fn for_key(key: &str) -> Self {
        let mut s = Self::default();
        match key {
            "survey_area" => {
                s.fill = [180, 83, 9, 70];
                s.stroke = [146, 64, 14, 255];
                s.stroke_width_mm = 1.6;
            }
            "feature_poly" => {
                s.fill = [22, 163, 74, 90];
                s.stroke = [17, 94, 44, 255];
                s.stroke_width_mm = 1.8;
            }
            "feature_line" => {
                // 선 레이어는 채우지 않는다(C++ 도 fill 을 건드리지 않고 기본값이 쓰이지만
                // 선 심볼은 fill 을 보지 않으므로 결과가 같다).
                s.stroke = [202, 138, 4, 255];
                s.stroke_width_mm = 1.8;
            }
            "section_line" => {
                s.stroke = [190, 24, 93, 255];
                s.stroke_width_mm = 1.8;
                // 위성·지적 위에서도 절단선이 읽히도록 흰 케이싱을 깐다.
                s.casing = true;
                s.casing_color = [255, 255, 255, 255];
                s.casing_width_mm = 3.0;
            }
            "control_points" => {
                s.fill = [234, 179, 8, 255];
                s.stroke = [161, 98, 7, 255];
                s.marker_size_mm = 4.0;
            }
            "artifact_point" => {
                s.fill = [185, 28, 28, 255];
                s.stroke = [127, 29, 29, 255];
                s.marker_size_mm = 3.6;
            }
            "trial_trench" => {
                // 시굴 트렌치: 붉은 윤곽선 0.5mm, 채움은 거의 없음
                // (위성·지적 위에서 조사 구역이 보이도록).
                s.fill = [220, 38, 38, 18];
                s.stroke = [220, 38, 38, 255];
                s.stroke_width_mm = 0.5;
            }
            _ => {}
        }
        s
    }

    /// 채움색을 그릴지. 알파가 0 이면 그리지 않는다.
    #[must_use]
    pub fn fills(&self) -> bool {
        self.fill[3] > 0
    }

    /// 선을 그릴지. 굵기가 0 이면 그리지 않는다.
    #[must_use]
    pub fn strokes(&self) -> bool {
        self.stroke_width_mm > 0.0 && self.stroke[3] > 0
    }

    /// 케이싱을 깔지.
    #[must_use]
    pub fn use_casing(&self) -> bool {
        self.casing && self.casing_width_mm > 0.0 && self.stroke_width_mm > 0.0
    }
}

/// 도메인 레이어 키 전체(C++ `LayerOps::domainLayerKeys()`).
pub const DOMAIN_LAYER_KEYS: [&str; 7] = [
    "survey_area",
    "feature_poly",
    "feature_line",
    "section_line",
    "control_points",
    "artifact_point",
    "trial_trench",
];

/// 레이어 키 → 사람이 읽는 이름(범례·목록용).
#[must_use]
pub fn label_ko(key: &str) -> &'static str {
    match key {
        "survey_area" => "조사구역",
        "feature_poly" => "유구(면)",
        "feature_line" => "유구(선)",
        "section_line" => "단면선",
        "control_points" => "기준점",
        "artifact_point" => "유물",
        "trial_trench" => "시굴격자",
        _ => "레이어",
    }
}
