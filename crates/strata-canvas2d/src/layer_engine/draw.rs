//! 레이어를 화면에 그리기 위한 **중립 명령** 목록.
//!
//! C++ 은 QGIS 의 `QgsSymbol` 이 직접 QPainter 로 그린다. Rust 엔진은 egui 를
//! 모르게 두고, "무엇을 어디에 어떤 색으로" 만 정해서 넘긴다. 그러면
//! 같은 엔진을 egui·PDF·SHP 어디에도 쓸 수 있다.
//!
//! `strata-canvas2d` 는 `egui` 에 의존하지 않는다 — 이 모듈이 그 경계다.

use super::style::DomainStyle;

/// 그릴 도형 하나.
#[derive(Debug, Clone, PartialEq)]
pub enum DrawShape {
    /// 채운 면. `pts` 는 닫힌 링.
    Polygon {
        pts: Vec<[f32; 2]>,
        fill: [u8; 4],
        stroke: [u8; 4],
        stroke_width_px: f32,
    },
    /// 선.
    Line {
        pts: Vec<[f32; 2]>,
        color: [u8; 4],
        width_px: f32,
    },
    /// 점(원).
    Point {
        at: [f32; 2],
        radius_px: f32,
        fill: [u8; 4],
        stroke: [u8; 4],
    },
    /// 글자.
    Label {
        at: [f32; 2],
        text: String,
        size_px: f32,
        color: [u8; 4],
    },
}

/// 한 레이어를 그린 결과.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerDraw {
    /// 어느 레이어인지(디버그·선택 판정용).
    pub key: String,
    /// 그릴 도형들(그린 순서대로).
    pub shapes: Vec<DrawShape>,
}

impl LayerDraw {
    /// 빈 결과.
    #[must_use]
    pub fn new(key: &str) -> Self {
        Self {
            key: key.to_owned(),
            shapes: Vec::new(),
        }
    }

    /// 도형 수.
    #[must_use]
    pub fn len(&self) -> usize {
        self.shapes.len()
    }

    /// 비었는지.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.shapes.is_empty()
    }
}

/// 화면 변환 — 지도 좌표(work CRS 미터)를 화면 픽셀로.
///
/// C++ 은 `QgsMapCanvas` 가 이 일을 한다. 여기서는 최소값만 둔다.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenTransform {
    /// 화면 좌상단이 나타내는 지도 좌표(미터).
    pub origin: [f64; 2],
    /// 화면 1px 당 미터.
    pub meters_per_px: f64,
    /// 그리는 영역의 화면 좌상단(px).
    pub screen_origin: [f32; 2],
}

impl ScreenTransform {
    /// 만든다. `meters_per_px` 가 0 이하이면 1 로 둔다(0 나누기 방지).
    #[must_use]
    pub fn new(origin: [f64; 2], meters_per_px: f64, screen_origin: [f32; 2]) -> Self {
        Self {
            origin,
            meters_per_px: if meters_per_px > 0.0 && meters_per_px.is_finite() {
                meters_per_px
            } else {
                1.0
            },
            screen_origin,
        }
    }

    /// 지도 좌표 → 화면 좌표. 화면 y 는 아래로 증가하므로 부호를 뒤집는다.
    #[must_use]
    pub fn world_to_screen(&self, w: [f64; 2]) -> [f32; 2] {
        let dx = (w[0] - self.origin[0]) / self.meters_per_px;
        let dy = (self.origin[1] - w[1]) / self.meters_per_px;
        [
            self.screen_origin[0] + dx as f32,
            self.screen_origin[1] + dy as f32,
        ]
    }

    /// 화면 좌표 → 지도 좌표(선택 판정에 쓴다).
    #[must_use]
    pub fn screen_to_world(&self, s: [f32; 2]) -> [f64; 2] {
        let dx = f64::from(s[0] - self.screen_origin[0]) * self.meters_per_px;
        let dy = f64::from(s[1] - self.screen_origin[1]) * self.meters_per_px;
        [self.origin[0] + dx, self.origin[1] - dy]
    }
}

/// mm 를 화면 px 로 바꾼다.
///
/// C++ QGIS 는 심볼 굵기를 mm 로 들고 화면 DPI 로 환산한다. 화면에서는
/// 1mm ≈ 3.78px(96dpi) 이므로 그 값을 쓴다. `pixels_per_point`(OS 배율)는
/// egui 가 따로 처리하므로 여기서는 곱하지 않는다.
#[must_use]
pub fn mm_to_px(mm: f32) -> f32 {
    mm * 3.779_527_6
}

/// 스타일에서 폴리곤 도형을 만든다.
#[must_use]
pub fn polygon_shape(pts: Vec<[f32; 2]>, style: &DomainStyle) -> DrawShape {
    DrawShape::Polygon {
        pts,
        fill: if style.fills() { style.fill } else { [0, 0, 0, 0] },
        stroke: style.stroke,
        stroke_width_px: mm_to_px(style.stroke_width_mm),
    }
}

/// 스타일에서 선 도형을 만든다.
#[must_use]
pub fn line_shape(pts: Vec<[f32; 2]>, style: &DomainStyle) -> DrawShape {
    DrawShape::Line {
        pts,
        color: style.stroke,
        width_px: mm_to_px(style.stroke_width_mm),
    }
}

/// 스타일에서 점 도형을 만든다.
#[must_use]
pub fn point_shape(at: [f32; 2], style: &DomainStyle) -> DrawShape {
    DrawShape::Point {
        at,
        radius_px: mm_to_px(style.marker_size_mm) * 0.5,
        fill: style.fill,
        stroke: style.stroke,
    }
}

/// 스타일에서 케이싱(흰 테두리) 선을 만든다. `section_line` 에 쓴다.
#[must_use]
pub fn casing_shape(pts: Vec<[f32; 2]>, style: &DomainStyle) -> DrawShape {
    DrawShape::Line {
        pts,
        color: style.casing_color,
        width_px: mm_to_px(style.casing_width_mm),
    }
}
