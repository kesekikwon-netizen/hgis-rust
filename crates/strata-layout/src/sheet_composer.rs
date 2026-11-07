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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SheetOrientation {
    Portrait,
    Landscape,
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
    pub crs_label_rect: SheetRectMm,
    pub north_arrow_rect: SheetRectMm,
    pub legend_rect: SheetRectMm,
    pub legend_kept_on_page: bool,
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
            crs_label_rect: SheetRectMm { x_mm: 0.0, y_mm: 0.0, width_mm: 0.0, height_mm: 0.0 },
            north_arrow_rect: SheetRectMm { x_mm: 0.0, y_mm: 0.0, width_mm: 0.0, height_mm: 0.0 },
            legend_rect: SheetRectMm { x_mm: 0.0, y_mm: 0.0, width_mm: 0.0, height_mm: 0.0 },
            legend_kept_on_page: true,
        };
        sheet.recompute_layout();
        sheet
    }

    pub fn set_legend_kept_on_page(&mut self, kept_on_page: bool) {
        self.legend_kept_on_page = kept_on_page;
        self.recompute_layout();
    }

    pub fn change_paper(&mut self, paper_size: SheetPaperSize, orientation: SheetOrientation) {
        self.paper_size = paper_size;
        self.orientation = orientation;
        self.recompute_layout();
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
        self.scale_bar_rect = SheetRectMm {
            x_mm: margin,
            y_mm: band_y,
            width_mm: 55.0,
            height_mm: 16.0,
        };
        self.crs_label_rect = SheetRectMm {
            x_mm: margin + 60.0,
            y_mm: band_y,
            width_mm: 65.0,
            height_mm: 16.0,
        };
        self.north_arrow_rect = SheetRectMm {
            x_mm: w - margin - 20.0,
            y_mm: margin + 4.0,
            width_mm: 16.0,
            height_mm: 22.0,
        };
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
}
