//! 리본 분류·보조도구 모드·그리기 하위 대상 열거형.
//!
//! `ribbon.rs` 가 300줄 규칙에 맞춰 나눈 파일이다. 옮긴 항목은 그대로다.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RibbonCategory {
    Survey,
    Record,
    Download,
    Basemap,
    Georef,
    Export,
    Misc,
}

impl RibbonCategory {
    #[must_use]
    pub const fn label_ko(self) -> &'static str {
        match self {
            Self::Survey => "조사",
            Self::Record => "기록",
            Self::Download => "자료 받기",
            Self::Basemap => "배경 지도",
            Self::Georef => "정합",
            Self::Export => "내보내기",
            Self::Misc => "기타",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubToolsMode {
    None,
    Draw,
    Buffer,
    Align,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawSubTarget {
    EasyDraw,
    SurveyArea,
    FeaturePoly,
    FeatureLine,
    Artifact,
}

impl DrawSubTarget {
    #[must_use]
    pub const fn label_ko(self) -> &'static str {
        match self {
            Self::EasyDraw => "쉽게그리기",
            Self::SurveyArea => "조사구역",
            Self::FeaturePoly => "유구(면)",
            Self::FeatureLine => "유구(선)",
            Self::Artifact => "유물",
        }
    }
}
