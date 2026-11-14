//! 표시 옵션(`DisplayOptions`)과 단추 치수(`ButtonMetrics`).
//!
//! `theme_tokens` 에서 분리했다.

use serde::{Deserialize, Serialize};

use crate::theme_tokens_base::ColorTheme;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DisplayOptions {
    pub high_contrast: bool,
    pub large_text: bool,
    pub bundled_fonts: bool,
    pub paper_look: bool,
    pub color_theme: ColorTheme,
}

pub const HGIS_FONT_BASE_PT: u32 = 12;

pub const HGIS_FONT_LARGE_RATIO: f64 = 1.25;

pub const HGIS_FONT_LARGE_PT: u32 = 15;

pub const HGIS_WCAG_AA_MIN_CONTRAST: f64 = 4.5;

pub const HGIS_WCAG_AAA_MIN_CONTRAST: f64 = 7.0;

impl DisplayOptions {
    /// C++ `KaThemeManager::setLargeFontMode` — 기본 12pt 기준 1.25배(15pt) 포인트 크기 반환
    #[must_use]
    pub const fn font_point_size(&self) -> u32 {
        if self.large_text {
            HGIS_FONT_LARGE_PT
        } else {
            HGIS_FONT_BASE_PT
        }
    }

    pub const fn ui_font_size(&self) -> u32 {
        if self.large_text {
            15
        } else {
            13
        }
    }

    pub const fn scaled_ui_px(&self, base_px: u32) -> u32 {
        (base_px * self.ui_font_size() + 6) / 13
    }

    pub const fn button_edge_width(&self) -> u32 {
        if self.paper_look {
            1
        } else {
            2
        }
    }

    pub const fn depicted_paper_radius(&self) -> u32 {
        if self.paper_look {
            0
        } else {
            3
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ButtonMetrics {
    pub ribbon_icon_size: u32,
    pub ribbon_font_size: u32,
    pub ribbon_min_font_size: u32,
    pub ribbon_chip_width: u32,
    pub ribbon_min_width: u32,
    pub ribbon_label_padding: u32,
    pub ribbon_max_label_width: u32,
    pub scale_button_height: u32,
    pub scale_button_min_width: u32,
    pub layout_icon_size: u32,
    pub layout_button_height: u32,
    pub panel_margin: u32,
    pub chip_height: u32,
    pub chip_radius: u32,
}

impl Default for ButtonMetrics {
    fn default() -> Self {
        Self {
            ribbon_icon_size: 32,
            ribbon_font_size: 13,
            ribbon_min_font_size: 12,
            // Default ribbon chip width and minimum width (target/cpp-ref/src/app/KaTheme.h:97-98).
            ribbon_chip_width: 40,
            ribbon_min_width: 40,
            ribbon_label_padding: 8,
            ribbon_max_label_width: 64,
            scale_button_height: 30,
            scale_button_min_width: 54,
            layout_icon_size: 32,
            layout_button_height: 64,
            panel_margin: 8,
            chip_height: 22,
            chip_radius: 11,
        }
    }
}

impl ButtonMetrics {
    /// C++ `KaBeginnerRibbon.cpp:31-36` `chipWidth(look, labelPx)` 1:1.
    ///
    /// 라벨이 보이는 칩(look 12, `tile = 32`)은 `ribbon_chip_width`, `tile + 8`,
    /// `min(라벨폭, ribbon_max_label_width) + ribbon_label_padding` 중 가장 큰 값이다.
    #[must_use]
    pub fn ribbon_chip_width_for_label(&self, label_px: f32) -> u32 {
        let icon_floor = self.ribbon_icon_size + 8;
        let label_ceiling = label_px.min(self.ribbon_max_label_width as f32);
        let label_width = label_ceiling + self.ribbon_label_padding as f32;
        let widest = label_width.max(icon_floor as f32);
        self.ribbon_chip_width.max(widest as u32)
    }
}
