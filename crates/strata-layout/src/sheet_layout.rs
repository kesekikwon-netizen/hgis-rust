//! 조판 크롬(지도·표제란·범례 자리)과 축척/방위 표시.
//!
//! `sheet_composer` 에서 분리했다.

use crate::sheet_geometry::{SheetChromeRects, SheetRectMm};

/// C++ `KaDrawingStudio::defaultMapRect()` (`target/cpp-ref/src/app/KaDrawingStudio.cpp:1344`)
#[must_use]
pub fn default_map_rect_mm(paper_w_mm: f64, paper_h_mm: f64) -> SheetRectMm {
    const K_FIELD_EDGE_MM: f64 = 10.0;
    const K_FIELD_BOTTOM_CHROME_MM: f64 = 38.0;
    SheetRectMm {
        x_mm: K_FIELD_EDGE_MM,
        y_mm: K_FIELD_EDGE_MM,
        width_mm: (paper_w_mm - K_FIELD_EDGE_MM * 2.0).max(40.0),
        height_mm: (paper_h_mm - K_FIELD_EDGE_MM - K_FIELD_BOTTOM_CHROME_MM).max(40.0),
    }
}

/// C++ `LayoutService::standardSheetChrome(const QRectF& page, const QRectF& requestedMap)`
/// (`target/cpp-ref/src/core/LayoutService.cpp:895-966`) 1:1 이식
#[must_use]
pub fn standard_sheet_chrome(page: SheetRectMm, requested_map: SheetRectMm) -> SheetChromeRects {
    const K_CHROME_H: f64 = 33.0;
    const K_GAP: f64 = 7.0;
    const K_MARGIN: f64 = 8.0;
    const K_MIN_MAP_H: f64 = 40.0;
    const K_BAR_H: f64 = 12.0;
    const K_LABEL_H: f64 = 7.0;
    const K_LABEL_GAP: f64 = 2.5;
    const K_NORTH: f64 = 20.0;
    const K_CRS_H: f64 = 7.0;
    const K_CRS_W: f64 = 58.0;
    const K_CRS_GAP: f64 = 6.0;

    let mut map = requested_map;
    if !map.is_valid() || map.width_mm < 8.0 || map.height_mm < 8.0 {
        map = SheetRectMm {
            x_mm: K_MARGIN,
            y_mm: K_MARGIN,
            width_mm: (page.width_mm - 2.0 * K_MARGIN).max(40.0),
            height_mm: (page.height_mm - 2.0 * K_MARGIN - K_CHROME_H).max(K_MIN_MAP_H),
        };
    }

    if map.bottom() + K_CHROME_H > page.bottom() - K_MARGIN {
        let max_bottom = page.bottom() - K_MARGIN - K_CHROME_H;
        let new_h = (max_bottom - map.top()).max(K_MIN_MAP_H);
        map.height_mm = new_h;
    }

    let row_top = map.bottom() + K_GAP;
    let mut north = SheetRectMm {
        x_mm: map.right() - K_NORTH,
        y_mm: row_top,
        width_mm: K_NORTH,
        height_mm: K_NORTH,
    };
    let mut crs = SheetRectMm {
        x_mm: north.left() - K_CRS_GAP - K_CRS_W,
        y_mm: row_top + ((K_BAR_H - K_CRS_H) * 0.5).max(0.0),
        width_mm: K_CRS_W,
        height_mm: K_CRS_H,
    };

    let bar_max_right = crs.left() - 8.0;
    let bar_w = 88.0_f64
        .min((map.width_mm * 0.38).max(48.0))
        .min((bar_max_right - map.left()).max(32.0));
    let mut scale_bar = SheetRectMm {
        x_mm: map.left(),
        y_mm: row_top,
        width_mm: bar_w,
        height_mm: K_BAR_H,
    };
    let mut scale_label = SheetRectMm {
        x_mm: map.left(),
        y_mm: scale_bar.bottom() + K_LABEL_GAP,
        width_mm: bar_w,
        height_mm: K_LABEL_H,
    };

    let clamp_on_page = |r: &mut SheetRectMm| {
        if r.top() < map.bottom() {
            r.y_mm = map.bottom();
        }
        if r.bottom() > page.bottom() - K_MARGIN {
            r.height_mm = (page.bottom() - K_MARGIN - r.top()).max(4.0);
        }
        if r.left() < page.left() {
            r.x_mm = page.left();
        }
        if r.right() > page.right() {
            r.width_mm = (page.right() - r.left()).max(4.0);
        }
    };
    clamp_on_page(&mut scale_bar);
    clamp_on_page(&mut scale_label);
    clamp_on_page(&mut north);
    clamp_on_page(&mut crs);

    if crs.right() > north.left() - 2.0 {
        let new_right = (crs.left() + 4.0).max(north.left() - 2.0);
        crs.width_mm = new_right - crs.left();
    }
    if scale_bar.right() > crs.left() - 4.0 {
        scale_bar.width_mm = (crs.left() - 4.0 - scale_bar.left()).max(32.0);
    }
    scale_label.x_mm = scale_bar.left();
    scale_label.width_mm = scale_bar.width_mm;
    if scale_label.top() < scale_bar.bottom() {
        scale_label.y_mm = scale_bar.bottom() + K_LABEL_GAP;
    }
    clamp_on_page(&mut scale_label);

    SheetChromeRects {
        map,
        scale_bar,
        scale_label,
        crs,
        north,
    }
}

/// C++ `KaTitleBlock::defaultRect(const QRectF& page, const QRectF& mapRect)`
/// (`target/cpp-ref/src/app/KaTitleBlock.cpp:67-77`) 1:1 이식
#[must_use]
pub fn default_title_block_rect_mm(page: SheetRectMm, map_rect: SheetRectMm) -> SheetRectMm {
    let chrome = standard_sheet_chrome(page, map_rect);
    let top = chrome.crs.bottom() + 1.5;
    let height = 18.0_f64.min(page.bottom() - 3.0 - top);
    let width = (chrome.north.left() - 2.0 - chrome.crs.left()).max(0.0);
    if height >= 13.0 && width >= 50.0 {
        return SheetRectMm {
            x_mm: chrome.crs.left(),
            y_mm: top,
            width_mm: width,
            height_mm: height,
        };
    }
    let map = chrome.map;
    let w = (map.width_mm * 0.4).clamp(40.0, 64.0);
    SheetRectMm {
        x_mm: map.right() - w - 2.0,
        y_mm: map.bottom() - 19.0,
        width_mm: w,
        height_mm: 17.0,
    }
}

/// C++ `KaDrawingStudio::defaultItemRect(kIdLegend)` (`target/cpp-ref/src/app/KaDrawingStudio.cpp:2321-2322`)
#[must_use]
pub fn default_legend_rect_mm(map_rect: SheetRectMm) -> SheetRectMm {
    SheetRectMm {
        x_mm: (map_rect.right() - 52.0).max(map_rect.left() + 4.0),
        y_mm: map_rect.top() + 6.0,
        width_mm: 48.0,
        height_mm: 74.0,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NorthArrowStyle {
    LetterOnly,
    #[default]
    SingleArrow,
    CompassRose,
    WindRose,
}

impl NorthArrowStyle {
    pub const ALL: [Self; 4] = [
        Self::LetterOnly,
        Self::SingleArrow,
        Self::CompassRose,
        Self::WindRose,
    ];

    #[must_use]
    pub const fn label_ko(self) -> &'static str {
        match self {
            Self::LetterOnly => "북 글자",
            Self::SingleArrow => "북 화살",
            Self::CompassRose => "나침반",
            Self::WindRose => "바람장미",
        }
    }

    #[must_use]
    pub const fn symbol_glyph(self) -> &'static str {
        match self {
            Self::LetterOnly => "▲\nN",
            Self::SingleArrow => "▲",
            Self::CompassRose => "◎",
            Self::WindRose => "✳",
        }
    }

    #[must_use]
    pub const fn svg_resource_path(self) -> &'static str {
        match self {
            Self::LetterOnly => ":/symbols/north_arrows/letter_n.svg",
            Self::SingleArrow => ":/symbols/north_arrows/single_arrow.svg",
            Self::CompassRose => ":/symbols/north_arrows/compass_rose.svg",
            Self::WindRose => ":/symbols/north_arrows/wind_rose.svg",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScaleBarStyle {
    #[default]
    DoubleBox,
    SingleBox,
    SteppedTicks,
}

impl ScaleBarStyle {
    pub const ALL: [Self; 3] = [Self::DoubleBox, Self::SingleBox, Self::SteppedTicks];

    #[must_use]
    pub const fn label_ko(self) -> &'static str {
        match self {
            Self::DoubleBox => "쌍칸",
            Self::SingleBox => "외칸",
            Self::SteppedTicks => "눈금",
        }
    }

    #[must_use]
    pub const fn symbol_glyph(self) -> &'static str {
        match self {
            Self::DoubleBox => "■■",
            Self::SingleBox => "□",
            Self::SteppedTicks => "lll",
        }
    }
}

/// 기본 축척자 분할 칸 수 (4, C++ `HGIS_DEFAULT_SCALEBAR_SEGMENTS`)
pub const HGIS_DEFAULT_SCALEBAR_SEGMENTS: u32 = 4;

/// 축척자 표준 세로 폭 (3.0 mm, C++ `HGIS_DEFAULT_SCALEBAR_HEIGHT_MM`)
pub const HGIS_DEFAULT_SCALEBAR_HEIGHT_MM: f64 = 3.0;

/// 축척 분모(`S`) 및 세그먼트 지상 거리(`L` m)로부터 용지상 물리 가로 폭(mm) 산출
/// (`W_seg_mm = (L_meters * 1000.0) / S`, 0m 이하 입력은 1.0m로 강제 클램핑, `S <= 0`은 500.0 폴백)
#[must_use]
pub fn compute_scalebar_segment_width_mm(scale_denominator: f64, segment_units_meters: f64) -> f64 {
    let eff_scale = if scale_denominator.is_finite() && scale_denominator > 0.0 {
        scale_denominator
    } else {
        500.0
    };
    let eff_meters = if segment_units_meters.is_finite() {
        segment_units_meters.max(1.0)
    } else {
        1.0
    };
    (eff_meters * 1000.0) / eff_scale
}

/// 캔버스 회전각(`canvas_rotation_deg`)과 도북-자북 편차각(`declination_deg`)을 합산한 방위표 회전 각도 산출
/// (C++ `LayoutNorthArrow::applyNorthArrowStyle` 1:1 대응)
#[must_use]
pub fn compute_north_arrow_rotation_with_declination(
    canvas_rotation_deg: f64,
    declination_deg: f64,
) -> f64 {
    let c = if canvas_rotation_deg.is_finite() {
        canvas_rotation_deg
    } else {
        0.0
    };
    let d = if declination_deg.is_finite() {
        declination_deg
    } else {
        0.0
    };
    c + d
}
