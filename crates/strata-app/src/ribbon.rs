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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PnuBuildError {
    InvalidBjdCode,
    InvalidBunNumber,
}

impl KaAppBarSearch {
    pub const HGIS_PNU_TOTAL_LENGTH: usize = 19;
    pub const HGIS_BUN_PADDING_WIDTH: usize = 4;
    pub const HGIS_PARCEL_ZOOM_MARGIN_M: f64 = 50.0;

    /// C++ `KaAddressSearchService::buildPnuCode` — 10자리 법정동코드 + 산 여부('1'/'2') + 4자리 0패딩 본번 + 4자리 0패딩 부번으로 19자리 표준 PNU 조합
    pub fn build_pnu_code(
        bjd_code10: &str,
        is_mountain: bool,
        main_bun: i32,
        sub_bun: i32,
    ) -> Result<String, (PnuBuildError, String)> {
        let trimmed = bjd_code10.trim();
        if trimmed.len() != 10 || !trimmed.chars().all(|c| c.is_ascii_digit()) {
            return Err((
                PnuBuildError::InvalidBjdCode,
                "법정동 코드는 10자리 숫자여야 합니다.".to_owned(),
            ));
        }
        if !(1..=9999).contains(&main_bun) || !(0..=9999).contains(&sub_bun) {
            return Err((
                PnuBuildError::InvalidBunNumber,
                "본번은 1 이상의 정수여야 합니다.".to_owned(),
            ));
        }
        let m_char = if is_mountain { '2' } else { '1' };
        let pnu = format!("{trimmed}{m_char}{main_bun:04}{sub_bun:04}");
        debug_assert_eq!(pnu.len(), Self::HGIS_PNU_TOTAL_LENGTH);
        Ok(pnu)
    }

    /// C++ `KaAddressSearchService::zoomToParcelExtent` — 필지 외곽 경계 사각형 전 방향에 50.0m 여백 가산
    #[must_use]
    pub fn zoom_to_parcel_extent(parcel_bbox: [f64; 4]) -> [f64; 4] {
        let m = Self::HGIS_PARCEL_ZOOM_MARGIN_M;
        [
            parcel_bbox[0] - m,
            parcel_bbox[1] - m,
            parcel_bbox[2] + m,
            parcel_bbox[3] + m,
        ]
    }
}

#[derive(Debug, Clone)]
pub struct KaBeginnerRibbon {
    pub active_category: RibbonCategory,
    pub sub_toolbar_visible: bool,
    pub sub_mode: SubToolsMode,
    pub draw_sub_target: DrawSubTarget,
    pub snap_assist_menu_open: bool,
    pub app_bar: KaAppBarSearch,
}

impl Default for KaBeginnerRibbon {
    fn default() -> Self {
        Self {
            active_category: RibbonCategory::Survey,
            sub_toolbar_visible: false,
            sub_mode: SubToolsMode::None,
            draw_sub_target: DrawSubTarget::SurveyArea,
            snap_assist_menu_open: false,
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

    pub fn toggle_sub_mode(&mut self, mode: SubToolsMode) {
        if self.sub_toolbar_visible && self.sub_mode == mode {
            self.sub_toolbar_visible = false;
            self.sub_mode = SubToolsMode::None;
        } else {
            self.sub_toolbar_visible = true;
            self.sub_mode = mode;
        }
    }

    pub const HGIS_RIBBON_COMPACT_WIDTH_PX: f32 = 1100.0;

    /// C++ `KaRibbonBar::isCompactMode` — 창 너비 1,100px 미만 시 24x24 컴팩트 아이콘 모드 전환
    #[must_use]
    pub fn is_compact_mode(window_width_px: f32) -> bool {
        window_width_px < Self::HGIS_RIBBON_COMPACT_WIDTH_PX
    }

    /// C++ `KaRibbonBar::setActiveTool` — 도구 전환 시 미완성 러버밴드 버퍼 초기화 및 하단 상태바 안내 문구 매핑
    pub fn switch_active_tool_clearing_rubberband(
        active_tool: &mut MapToolKind,
        sketch_buffer: &mut Vec<[f64; 2]>,
        next_tool: MapToolKind,
    ) -> &'static str {
        if *active_tool != next_tool {
            sketch_buffer.clear();
            *active_tool = next_tool;
        }
        Self::status_bar_hint_for_tool(*active_tool)
    }

    /// C++ `KaStatusBar` 도구별 하단 상태바 작업 안내 문구 매핑 1:1
    #[must_use]
    pub const fn status_bar_hint_for_tool(tool: MapToolKind) -> &'static str {
        match tool {
            MapToolKind::Pan => "지도를 드래그하여 이동하거나 마우스 휠로 확대/축소하세요.",
            MapToolKind::Select => "피처를 클릭하여 선택하거나 드래그하여 다중 선택하세요.",
            MapToolKind::DrawPolygon => {
                "시작점을 클릭하세요. [Shift]: 직교/45도 스냅, [우클릭]: 폐합 완료"
            }
            MapToolKind::DrawLine => "선 시작점을 클릭하세요. [우클릭]: 선 작도 완료",
            MapToolKind::PlacePoint => "지도 위 원하는 위치를 클릭하여 유물·기준점을 배치하세요.",
            MapToolKind::TrenchPlace => {
                "조사구역 내부를 클릭하여 표준 시굴 트렌치(20m×2m / L자형)를 배치하세요."
            }
            MapToolKind::GeorefAlign => {
                "정점을 클릭하여 드래그 이동하세요. [Delete]: 선택 정점 삭제"
            }
            MapToolKind::Measure => {
                "클릭하여 측정을 시작하세요. [우클릭/더블클릭]: 측정 완료, [ESC]: 취소"
            }
            MapToolKind::FeatureCardNote => "유구를 클릭하여 조사카드 상세 기록 창을 여세요.",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ribbon_compact_mode_1024px_and_exclusive_tool_switch_clears_rubberband() {
        assert!(KaBeginnerRibbon::is_compact_mode(1024.0));
        assert!(!KaBeginnerRibbon::is_compact_mode(1100.0));
        assert!(!KaBeginnerRibbon::is_compact_mode(1440.0));

        let mut tool = MapToolKind::DrawPolygon;
        let mut rubberband = vec![[100.0, 200.0], [110.0, 210.0]];
        let hint = KaBeginnerRibbon::switch_active_tool_clearing_rubberband(
            &mut tool,
            &mut rubberband,
            MapToolKind::Measure,
        );
        assert_eq!(tool, MapToolKind::Measure);
        assert!(rubberband.is_empty());
        assert_eq!(
            hint,
            "클릭하여 측정을 시작하세요. [우클릭/더블클릭]: 측정 완료, [ESC]: 취소"
        );
    }

    #[test]
    fn test_build_pnu_gyeongju_4713010300_76_0_yields_exact_19_digits_and_50m_margin() {
        let pnu = KaAppBarSearch::build_pnu_code("4713010300", false, 76, 0)
            .expect("valid Gyeongju PNU");
        assert_eq!(pnu, "4713010300100760000");
        assert_eq!(pnu.len(), KaAppBarSearch::HGIS_PNU_TOTAL_LENGTH);

        let err = KaAppBarSearch::build_pnu_code("4713010300", false, 0, 0)
            .expect_err("main_bun 0 must fail");
        assert_eq!(err.0, PnuBuildError::InvalidBunNumber);
        assert_eq!(err.1, "본번은 1 이상의 정수여야 합니다.");

        let zoomed = KaAppBarSearch::zoom_to_parcel_extent([1000.0, 2000.0, 1050.0, 2040.0]);
        assert_eq!(zoomed, [950.0, 1950.0, 1100.0, 2090.0]);
    }
}
