//! 국가유산 데이터 종류와 배지·번호 매기기.
//!
//! `sheet_composer` 에서 분리했다.

/// 국가유산청 6대 공간정보 데이터셋 구분 (C++ `HeritageDataset` / `kDatasetKeys`)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HeritageDatasetKind {
    DesignatedHeritage,
    AlterationStandard,
    BuriedHeritageArea,
    HeritageDistributionMap,
    SurfaceSurveyArea,
    ExcavationSurveyArea,
}

impl HeritageDatasetKind {
    /// 프로젝트 파일 저장용 고정 ASCII 키 (C++ `kDatasetKeys` in `HeritageLayoutNumbers.cpp`)
    #[must_use]
    pub const fn ascii_key(self) -> &'static str {
        match self {
            Self::DesignatedHeritage => "designated_heritage",
            Self::AlterationStandard => "alteration_standard",
            Self::BuriedHeritageArea => "buried_heritage_area",
            Self::HeritageDistributionMap => "heritage_distribution_map",
            Self::SurfaceSurveyArea => "surface_survey_area",
            Self::ExcavationSurveyArea => "excavation_survey_area",
        }
    }

    /// 도면 범례 기본 색상 HEX (C++ `kTable` in `HeritageStyle.cpp`)
    #[must_use]
    pub const fn default_hex_color(self) -> &'static str {
        match self {
            Self::DesignatedHeritage => "#8E44AD",
            Self::AlterationStandard => "#E67E22",
            Self::BuriedHeritageArea => "#2E86C1",
            Self::HeritageDistributionMap => "#27AE60",
            Self::SurfaceSurveyArea => "#D4AA00",
            Self::ExcavationSurveyArea => "#C2187D",
        }
    }

    /// 한국어 표준 레이어 이름 (C++ `HeritageStyle::layerName`)
    #[must_use]
    pub const fn layer_name_ko(self) -> &'static str {
        match self {
            Self::DesignatedHeritage => "지정유산",
            Self::AlterationStandard => "현상변경허용기준",
            Self::BuriedHeritageArea => "매장유산유존지역",
            Self::HeritageDistributionMap => "문화유적분포지도",
            Self::SurfaceSurveyArea => "지표조사구역",
            Self::ExcavationSurveyArea => "발굴조사구역",
        }
    }

    /// 인트라넷 탭 코드 (C++ `HeritageStyle::tabCode`)
    #[must_use]
    pub const fn tab_code(self) -> &'static str {
        match self {
            Self::DesignatedHeritage => "",
            Self::AlterationStandard => "P",
            Self::BuriedHeritageArea => "B",
            Self::HeritageDistributionMap => "U",
            Self::SurfaceSurveyArea => "R",
            Self::ExcavationSurveyArea => "E",
        }
    }

    /// 구형 색상(`#16A085`, `#A0522D`)을 최신 표준 도면 색상(`#D4AA00`, `#C2187D`)으로 자동 매핑
    /// (C++ `HeritageStyle::layoutColor` 1:1 대응)
    #[must_use]
    pub fn layout_color_hex(self, source_hex: &str) -> &str {
        let u = source_hex.trim();
        if self == Self::SurfaceSurveyArea && u.eq_ignore_ascii_case("#16A085") {
            "#D4AA00"
        } else if self == Self::ExcavationSurveyArea && u.eq_ignore_ascii_case("#A0522D") {
            "#C2187D"
        } else {
            u
        }
    }

    /// 한국어 레이어 이름으로 데이터셋 역조회 (C++ `HeritageStyle::fromLayerName`)
    #[must_use]
    pub fn from_layer_name_ko(name: &str) -> Option<Self> {
        match name.trim() {
            "지정유산" => Some(Self::DesignatedHeritage),
            "현상변경허용기준" => Some(Self::AlterationStandard),
            "매장유산유존지역" => Some(Self::BuriedHeritageArea),
            "문화유적분포지도" => Some(Self::HeritageDistributionMap),
            "지표조사구역" => Some(Self::SurfaceSurveyArea),
            "발굴조사구역" => Some(Self::ExcavationSurveyArea),
            _ => None,
        }
    }

    /// 표준 외곽선 두께 (`0.35 mm`, C++ `HeritageStyle::outlineWidthMm`)
    #[must_use]
    pub const fn outline_width_mm() -> f64 {
        0.35
    }

    /// 유적명 라벨 최소 표시 축척 (`10000.0`, C++ `HeritageStyle::nameLabelMinScale`)
    #[must_use]
    pub const fn name_label_min_scale() -> f64 {
        10_000.0
    }

    /// 이름 없는 유적 기본 라벨 (`"무명"`, C++ `HeritageStyle::unnamedLabel`)
    #[must_use]
    pub const fn unnamed_label_ko() -> &'static str {
        "무명"
    }
}

/// 배지 배경색 `[r, g, b]`의 상대 휘도(`0.299R + 0.587G + 0.114B > 0.62`)에 따라 흑/백 텍스트 잉크 색상 결정
/// (C++ `inkFor(const QColor&)` in `HeritageLayoutNumbers.cpp`)
#[must_use]
pub fn ink_for_badge_rgb(rgb: [u8; 3]) -> [u8; 3] {
    let lum =
        (f64::from(rgb[0]) * 0.299 + f64::from(rgb[1]) * 0.587 + f64::from(rgb[2]) * 0.114) / 255.0;
    if lum > 0.62 {
        [0, 0, 0]
    } else {
        [255, 255, 255]
    }
}

/// 도면 유적 번호 원형 배지 직경(mm) 계산: `max(2.3, 1.0 + digits * 0.725) * 1.3`
/// (C++ `circleSize(int number)` in `HeritageLayoutNumbers.cpp`)
#[must_use]
pub fn layout_badge_circle_size_mm(number: usize) -> f64 {
    let digits = number.max(1).to_string().len() as f64;
    (1.0 + digits * 0.725).max(2.3) * 1.3
}

/// 도면 번호 채번 대상 유적 후보 (C++ `HeritageSpatialFeature`)
#[derive(Debug, Clone, PartialEq)]
pub struct HeritageSiteCandidate {
    pub site_name: String,
    pub era_category: String,
    pub dataset: HeritageDatasetKind,
    pub pos_xy: [f64; 2],
}

/// 도면 번호가 부여된 유적 범례 항목 (C++ `LegendItemSpec`)
#[derive(Debug, Clone, PartialEq)]
pub struct HeritageLegendEntry {
    pub index_number: usize,
    pub site_name: String,
    pub era_category: String,
    pub marker_color_hex: String,
    pub label_anchor_pos: [f64; 2],
}

/// 뷰포트 내 유적들을 북서(North-West: 높은 Y 우선, 같은 행에서는 낮은 X 우선)에서 남동(South-East) 순으로 정렬하여 `1..=N` 번호 부여
/// (C++ `HeritageLayoutNumbers::assignSequentialLayoutNumbers` 1:1 대응)
#[must_use]
pub fn assign_sequential_layout_numbers(
    sites: &[HeritageSiteCandidate],
    viewport_extent: [f64; 4],
) -> Vec<HeritageLegendEntry> {
    let [min_x, min_y, max_x, max_y] = [
        viewport_extent[0].min(viewport_extent[2]),
        viewport_extent[1].min(viewport_extent[3]),
        viewport_extent[0].max(viewport_extent[2]),
        viewport_extent[1].max(viewport_extent[3]),
    ];
    let mut filtered: Vec<&HeritageSiteCandidate> = sites
        .iter()
        .filter(|s| {
            s.pos_xy[0] >= min_x
                && s.pos_xy[0] <= max_x
                && s.pos_xy[1] >= min_y
                && s.pos_xy[1] <= max_y
        })
        .collect();

    // Sort North-to-South (descending Y), then West-to-East (ascending X)
    filtered.sort_by(|a, b| {
        b.pos_xy[1]
            .total_cmp(&a.pos_xy[1])
            .then_with(|| a.pos_xy[0].total_cmp(&b.pos_xy[0]))
    });

    filtered
        .into_iter()
        .enumerate()
        .map(|(i, site)| HeritageLegendEntry {
            index_number: i + 1,
            site_name: site.site_name.clone(),
            era_category: site.era_category.clone(),
            marker_color_hex: site.dataset.default_hex_color().to_owned(),
            label_anchor_pos: site.pos_xy,
        })
        .collect()
}
