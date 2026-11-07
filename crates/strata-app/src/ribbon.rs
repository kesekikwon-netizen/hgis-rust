use strata_canvas2d::map_tools::MapToolKind;

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RegionGridTile {
    pub name: &'static str,
    pub row: usize,
    pub col: usize,
    pub center_5187: [f64; 2],
}

/// `KaRegionLocator.cpp:432-439` — 6행 4열 한반도 모양 17개 시·도 타일 지도
pub const PENINSULA_REGION_TILES: [RegionGridTile; 17] = [
    RegionGridTile { name: "서울", row: 0, col: 1, center_5187: [95_300.0, 552_000.0] },
    RegionGridTile { name: "인천", row: 1, col: 0, center_5187: [70_500.0, 540_200.0] },
    RegionGridTile { name: "경기", row: 1, col: 1, center_5187: [104_200.0, 520_800.0] },
    RegionGridTile { name: "강원", row: 1, col: 2, center_5187: [182_000.0, 565_000.0] },
    RegionGridTile { name: "충남", row: 2, col: 0, center_5187: [72_000.0, 450_000.0] },
    RegionGridTile { name: "세종", row: 2, col: 1, center_5187: [125_400.0, 432_000.0] },
    RegionGridTile { name: "충북", row: 2, col: 2, center_5187: [152_000.0, 455_000.0] },
    RegionGridTile { name: "경북", row: 2, col: 3, center_5187: [219_850.0, 360_120.0] },
    RegionGridTile { name: "전북", row: 3, col: 0, center_5187: [108_000.0, 368_000.0] },
    RegionGridTile { name: "대전", row: 3, col: 1, center_5187: [136_000.0, 415_000.0] },
    RegionGridTile { name: "대구", row: 3, col: 2, center_5187: [195_200.0, 363_800.0] },
    RegionGridTile { name: "울산", row: 3, col: 3, center_5187: [238_500.0, 338_000.0] },
    RegionGridTile { name: "광주", row: 4, col: 0, center_5187: [90_200.0, 295_000.0] },
    RegionGridTile { name: "전남", row: 4, col: 1, center_5187: [95_000.0, 265_000.0] },
    RegionGridTile { name: "경남", row: 4, col: 2, center_5187: [180_000.0, 305_000.0] },
    RegionGridTile { name: "부산", row: 4, col: 3, center_5187: [220_000.0, 296_000.0] },
    RegionGridTile { name: "제주", row: 5, col: 1, center_5187: [66_000.0, 92_000.0] },
];

#[derive(Debug, Clone)]
pub struct KaAppBarSearch {
    pub query: String,
    pub selected_sido: String,
    pub region_popup_open: bool,
    pub city_input: String,
    pub dong_input: String,
    pub lot_input: String,
}

impl Default for KaAppBarSearch {
    fn default() -> Self {
        Self {
            query: String::new(),
            selected_sido: "경북".to_owned(),
            region_popup_open: false,
            city_input: "경주시".to_owned(),
            dong_input: "인왕동".to_owned(),
            lot_input: "76".to_owned(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct KaBeginnerRibbon {
    pub active_category: RibbonCategory,
    pub sub_toolbar_visible: bool,
    pub sub_mode: SubToolsMode,
    pub draw_sub_target: DrawSubTarget,
    pub app_bar: KaAppBarSearch,
}

impl Default for KaBeginnerRibbon {
    fn default() -> Self {
        Self {
            active_category: RibbonCategory::Survey,
            sub_toolbar_visible: false,
            sub_mode: SubToolsMode::None,
            draw_sub_target: DrawSubTarget::SurveyArea,
            app_bar: KaAppBarSearch::default(),
        }
    }
}

impl KaBeginnerRibbon {
    #[must_use]
    pub const fn categories() -> &'static [RibbonCategory] {
        &[
            RibbonCategory::Survey,
            RibbonCategory::Record,
            RibbonCategory::Download,
            RibbonCategory::Basemap,
            RibbonCategory::Georef,
            RibbonCategory::Export,
            RibbonCategory::Misc,
        ]
    }

    #[must_use]
    pub const fn is_map_only_tool(tool: MapToolKind) -> bool {
        matches!(
            tool,
            MapToolKind::DrawPolygon
                | MapToolKind::DrawLine
                | MapToolKind::PlacePoint
                | MapToolKind::TrenchPlace
                | MapToolKind::GeorefAlign
                | MapToolKind::Measure
                | MapToolKind::FeatureCardNote
        )
    }
}
