use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ColorTheme {
    #[default]
    Default,
    SkySnow,
    BoltYellow,
    BlossomPastel,
}

impl ColorTheme {
    pub const ALL: [Self; 4] = [
        Self::Default,
        Self::SkySnow,
        Self::BoltYellow,
        Self::BlossomPastel,
    ];

    /// C++ `KaThemeOptions.cpp` 4개 색상 테마 한글 라벨 1:1
    #[must_use]
    pub const fn label_ko(self) -> &'static str {
        match self {
            Self::Default => "기본",
            Self::SkySnow => "하늘 눈밭",
            Self::BoltYellow => "번개 노랑",
            Self::BlossomPastel => "벚꽃 파스텔",
        }
    }

    /// C++ `KaThemeOptions.cpp` 4개 색상 테마 툴팁 1:1
    #[must_use]
    pub const fn tooltip_ko(self) -> &'static str {
        match self {
            Self::Default => "종이·흙색 또는 기본 회백 화면을 씁니다.",
            Self::SkySnow => "맑은 하늘빛과 눈밭처럼 밝은 화면으로 바꿉니다.",
            Self::BoltYellow => "따뜻한 크림 바탕에 노란 강조색을 씁니다.",
            Self::BlossomPastel => "차분한 벚꽃 파스텔 바탕과 자주빛 강조색을 씁니다.",
        }
    }
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

    pub const fn sky_snow_tokens(high_contrast: bool) -> Self {
        let mut t = Self::paper_tokens(false);
        t.accent = rgb(0x25, 0x6B, 0xA8);
        t.accent_hover = rgb(0x1D, 0x5B, 0x91);
        t.accent_deep = rgb(0x16, 0x48, 0x73);
        t.accent_wash = rgb(0xDC, 0xEC, 0xFA);
        t.ink = rgb(0x14, 0x22, 0x30);
        t.ink_muted = rgb(0x4E, 0x62, 0x75);
        t.ink_disabled = rgb(0x5E, 0x6D, 0x7B);
        t.surface = rgb(0xFF, 0xFF, 0xFF);
        t.desk = rgb(0xEE, 0xF5, 0xFB);
        t.alt_row = rgb(0xF7, 0xFA, 0xFD);
        t.stripe = rgb(0xF1, 0xF7, 0xFC);
        t.hover = rgb(0xE8, 0xF2, 0xFA);
        t.selected = rgb(0xD6, 0xE7, 0xF7);
        t.pressed = rgb(0xC9, 0xDF, 0xF4);
        t.disabled_surface = rgb(0xE5, 0xEE, 0xF6);
        t.dialog_ground = rgb(0xF7, 0xFA, 0xFD);
        t.chrome = rgb(0xF5, 0xF9, 0xFC);
        t.chosen_fill = rgb(0xD6, 0xE7, 0xF7);
        t.border = rgb(0xCE, 0xDC, 0xEB);
        t.border_strong = rgb(0xAB, 0xC1, 0xD6);
        t.primary = rgb(0x25, 0x6B, 0xA8);
        t.primary_text = rgb(0xFF, 0xFF, 0xFF);
        t.tile = rgb(0xE8, 0xF1, 0xF9);
        t.tile_on = rgb(0x18, 0x38, 0x54);
        t.glyph = rgb(0x1B, 0x35, 0x50);
        t.glyph_on = rgb(0xF7, 0xFA, 0xFD);
        if high_contrast {
            t.ink_muted = rgb(0x2F, 0x43, 0x56);
            t.border = rgb(0x6F, 0x8D, 0xA8);
            t.border_strong = rgb(0x4E, 0x6D, 0x8A);
            t.warn = rgb(0x5C, 0x37, 0x00);
        }
        t
    }

    pub const fn bolt_yellow_tokens(high_contrast: bool) -> Self {
        let mut t = Self::paper_tokens(false);
        t.accent = rgb(0x9A, 0x65, 0x00);
        t.accent_hover = rgb(0x82, 0x55, 0x00);
        t.accent_deep = rgb(0x6B, 0x45, 0x00);
        t.accent_wash = rgb(0xFA, 0xED, 0xC2);
        t.ink = rgb(0x1F, 0x1A, 0x0E);
        t.ink_muted = rgb(0x65, 0x5A, 0x3E);
        t.ink_disabled = rgb(0x70, 0x65, 0x4A);
        t.surface = rgb(0xFF, 0xFE, 0xFA);
        t.desk = rgb(0xFA, 0xF4, 0xDF);
        t.alt_row = rgb(0xFD, 0xF9, 0xEC);
        t.stripe = rgb(0xFA, 0xF3, 0xDC);
        t.hover = rgb(0xF6, 0xEC, 0xCB);
        t.selected = rgb(0xEF, 0xE0, 0xAD);
        t.pressed = rgb(0xE5, 0xD4, 0x9B);
        t.disabled_surface = rgb(0xEF, 0xE9, 0xD4);
        t.dialog_ground = rgb(0xFD, 0xF9, 0xEC);
        t.chrome = rgb(0xFB, 0xF6, 0xE4);
        t.chosen_fill = rgb(0xEF, 0xE0, 0xAD);
        t.border = rgb(0xDF, 0xD4, 0xB4);
        t.border_strong = rgb(0xC5, 0xB6, 0x8C);
        t.primary = rgb(0x9A, 0x65, 0x00);
        t.primary_text = rgb(0xFF, 0xFF, 0xFF);
        t.tile = rgb(0xF2, 0xE9, 0xCC);
        t.tile_on = rgb(0x2E, 0x25, 0x10);
        t.glyph = rgb(0x2E, 0x25, 0x10);
        t.glyph_on = rgb(0xFD, 0xF9, 0xEC);
        if high_contrast {
            t.ink_muted = rgb(0x3F, 0x36, 0x1F);
            t.border = rgb(0x8F, 0x7E, 0x4B);
            t.border_strong = rgb(0x6E, 0x5F, 0x32);
            t.accent = rgb(0x7F, 0x52, 0x00);
            t.primary = rgb(0x7F, 0x52, 0x00);
        }
        t
    }

    pub const fn blossom_pastel_tokens(high_contrast: bool) -> Self {
        let mut t = Self::paper_tokens(false);
        t.accent = rgb(0xA8, 0x44, 0x68);
        t.accent_hover = rgb(0x91, 0x38, 0x58);
        t.accent_deep = rgb(0x78, 0x2C, 0x48);
        t.accent_wash = rgb(0xF8, 0xDF, 0xE8);
        t.ink = rgb(0x26, 0x18, 0x20);
        t.ink_muted = rgb(0x6B, 0x55, 0x61);
        t.ink_disabled = rgb(0x74, 0x5F, 0x6B);
        t.surface = rgb(0xFF, 0xFC, 0xFD);
        t.desk = rgb(0xFA, 0xF0, 0xF4);
        t.alt_row = rgb(0xFD, 0xF6, 0xF9);
        t.stripe = rgb(0xF9, 0xEE, 0xF3);
        t.hover = rgb(0xF6, 0xE6, 0xEE);
        t.selected = rgb(0xEE, 0xD6, 0xE2);
        t.pressed = rgb(0xE4, 0xC7, 0xD6);
        t.disabled_surface = rgb(0xF1, 0xE6, 0xEC);
        t.dialog_ground = rgb(0xFD, 0xF6, 0xF9);
        t.chrome = rgb(0xFB, 0xF2, 0xF6);
        t.chosen_fill = rgb(0xEE, 0xD6, 0xE2);
        t.border = rgb(0xE3, 0xCE, 0xD8);
        t.border_strong = rgb(0xC9, 0xAE, 0xBC);
        t.primary = rgb(0xA8, 0x44, 0x68);
        t.primary_text = rgb(0xFF, 0xFF, 0xFF);
        t.tile = rgb(0xF2, 0xE2, 0xEA);
        t.tile_on = rgb(0x38, 0x1E, 0x2B);
        t.glyph = rgb(0x38, 0x1E, 0x2B);
        t.glyph_on = rgb(0xFD, 0xF6, 0xF9);
        if high_contrast {
            t.ink_muted = rgb(0x46, 0x31, 0x3D);
            t.border = rgb(0x98, 0x77, 0x88);
            t.border_strong = rgb(0x78, 0x58, 0x69);
            t.warn = rgb(0x5C, 0x37, 0x00);
        }
        t
    }

    pub const fn for_options(opts: &DisplayOptions) -> Self {
        match opts.color_theme {
            ColorTheme::SkySnow => Self::sky_snow_tokens(opts.high_contrast),
            ColorTheme::BoltYellow => Self::bolt_yellow_tokens(opts.high_contrast),
            ColorTheme::BlossomPastel => Self::blossom_pastel_tokens(opts.high_contrast),
            ColorTheme::Default => {
                if opts.paper_look {
                    Self::paper_tokens(opts.high_contrast)
                } else {
                    Self::stock_tokens(opts.high_contrast)
                }
            }
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
        assert!(
            ((f64::from(base_opts.font_point_size()) * HGIS_FONT_LARGE_RATIO) - 15.0).abs() < 1e-9
        );

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

    #[test]
    fn test_three_kingdoms_color_mapping_and_era_presets() {
        // TestKaArchaeologicalThemePreset::testThreeKingdomsColorMapping:
        // EraPreset::ThreeKingdoms -> #2C3E50 (RGB: 44, 62, 80)
        assert_eq!(EraPreset::ThreeKingdoms.standard_rgb(), [44, 62, 80]);
        assert_eq!(EraPreset::ThreeKingdoms.hex_color(), "#2C3E50");
        assert_eq!(EraPreset::Paleolithic.standard_rgb(), [110, 44, 0]);
        assert_eq!(EraPreset::BronzeAge.standard_rgb(), [211, 84, 0]);
        assert_eq!(EraPreset::Goryeo.standard_rgb(), [17, 120, 100]);

        let dwelling =
            ArchaeologicalFeatureStyleRule::for_feature_type("Dwelling", EraPreset::ThreeKingdoms);
        assert_eq!(dwelling.stroke_rgb, [44, 62, 80]);
        assert!((dwelling.outline_width_mm - 0.35).abs() < 1e-6);

        let kiln =
            ArchaeologicalFeatureStyleRule::for_feature_type("Kiln", EraPreset::ThreeKingdoms);
        assert_eq!(kiln.fill_rgb, [0xE7, 0x4C, 0x3C]);
        assert!((kiln.fill_opacity - 0.70).abs() < 1e-6);
    }
}

/// 고고학 9대 시대별 표준 테마 프리셋 (C++ `EraPreset` & `HGIS_ERA_STANDARD_COLORS`)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EraPreset {
    /// 구석기 (`#6E2C00`)
    Paleolithic,
    /// 신석기 (`#B9770E`)
    Neolithic,
    /// 청동기 (`#D35400`)
    BronzeAge,
    /// 원삼국 (`#C0392B`)
    ProtoThree,
    /// 삼국 (`#2C3E50`)
    ThreeKingdoms,
    /// 통일신라 (`#D4AC0D`)
    UnifiedSilla,
    /// 고려 (`#117864`)
    Goryeo,
    /// 조선 (`#7F8C8D`)
    Joseon,
    /// 미상 (`#95A5A6`)
    Unknown,
}

impl EraPreset {
    /// C++ `HGIS_ERA_STANDARD_COLORS` 1:1 RGB 반환
    #[must_use]
    pub const fn standard_rgb(self) -> [u8; 3] {
        match self {
            Self::Paleolithic => [110, 44, 0],
            Self::Neolithic => [185, 119, 14],
            Self::BronzeAge => [211, 84, 0],
            Self::ProtoThree => [192, 57, 43],
            Self::ThreeKingdoms => [44, 62, 80],
            Self::UnifiedSilla => [212, 172, 13],
            Self::Goryeo => [17, 120, 100],
            Self::Joseon => [127, 140, 141],
            Self::Unknown => [149, 165, 166],
        }
    }

    /// 16진수 색상 코드(`#RRGGBB`) 반환
    #[must_use]
    pub const fn hex_color(self) -> &'static str {
        match self {
            Self::Paleolithic => "#6E2C00",
            Self::Neolithic => "#B9770E",
            Self::BronzeAge => "#D35400",
            Self::ProtoThree => "#C0392B",
            Self::ThreeKingdoms => "#2C3E50",
            Self::UnifiedSilla => "#D4AC0D",
            Self::Goryeo => "#117864",
            Self::Joseon => "#7F8C8D",
            Self::Unknown => "#95A5A6",
        }
    }
}

/// 유구종별(`FEAT_TYPE`) 룰 기반 심볼 스타일 프리셋 (C++ `KaArchaeologicalThemePreset::applyArchaeologicalTheme`)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ArchaeologicalFeatureStyleRule {
    pub stroke_rgb: [u8; 3],
    pub fill_rgb: [u8; 3],
    pub outline_width_mm: f64,
    pub fill_opacity: f64,
    pub hatch_pattern: &'static str,
}

impl ArchaeologicalFeatureStyleRule {
    #[must_use]
    pub fn for_feature_type(feat_type: &str, era: EraPreset) -> Self {
        let base_rgb = era.standard_rgb();
        match feat_type.trim() {
            "Dwelling" | "주거지" | "수혈주거지" => Self {
                stroke_rgb: base_rgb,
                fill_rgb: base_rgb,
                outline_width_mm: 0.35,
                fill_opacity: 0.55,
                hatch_pattern: "BDiagPattern",
            },
            "Pit" | "수혈" => Self {
                stroke_rgb: base_rgb,
                fill_rgb: base_rgb,
                outline_width_mm: 0.20,
                fill_opacity: 0.40,
                hatch_pattern: "SolidPattern",
            },
            "Ditch" | "구상유구" => Self {
                stroke_rgb: base_rgb,
                fill_rgb: base_rgb,
                outline_width_mm: 0.40,
                fill_opacity: 0.50,
                hatch_pattern: "Dense5Pattern",
            },
            "Tomb" | "석관묘" | "고분" => Self {
                stroke_rgb: base_rgb,
                fill_rgb: [0x3E, 0x27, 0x23],
                outline_width_mm: 0.50,
                fill_opacity: 0.85,
                hatch_pattern: "DoubleOutlineSolid",
            },
            "Kiln" | "가마" | "소성유구" => Self {
                stroke_rgb: [0xE7, 0x4C, 0x3C],
                fill_rgb: [0xE7, 0x4C, 0x3C],
                outline_width_mm: 0.40,
                fill_opacity: 0.70,
                hatch_pattern: "DiagCrossPattern",
            },
            _ => Self {
                stroke_rgb: base_rgb,
                fill_rgb: base_rgb,
                outline_width_mm: 0.26,
                fill_opacity: 0.45,
                hatch_pattern: "SolidPattern",
            },
        }
    }
}
