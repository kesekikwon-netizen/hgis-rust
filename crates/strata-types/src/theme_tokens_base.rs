//! 색상 테마(`ColorTheme`)와 기본 토큰 팔레트(`ThemeTokens`)의 정의.
//!
//! 팔레트 메서드는 `theme_palette_light`·`theme_palette_themed` 로 나눴다.

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

pub(crate) const fn rgb(r: u8, g: u8, b: u8) -> [u8; 4] {
    [r, g, b, 0xFF]
}

impl Default for ThemeTokens {
    fn default() -> Self {
        Self::stock_tokens(false)
    }
}

impl ColorTheme {
    /// 색상 테마 저장 키 (C++ `KaTheme::colorThemeKey`)
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::SkySnow => "sky",
            Self::BoltYellow => "bolt",
            Self::BlossomPastel => "blossom",
        }
    }

    /// 색상 테마 저장 키로부터 열거형 복원 (C++ `KaTheme::colorThemeFromKey`)
    #[must_use]
    pub fn from_key(key: &str) -> Self {
        match key.trim() {
            "sky" => Self::SkySnow,
            "bolt" => Self::BoltYellow,
            "blossom" => Self::BlossomPastel,
            _ => Self::Default,
        }
    }
}
