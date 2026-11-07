#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MapToolKind {
    #[default]
    Pan,
    Select,
    DrawPolygon,
    DrawLine,
    PlacePoint,
    Measure,
    GeorefAlign,
    TrenchPlace,
    FeatureCardNote,
}

impl MapToolKind {
    pub const fn guide_hint_ko(self) -> &'static str {
        match self {
            Self::Pan => "",
            Self::Select => "도형을 클릭하면 수정점이 나옵니다. 점을 끌어 옮깁니다",
            Self::DrawPolygon | Self::DrawLine => {
                "클릭으로 점을 찍고 우클릭으로 마칩니다 (Esc 취소)"
            }
            Self::PlacePoint => "클릭한 자리에 점을 놓습니다",
            Self::Measure => "두 점을 클릭하면 거리가, 셋 이상이면 면적이 나옵니다",
            Self::GeorefAlign => "사진과 지도에서 같은 자리를 짝지어 클릭합니다",
            Self::TrenchPlace => "격자를 놓을 원점을 클릭합니다",
            Self::FeatureCardNote => "도형을 클릭하면 조사카드가 열립니다",
        }
    }
}
