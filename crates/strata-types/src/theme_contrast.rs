//! WCAG 명암비 계산(`ThemeTokens` 의 대비 관련 메서드).
//!
//! `impl ThemeTokens` 가 245줄이라 여기로 나눴다.

use super::theme_tokens_base::ThemeTokens;

impl ThemeTokens {

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
