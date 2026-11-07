#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SheetPaperSize {
    A4,
    A3,
    A2,
    A1,
    A0,
}

impl SheetPaperSize {
    #[must_use]
    pub const fn portrait_dimensions_mm(self) -> (f64, f64) {
        match self {
            Self::A4 => (210.0, 297.0),
            Self::A3 => (297.0, 420.0),
            Self::A2 => (420.0, 594.0),
            Self::A1 => (594.0, 841.0),
            Self::A0 => (841.0, 1189.0),
        }
    }

    #[must_use]
    pub const fn label_ko(self) -> &'static str {
        match self {
            Self::A4 => "A4",
            Self::A3 => "A3",
            Self::A2 => "A2",
            Self::A1 => "A1",
            Self::A0 => "A0",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SheetOrientation {
    Portrait,
    Landscape,
}

impl SheetOrientation {
    #[must_use]
    pub const fn label_ko(self) -> &'static str {
        match self {
            Self::Portrait => "세로",
            Self::Landscape => "가로",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SheetRectMm {
    pub x_mm: f64,
    pub y_mm: f64,
    pub width_mm: f64,
    pub height_mm: f64,
}

impl SheetRectMm {
    #[must_use]
    pub const fn left(&self) -> f64 {
        self.x_mm
    }

    #[must_use]
    pub const fn top(&self) -> f64 {
        self.y_mm
    }

    #[must_use]
    pub fn right(&self) -> f64 {
        self.x_mm + self.width_mm
    }

    #[must_use]
    pub fn bottom(&self) -> f64 {
        self.y_mm + self.height_mm
    }

    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.width_mm > 0.0
            && self.height_mm > 0.0
            && self.x_mm.is_finite()
            && self.y_mm.is_finite()
            && self.width_mm.is_finite()
            && self.height_mm.is_finite()
    }

    #[must_use]
    pub fn inside_sheet(&self, sheet_w: f64, sheet_h: f64) -> bool {
        self.x_mm >= 0.0
            && self.y_mm >= 0.0
            && (self.x_mm + self.width_mm) <= sheet_w
            && (self.y_mm + self.height_mm) <= sheet_h
    }
}

/// C++ `LayoutService::SheetChromeRects` (`target/cpp-ref/src/core/LayoutService.h`)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SheetChromeRects {
    pub map: SheetRectMm,
    pub scale_bar: SheetRectMm,
    pub scale_label: SheetRectMm,
    pub crs: SheetRectMm,
    pub north: SheetRectMm,
}

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

#[derive(Debug, Clone, PartialEq)]
pub struct DrawingSheet {
    pub paper_size: SheetPaperSize,
    pub orientation: SheetOrientation,
    pub width_mm: f64,
    pub height_mm: f64,
    pub map_frame: SheetRectMm,
    pub scale_bar_rect: SheetRectMm,
    pub scale_text_rect: SheetRectMm,
    pub crs_label_rect: SheetRectMm,
    pub north_arrow_rect: SheetRectMm,
    pub legend_rect: SheetRectMm,
    pub title_block_rect: SheetRectMm,
    pub legend_kept_on_page: bool,
    pub scale_denominator: u32,
    pub grid_enabled: bool,
    pub grid_interval_m: f64,
    pub grid_follow_map: bool,
    pub grid_offset_xy_m: [f64; 2],
    pub show_site_names_under_5000: bool,
    pub north_arrow_style: NorthArrowStyle,
    pub north_arrow_size_mm: f64,
    pub scale_bar_style: ScaleBarStyle,
    pub legend_title: String,
    pub legend_visible: bool,
    pub title_block_visible: bool,
    pub legend_font_pt: u32,
    pub legend_bold: bool,
    pub legend_italic: bool,
}

impl DrawingSheet {
    #[must_use]
    pub fn new(paper_size: SheetPaperSize, orientation: SheetOrientation) -> Self {
        let mut sheet = Self {
            paper_size,
            orientation,
            width_mm: 0.0,
            height_mm: 0.0,
            map_frame: SheetRectMm {
                x_mm: 0.0,
                y_mm: 0.0,
                width_mm: 0.0,
                height_mm: 0.0,
            },
            scale_bar_rect: SheetRectMm {
                x_mm: 0.0,
                y_mm: 0.0,
                width_mm: 0.0,
                height_mm: 0.0,
            },
            scale_text_rect: SheetRectMm {
                x_mm: 0.0,
                y_mm: 0.0,
                width_mm: 0.0,
                height_mm: 0.0,
            },
            crs_label_rect: SheetRectMm {
                x_mm: 0.0,
                y_mm: 0.0,
                width_mm: 0.0,
                height_mm: 0.0,
            },
            north_arrow_rect: SheetRectMm {
                x_mm: 0.0,
                y_mm: 0.0,
                width_mm: 0.0,
                height_mm: 0.0,
            },
            legend_rect: SheetRectMm {
                x_mm: 0.0,
                y_mm: 0.0,
                width_mm: 0.0,
                height_mm: 0.0,
            },
            title_block_rect: SheetRectMm {
                x_mm: 0.0,
                y_mm: 0.0,
                width_mm: 0.0,
                height_mm: 0.0,
            },
            legend_kept_on_page: true,
            scale_denominator: 5_000,
            grid_enabled: true,
            grid_interval_m: 100.0,
            grid_follow_map: false,
            grid_offset_xy_m: [0.0, 0.0],
            show_site_names_under_5000: false,
            north_arrow_style: NorthArrowStyle::SingleArrow,
            north_arrow_size_mm: 28.0,
            scale_bar_style: ScaleBarStyle::DoubleBox,
            legend_title: "범례".to_owned(),
            legend_visible: true,
            title_block_visible: true,
            legend_font_pt: 10,
            legend_bold: true,
            legend_italic: false,
        };
        sheet.recompute_layout();
        sheet
    }

    pub fn set_legend_kept_on_page(&mut self, kept_on_page: bool) {
        self.legend_kept_on_page = kept_on_page;
        self.recompute_layout();
    }

    /// `KaDrawingStudio::resetPaper` (`0001a93`, `c9e58e3`) — 용지를 바꿔도 축척자·축척 글·좌표계·방위는 아래 띠 자리로 고정하고 용지 밖 고립 항목을 복구
    pub fn change_paper(&mut self, paper_size: SheetPaperSize, orientation: SheetOrientation) {
        self.paper_size = paper_size;
        self.orientation = orientation;
        self.recompute_layout();
        self.replace_sheet_chrome();
        self.recover_orphaned_items();
    }

    /// `KaDrawingStudio::replaceSheetChrome` (`0001a93`) — `LayoutService::standardSheetChrome` (`target/cpp-ref/src/core/LayoutService.cpp:895`)으로 지도·축척자·축척 글·좌표계·방위·표제란 사각형 계산
    pub fn replace_sheet_chrome(&mut self) {
        let page = SheetRectMm {
            x_mm: 0.0,
            y_mm: 0.0,
            width_mm: self.width_mm,
            height_mm: self.height_mm,
        };
        let requested_map = default_map_rect_mm(self.width_mm, self.height_mm);
        let chrome = standard_sheet_chrome(page, requested_map);
        self.map_frame = chrome.map;
        self.scale_bar_rect = chrome.scale_bar;
        self.scale_text_rect = chrome.scale_label;
        self.crs_label_rect = chrome.crs;
        self.north_arrow_rect = chrome.north;
        self.title_block_rect = default_title_block_rect_mm(page, self.map_frame);
    }

    /// `KaDrawingStudio.cpp` (`c9e58e3`) — A3 -> A4 전환 등에서 용지 밖으로 밀려난 범례/장식을 `clamp(pos, 4.0, max(4.0, page - size - 4.0))`로 복구
    pub fn recover_orphaned_items(&mut self) {
        if self.legend_kept_on_page {
            let max_x = (self.width_mm - self.legend_rect.width_mm - 4.0).max(4.0);
            let max_y = (self.height_mm - self.legend_rect.height_mm - 4.0).max(4.0);
            self.legend_rect.x_mm = self.legend_rect.x_mm.clamp(4.0, max_x);
            self.legend_rect.y_mm = self.legend_rect.y_mm.clamp(4.0, max_y);
        }
        let tb_max_x = (self.width_mm - self.title_block_rect.width_mm - 4.0).max(4.0);
        let tb_max_y = (self.height_mm - self.title_block_rect.height_mm - 3.0).max(3.0);
        self.title_block_rect.x_mm = self.title_block_rect.x_mm.clamp(4.0, tb_max_x);
        self.title_block_rect.y_mm = self.title_block_rect.y_mm.clamp(3.0, tb_max_y);
    }

    /// `LayoutService::applyMapCoordinateGrid` (`5362ef6`) — 지도에 미터 좌표격자가 켜져 있고 북쪽이 위(회전 0°)일 때 도면 격자를 지도 좌표격자와 같은 간격·같은 교점 오프셋으로 동기화
    pub fn apply_map_coordinate_grid(
        &mut self,
        map_grid_enabled: bool,
        map_rotation_deg: f64,
        step_m: f64,
        offset_xy_m: [f64; 2],
    ) -> bool {
        let north_up = (map_rotation_deg % 360.0).abs() < 1e-6;
        if map_grid_enabled && north_up && step_m > 0.0 {
            self.grid_enabled = true;
            self.grid_follow_map = true;
            self.grid_interval_m = step_m;
            self.grid_offset_xy_m = [
                offset_xy_m[0].rem_euclid(step_m),
                offset_xy_m[1].rem_euclid(step_m),
            ];
            true
        } else {
            self.grid_follow_map = false;
            false
        }
    }

    fn recompute_layout(&mut self) {
        let (pw, ph) = self.paper_size.portrait_dimensions_mm();
        let (w, h) = match self.orientation {
            SheetOrientation::Portrait => (pw, ph),
            SheetOrientation::Landscape => (ph, pw),
        };
        self.width_mm = w;
        self.height_mm = h;

        self.replace_sheet_chrome();
        // KaDrawingStudio::defaultItemRect(kIdLegend): QRectF(mapR.right() - 52.0, mapR.top() + 6.0, 48.0, 74.0)
        self.legend_rect = if self.legend_kept_on_page {
            default_legend_rect_mm(self.map_frame)
        } else {
            SheetRectMm {
                x_mm: w + 15.0,
                y_mm: 10.0,
                width_mm: 48.0,
                height_mm: 74.0,
            }
        };
    }

    #[must_use]
    pub fn all_decorations_inside_sheet(&self) -> bool {
        let in_frame = self.map_frame.inside_sheet(self.width_mm, self.height_mm)
            && self
                .scale_bar_rect
                .inside_sheet(self.width_mm, self.height_mm)
            && self
                .scale_text_rect
                .inside_sheet(self.width_mm, self.height_mm)
            && self
                .crs_label_rect
                .inside_sheet(self.width_mm, self.height_mm)
            && self
                .north_arrow_rect
                .inside_sheet(self.width_mm, self.height_mm)
            && self
                .title_block_rect
                .inside_sheet(self.width_mm, self.height_mm);
        if self.legend_kept_on_page {
            in_frame && self.legend_rect.inside_sheet(self.width_mm, self.height_mm)
        } else {
            in_frame
        }
    }

    /// 지상 좌표 `world_xy` (m)를 현재 용지의 `map_frame` 내부 용지 밀리미터 좌표 `[x_mm, y_mm]`로 투영
    /// (`LayoutService::extentForPaperScale` 및 `layoutMapItemFromXy` 1:1 대응)
    #[must_use]
    pub fn world_to_sheet_mm(&self, center_world_xy: [f64; 2], world_xy: [f64; 2]) -> [f64; 2] {
        let scale = f64::from(self.scale_denominator.max(1));
        let mm_per_meter = 1000.0 / scale;
        let mc_x = self.map_frame.x_mm + self.map_frame.width_mm * 0.5;
        let mc_y = self.map_frame.y_mm + self.map_frame.height_mm * 0.5;
        [
            mc_x + (world_xy[0] - center_world_xy[0]) * mm_per_meter,
            mc_y - (world_xy[1] - center_world_xy[1]) * mm_per_meter,
        ]
    }

    /// C++ `PlateScaleBar::calculateDivisions(mapScale, targetWidthMm)` — 1-2-5 계열 정수 단위 축척막대 간격(m) 산출 1:1
    #[must_use]
    pub fn calculate_scale_bar_nice_interval_m(map_scale_denom: f64, target_width_mm: f64) -> f64 {
        Self::compute_nice_scale_bar_length(
            map_scale_denom,
            target_width_mm - 20.0,
            target_width_mm + 20.0,
        )
    }

    pub const HGIS_SCALEBAR_MIN_WIDTH_MM: f64 = 40.0;
    pub const HGIS_SCALEBAR_MAX_WIDTH_MM: f64 = 80.0;
    pub const HGIS_FALLBACK_SCALEBAR_LENGTH_M: f64 = 50.0;
    pub const HGIS_MAX_SCALEBAR_LENGTH_M: f64 = 100_000.0;
    pub const HGIS_NICE_MANTISSAS: [f64; 4] = [1.0, 2.0, 5.0, 10.0];

    /// C++ `KaScaleBarItem::computeNiceScaleBarLength` — 목표 폭 `[40.0mm, 80.0mm]` 및 `1·2·5·10` 가수 배열 기반 축척 막대 길이(m) 산출, 비정상 축척 시 `50.0m` 폴백
    #[must_use]
    pub fn compute_nice_scale_bar_length(
        map_scale_denom: f64,
        target_width_min_mm: f64,
        target_width_max_mm: f64,
    ) -> f64 {
        if map_scale_denom <= 1e-6 || !map_scale_denom.is_finite() {
            return Self::HGIS_FALLBACK_SCALEBAR_LENGTH_M;
        }
        let min_mm = target_width_min_mm.max(10.0);
        let max_mm = target_width_max_mm.max(min_mm);
        let mid_mm = 0.5 * (min_mm + max_mm);
        let raw_meters = map_scale_denom * (mid_mm / 1000.0);
        if raw_meters <= 1.0 {
            return 1.0;
        }
        let exp = raw_meters.log10().floor();
        let base = 10.0_f64.powf(exp);
        let mut best_len = Self::HGIS_FALLBACK_SCALEBAR_LENGTH_M;
        let mut best_dist = f64::INFINITY;
        for &m in &Self::HGIS_NICE_MANTISSAS {
            let cand_m = m * base;
            let bar_mm = (cand_m * 1000.0) / map_scale_denom;
            let in_range = (min_mm..=max_mm).contains(&bar_mm);
            let dist = if in_range {
                // 목표 범위 내에서는 5·2·1 정수 단위 중 mid_mm 이하에서 가장 큰 깔끔한 단위 우선
                if cand_m <= raw_meters {
                    raw_meters - cand_m
                } else {
                    (cand_m - raw_meters) + 1e3
                }
            } else {
                (bar_mm - mid_mm).abs() + 1e6
            };
            if dist < best_dist {
                best_dist = dist;
                best_len = cand_m;
            }
        }
        best_len.min(Self::HGIS_MAX_SCALEBAR_LENGTH_M)
    }

    /// C++ `KaLayoutComposer::setPaperOrientation` — 용지 가로/세로 전환 시 폭·높이 스왑 및 지도 중심 좌표 `(cx, cy)` 보존
    pub fn set_paper_orientation_preserving_center(
        &mut self,
        orientation: SheetOrientation,
        current_extent: [f64; 4],
    ) -> [f64; 4] {
        let cx = 0.5 * (current_extent[0] + current_extent[2]);
        let cy = 0.5 * (current_extent[1] + current_extent[3]);
        self.change_paper(self.paper_size, orientation);
        let scale = f64::from(self.scale_denominator.max(1));
        let half_w_m = (self.map_frame.width_mm / 1000.0) * scale * 0.5;
        let half_h_m = (self.map_frame.height_mm / 1000.0) * scale * 0.5;
        [cx - half_w_m, cy - half_h_m, cx + half_w_m, cy + half_h_m]
    }
}

/// C++ `RasterMaskClipper::clipRasterByPolygon` — 조사구역 내·외부 픽셀 알파 채널 마스킹(`inside=255`, `outside=round(outsideAlpha * 255)`) 1:1
pub fn apply_polygon_alpha_mask_rgba(
    rgba_buf: &mut [u8],
    width: usize,
    height: usize,
    inside_predicate: impl Fn(usize, usize) -> bool,
    outside_alpha_ratio: f32,
) {
    let out_a = (outside_alpha_ratio.clamp(0.0, 1.0) * 255.0).floor() as u8;
    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) * 4;
            if idx + 3 < rgba_buf.len() {
                rgba_buf[idx + 3] = if inside_predicate(x, y) { 255 } else { out_a };
            }
        }
    }
}

/// C++ `NorthArrowRenderer::computeGridConvergenceAngle` — TM 투영계 중심 경위도 기준 도북-진북 자오선 수렴각(도 단위) 계산
#[must_use]
pub fn compute_grid_convergence_angle_deg(
    lon_deg: f64,
    lat_deg: f64,
    central_meridian_deg: f64,
) -> f64 {
    let dlon_rad = (lon_deg - central_meridian_deg).to_radians();
    let lat_rad = lat_deg.to_radians();
    let sin_lat = lat_rad.sin();
    let cos_lat = lat_rad.cos();
    // GRS80 제2이심률 제곱 e'^2 ≈ 0.006739496742
    let eta2 = 0.006_739_496_742 * cos_lat * cos_lat;
    let gamma_rad = dlon_rad * sin_lat
        + (dlon_rad.powi(3) / 3.0) * sin_lat * cos_lat * cos_lat * (1.0 + 3.0 * eta2);
    gamma_rad.to_degrees()
}

/// C++ `LegendBoxComposer` — 고고학 도면 표준 범례 레이어 정렬 순서(1. 유물 점 -> 2. 기준점 -> 3. 단면선 -> 4. 유구 선 -> 5. 유구 면 -> 6. 트렌치 -> 7. 조사구역)
pub fn sort_legend_layers_canonical(layers: &mut [strata_types::schema::DomainLayerKey]) {
    use strata_types::schema::DomainLayerKey;
    let rank = |k: DomainLayerKey| -> u8 {
        match k {
            DomainLayerKey::ArtifactPoint => 1,
            DomainLayerKey::ControlPoints => 2,
            DomainLayerKey::SectionLine => 3,
            DomainLayerKey::FeatureLine => 4,
            DomainLayerKey::FeaturePoly => 5,
            DomainLayerKey::TrialTrench => 6,
            DomainLayerKey::SurveyArea => 7,
        }
    };
    layers.sort_by_key(|&k| rank(k));
}

/// C++ `TitleBlockComposer` (`QFontMetrics::elidedText(..., Qt::ElideRight)`) — 표제란 텍스트 우측 말줄임 처리
#[must_use]
pub fn elide_title_block_text_right(raw_text: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    let char_count = raw_text.chars().count();
    if char_count <= max_chars {
        return raw_text.to_owned();
    }
    let keep = max_chars.saturating_sub(1);
    let prefix: String = raw_text.chars().take(keep).collect();
    format!("{prefix}…")
}

pub const HGIS_TITLE_BLOCK_DEFAULT_W_MM: f64 = 180.0;
pub const HGIS_TITLE_BLOCK_ROW_H_MM: f64 = 8.0;
pub const HGIS_MIN_INSET_BOX_SIZE_MM: f64 = 3.0;
pub const HGIS_INSET_CROSSHAIR_LEN_MM: f64 = 2.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InsetExtentBoxMm {
    pub rect_mm: SheetRectMm,
    pub has_crosshair: bool,
    pub crosshair_len_mm: f64,
}

/// C++ `KaInsetMapItem` / `KaOverviewMapSync` — 광역 위치도 내 메인 지도 영역 사각형 최소 크기(`3.0mm × 3.0mm`) 보장 클램핑 및 중심 십자선(`2.0mm`) 표시
#[must_use]
pub fn compute_inset_overview_box_mm(projected_rect_mm: SheetRectMm) -> InsetExtentBoxMm {
    let cx = projected_rect_mm.x_mm + projected_rect_mm.width_mm * 0.5;
    let cy = projected_rect_mm.y_mm + projected_rect_mm.height_mm * 0.5;
    let clamped_w = projected_rect_mm.width_mm.max(HGIS_MIN_INSET_BOX_SIZE_MM);
    let clamped_h = projected_rect_mm.height_mm.max(HGIS_MIN_INSET_BOX_SIZE_MM);
    let has_crosshair = projected_rect_mm.width_mm <= HGIS_MIN_INSET_BOX_SIZE_MM
        || projected_rect_mm.height_mm <= HGIS_MIN_INSET_BOX_SIZE_MM;
    InsetExtentBoxMm {
        rect_mm: SheetRectMm {
            x_mm: cx - clamped_w * 0.5,
            y_mm: cy - clamped_h * 0.5,
            width_mm: clamped_w,
            height_mm: clamped_h,
        },
        has_crosshair,
        crosshair_len_mm: HGIS_INSET_CROSSHAIR_LEN_MM,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LegendLayerCandidate {
    pub layer_key: strata_types::schema::DomainLayerKey,
    pub visible: bool,
    pub feature_count: usize,
}

/// C++ `KaLegendItem::collectVisibleLegendEntries` — 가시(`visible == true`) 및 비어 있지 않은(`featureCount > 0`) 레이어만 범례 항목으로 수집하고 표준 순서로 정렬
#[must_use]
pub fn collect_visible_non_empty_legend_entries(
    candidates: &[LegendLayerCandidate],
) -> Vec<strata_types::schema::DomainLayerKey> {
    let mut keys: Vec<strata_types::schema::DomainLayerKey> = candidates
        .iter()
        .filter(|c| c.visible && c.feature_count > 0)
        .map(|c| c.layer_key)
        .collect();
    sort_legend_layers_canonical(&mut keys);
    keys
}

/// 단일 시트 내 최대 배치 뷰포트 수 (C++ `HGIS_MAX_FIGURES_PER_SHEET`)
pub const HGIS_MAX_FIGURES_PER_SHEET: usize = 8;
/// 주 평면도 기본 축척 분모 (1:500, C++ `HGIS_DEFAULT_PLAN_SCALE`)
pub const HGIS_DEFAULT_PLAN_SCALE: f64 = 500.0;
/// 세부 유구도 기본 축척 분모 (1:50, C++ `HGIS_DEFAULT_DETAIL_SCALE`)
pub const HGIS_DEFAULT_DETAIL_SCALE: f64 = 50.0;

/// 다중 뷰포트 프레임 역할 분류 (C++ `ViewportRole`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewportRole {
    /// 주 평면도
    MainPlan,
    /// 토층/유구 횡단면도
    CrossSection,
    /// 세부 유구 확대도
    DetailFeature,
}

/// 다중 뷰포트 프레임 설정 (C++ `ViewportFrameConfig`)
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportFrameConfig {
    pub role: ViewportRole,
    pub page_layout_rect_mm: SheetRectMm,
    pub world_extent: [f64; 4],
    pub scale_denominator: f64,
    pub auto_figure_label: String,
}

/// 축척 분모가 0 이하이거나 비정상일 때 기본 축척(`1:500`)으로 폴백
#[must_use]
pub fn sanitize_viewport_scale(scale_denominator: f64) -> f64 {
    if scale_denominator <= 0.0 || !scale_denominator.is_finite() {
        HGIS_DEFAULT_PLAN_SCALE
    } else {
        scale_denominator
    }
}

/// 시트 번호와 서브 프레임 인덱스에 따라 국가유산청 보고서 규격 도면 번호 포맷팅
/// (C++ `KaFigureNumberingEngine::formatFigureNumber` 1:1 대응)
#[must_use]
pub fn format_figure_number(sheet_no: usize, sub_index_1based: Option<usize>) -> String {
    let s = sheet_no.max(1);
    match sub_index_1based {
        Some(sub) => format!("도면 {s}-{}", sub.max(1)),
        None => format!("도면 {s}"),
    }
}

/// 단일 시트 내 다중 뷰포트 목록에 대해 `"도면 {sheet}-{idx}"` 연속 도면 번호 자동 채번
/// (C++ `KaMultiViewportSheet::layoutMultiViewportSheet` & `KaFigureNumberingEngine`)
#[must_use]
pub fn assign_cascading_figure_numbers(
    base_sheet_number: usize,
    roles: &[ViewportRole],
) -> Vec<String> {
    let clamped = roles.len().min(HGIS_MAX_FIGURES_PER_SHEET);
    (0..clamped)
        .map(|idx| format_figure_number(base_sheet_number, Some(idx + 1)))
        .collect()
}

pub const HGIS_LEADER_LINE_MAX_DIST_MM: f64 = 25.0;
pub const HGIS_LABEL_COLLISION_PENALTY: f64 = 1000.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelCandidatePos {
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Left,
    TopLeft,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LabelPlacementTask {
    pub fid: i64,
    pub label_text: String,
    pub feature_anchor_point_mm: [f64; 2],
    pub text_size_mm: [f64; 2],
    pub priority: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlacedLabelResult {
    pub fid: i64,
    pub final_label_rect_mm: SheetRectMm,
    pub needs_leader_line: bool,
    pub leader_line_path_mm: Vec<[f64; 2]>,
}

fn rects_overlap_mm(a: SheetRectMm, b: SheetRectMm) -> f64 {
    let ox = (a.x_mm + a.width_mm).min(b.x_mm + b.width_mm) - a.x_mm.max(b.x_mm);
    let oy = (a.y_mm + a.height_mm).min(b.y_mm + b.height_mm) - a.y_mm.max(b.y_mm);
    if ox > 0.0 && oy > 0.0 {
        ox * oy
    } else {
        0.0
    }
}

/// C++ `KaLabelPlacementEngine::solveLabelPlacement` & `KaLeaderLineRouter` — 8방향 후보 위치 비용 함수 최소화, 장애물 충돌 회피 및 2단 굴절 지시선(`leaderLinePath`) 생성
#[must_use]
pub fn solve_label_placement(
    tasks: &[LabelPlacementTask],
    obstacle_rects_mm: &[SheetRectMm],
    viewport_bounds_mm: SheetRectMm,
) -> Vec<PlacedLabelResult> {
    let dirs: [(LabelCandidatePos, [f64; 2], f64); 8] = [
        (LabelCandidatePos::TopRight, [1.0, 1.0], 0.0),
        (LabelCandidatePos::Right, [1.0, 0.0], 0.2),
        (LabelCandidatePos::Top, [0.0, 1.0], 0.4),
        (LabelCandidatePos::BottomRight, [1.0, -1.0], 0.6),
        (LabelCandidatePos::TopLeft, [-1.0, 1.0], 0.8),
        (LabelCandidatePos::Left, [-1.0, 0.0], 1.0),
        (LabelCandidatePos::Bottom, [0.0, -1.0], 1.2),
        (LabelCandidatePos::BottomLeft, [-1.0, -1.0], 1.4),
    ];
    let radii_mm = [2.0_f64, 6.0, 12.0, 20.0];
    let leader_threshold_mm = 5.0;

    let mut placed_obstacles: Vec<SheetRectMm> = obstacle_rects_mm.to_vec();
    let mut results = Vec::with_capacity(tasks.len());

    for task in tasks {
        let [ax, ay] = task.feature_anchor_point_mm;
        let [tw, th] = [task.text_size_mm[0].max(1.0), task.text_size_mm[1].max(1.0)];
        let mut best_rect: Option<(f64, SheetRectMm, f64)> = None;

        for &r in &radii_mm {
            if r > HGIS_LEADER_LINE_MAX_DIST_MM {
                continue;
            }
            for &(_, dir, dir_bias) in &dirs {
                let norm = dir[0].hypot(dir[1]).max(1.0);
                let cx = ax + (dir[0] / norm) * r;
                let cy = ay + (dir[1] / norm) * r;
                let cand_rect = SheetRectMm {
                    x_mm: if dir[0] >= 0.0 { cx } else { cx - tw },
                    y_mm: if dir[1] >= 0.0 { cy } else { cy - th },
                    width_mm: tw,
                    height_mm: th,
                };
                // Check inside viewport bounds
                let inside = cand_rect.x_mm >= viewport_bounds_mm.x_mm
                    && cand_rect.y_mm >= viewport_bounds_mm.y_mm
                    && (cand_rect.x_mm + cand_rect.width_mm)
                        <= (viewport_bounds_mm.x_mm + viewport_bounds_mm.width_mm)
                    && (cand_rect.y_mm + cand_rect.height_mm)
                        <= (viewport_bounds_mm.y_mm + viewport_bounds_mm.height_mm);
                if !inside {
                    continue;
                }
                let overlap_sum: f64 = placed_obstacles
                    .iter()
                    .map(|&obs| rects_overlap_mm(cand_rect, obs))
                    .sum();
                let cost = if overlap_sum > 0.0 {
                    HGIS_LABEL_COLLISION_PENALTY * (1.0 + overlap_sum)
                } else {
                    0.0
                } + 2.5 * r
                    + dir_bias;

                if best_rect.is_none_or(|(best_cost, _, _)| cost < best_cost) {
                    best_rect = Some((cost, cand_rect, r));
                }
            }
        }

        if let Some((_, rect, dist)) = best_rect {
            placed_obstacles.push(rect);
            let needs_leader_line = dist > leader_threshold_mm;
            let leader_line_path_mm = if needs_leader_line {
                let end_x = if rect.x_mm >= ax {
                    rect.x_mm
                } else {
                    rect.x_mm + rect.width_mm
                };
                let end_y = rect.y_mm + rect.height_mm * 0.5;
                let elbow_x = if rect.x_mm >= ax {
                    end_x - 3.0
                } else {
                    end_x + 3.0
                };
                vec![[ax, ay], [elbow_x, end_y], [end_x, end_y]]
            } else {
                Vec::new()
            };
            results.push(PlacedLabelResult {
                fid: task.fid,
                final_label_rect_mm: rect,
                needs_leader_line,
                leader_line_path_mm,
            });
        }
    }

    results
}

/// QGIS 조판 템플릿(`.qpt` XML) 내 동적 슬롯 플레이스홀더(`[% $KEY %]` 또는 `[%KEY%]`)를 바인딩 값으로 치환
/// (C++ `PrintLayoutTemplate::instantiateFromQpt` 1:1 대응)
#[must_use]
pub fn substitute_qpt_template_slots(template_xml: &str, bindings: &[(&str, &str)]) -> String {
    let mut out = template_xml.to_owned();
    for &(key, val) in bindings {
        let bare = key.trim().trim_start_matches('$');
        for pattern in [
            format!("[% ${bare} %]"),
            format!("[%${bare}%]"),
            format!("[% {bare} %]"),
            format!("[%{bare}%]"),
        ] {
            out = out.replace(&pattern, val);
        }
    }
    out
}

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

/// C++ `LayoutService::niceScaleBarSegmentMeters(double mapWidthMm, double scaleDenominator, int segments)` 1:1 대응
#[must_use]
pub fn nice_scale_bar_segment_meters(
    map_width_mm: f64,
    scale_denominator: f64,
    segments: u32,
) -> f64 {
    let segs = f64::from(segments.max(1));
    if !map_width_mm.is_finite()
        || !scale_denominator.is_finite()
        || map_width_mm <= 0.0
        || scale_denominator <= 0.0
    {
        return 1.0;
    }
    let target_bar_mm = (map_width_mm * 0.35).clamp(40.0, 160.0);
    let raw_seg = (target_bar_mm / 1000.0 * scale_denominator) / segs;
    const K_LEN: [f64; 19] = [
        0.1, 0.2, 0.4, 0.5, 1.0, 2.0, 4.0, 5.0, 10.0, 20.0, 40.0, 50.0, 100.0, 200.0, 400.0, 500.0,
        1000.0, 2000.0, 5000.0,
    ];
    let mut best = K_LEN[0];
    let mut best_rel = f64::INFINITY;
    for &n in &K_LEN {
        let rel = (raw_seg - n).abs() / n;
        if rel < best_rel {
            best_rel = rel;
            best = n;
        }
    }
    best
}

/// C++ `LayoutService::niceGridIntervalMeters(double scaleDenominator, double mapWidthMm)` 1:1 대응
#[must_use]
pub fn nice_grid_interval_meters(scale_denominator: f64, map_width_mm: f64) -> f64 {
    if !scale_denominator.is_finite() || scale_denominator <= 0.0 {
        return 100.0;
    }
    let paper_mm = if map_width_mm > 0.0 && map_width_mm.is_finite() {
        map_width_mm
    } else {
        180.0
    };
    let target_mm = (paper_mm * 0.24).clamp(25.0, 70.0);
    let raw_m = target_mm / 1000.0 * scale_denominator;
    const K_STEPS: [f64; 18] = [
        0.1, 0.2, 0.5, 1.0, 2.0, 5.0, 10.0, 20.0, 25.0, 50.0, 100.0, 200.0, 250.0, 500.0, 1000.0,
        2000.0, 5000.0, 10000.0,
    ];
    let mut best = K_STEPS[0];
    let mut best_rel = f64::INFINITY;
    for &s in &K_STEPS {
        let rel = (raw_m - s).abs() / s;
        if rel < best_rel {
            best_rel = rel;
            best = s;
        }
    }
    best
}

/// C++ `KaTitleBlock::text` (`target/cpp-ref/src/app/KaTitleBlock.cpp:57-65`) — 표제란 5행 생성
/// `도면명 / 조사명 / 유적명 / 축  척 / 작성일`, 빈 값은 `―`(U+2015)로 채움 (`orDash`)
#[must_use]
pub fn format_title_block_lines(
    drawing_title: &str,
    survey_name: &str,
    site_name: &str,
    scale_denominator: u32,
    date_ymd: &str,
) -> [String; 5] {
    let or_dash = |s: &str| -> String {
        let t = s.trim();
        if t.is_empty() {
            "―".to_owned()
        } else {
            t.to_owned()
        }
    };
    let scale_str = if scale_denominator > 0 {
        format!("1 : {scale_denominator}")
    } else {
        "1 : ―".to_owned()
    };
    [
        format!("도면명   {}", or_dash(drawing_title)),
        format!("조사명   {}", or_dash(survey_name)),
        format!("유적명   {}", or_dash(site_name)),
        format!("축  척   {scale_str}"),
        format!("작성일   {}", or_dash(date_ymd)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paper_size_and_orientation_change_keeps_decorations_in_bounds_and_preserves_off_page_legend()
    {
        let mut sheet = DrawingSheet::new(SheetPaperSize::A4, SheetOrientation::Portrait);
        assert!(sheet.all_decorations_inside_sheet());
        sheet.set_legend_kept_on_page(false);
        assert!(sheet.legend_rect.x_mm >= sheet.width_mm);

        sheet.change_paper(SheetPaperSize::A3, SheetOrientation::Landscape);
        assert_eq!((sheet.width_mm, sheet.height_mm), (420.0, 297.0));
        assert!(sheet.all_decorations_inside_sheet());
        assert!(!sheet.legend_kept_on_page);
        assert!(sheet.legend_rect.x_mm >= sheet.width_mm);
    }

    #[test]
    fn paper_change_puts_chrome_back_in_bottom_strip_and_recovers_orphaned_legend() {
        let mut sheet = DrawingSheet::new(SheetPaperSize::A3, SheetOrientation::Landscape);
        // Drag chrome and legend to arbitrary/orphaned coordinates
        sheet.scale_bar_rect.x_mm = 380.0;
        sheet.scale_bar_rect.y_mm = 10.0;
        sheet.north_arrow_rect.x_mm = 5.0;
        sheet.north_arrow_rect.y_mm = 5.0;
        sheet.legend_rect.x_mm = 390.0;
        sheet.legend_rect.y_mm = 270.0;

        // Switch down to A4 Portrait
        sheet.change_paper(SheetPaperSize::A4, SheetOrientation::Portrait);
        assert!(sheet.all_decorations_inside_sheet());
        let expected_row_top = sheet.map_frame.bottom() + 7.0;
        assert!((sheet.scale_bar_rect.y_mm - expected_row_top).abs() < 1e-6);
        assert!((sheet.crs_label_rect.y_mm - (expected_row_top + 2.5)).abs() < 1e-6);
        assert!((sheet.north_arrow_rect.y_mm - expected_row_top).abs() < 1e-6);
    }

    #[test]
    fn map_coordinate_grid_syncs_step_and_offset_only_when_north_up() {
        let mut sheet = DrawingSheet::new(SheetPaperSize::A4, SheetOrientation::Portrait);
        assert!(sheet.apply_map_coordinate_grid(true, 0.0, 20.0, [222_385.0, 260_115.0]));
        assert!(sheet.grid_follow_map);
        assert!((sheet.grid_interval_m - 20.0).abs() < 1e-6);
        assert!((sheet.grid_offset_xy_m[0] - 5.0).abs() < 1e-6);
        assert!((sheet.grid_offset_xy_m[1] - 15.0).abs() < 1e-6);

        // Rotated map grid cannot be applied to sheet coordinate grid
        assert!(!sheet.apply_map_coordinate_grid(true, 15.0, 20.0, [0.0, 0.0]));
        assert!(!sheet.grid_follow_map);
    }

    #[test]
    fn test_plate_scale_bar_nice_interval_1200_at_60mm_yields_50m_and_raster_mask_alpha_255_76() {
        let interval = DrawingSheet::calculate_scale_bar_nice_interval_m(1_200.0, 60.0);
        assert!((interval - 50.0).abs() < 1e-6);

        let mut rgba = vec![200_u8; 4 * 2 * 4];
        apply_polygon_alpha_mask_rgba(&mut rgba, 2, 2, |x, y| x == 0 && y == 0, 0.3);
        assert_eq!(rgba[3], 255);
        assert_eq!(rgba[7], 76);
        assert_eq!(rgba[11], 76);
    }

    #[test]
    fn test_north_arrow_grid_convergence_and_legend_sort_and_title_block_elide() {
        use strata_types::schema::DomainLayerKey;

        // 중부원점(127.0E) 정중앙에서는 수렴각 0.0도, 128.0E(위도 36.0N)에서는 양(+)의 수렴각
        let zero_conv = compute_grid_convergence_angle_deg(127.0, 36.0, 127.0);
        assert!(zero_conv.abs() < 1e-9);
        let east_conv = compute_grid_convergence_angle_deg(128.0, 36.0, 127.0);
        assert!((east_conv - 36.0_f64.to_radians().sin()).abs() < 1e-3);

        let mut layers = vec![
            DomainLayerKey::SurveyArea,
            DomainLayerKey::FeaturePoly,
            DomainLayerKey::ArtifactPoint,
            DomainLayerKey::TrialTrench,
            DomainLayerKey::FeatureLine,
        ];
        sort_legend_layers_canonical(&mut layers);
        assert_eq!(
            layers,
            vec![
                DomainLayerKey::ArtifactPoint,
                DomainLayerKey::FeatureLine,
                DomainLayerKey::FeaturePoly,
                DomainLayerKey::TrialTrench,
                DomainLayerKey::SurveyArea,
            ]
        );

        let elided = elide_title_block_text_right("경주 월성 발굴조사 지구 종합현황도", 10);
        assert_eq!(elided.chars().count(), 10);
        assert!(elided.ends_with('…'));
    }

    #[test]
    fn test_scale_bar_nice_1000_yields_50m_and_a4_orientation_swap_preserves_center() {
        let len_1000 = DrawingSheet::compute_nice_scale_bar_length(1_000.0, 40.0, 80.0);
        assert!((len_1000 - 50.0).abs() < 1e-9);
        assert!(
            (DrawingSheet::compute_nice_scale_bar_length(-5.0, 40.0, 80.0) - 50.0).abs() < 1e-9
        );

        let mut sheet = DrawingSheet::new(SheetPaperSize::A4, SheetOrientation::Landscape);
        assert_eq!((sheet.width_mm, sheet.height_mm), (297.0, 210.0));
        let new_ext = sheet.set_paper_orientation_preserving_center(
            SheetOrientation::Portrait,
            [199_900.0, 499_900.0, 200_100.0, 500_100.0],
        );
        assert_eq!((sheet.width_mm, sheet.height_mm), (210.0, 297.0));
        let cx = 0.5 * (new_ext[0] + new_ext[2]);
        let cy = 0.5 * (new_ext[1] + new_ext[3]);
        assert!((cx - 200_000.0).abs() < 1e-6);
        assert!((cy - 500_000.0).abs() < 1e-6);
    }

    #[test]
    fn test_exclude_empty_layer_from_legend_4_visible_with_1_empty_yields_3() {
        use strata_types::schema::DomainLayerKey;
        let candidates = [
            LegendLayerCandidate {
                layer_key: DomainLayerKey::SurveyArea,
                visible: true,
                feature_count: 10,
            },
            LegendLayerCandidate {
                layer_key: DomainLayerKey::TrialTrench,
                visible: true,
                feature_count: 5,
            },
            LegendLayerCandidate {
                layer_key: DomainLayerKey::FeaturePoly,
                visible: true,
                feature_count: 0,
            },
            LegendLayerCandidate {
                layer_key: DomainLayerKey::ArtifactPoint,
                visible: true,
                feature_count: 2,
            },
        ];
        let entries = collect_visible_non_empty_legend_entries(&candidates);
        assert_eq!(entries.len(), 3);
        assert!(!entries.contains(&DomainLayerKey::FeaturePoly));

        let tiny = SheetRectMm {
            x_mm: 20.0,
            y_mm: 30.0,
            width_mm: 1.2,
            height_mm: 0.8,
        };
        let inset_box = compute_inset_overview_box_mm(tiny);
        assert!((inset_box.rect_mm.width_mm - 3.0).abs() < 1e-9);
        assert!((inset_box.rect_mm.height_mm - 3.0).abs() < 1e-9);
        assert!(inset_box.has_crosshair);

        // TestKaFigureNumberingEngine::testCascadingFigureNumbering
        let roles = [
            ViewportRole::MainPlan,
            ViewportRole::CrossSection,
            ViewportRole::DetailFeature,
        ];
        let labels = assign_cascading_figure_numbers(2, &roles);
        assert_eq!(labels, vec!["도면 2-1", "도면 2-2", "도면 2-3"]);
        assert!((sanitize_viewport_scale(0.0) - HGIS_DEFAULT_PLAN_SCALE).abs() < 1e-6);

        // TestKaLabelPlacementEngine::testCollisionAvoidanceLeaderLine
        let obstacles = [SheetRectMm {
            x_mm: 40.0,
            y_mm: 40.0,
            width_mm: 20.0,
            height_mm: 20.0,
        }];
        let tasks = [LabelPlacementTask {
            fid: 1,
            label_text: "1호 주거지".to_string(),
            feature_anchor_point_mm: [50.0, 50.0],
            text_size_mm: [14.0, 5.0],
            priority: 1.0,
        }];
        let viewport = SheetRectMm {
            x_mm: 0.0,
            y_mm: 0.0,
            width_mm: 200.0,
            height_mm: 200.0,
        };
        let placed = solve_label_placement(&tasks, &obstacles, viewport);
        assert_eq!(placed.len(), 1);
        assert!(placed[0].needs_leader_line);
        assert_eq!(
            placed[0].leader_line_path_mm.first().copied(),
            Some([50.0, 50.0])
        );
        assert!(rects_overlap_mm(placed[0].final_label_rect_mm, obstacles[0]) <= 1e-9);

        // TestLayoutScaleBar::testDoubleBoxSegmentWidthCalculation
        let seg_w = compute_scalebar_segment_width_mm(500.0, 10.0);
        assert!((seg_w - 20.0).abs() < 1e-4);
        assert!((compute_scalebar_segment_width_mm(500.0, 0.0) - 2.0).abs() < 1e-4);

        // TestLayoutNorthArrow::testNorthArrowRotationWithDeclination
        let rot = compute_north_arrow_rotation_with_declination(30.0, -5.5);
        assert!((rot - 24.5).abs() < 1e-6);

        // TestPrintLayoutTemplate::testSlotSubstitution
        let qpt = r#"<LayoutItemLabel text="조사명: [% $SURVEY_NAME %]"/>"#;
        let instantiated =
            substitute_qpt_template_slots(qpt, &[("$SURVEY_NAME", "경주 황남동 유적")]);
        assert_eq!(
            instantiated,
            r#"<LayoutItemLabel text="조사명: 경주 황남동 유적"/>"#
        );

        // TestHeritageLayoutNumbers::testNorthWestOrdering
        let sites = [
            HeritageSiteCandidate {
                site_name: "유적B".to_owned(),
                era_category: "신라".to_owned(),
                dataset: HeritageDatasetKind::DesignatedHeritage,
                pos_xy: [300.0, 200.0],
            },
            HeritageSiteCandidate {
                site_name: "유적A".to_owned(),
                era_category: "청동기".to_owned(),
                dataset: HeritageDatasetKind::BuriedHeritageArea,
                pos_xy: [100.0, 500.0],
            },
        ];
        let legend = assign_sequential_layout_numbers(&sites, [0.0, 0.0, 1000.0, 1000.0]);
        assert_eq!(legend.len(), 2);
        assert_eq!(legend[0].index_number, 1);
        assert_eq!(legend[0].site_name, "유적A");
        assert_eq!(legend[1].index_number, 2);
        assert_eq!(legend[1].site_name, "유적B");
        assert_eq!(ink_for_badge_rgb([255, 255, 0]), [0, 0, 0]);
        assert_eq!(ink_for_badge_rgb([231, 76, 60]), [255, 255, 255]);
        assert!(layout_badge_circle_size_mm(12) > layout_badge_circle_size_mm(1));
    }

    #[test]
    fn test_world_to_sheet_mm_scale_and_nice_scalebar_grid_and_title_block() {
        let mut sheet = DrawingSheet::new(SheetPaperSize::A4, SheetOrientation::Portrait);
        sheet.scale_denominator = 5_000;
        let center = [219_745.6, 360_930.5];
        let p_center = sheet.world_to_sheet_mm(center, center);
        let p_east_500m = sheet.world_to_sheet_mm(center, [center[0] + 500.0, center[1]]);
        // At 1:5,000, 500m on ground = 100mm on paper
        assert!((p_east_500m[0] - p_center[0] - 100.0).abs() < 1e-6);

        let seg_m = nice_scale_bar_segment_meters(186.0, 5_000.0, 4);
        assert!((seg_m - 100.0).abs() < 1e-6);
        let grid_m = nice_grid_interval_meters(5_000.0, 186.0);
        assert!((grid_m - 250.0).abs() < 1e-6);

        let tb_lines = format_title_block_lines("조사도면", "조사구역", "", 5_000, "2026-11-08");
        assert_eq!(tb_lines[0], "도면명   조사도면");
        assert_eq!(tb_lines[1], "조사명   조사구역");
        assert_eq!(tb_lines[2], "유적명   ―");
        assert_eq!(tb_lines[3], "축  척   1 : 5000");
        assert_eq!(tb_lines[4], "작성일   2026-11-08");
        assert!(sheet.all_decorations_inside_sheet());
    }
}
