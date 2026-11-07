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
    pub fn inside_sheet(&self, sheet_w: f64, sheet_h: f64) -> bool {
        self.x_mm >= 0.0
            && self.y_mm >= 0.0
            && (self.x_mm + self.width_mm) <= sheet_w
            && (self.y_mm + self.height_mm) <= sheet_h
    }
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
    pub legend_kept_on_page: bool,
    pub scale_denominator: u32,
    pub grid_enabled: bool,
    pub grid_interval_m: f64,
    pub grid_follow_map: bool,
    pub grid_offset_xy_m: [f64; 2],
    pub show_site_names_under_5000: bool,
}

impl DrawingSheet {
    #[must_use]
    pub fn new(paper_size: SheetPaperSize, orientation: SheetOrientation) -> Self {
        let mut sheet = Self {
            paper_size,
            orientation,
            width_mm: 0.0,
            height_mm: 0.0,
            map_frame: SheetRectMm { x_mm: 0.0, y_mm: 0.0, width_mm: 0.0, height_mm: 0.0 },
            scale_bar_rect: SheetRectMm { x_mm: 0.0, y_mm: 0.0, width_mm: 0.0, height_mm: 0.0 },
            scale_text_rect: SheetRectMm { x_mm: 0.0, y_mm: 0.0, width_mm: 0.0, height_mm: 0.0 },
            crs_label_rect: SheetRectMm { x_mm: 0.0, y_mm: 0.0, width_mm: 0.0, height_mm: 0.0 },
            north_arrow_rect: SheetRectMm { x_mm: 0.0, y_mm: 0.0, width_mm: 0.0, height_mm: 0.0 },
            legend_rect: SheetRectMm { x_mm: 0.0, y_mm: 0.0, width_mm: 0.0, height_mm: 0.0 },
            legend_kept_on_page: true,
            scale_denominator: 5_000,
            grid_enabled: true,
            grid_interval_m: 100.0,
            grid_follow_map: false,
            grid_offset_xy_m: [0.0, 0.0],
            show_site_names_under_5000: false,
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
    }

    /// `KaDrawingStudio.cpp` (`c9e58e3`) — A3 -> A4 전환 등에서 용지 밖으로 밀려난 범례/장식을 `clamp(pos, 4.0, max(4.0, page - size - 4.0))`로 복구
    pub fn recover_orphaned_items(&mut self) {
        if self.legend_kept_on_page {
            let max_x = (self.width_mm - self.legend_rect.width_mm - 4.0).max(4.0);
            let max_y = (self.height_mm - self.legend_rect.height_mm - 4.0).max(4.0);
            self.legend_rect.x_mm = self.legend_rect.x_mm.clamp(4.0, max_x);
            self.legend_rect.y_mm = self.legend_rect.y_mm.clamp(4.0, max_y);
        }
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

        let margin = 12.0;
        let bottom_band_h = 22.0;
        self.map_frame = SheetRectMm {
            x_mm: margin,
            y_mm: margin,
            width_mm: (w - margin * 2.0).max(50.0),
            height_mm: (h - margin * 2.0 - bottom_band_h).max(50.0),
        };
        let band_y = h - margin - bottom_band_h + 2.0;
        self.replace_sheet_chrome();
        self.legend_rect = if self.legend_kept_on_page {
            SheetRectMm {
                x_mm: w - margin - 52.0,
                y_mm: band_y - 44.0,
                width_mm: 48.0,
                height_mm: 40.0,
            }
        } else {
            SheetRectMm {
                x_mm: w + 15.0,
                y_mm: margin,
                width_mm: 48.0,
                height_mm: 40.0,
            }
        };
    }

    #[must_use]
    pub fn all_decorations_inside_sheet(&self) -> bool {
        let in_frame = self.map_frame.inside_sheet(self.width_mm, self.height_mm)
            && self.scale_bar_rect.inside_sheet(self.width_mm, self.height_mm)
            && self.scale_text_rect.inside_sheet(self.width_mm, self.height_mm)
            && self.crs_label_rect.inside_sheet(self.width_mm, self.height_mm)
            && self.north_arrow_rect.inside_sheet(self.width_mm, self.height_mm);
        if self.legend_kept_on_page {
            in_frame && self.legend_rect.inside_sheet(self.width_mm, self.height_mm)
        } else {
            in_frame
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paper_size_and_orientation_change_keeps_decorations_in_bounds_and_preserves_off_page_legend() {
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
        let expected_band_y = sheet.height_mm - 12.0 - 22.0 + 2.0;
        assert!((sheet.scale_bar_rect.y_mm - expected_band_y).abs() < 1e-6);
        assert!((sheet.crs_label_rect.y_mm - expected_band_y).abs() < 1e-6);
        assert!((sheet.north_arrow_rect.y_mm - expected_band_y).abs() < 1e-6);
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
}
