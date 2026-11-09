//! 시굴 트렌치 표준 길이·모양 열거형.
//!

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrenchStandardLength {
    M20,
    M15,
    M10,
}

impl TrenchStandardLength {
    pub const fn meters(self) -> f64 {
        match self {
            Self::M20 => 20.0,
            Self::M15 => 15.0,
            Self::M10 => 10.0,
        }
    }

    pub const fn label_ko(self) -> &'static str {
        match self {
            Self::M20 => "20 m",
            Self::M15 => "15 m",
            Self::M10 => "10 m",
        }
    }
}

/// `KaTrenchDialogPlan.cpp:321-324` — 곧은 트렌치 및 ㄱ·ㄴ자 4방향 트렌치(팔 15m + 15m, 폭 2m)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TrenchShapeKind {
    #[default]
    Straight,
    LShapeSw,
    LShapeNe,
    LShapeSe,
    LShapeNw,
}

impl TrenchShapeKind {
    pub const ALL: [Self; 5] = [
        Self::Straight,
        Self::LShapeSw,
        Self::LShapeNe,
        Self::LShapeSe,
        Self::LShapeNw,
    ];

    #[must_use]
    pub const fn label_ko(self) -> &'static str {
        match self {
            Self::Straight => "곧은 트렌치",
            Self::LShapeSw => "ㄴ자 트렌치 (15 m + 15 m)",
            Self::LShapeNe => "ㄱ자 트렌치 (15 m + 15 m)",
            Self::LShapeSe => "ㄴ자 좌우 반대 트렌치 (15 m + 15 m)",
            Self::LShapeNw => "ㄱ자 좌우 반대 트렌치 (15 m + 15 m)",
        }
    }

    #[must_use]
    pub const fn arm_bend(self) -> Option<u8> {
        match self {
            Self::Straight => None,
            Self::LShapeSw => Some(0),
            Self::LShapeNe => Some(1),
            Self::LShapeSe => Some(2),
            Self::LShapeNw => Some(3),
        }
    }
}
