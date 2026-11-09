//! 용지 크기·방향·사각형과 기본 레이아웃 기하.
//!
//! `sheet_composer` 에서 분리했다.

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
