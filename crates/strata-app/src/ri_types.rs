//! 오른쪽 검사기의 타입·상수 정의.
//!
//! `right_inspector.rs` 가 300줄을 넘어 나눴다.
//! 기존 `crate::right_inspector::<이름>` 경로는 facade 재수출로 유지된다.




/// C++ `KaStylePalette.h` 8대 표준 제도 색상 (빨, 주, 노, 초, 파, 남, 보, 갈)
pub const KA_STYLE_PALETTE: [(u8, u8, u8, &str); 8] = [
    (0xDC, 0x26, 0x26, "빨강"),
    (0xEA, 0x58, 0x0C, "주황"),
    (0xCA, 0x8A, 0x04, "노랑"),
    (0x16, 0xA3, 0x4A, "초록"),
    (0x25, 0x63, 0xEB, "파랑"),
    (0x1E, 0x3A, 0x8A, "남색"),
    (0x7C, 0x3A, 0xED, "보라"),
    (0x92, 0x40, 0x0E, "갈색"),
];

pub const HGIS_DEFAULT_ELEVATION_VAL: f64 = 0.0;
pub const HGIS_MIN_ELEVATION_M: f64 = -500.0;
pub const HGIS_MAX_ELEVATION_M: f64 = 9000.0;

pub const HGIS_STANDARD_PERIOD_CODES: [&str; 11] = [
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttributeSaveResult {
    Success,
    MissingRequiredField,
    InvalidNumericElevation,
}

/// C++ `FeaturePresets.cpp:19-44` — 기본 유구 종류 8종
pub const KA_BUILTIN_KINDS: [&str; 8] = [
    "주거지",
    "수혈",
    "구",
    "분묘",
    "주초",
    "가마",
    "수전",
    "기타",
];

/// C++ `FeaturePresets.cpp:19-44` — 기본 시대 10종과 시대 띠 색
/// 시대 띠 색 (r,g,b). None 이면 색 없음(미상).
pub type KaPeriodInk = Option<(u8, u8, u8)>;

pub const KA_BUILTIN_PERIODS: [(&str, KaPeriodInk); 10] = [
    ("구석기", Some((0x53, 0x29, 0x1B))),
    ("신석기", Some((0x67, 0x40, 0x28))),
    ("청동기", Some((0x7A, 0x57, 0x37))),
    ("초기철기", Some((0x8B, 0x70, 0x48))),
    ("원삼국", Some((0x9C, 0x8A, 0x5B))),
    ("삼국", Some((0xAB, 0xA5, 0x71))),
    ("통일신라", Some((0xBA, 0xC0, 0x8A))),
    ("고려", Some((0xC7, 0xDC, 0xA6))),
    ("조선", Some((0xD3, 0xF8, 0xC4))),
    ("미상", None),
];

/// C++ `KaFeatureCardEditors.cpp:157-170` — "다음 번호" 제안.
#[must_use]
pub fn next_feature_number(current: &str) -> String {
    let digits: String = current.chars().take_while(|c| c.is_ascii_digit()).collect();
    let rest = &current[digits.len()..];
    let n = digits.parse::<u32>().unwrap_or(1);
    format!("{}{}", n + 1, rest)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchUpdateResult {
    pub updated_count: usize,
    pub failed_count: usize,
    pub failed_fids: Vec<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InspectorCardTab {
    #[default]
    FeatureAttributeCard,
    LayerStyleCard,
    BasemapQuickCard,
}

#[derive(Debug, Clone, Default)]
pub struct RightInspectorState {
    pub active_card: InspectorCardTab,
    pub selected_layer_key: Option<String>,
    pub selected_feature_id: Option<i64>,
    pub survey_name: String,
    pub feature_no: String,
    pub feature_type: String,
    pub feature_period: String,
    pub feature_memo: String,
    pub survey_card_modal_open: bool,
    pub style_color_index: usize,
    pub no_fill: bool,
    pub no_stroke: bool,
    pub stroke_width_mm: f32,
    pub fill_opacity: f32,
    pub show_draw_tooltip: bool,
    pub left_panel_collapsed: bool,
    pub right_panel_collapsed: bool,
    pub map_focused: bool,
    // KaFeatureCard.cpp / KaFeatureCardEditors.cpp — 유구 카드 추가 필드
    /// `feature_poly` 이름(name)
    pub feature_name: String,
    /// `survey_area` 유적명(site_name)
    pub site_name: String,
    /// `control_points` 기준점 번호
    pub control_point_id: String,
    pub control_x: f64,
    pub control_y: f64,
    pub control_z: f64,
    /// `section_line` 단면 번호 (예: A-A')
    pub section_id: String,
    /// KaControlCsvBox.cpp — CSV 경로
    pub control_csv_path: String,
    /// C++ `KaFeatureCard.cpp:202-278` (`d7afcc8`) — 선택한 시추공의 주상도 층서 카드.
    /// `hole_code` 가 비면 시추 카드를 그리지 않는다(유구 카드 그대로).
    pub borehole: crate::borehole_card::BoreholeCardData,
}
