use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ColorTheme {
    #[default]
    Default,
    SkySnow,
    BoltYellow,
    BlossomPastel,
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemeTokens {
    pub accent: [u8; 4],
    pub accent_hover: [u8; 4],
    pub accent_deep: [u8; 4],
    pub accent_wash: [u8; 4],
    pub ink: [u8; 4],
    pub ink_muted: [u8; 4],
    pub ink_disabled: [u8; 4],
    pub surface: [u8; 4],
    pub desk: [u8; 4],
    pub alt_row: [u8; 4],
    pub stripe: [u8; 4],
    pub hover: [u8; 4],
    pub selected: [u8; 4],
    pub pressed: [u8; 4],
    pub disabled_surface: [u8; 4],
    pub dialog_ground: [u8; 4],
    pub chrome: [u8; 4],
    pub chosen_fill: [u8; 4],
    pub border: [u8; 4],
    pub border_strong: [u8; 4],
    pub danger: [u8; 4],
    pub danger_surface: [u8; 4],
    pub ok: [u8; 4],
    pub success_surface: [u8; 4],
    pub warn: [u8; 4],
    pub warn_surface: [u8; 4],
    pub primary: [u8; 4],
    pub primary_text: [u8; 4],
    pub tile: [u8; 4],
    pub tile_on: [u8; 4],
    pub glyph: [u8; 4],
    pub glyph_on: [u8; 4],
}

const fn rgb(r: u8, g: u8, b: u8) -> [u8; 4] {
    [r, g, b, 0xFF]
}

impl Default for ThemeTokens {
    fn default() -> Self {
        Self::stock_tokens(false)
    }
}

impl ThemeTokens {
    pub const fn stock_tokens(high_contrast: bool) -> Self {
        let mut t = Self {
            accent: rgb(0x1F, 0x6F, 0xB2),
            accent_hover: rgb(0x18, 0x5E, 0x99),
            accent_deep: rgb(0x12, 0x55, 0x8D),
            accent_wash: rgb(0xE3, 0xEE, 0xF8),
            ink: rgb(0x1D, 0x27, 0x33),
            ink_muted: rgb(0x5B, 0x68, 0x75),
            ink_disabled: rgb(0x59, 0x68, 0x74),
            surface: rgb(0xFF, 0xFF, 0xFF),
            desk: rgb(0xF4, 0xF6, 0xF8),
            alt_row: rgb(0xF7, 0xF9, 0xFB),
            stripe: rgb(0xF2, 0xF6, 0xFA),
            hover: rgb(0xEE, 0xF4, 0xFA),
            selected: rgb(0xE6, 0xF0, 0xFA),
            pressed: rgb(0xE1, 0xEC, 0xF7),
            disabled_surface: rgb(0xEE, 0xF1, 0xF4),
            dialog_ground: rgb(0xFF, 0xFF, 0xFF),
            chrome: rgb(0xFF, 0xFF, 0xFF),
            chosen_fill: rgb(0xE6, 0xF0, 0xFA),
            border: rgb(0xDC, 0xE3, 0xEA),
            border_strong: rgb(0xB8, 0xC4, 0xCF),
            danger: rgb(0xB4, 0x23, 0x18),
            danger_surface: rgb(0xFB, 0xED, 0xEA),
            ok: rgb(0x25, 0x6B, 0x42),
            success_surface: rgb(0xEA, 0xF4, 0xEE),
            warn: rgb(0x7A, 0x4A, 0x00),
            warn_surface: rgb(0xFF, 0xF4, 0xDB),
            primary: rgb(0x1F, 0x6F, 0xB2),
            primary_text: rgb(0xFF, 0xFF, 0xFF),
            tile: rgb(0xF4, 0xF6, 0xF8),
            tile_on: rgb(0x1F, 0x6F, 0xB2),
            glyph: rgb(0x1D, 0x27, 0x33),
            glyph_on: rgb(0xFF, 0xFF, 0xFF),
        };
        if high_contrast {
            t.ink_muted = rgb(0x3B, 0x46, 0x52);
            t.border = rgb(0x88, 0x95, 0xA3);
            t.border_strong = rgb(0x66, 0x74, 0x82);
        }
        t
    }

    pub const fn paper_tokens(high_contrast: bool) -> Self {
        let mut t = Self {
            accent: rgb(0xB5, 0x57, 0x3A),
            accent_hover: rgb(0xA4, 0x4D, 0x32),
            accent_deep: rgb(0x8F, 0x42, 0x29),
            accent_wash: rgb(0xF4, 0xE3, 0xDA),
            ink: rgb(0x14, 0x14, 0x13),
            ink_muted: rgb(0x5E, 0x5D, 0x59),
            ink_disabled: rgb(0x69, 0x68, 0x62),
            surface: rgb(0xFF, 0xFF, 0xFF),
            desk: rgb(0xF5, 0xF4, 0xED),
            alt_row: rgb(0xFA, 0xF9, 0xF5),
            stripe: rgb(0xF7, 0xF6, 0xF0),
            hover: rgb(0xF5, 0xF4, 0xED),
            selected: rgb(0xE3, 0xDA, 0xCC),
            pressed: rgb(0xDD, 0xD8, 0xCB),
            disabled_surface: rgb(0xF0, 0xEE, 0xE6),
            dialog_ground: rgb(0xFA, 0xF9, 0xF5),
            chrome: rgb(0xFA, 0xF9, 0xF5),
            chosen_fill: rgb(0xE3, 0xDA, 0xCC),
            border: rgb(0xDE, 0xDC, 0xD1),
            border_strong: rgb(0xC2, 0xC0, 0xB6),
            danger: rgb(0xA3, 0x3B, 0x3B),
            danger_surface: rgb(0xF3, 0xDE, 0xDA),
            ok: rgb(0x3F, 0x6B, 0x31),
            success_surface: rgb(0xE6, 0xEE, 0xDD),
            warn: rgb(0x7A, 0x5A, 0x00),
            warn_surface: rgb(0xF6, 0xEC, 0xC8),
            primary: rgb(0xB5, 0x57, 0x3A),
            primary_text: rgb(0xFF, 0xFF, 0xFF),
            tile: rgb(0xF0, 0xEE, 0xE6),
            tile_on: rgb(0x30, 0x30, 0x2E),
            glyph: rgb(0x30, 0x30, 0x2E),
            glyph_on: rgb(0xFA, 0xF9, 0xF5),
        };
        if high_contrast {
            t.ink_muted = rgb(0x3D, 0x3D, 0x3A);
            t.alt_row = rgb(0xF5, 0xF4, 0xED);
            t.stripe = rgb(0xF0, 0xEE, 0xE6);
            t.hover = rgb(0xF0, 0xEE, 0xE6);
            t.selected = rgb(0xD9, 0xCE, 0xBB);
            t.pressed = rgb(0xCF, 0xCA, 0xBC);
            t.border = rgb(0x87, 0x86, 0x7F);
            t.border_strong = rgb(0x73, 0x72, 0x6C);
            t.warn = rgb(0x5C, 0x37, 0x00);
        }
        t
    }

    pub const fn for_options(opts: &DisplayOptions) -> Self {
        if opts.paper_look {
            Self::paper_tokens(opts.high_contrast)
        } else {
            Self::stock_tokens(opts.high_contrast)
        }
    }

    fn relative_luminance(rgba: [u8; 4]) -> f64 {
        fn channel(v: u8) -> f64 {
            let s = f64::from(v) / 255.0;
            if s <= 0.04045 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        }
        0.2126 * channel(rgba[0]) + 0.7152 * channel(rgba[1]) + 0.0722 * channel(rgba[2])
    }

    pub fn contrast_ratio(&self, first: [u8; 4], second: [u8; 4]) -> f64 {
        Self::compute_wcag_contrast_ratio(first, second)
    }

    /// C++ `KaThemeManager::computeWcagContrastRatio` — sRGB 감마 해제 상대 휘도 기반 WCAG 2.1 명암비(1.0~21.0) 산출
    #[must_use]
    pub fn compute_wcag_contrast_ratio(fg: [u8; 4], bg: [u8; 4]) -> f64 {
        let l1 = Self::relative_luminance(fg);
        let l2 = Self::relative_luminance(bg);
        let (bright, dark) = if l1 >= l2 { (l1, l2) } else { (l2, l1) };
        (bright + 0.05) / (dark + 0.05)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paper_look_tokens_satisfy_wcag_contrast_and_clay_accent() {
        let opts = DisplayOptions {
            paper_look: true,
            ..Default::default()
        };
        let t = ThemeTokens::for_options(&opts);
        assert_eq!(t.accent, [0xB5, 0x57, 0x3A, 0xFF]);
        assert_eq!(t.dialog_ground, [0xFA, 0xF9, 0xF5, 0xFF]);
        assert!(t.contrast_ratio(t.ink_muted, t.surface) >= HGIS_WCAG_AA_MIN_CONTRAST);
        assert!(t.contrast_ratio(t.primary_text, t.primary) >= HGIS_WCAG_AA_MIN_CONTRAST);
    }

    #[test]
    fn test_large_font_mode_12pt_to_15pt_and_paper_bg_f5f3ed_fg_222222_wcag_13_72() {
        let base_opts = DisplayOptions::default();
        assert_eq!(base_opts.font_point_size(), 12);
        let large_opts = DisplayOptions {
            large_text: true,
            ..Default::default()
        };
        assert_eq!(large_opts.font_point_size(), 15);
        assert!(((f64::from(base_opts.font_point_size()) * HGIS_FONT_LARGE_RATIO) - 15.0).abs() < 1e-9);

        let ratio = ThemeTokens::compute_wcag_contrast_ratio(
            [0x22, 0x22, 0x22, 0xFF],
            [0xF5, 0xF3, 0xED, 0xFF],
        );
        assert!(
            ratio >= 13.7,
            "expected >= 13.7 contrast ratio, got {ratio}"
        );
        assert!(ratio >= HGIS_WCAG_AA_MIN_CONTRAST);
        assert!(ratio >= HGIS_WCAG_AAA_MIN_CONTRAST);
    }
}
