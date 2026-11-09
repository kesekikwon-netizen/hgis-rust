//! `DrawingSheet` — 도면 한 장의 상태와 레이아웃 재계산.
//!
//! `impl DrawingSheet` 가 332줄이라 두 갈래로 나눴다.
//! 축척자·격자 쪽은 `sheet_scalebar` 에 있다.

use crate::sheet_geometry::{SheetChromeRects, SheetOrientation, SheetPaperSize, SheetRectMm};
use crate::sheet_layout::{
    default_legend_rect_mm, default_map_rect_mm, default_title_block_rect_mm,
    standard_sheet_chrome, NorthArrowStyle, ScaleBarStyle,
};

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

    /// `KaDrawingStudio::replaceSheetChrome` (`0001a93`) — 장식 넷(축척자·축척 글·좌표계·방위)을 어디에 끌어 두었든 새 용지의 아래 띠 자리로 되돌림
    pub fn replace_sheet_chrome(&mut self) {
        let w = self.width_mm;
        let h = self.height_mm;
        let margin = 12.0;
        let bottom_band_h = 22.0;
        let band_y = h - margin - bottom_band_h + 2.0;
        self.scale_bar_rect = SheetRectMm {
            x_mm: margin,
            y_mm: band_y,
            width_mm: 55.0,
            height_mm: 8.5,
        };
        self.scale_text_rect = SheetRectMm {
            x_mm: margin,
            y_mm: band_y + 9.0,
            width_mm: 55.0,
            height_mm: 7.0,
        };
        self.crs_label_rect = SheetRectMm {
            x_mm: margin + 60.0,
            y_mm: band_y,
            width_mm: 65.0,
            height_mm: 16.0,
        };
        self.north_arrow_rect = SheetRectMm {
            x_mm: (w - margin - 18.0).max(margin + 128.0),
            y_mm: band_y,
            width_mm: 16.0,
            height_mm: 16.0,
        };
        let page = SheetRectMm {
            x_mm: 0.0,
            y_mm: 0.0,
            width_mm: w,
            height_mm: h,
        };
        self.title_block_rect = default_title_block_rect_mm(page, self.map_frame);
    }

    /// C++ `LayoutService::standardSheetChrome(page, defaultMapRect())` (`LayoutService.cpp:895`) 결과 반환
    #[must_use]
    pub fn standard_chrome(&self) -> SheetChromeRects {
        let page = SheetRectMm {
            x_mm: 0.0,
            y_mm: 0.0,
            width_mm: self.width_mm,
            height_mm: self.height_mm,
        };
        standard_sheet_chrome(page, default_map_rect_mm(self.width_mm, self.height_mm))
    }

    /// C++ `KaTitleBlock::defaultRect(page, mapRect)` (`KaTitleBlock.cpp:67`) 결과 반환
    #[must_use]
    pub fn default_title_block_rect(&self) -> SheetRectMm {
        let page = SheetRectMm {
            x_mm: 0.0,
            y_mm: 0.0,
            width_mm: self.width_mm,
            height_mm: self.height_mm,
        };
        let chrome = self.standard_chrome();
        default_title_block_rect_mm(page, chrome.map)
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

    fn recompute_layout(&mut self) {
        let (pw, ph) = self.paper_size.portrait_dimensions_mm();
        let (w, h) = match self.orientation {
            SheetOrientation::Portrait => (pw, ph),
            SheetOrientation::Landscape => (ph, pw),
        };
        self.width_mm = w;
        self.height_mm = h;

        let margin = 12.0;
        let bottom_band_h = 22.0;
        self.map_frame = SheetRectMm {
            x_mm: margin,
            y_mm: margin,
            width_mm: (w - margin * 2.0).max(50.0),
            height_mm: (h - margin * 2.0 - bottom_band_h).max(50.0),
        };
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
}
