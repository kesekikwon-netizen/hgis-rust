//! 고고학 시대 프리셋(`EraPreset`)과 유구종별 스타일 룰.
//!
//! `theme_tokens` 에서 분리했다.

use serde::{Deserialize, Serialize};

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
