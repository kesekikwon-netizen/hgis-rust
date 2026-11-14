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
    // target/cpp-ref/src/app/KaDrawingStudio.h:58-67 — A4 portrait, legacy A4 landscape,
    // and the 317x220 mm field sheet (A4 landscape + 10 mm on left, right, and top).
    pub const A4_PORTRAIT_WIDTH_MM: f64 = 210.0;
    pub const A4_PORTRAIT_HEIGHT_MM: f64 = 297.0;
    pub const A4_LANDSCAPE_WIDTH_MM: f64 = 297.0;
    pub const A4_LANDSCAPE_HEIGHT_MM: f64 = 210.0;
    pub const FIELD_PAPER_EXTRA_MM: f64 = 10.0;
    pub const FIELD_PAPER_WIDTH_MM: f64 =
        Self::A4_LANDSCAPE_WIDTH_MM + Self::FIELD_PAPER_EXTRA_MM * 2.0;
    pub const FIELD_PAPER_HEIGHT_MM: f64 =
        Self::A4_LANDSCAPE_HEIGHT_MM + Self::FIELD_PAPER_EXTRA_MM;

    #[must_use]
    pub fn new(paper_size: SheetPaperSize, orientation: SheetOrientation) -> Self {
        let zero = SheetRectMm { x_mm: 0.0, y_mm: 0.0, width_mm: 0.0, height_mm: 0.0 };
        let mut sheet = Self {
            paper_size,
            orientation,
            width_mm: 0.0,
            height_mm: 0.0,
            map_frame: zero,
            scale_bar_rect: zero,
            scale_text_rect: zero,
            crs_label_rect: zero,
            north_arrow_rect: zero,
            legend_rect: zero,
            title_block_rect: zero,
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
        let (w, h) = Self::preset_dimensions_mm(paper_size, orientation);
        sheet.reset_paper_mm(w, h);
        sheet
    }

    /// Preset dimensions in mm (`target/cpp-ref/src/app/KaPaperDialog.cpp:162-168`).
    #[must_use]
    pub const fn preset_dimensions_mm(
        paper_size: SheetPaperSize,
        orientation: SheetOrientation,
    ) -> (f64, f64) {
        if matches!(paper_size, SheetPaperSize::A4)
            && matches!(orientation, SheetOrientation::Landscape)
        {
            return (Self::FIELD_PAPER_WIDTH_MM, Self::FIELD_PAPER_HEIGHT_MM);
        }
        let (pw, ph) = paper_size.portrait_dimensions_mm();
        match orientation {
            SheetOrientation::Portrait => (pw, ph),
            SheetOrientation::Landscape => (ph, pw),
        }
    }

    /// `target/cpp-ref/src/app/KaDrawingStudio.cpp:432-435`
    #[must_use]
    pub fn is_legacy_a4_landscape_mm(width_mm: f64, height_mm: f64) -> bool {
        (width_mm - Self::A4_LANDSCAPE_WIDTH_MM).abs() < 0.51
            && (height_mm - Self::A4_LANDSCAPE_HEIGHT_MM).abs() < 0.51
    }

    /// `target/cpp-ref/src/app/KaDrawingStudio.cpp:437-441`
    pub fn apply_field_page_grow(&mut self) {
        if Self::is_legacy_a4_landscape_mm(self.width_mm, self.height_mm) {
            self.reset_paper_mm(Self::FIELD_PAPER_WIDTH_MM, Self::FIELD_PAPER_HEIGHT_MM);
        }
    }

    pub fn set_legend_kept_on_page(&mut self, kept_on_page: bool) {
        self.legend_kept_on_page = kept_on_page;
        self.legend_rect = if kept_on_page {
            default_legend_rect_mm(self.map_frame)
        } else {
            SheetRectMm { x_mm: self.width_mm + 15.0, y_mm: 10.0, width_mm: 48.0, height_mm: 74.0 }
        };
    }

    /// `KaDrawingStudio::resetPaper` (`target/cpp-ref/src/app/KaDrawingStudio.cpp:456-554`)
    pub fn change_paper(&mut self, paper_size: SheetPaperSize, orientation: SheetOrientation) {
        self.paper_size = paper_size;
        self.orientation = orientation;
        let (w, h) = Self::preset_dimensions_mm(paper_size, orientation);
        self.reset_paper_mm(w, h);
    }

    /// Resize sheet to arbitrary `(width_mm, height_mm)` while preserving existing layout items
    /// (`target/cpp-ref/src/app/KaDrawingStudio.cpp:456-554, 1350-1356`).
    pub fn reset_paper_mm(&mut self, width_mm: f64, height_mm: f64) {
        let (w, h) = (width_mm.max(20.0), height_mm.max(20.0));
        let (old_w, old_h) = (self.width_mm, self.height_mm);
        self.orientation = if w > h {
            SheetOrientation::Landscape
        } else {
            SheetOrientation::Portrait
        };
        if let Some(sz) = Self::match_known_paper_size(w, h) {
            self.paper_size = sz;
        }
        self.width_mm = w;
        self.height_mm = h;
        self.map_frame = default_map_rect_mm(w, h);
        if self.legend_kept_on_page {
            if old_w >= 20.0 && old_h >= 20.0 {
                let cx = self.legend_rect.x_mm + self.legend_rect.width_mm * 0.5;
                let cy = self.legend_rect.y_mm + self.legend_rect.height_mm * 0.5;
                if (-2.0..=old_w + 2.0).contains(&cx) && (-2.0..=old_h + 2.0).contains(&cy) {
                    if self.legend_rect.y_mm > old_h * 0.55 {
                        self.legend_rect.y_mm += h - old_h;
                    }
                    if self.legend_rect.x_mm > old_w * 0.5 {
                        self.legend_rect.x_mm += w - old_w;
                    }
                }
            } else {
                self.legend_rect = default_legend_rect_mm(self.map_frame);
            }
        } else {
            self.legend_rect.x_mm = w + 15.0;
        }
        self.replace_sheet_chrome();
        self.recover_orphaned_items();
    }

    fn match_known_paper_size(w: f64, h: f64) -> Option<SheetPaperSize> {
        if Self::is_legacy_a4_landscape_mm(w, h)
            || ((w - Self::FIELD_PAPER_WIDTH_MM).abs() < 0.51
                && (h - Self::FIELD_PAPER_HEIGHT_MM).abs() < 0.51)
        {
            return Some(SheetPaperSize::A4);
        }
        for sz in [
            SheetPaperSize::A4,
            SheetPaperSize::A3,
            SheetPaperSize::A2,
            SheetPaperSize::A1,
            SheetPaperSize::A0,
        ] {
            let (pw, ph) = sz.portrait_dimensions_mm();
            if ((w - pw).abs() < 0.51 && (h - ph).abs() < 0.51)
                || ((w - ph).abs() < 0.51 && (h - pw).abs() < 0.51)
            {
                return Some(sz);
            }
        }
        None
    }

    /// Places the bottom-strip chrome inside `[0..=w]x[0..=h]`. NOT a port: the numbers below (margin 12, band 22,
    /// north arrow 16, bar 55, crs 65) differ from `LayoutService::standardSheetChrome`
    /// (`target/cpp-ref/src/core/LayoutService.cpp:895-963`: gap 7, north 20, crs 58x7, bar width from the map
    /// width), which `standard_chrome()` below follows. The PDF takes these rectangles, the screen the others: P-G1b.
    pub fn replace_sheet_chrome(&mut self) {
        let (w, h) = (self.width_mm, self.height_mm);
        let (margin, bottom_band_h) = (12.0, 22.0);
        let band_y = (h - margin - bottom_band_h + 2.0).max(4.0);
        let north_w = 16.0_f64.min((w - 8.0).max(4.0));
        let north_x = (w - margin - 18.0).clamp(4.0, (w - north_w - 2.0).max(4.0));
        let bar_x = margin.min((w - 8.0).max(4.0));
        let bar_w = 55.0_f64.min((w - bar_x - 4.0).max(4.0));
        let crs_x = (margin + 60.0).min((north_x - 6.0).max(4.0));
        let crs_w = 65.0_f64.min((north_x - 2.0 - crs_x).max(4.0)).min((w - crs_x - 2.0).max(4.0));
        self.scale_bar_rect = SheetRectMm { x_mm: bar_x, y_mm: band_y, width_mm: bar_w, height_mm: 8.5 };
        self.scale_text_rect = SheetRectMm {
            x_mm: bar_x,
            y_mm: (band_y + 9.0).min((h - 7.5).max(4.0)),
            width_mm: bar_w,
            height_mm: 7.0,
        };
        self.crs_label_rect = SheetRectMm { x_mm: crs_x, y_mm: band_y, width_mm: crs_w, height_mm: 16.0 };
        self.north_arrow_rect = SheetRectMm { x_mm: north_x, y_mm: band_y, width_mm: north_w, height_mm: 16.0 };
        let page = SheetRectMm { x_mm: 0.0, y_mm: 0.0, width_mm: w, height_mm: h };
        self.title_block_rect = default_title_block_rect_mm(page, self.map_frame);
    }

    /// C++ `LayoutService::standardSheetChrome(page, defaultMapRect())` (`LayoutService.cpp:895`) 결과 반환
    #[must_use]
    pub fn standard_chrome(&self) -> SheetChromeRects {
        let page = SheetRectMm { x_mm: 0.0, y_mm: 0.0, width_mm: self.width_mm, height_mm: self.height_mm };
        standard_sheet_chrome(page, default_map_rect_mm(self.width_mm, self.height_mm))
    }

    /// C++ `KaTitleBlock::defaultRect(page, mapRect)` (`KaTitleBlock.cpp:67`) 결과 반환
    #[must_use]
    pub fn default_title_block_rect(&self) -> SheetRectMm {
        let page = SheetRectMm { x_mm: 0.0, y_mm: 0.0, width_mm: self.width_mm, height_mm: self.height_mm };
        default_title_block_rect_mm(page, self.standard_chrome().map)
    }

    /// `KaDrawingStudio.cpp:534-538` — 용지 밖으로 밀려난 범례·표제란을 `clamp(pos, 4.0, max(4.0, page - size - 4.0))`로 복구
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

    #[must_use]
    pub fn all_decorations_inside_sheet(&self) -> bool {
        let (w, h) = (self.width_mm, self.height_mm);
        let in_frame = self.map_frame.inside_sheet(w, h)
            && self.scale_bar_rect.inside_sheet(w, h)
            && self.scale_text_rect.inside_sheet(w, h)
            && self.crs_label_rect.inside_sheet(w, h)
            && self.north_arrow_rect.inside_sheet(w, h)
            && self.title_block_rect.inside_sheet(w, h);
        if self.legend_kept_on_page {
            in_frame && self.legend_rect.inside_sheet(w, h)
        } else {
            in_frame
        }
    }
}

#[cfg(test)]
#[path = "sheet_drawing_tests.rs"]
mod tests;
