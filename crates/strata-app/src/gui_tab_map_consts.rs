//! 지도 탭 3분할 폭과 좌표격자 설정·표본 색.

use eframe::egui::Color32;

pub const UX_MAP_LEFT_WIDTH: f32 = 351.0;
pub const UX_MAP_CANVAS_WIDTH: f32 = 954.0;
pub const UX_MAP_RIGHT_WIDTH: f32 = 236.0;

/// KaShellGridControls.cpp:41-145 KaShellGridSettings — 2줄 격자 세부 설정
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShellGridConfig {
    /// 0.1~10000 m
    pub step_m: f64,
    pub show_labels: bool,
    /// `미터 격자` = false, `경위도 격자` = true
    pub is_geo: bool,
    /// `SHELL_GRID_SWATCHES` index
    pub color_idx: usize,
    /// 0.0~360.0°, meters grid only
    pub rotation_deg: f32,
    /// 0.5~5.0
    pub line_width: f32,
    /// 0 = 실선, 1 = 점선, 2 = 점
    pub dash_idx: usize,
}

impl Default for ShellGridConfig {
    fn default() -> Self {
        Self {
            step_m: 20.0,
            show_labels: true,
            is_geo: false,
            color_idx: 0,
            rotation_deg: 0.0,
            line_width: 1.2,
            dash_idx: 1,
        }
    }
}
pub const SHELL_GRID_DASH_LABELS: [&str; 3] = ["실선", "점선", "점"];

/// `KaShellGridControls.cpp:83-87` — 격자 선 색 견본 셋
pub const SHELL_GRID_SWATCHES: [(Color32, &str); 3] = [
    (Color32::from_rgb(0xD9, 0x2B, 0x2B), "빨간색 격자"),
    (Color32::from_rgb(0x1D, 0x4E, 0xD8), "파란색 격자"),
    (Color32::from_rgb(0x1F, 0x29, 0x37), "검정색 격자"),
];
